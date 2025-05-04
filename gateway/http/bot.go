package http

import (
	"net/http"

	pb "github.com/dafsic/toad/proto_go/kraken_grid"
	"github.com/gin-gonic/gin"
	"go.uber.org/zap"
)

type BotStopRequest struct {
	Bot    string `json:"bot" binding:"required"`
	Reason string `json:"reason" binding:"required"`
}

type BotStopResponse struct {
	Success bool   `json:"success"`
	Message string `json:"message"`
}

func (s *GinServer) botStop(c *gin.Context) {
	var req BotStopRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		s.logger.Error("Invalid request body", zap.Error(err))
		c.JSON(http.StatusBadRequest, BotStopResponse{
			Success: false,
			Message: "Invalid request: " + err.Error(),
		})
		return
	}

	client, ok := s.clients[req.Bot]
	if !ok {
		c.JSON(http.StatusBadRequest, BotStopResponse{
			Success: false,
			Message: "Bot not found",
		})
		return
	}

	s.logger.Info("Stopping bot", zap.String("bot", req.Bot))

	resp, err := client.Stop(c.Request.Context(), &pb.StopRequest{Reason: req.Reason})
	if err != nil {
		s.logger.Error("Failed to stop bot", zap.String("bot", req.Bot), zap.Error(err))
		c.JSON(http.StatusInternalServerError, BotStopResponse{
			Success: false,
			Message: "Failed to stop bot: " + err.Error(),
		})
		return
	}

	c.JSON(http.StatusOK, BotStopResponse{
		Success: resp.Success,
		Message: resp.Message,
	})
}

type RunBotRequest struct {
	Bot string `json:"bot" binding:"required"`
}

type RunBotResponse struct {
	Success bool   `json:"success"`
	Message string `json:"message"`
}

func (s *GinServer) botRun(c *gin.Context) {
	var req RunBotRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		s.logger.Error("Invalid request body", zap.Error(err))
		c.JSON(http.StatusBadRequest, RunBotResponse{
			Success: false,
			Message: "Invalid request: " + err.Error(),
		})
		return
	}

	client, ok := s.clients[req.Bot]
	if !ok {
		c.JSON(http.StatusBadRequest, RunBotResponse{
			Success: false,
			Message: "Bot not found",
		})
		return
	}

	s.logger.Info("Running bot", zap.String("bot", req.Bot))

	resp, err := client.Run(c.Request.Context(), &pb.RunRequest{})
	if err != nil {
		s.logger.Error("Failed to run bot", zap.String("bot", req.Bot), zap.Error(err))
		c.JSON(http.StatusInternalServerError, RunBotResponse{
			Success: false,
			Message: "Failed to run bot: " + err.Error(),
		})
		return
	}

	c.JSON(http.StatusOK, RunBotResponse{
		Success: resp.Success,
		Message: resp.Message,
	})
}

type BotStatusRequest struct {
	Bot string `json:"bot" binding:"required"`
}
type BotStatusResponse struct {
	Status  string `json:"status"`
	Message string `json:"message"`
}

func (s *GinServer) botStatus(c *gin.Context) {
	var req BotStatusRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		s.logger.Error("Invalid request body", zap.Error(err))
		c.JSON(http.StatusBadRequest, BotStatusResponse{
			Status:  "error",
			Message: "Invalid request: " + err.Error(),
		})
		return
	}

	client, ok := s.clients[req.Bot]
	if !ok {
		c.JSON(http.StatusBadRequest, BotStatusResponse{
			Status:  "error",
			Message: "Bot not found",
		})
		return
	}

	s.logger.Info("Getting status of bot", zap.String("bot", req.Bot))

	resp, err := client.Status(c.Request.Context(), &pb.StatusRequest{})
	if err != nil {
		s.logger.Error("Failed to get status of bot", zap.String("bot", req.Bot), zap.Error(err))
		c.JSON(http.StatusInternalServerError, BotStatusResponse{
			Status:  "error",
			Message: "Failed to get status of bot: " + err.Error(),
		})
		return
	}

	c.JSON(http.StatusOK, BotStatusResponse{
		Status:  resp.Status,
		Message: resp.Message,
	})
}
