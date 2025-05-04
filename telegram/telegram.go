// Package telegram provides a Telegram bot client that communicates with kraken_grid service via gRPC
package telegram

import (
	"context"
	"fmt"
	"strconv"
	"strings"
	"time"

	tgbotapi "github.com/go-telegram-bot-api/telegram-bot-api/v5"
	"go.uber.org/zap"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"

	pb "github.com/dafsic/toad/proto_go/kraken_grid"
	"slices"
)

// Config holds the configuration for the Telegram bot
type Config struct {
	BotToken      string  // Telegram bot API token
	GrpcServerURL string  // URL of the gRPC server
	AllowedUsers  []int64 // List of allowed Telegram user IDs (optional)
}

// BotService represents the Telegram bot service
type BotService struct {
	bot        *tgbotapi.BotAPI
	grpcClient pb.KrakenGridServiceClient
	config     Config
	logger     *zap.Logger
	commands   map[string]commandHandler
	conn       *grpc.ClientConn
}

type commandHandler func(update tgbotapi.Update) string

// NewBotService creates a new Telegram bot service
func NewBotService(config Config, logger *zap.Logger) (*BotService, error) {
	// Initialize Telegram bot
	bot, err := tgbotapi.NewBotAPI(config.BotToken)
	if err != nil {
		return nil, fmt.Errorf("failed to create Telegram bot: %w", err)
	}

	// Connect to gRPC service
	conn, err := grpc.NewClient(config.GrpcServerURL, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		return nil, fmt.Errorf("failed to connect to gRPC server: %w", err)
	}

	// Create gRPC client
	grpcClient := pb.NewKrakenGridServiceClient(conn)

	service := &BotService{
		bot:        bot,
		grpcClient: grpcClient,
		config:     config,
		logger:     logger,
		conn:       conn,
	}

	// Register command handlers
	service.commands = map[string]commandHandler{
		"status": service.handleStatusCommand,
		"run":    service.handleRunCommand,
		"stop":   service.handleStopCommand,
		"order":  service.handleOrderCommand,
		"help":   service.handleHelpCommand,
		"start":  service.handleStartCommand,
	}

	logger.Info("Telegram bot created successfully", zap.String("bot_name", bot.Self.UserName))
	return service, nil
}

// Close closes the gRPC connection
func (s *BotService) Close() {
	if s.conn != nil {
		s.conn.Close()
	}
}

// Start starts the bot and begins listening for messages
func (s *BotService) Start() error {
	s.logger.Info("Starting Telegram bot")

	u := tgbotapi.NewUpdate(0)
	u.Timeout = 60

	updates := s.bot.GetUpdatesChan(u)

	for update := range updates {
		if update.Message == nil {
			continue
		}

		// Check user permissions
		if !s.isUserAllowed(update.Message.From.ID) {
			msg := tgbotapi.NewMessage(update.Message.Chat.ID,
				"You don't have permission to use this bot.")
			s.bot.Send(msg)
			continue
		}

		// Handle commands
		if update.Message.IsCommand() {
			command := update.Message.Command()
			handler, exists := s.commands[command]

			if exists {
				// Create goroutine to handle command processing
				go func(update tgbotapi.Update, handler commandHandler) {
					response := handler(update)
					msg := tgbotapi.NewMessage(update.Message.Chat.ID, response)
					s.bot.Send(msg)
				}(update, handler)
			} else {
				msg := tgbotapi.NewMessage(update.Message.Chat.ID, "Unknown command. Send /help for available commands.")
				s.bot.Send(msg)
			}
		}
	}

	return nil
}

// Check if user is allowed to use the bot
func (s *BotService) isUserAllowed(userID int64) bool {
	// If no allowed users specified, allow all users
	if len(s.config.AllowedUsers) == 0 {
		return true
	}

	if slices.Contains(s.config.AllowedUsers, userID) {
		return true
	}
	s.logger.Warn("Unauthorized access attempt", zap.Int64("user_id", userID))
	return false
}

// Handle /status command
func (s *BotService) handleStatusCommand(update tgbotapi.Update) string {
	s.logger.Info("Processing status command",
		zap.Int64("user_id", update.Message.From.ID),
		zap.String("username", update.Message.From.UserName))

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	req := &pb.StatusRequest{
		RequestId: fmt.Sprintf("tg_%d_%d", update.Message.From.ID, time.Now().Unix()),
	}

	resp, err := s.grpcClient.Status(ctx, req)
	if err != nil {
		s.logger.Error("Status gRPC call failed", zap.Error(err))
		return "Failed to get status: " + err.Error()
	}

	return fmt.Sprintf("System status: %s\nMessage: %s", resp.Status, resp.Message)
}

// Handle /run command
func (s *BotService) handleRunCommand(update tgbotapi.Update) string {
	s.logger.Info("Processing run command",
		zap.Int64("user_id", update.Message.From.ID),
		zap.String("username", update.Message.From.UserName))

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	req := &pb.RunRequest{
		RequestId: fmt.Sprintf("tg_%d_%d", update.Message.From.ID, time.Now().Unix()),
	}

	resp, err := s.grpcClient.Run(ctx, req)
	if err != nil {
		s.logger.Error("Run gRPC call failed", zap.Error(err))
		return "Failed to start system: " + err.Error()
	}

	if resp.Success {
		return "System started successfully"
	}
	return "Failed to start: " + resp.Message
}

// Handle /stop command
func (s *BotService) handleStopCommand(update tgbotapi.Update) string {
	args := strings.Fields(update.Message.CommandArguments())

	reason := "Stopped via Telegram"
	if len(args) > 0 {
		reason = strings.Join(args, " ")
	}

	s.logger.Info("Processing stop command",
		zap.Int64("user_id", update.Message.From.ID),
		zap.String("username", update.Message.From.UserName),
		zap.String("reason", reason))

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	req := &pb.StopRequest{
		Reason: reason,
	}

	resp, err := s.grpcClient.Stop(ctx, req)
	if err != nil {
		s.logger.Error("Stop gRPC call failed", zap.Error(err))
		return "Failed to stop system: " + err.Error()
	}

	if resp.Success {
		return "System stopped successfully"
	}
	return "Failed to stop: " + resp.Message
}

// Handle /order command
func (s *BotService) handleOrderCommand(update tgbotapi.Update) string {
	args := strings.Fields(update.Message.CommandArguments())

	if len(args) < 3 {
		return "Insufficient parameters. Usage: /order <buy|sell> <multiplier> <price>"
	}

	side := args[0]
	if side != "buy" && side != "sell" {
		return "Invalid order side. Must be 'buy' or 'sell'"
	}

	multiplier, err := strconv.ParseInt(args[1], 10, 32)
	if err != nil {
		return "Invalid multiplier: " + err.Error()
	}

	price, err := strconv.ParseFloat(args[2], 64)
	if err != nil {
		return "Invalid price: " + err.Error()
	}

	s.logger.Info("Processing order command",
		zap.Int64("user_id", update.Message.From.ID),
		zap.String("username", update.Message.From.UserName),
		zap.String("side", side),
		zap.Int64("multiplier", multiplier),
		zap.Float64("price", price))

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	req := &pb.PlaceOrderRequest{
		Side:       side,
		Multiplier: int32(multiplier),
		Price:      price,
	}

	resp, err := s.grpcClient.PlaceOrder(ctx, req)
	if err != nil {
		s.logger.Error("PlaceOrder gRPC call failed", zap.Error(err))
		return "Failed to place order: " + err.Error()
	}

	if resp.Success {
		return "Order placed successfully"
	}
	return "Failed to place order: " + resp.Message
}

// Handle /help command
func (s *BotService) handleHelpCommand(update tgbotapi.Update) string {
	return `Available commands:
/status - Check system status
/run - Start the system
/stop [reason] - Stop the system with optional reason
/order <buy|sell> <multiplier> <price> - Place an order
/help - Show this help message`
}

// Handle /start command
func (s *BotService) handleStartCommand(update tgbotapi.Update) string {
	return fmt.Sprintf("Welcome %s!\n\nThis bot allows you to control your trading system.\nSend /help to see available commands.",
		update.Message.From.FirstName)
}
