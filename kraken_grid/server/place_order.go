package server

import (
	"context"
	"fmt"

	"github.com/dafsic/toad/kraken_grid/bot"
	pb "github.com/dafsic/toad/proto_go/kraken_grid"
	"go.uber.org/zap"
)

// PlaceOrder implements the order placement service, calling the API to place an order
func (s *server) PlaceOrder(ctx context.Context, req *pb.PlaceOrderRequest) (*pb.Response, error) {
	s.logger.Info("Received order request",
		zap.String("side", req.Side),
		zap.Int32("multiplier", req.Multiplier))

	if req.Side != bot.OrderBuy && req.Side != bot.OrderSell {
		s.logger.Error("Invalid order side", zap.String("side", req.Side))
		return &pb.Response{
			Success: false,
			Message: fmt.Sprintf("Invalid order side: %s", req.Side),
		}, nil
	}

	if req.Multiplier <= 0 {
		s.logger.Error("Invalid multiplier", zap.Int32("multiplier", req.Multiplier))
		return &pb.Response{
			Success: false,
			Message: "Invalid multiplier: must be positive",
		}, nil
	}

	if req.Price <= 0 {
		s.logger.Error("Invalid price", zap.Float64("price", req.Price))
		return &pb.Response{
			Success: false,
			Message: "Invalid price: must be positive",
		}, nil
	}

	err := s.api.PlaceOrder(req.Side, int(req.Multiplier), req.Price)
	if err != nil {
		s.logger.Error("Failed to place order",
			zap.String("side", req.Side),
			zap.Int32("multiplier", req.Multiplier),
			zap.Error(err))
		return &pb.Response{
			Success: false,
			Message: fmt.Sprintf("Failed to place order: %v", err),
		}, nil
	}

	s.logger.Info("Order placed successfully",
		zap.String("side", req.Side),
		zap.Int32("multiplier", req.Multiplier))

	return &pb.Response{
		Success: true,
		Message: "Order placed successfully",
	}, nil
}
