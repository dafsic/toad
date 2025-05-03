package main

import (
	"context"
	"fmt"
	"strings"

	"github.com/dafsic/toad/app"
	"github.com/dafsic/toad/log"
	"github.com/dafsic/toad/telegram"
	"github.com/urfave/cli/v2"
	"go.uber.org/fx"
	"go.uber.org/zap"
)

func main() {
	app := app.NewApplication("Telegram Bot", "Telegram Bot for Toad microservices")
	app.Install(
		&log.Module{},
		&TelegramModule{},
	)
	if err := app.Run(); err != nil {
		fmt.Println(err)
		return
	}
}

type TelegramModule struct{}

func (m *TelegramModule) Configure(app *cli.App) {
	app.Flags = append(app.Flags,
		&cli.StringFlag{
			Name:     "token",
			EnvVars:  []string{"TELEGRAM_BOT_TOKEN"},
			Required: true,
			Usage:    "Telegram bot token from BotFather",
		},
		&cli.StringFlag{
			Name:    "grpc_addr",
			EnvVars: []string{"KRAKEN_GRID_ADDR"},
			Value:   "localhost:50051",
			Usage:   "Kraken Grid gRPC server address",
		},
		&cli.StringFlag{
			Name:    "allowed_users",
			EnvVars: []string{"TELEGRAM_ALLOWED_USERS"},
			Usage:   "Comma-separated list of allowed Telegram user IDs",
		},
	)
}

func (m *TelegramModule) Install(ctx *cli.Context) fx.Option {
	botToken := ctx.String("token")
	grpcAddr := ctx.String("grpc_addr")
	allowedUsersStr := ctx.String("allowed_users")

	return fx.Module("telegram",
		fx.Provide(
			func(logger *zap.Logger) (*telegram.BotService, error) {
				// Create bot configuration
				config := telegram.Config{
					BotToken:      botToken,
					GrpcServerURL: grpcAddr,
					AllowedUsers:  parseAllowedUsers(allowedUsersStr),
				}

				// Create bot service
				return telegram.NewBotService(config, logger)
			},
		),
		fx.Invoke(
			func(lc fx.Lifecycle, bot *telegram.BotService, logger *zap.Logger) {
				lc.Append(fx.Hook{
					OnStart: func(c context.Context) error {
						logger.Info("Starting Telegram bot",
							zap.String("grpc_server", grpcAddr),
							zap.Int("allowed_users", len(parseAllowedUsers(allowedUsersStr))),
						)

						// Start bot in a new goroutine
						go func() {
							if err := bot.Start(); err != nil {
								logger.Fatal("Bot service failed", zap.Error(err))
							}
						}()

						return nil
					},
					OnStop: func(c context.Context) error {
						logger.Info("Shutting down Telegram bot")
						bot.Close()
						return nil
					},
				})
			},
		),
	)
}

// Parse comma-separated list of user IDs
func parseAllowedUsers(usersStr string) []int64 {
	if usersStr == "" {
		return nil
	}

	var allowedUsers []int64
	for _, idStr := range strings.Split(usersStr, ",") {
		var id int64
		if _, err := fmt.Sscanf(strings.TrimSpace(idStr), "%d", &id); err == nil && id > 0 {
			allowedUsers = append(allowedUsers, id)
		}
	}
	return allowedUsers
}
