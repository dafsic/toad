package server

import (
	"context"

	"github.com/dafsic/toad/kraken_grid/api"
	pb "github.com/dafsic/toad/proto_go/kraken_grid"
	"go.uber.org/zap"
)

// Server defines the server interface
type Server interface {
	PlaceOrder(ctx context.Context, req *pb.PlaceOrderRequest) (*pb.PlaceOrderResponse, error)
	SetBasePrice(ctx context.Context, req *pb.SetBasePriceRequest) (*pb.SetBasePriceResponse, error)
}

// server implements the Server interface
type server struct {
	api    api.API
	logger *zap.Logger
}

// NewServer creates a new service instance
func NewServer(api api.API, logger *zap.Logger) Server {
	return &server{
		api:    api,
		logger: logger,
	}
}
