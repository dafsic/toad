package server

import (
	"context"
	"fmt"

	pb "github.com/dafsic/toad/proto_go/kraken_grid"
	"go.uber.org/zap"
)

func (s *server) Stop(ctx context.Context, req *pb.StopRequest) (*pb.Response, error) {
	s.api.Stop(req.Reason)

	return &pb.Response{
		Success: true,
		Message: "Bot stopped successfully",
	}, nil
}

func (s *server) Run(ctx context.Context, req *pb.RunRequest) (*pb.Response, error) {
	err := s.api.Run()
	if err != nil {
		s.logger.Error("Failed to run bot", zap.Error(err))
		return &pb.Response{
			Success: false,
			Message: fmt.Sprintf("Failed to run bot: %v", err),
		}, nil
	}

	return &pb.Response{
		Success: true,
		Message: "Bot started successfully",
	}, nil
}

func (s *server) Status(ctx context.Context, req *pb.StatusRequest) (*pb.StatusResponse, error) {
	status, message := s.api.Status()

	return &pb.StatusResponse{
		Status:  status,
		Message: message,
	}, nil
}
