package client

import (
	"context"

	pb "github.com/dafsic/toad/proto_go/kraken_grid"
)

type GrpcClient interface {
	Name() string
	Connect() error
	Close() error
	Stop(ctx context.Context, req *pb.StopRequest) (*pb.Response, error)
	Run(ctx context.Context, req *pb.RunRequest) (*pb.Response, error)
	Status(ctx context.Context, req *pb.StatusRequest) (*pb.StatusResponse, error)
	PlaceOrder(ctx context.Context, req *pb.PlaceOrderRequest) (*pb.Response, error)
}
