package server

import (
	"context"
	"fmt"

	pb "github.com/dafsic/toad/proto_go/kraken_grid"
	"go.uber.org/zap"
)

// SetBasePrice implements the base price setting service, calling the API to set a base price
func (s *server) SetBasePrice(ctx context.Context, req *pb.SetBasePriceRequest) (*pb.Response, error) {
	s.logger.Info("Received base price request",
		zap.Float64("price", req.BasePrice))

	if req.BasePrice <= 0 {
		s.logger.Error("Invalid base price", zap.Float64("price", req.BasePrice))
		return &pb.Response{
			Success: false,
			Message: fmt.Sprintf("Invalid base price: %f", req.BasePrice),
		}, nil
	}

	s.api.SetBasePrice(req.BasePrice)

	s.logger.Info("Base price set successfully",
		zap.Float64("price", req.BasePrice))

	return &pb.Response{
		Success: true,
		Message: "Base price set successfully",
	}, nil
}
