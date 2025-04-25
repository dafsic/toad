package server

import (
	"context"

	"github.com/dafsic/toad/kraken_grid/api"
	pb "github.com/dafsic/toad/proto_go/kraken_grid"
	"go.uber.org/zap"
)

// Server defines the server interface
type Server interface {
	Stop(ctx context.Context, req *pb.StopRequest) (*pb.Response, error)
	Run(ctx context.Context, req *pb.RunRequest) (*pb.Response, error)
	Status(ctx context.Context, req *pb.StatusRequest) (*pb.StatusResponse, error)
	PlaceOrder(ctx context.Context, req *pb.PlaceOrderRequest) (*pb.Response, error)
	SetBasePrice(ctx context.Context, req *pb.SetBasePriceRequest) (*pb.Response, error)
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
