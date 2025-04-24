package client

import "context"

type GrpcClient interface {
	Name() string
	Connect() error
	Close() error
	PlaceOrder(ctx context.Context, side string, multiplier int32, price float64) (bool, string, error)
}
