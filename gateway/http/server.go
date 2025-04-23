package http

import (
	"context"
	"net"
	"net/http"

	"github.com/dafsic/toad/gateway/client"
	"github.com/dafsic/toad/gateway/http/middlewares"
	"github.com/gin-gonic/gin"
	"go.uber.org/zap"
)

// Server defines the HTTP server interface
type Server interface {
	Run(addr string) error
	Close() error
}

// GinServer implements an HTTP server using the Gin framework
type GinServer struct {
	router  *gin.Engine
	logger  *zap.Logger
	srv     *http.Server
	clients map[string]client.GrpcClient
}

// NewGinServer creates a new Gin HTTP server
func NewGinServer(logger *zap.Logger, clients ...client.GrpcClient) *GinServer {
	router := gin.New()
	router.Use(gin.Recovery())

	// Use custom Logger middleware
	router.Use(middlewares.Logger(logger))
	router.Use(middlewares.CORS())

	server := &GinServer{
		router:  router,
		logger:  logger,
		clients: make(map[string]client.GrpcClient),
	}

	for _, client := range clients {
		server.clients[client.Name()] = client
	}

	// Initialize routes
	server.setupRoutes()

	return server
}

// setupRoutes configures HTTP routes
func (s *GinServer) setupRoutes() {
	// Add health check route
	s.router.GET("/ping", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{
			"message": "pong",
		})
	})

	// Create API group
	api := s.router.Group("/api/v1")
	{
		// Kraken Grid trading API
		krakenGrid := api.Group("/kraken-grid")
		{
			krakenGrid.POST("/orders", s.placeOrder)
		}
	}
}

// Run starts the HTTP server
func (s *GinServer) Run(addr string) error {
	for _, client := range s.clients {
		if err := client.Connect(); err != nil {
			s.logger.Error("Failed to connect to gRPC client", zap.Error(err))
			return err
		}
	}

	s.logger.Info("Starting HTTP server", zap.String("address", addr))
	s.srv = &http.Server{
		Addr:    addr,
		Handler: s.router,
	}

	ln, err := net.Listen("tcp", s.srv.Addr) // the web server starts listening
	if err != nil {
		s.logger.Error("Failed to listen", zap.String("address", addr), zap.Error(err))
		return err
	}

	go func() {
		if s.srv.Serve(ln); err != nil {
			s.logger.Error("HTTP server error", zap.Error(err))
		}
	}()
	return nil
}

// Close shuts down the HTTP server
func (s *GinServer) Close() error {
	if s.srv != nil {
		s.logger.Info("Shutting down HTTP server")
		return s.srv.Shutdown(context.Background())
	}

	for _, client := range s.clients {
		if err := client.Close(); err != nil {
			s.logger.Error("Error closing gRPC client", zap.Error(err))
		}
	}
	return nil
}
