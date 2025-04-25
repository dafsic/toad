package http

import (
	"net/http"

	"github.com/gin-gonic/gin"
	"go.uber.org/zap"
)

// PlaceOrderRequest is the structure for order request in HTTP requests
type PlaceOrderRequest struct {
	Bot        string  `json:"bot" binding:"required"`
	Side       string  `json:"side" binding:"required"`
	Multiplier int32   `json:"multiplier" binding:"required"`
	Price      float64 `json:"price" binding:"required"`
}

// PlaceOrderResponse is the structure for order results in HTTP responses
type PlaceOrderResponse struct {
	Success bool   `json:"success"`
	Message string `json:"message"`
}

// placeOrder handles order requests, forwarding them to the gRPC service
func (s *GinServer) placeOrder(c *gin.Context) {
	var req PlaceOrderRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		s.logger.Error("Invalid request body", zap.Error(err))
		c.JSON(http.StatusBadRequest, PlaceOrderResponse{
			Success: false,
			Message: "Invalid request: " + err.Error(),
		})
		return
	}

	s.logger.Info("Processing order request",
		zap.String("side", req.Side),
		zap.Int32("multiplier", req.Multiplier),
		zap.Float64("price", req.Price),
	)

	// Validate request parameters
	if req.Side != "buy" && req.Side != "sell" {
		c.JSON(http.StatusBadRequest, PlaceOrderResponse{
			Success: false,
			Message: "Invalid side: must be 'buy' or 'sell'",
		})
		return
	}

	if req.Multiplier <= 0 {
		c.JSON(http.StatusBadRequest, PlaceOrderResponse{
			Success: false,
			Message: "Invalid multiplier: must be positive",
		})
		return
	}

	client, ok := s.clients[req.Bot]
	if !ok {
		c.JSON(http.StatusBadRequest, PlaceOrderResponse{
			Success: false,
			Message: "Invalid bot name: " + req.Bot,
		})
		return
	}

	success, message, err := client.PlaceOrder(c.Request.Context(), req.Side, req.Multiplier, req.Price)
	if err != nil {
		s.logger.Error("gRPC client error", zap.Error(err))
		c.JSON(http.StatusInternalServerError, PlaceOrderResponse{
			Success: false,
			Message: "Internal server error: " + err.Error(),
		})
		return
	}

	// Construct response
	response := PlaceOrderResponse{
		Success: success,
		Message: message,
	}

	c.JSON(http.StatusOK, response)
}
