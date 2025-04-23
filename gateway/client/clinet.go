package client

import "context"

type GrpcClient interface {
	Name() string
	Connect() error
	Close() error
	PlaceOrder(ctx context.Context, side string, multiplier int32) (bool, string, error)
}
