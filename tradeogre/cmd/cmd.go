package main

import (
	"os"
	"os/signal"

	"github.com/dafsic/toad/log"
	ogre "github.com/dafsic/toad/tradeogre"
	"github.com/dafsic/toad/websocket"
	"go.uber.org/zap"
)

type Bot struct {
	logger  *zap.Logger
	ws      *websocket.Socket
	errChan chan error
}

func main() {
	// This is a placeholder for the main function.
	// The actual implementation would go here.
	logger, err := log.NewLogger(log.NewConfig(log.WithLevel("debug")))
	if err != nil {
		panic(err)
	}

	bot := &Bot{
		logger: logger,
	}

	ws := bot.newSocket(ogre.WSURL)
	bot.ws = ws

	// {"a":"subscribe","e":"trade","t":"*"}
	err = bot.ws.SendText(`{"a":"subscribe","e":"trade","t":"XMR-BTC"}`)
	//err = bot.ws.SendText(`{"a":"subscribe","e":"book","t":"XMR-BTC"}`)
	if err != nil {
		bot.logger.Error("Failed to send subscribe message", zap.Error(err))
		return
	}

	signalChan := make(chan os.Signal, 1)
	signal.Notify(signalChan, os.Interrupt)
	// Wait for a signal to exit
	select {
	case <-signalChan:
		bot.logger.Info("Received interrupt signal, shutting down...")
		bot.ws.Close()
		bot.logger.Info("WebSocket closed")
		return
	case err := <-bot.errChan:
		bot.logger.Error("WebSocket error", zap.Error(err))
		bot.ws.Close()
		bot.logger.Info("WebSocket closed due to error")
		return
	}

}

func (b *Bot) newSocket(url string) *websocket.Socket {
	// Create a new websocket connection
	socket := websocket.New(url, b.logger)

	socket.OnBinaryMessage = b.OnBinaryMessage
	socket.OnTextMessage = b.OnTextMessage

	socket.Connect()
	return socket
}

func (b *Bot) OnBinaryMessage(data []byte, socket *websocket.Socket) {
	b.logger.Info("WebSocket binary message received", zap.ByteString("message", data))
}

func (b *Bot) OnTextMessage(data string, socket *websocket.Socket) {
	b.logger.Info("WebSocket text message received", zap.String("message", data))
}
