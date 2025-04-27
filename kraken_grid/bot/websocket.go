package bot

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"math"
	"time"

	"github.com/dafsic/toad/kraken_grid/model"
	"github.com/dafsic/toad/utils"
	"github.com/dafsic/toad/websocket"
	"go.uber.org/zap"
)

func (b *GridBot) OnBinaryMessage(data []byte, socket *websocket.Socket) {
	b.logger.Info("WebSocket binary message received", zap.ByteString("message", data))
}

func (b *GridBot) OnTextMessage(data string, socket *websocket.Socket) {
	// b.logger.Info("WebSocket text message received", zap.String("message", data), zap.String("url", socket.Url))
	var message any
	err := json.Unmarshal(utils.StringToBytes(data), &message)
	if err != nil {
		b.stopChan <- err
		return
	}

	switch message := message.(type) {
	case map[string]any:
		b.handleMapMessage(message)
	case []any:
		for _, msg := range message {
			if msgMap, ok := msg.(map[string]any); ok {
				b.handleMapMessage(msgMap)
			} else {
				b.stopChan <- err
				return
			}
		}
	default:
		b.stopChan <- errors.New("message is not a map or slice")
		return
	}
}

func (b *GridBot) newSocket(url string) *websocket.Socket {
	socket := websocket.New(url, b.logger)

	socket.OnBinaryMessage = b.OnBinaryMessage
	socket.OnTextMessage = b.OnTextMessage

	socket.Connect()
	return socket
}

func (b *GridBot) handleMapMessage(message map[string]any) {
	if message["method"] != nil {
		switch message["method"] {
		case "add_order", "cannel_order", "subscribe":
			b.handleMethodResponse(message)
		default:
			b.logger.Info("WebSocket message ignored", zap.Any("method", message["method"]))
		}
		return
	}

	if message["channel"] != nil {
		switch message["channel"] {
		case "status", "heartbeat":
			// b.logger.Info("WebSocket admin message", zap.Any("channel", message["channel"]))
		case "ticker":
			b.handleTickerChannel(message)
		case "executions":
			b.handleExecutionsChannel(message)
		default:
			b.logger.Info("WebSocket message ignored", zap.Any("channel", message["channel"]))
		}
	}
}

func (b *GridBot) handleMethodResponse(message map[string]any) {
	b.logger.Info("WebSocket method response", zap.Any("method", message["method"]), zap.Any("result", message["result"]), zap.Bool("success", message["success"].(bool)))
	if success, ok := message["success"].(bool); !ok || !success {
		// b.stopChan <- errors.New("WebSocket method response not successful: " + message["method"].(string))
		b.logger.Error("WebSocket method response not successful",
			zap.String("method", message["method"].(string)),
			zap.Any("result", message["result"]),
		)
		return
	}
}

func (b *GridBot) handleTickerChannel(message map[string]any) {
	data, ok := message["data"].([]any)
	if !ok || len(data) == 0 {
		return
	}

	tickerData, ok := data[0].(map[string]any)
	if !ok {
		return
	}

	if price, ok := tickerData["last"].(float64); ok {
		b.config.currentPrice = price
	}
	b.logger.Info("WebSocket ticker message",
		zap.Float64("current price", b.config.currentPrice),
		zap.Float64("base price", b.GetBasePrice()),
	)

	basePrice := b.GetBasePrice()
	if math.Abs(b.config.currentPrice-basePrice) > b.threshold {
		b.logger.Info("Price exceeded threshold",
			zap.Float64("current price", b.config.currentPrice),
			zap.Float64("base price", basePrice),
		)
		b.config.timer.Start()
		if b.config.timer.IsExpired() {
			b.config.timer.Reset()
			b.SetBasePrice(b.config.currentPrice)
			b.rebaseOrders()
		}
	} else {
		b.config.timer.Reset()
	}

}

func (b *GridBot) handleExecutionsChannel(message map[string]any) {
	data, ok := message["data"].([]any)
	if !ok {
		return
	}

	for _, execution := range data {
		exec := execution.(map[string]any) // assume execution is a map, panic if not
		orderID := exec["order_id"].(string)
		userref := exec["order_userref"].(float64)
		order, err := b.dao.GetOrder(context.TODO(), int(userref))
		if err != nil {
			b.stopChan <- fmt.Errorf("failed to get order[%s] from database: %w", orderID, err)
			return
		}

		order.OrderID = utils.Pointer(orderID)
		order.UpdatedAt = utils.Pointer(time.Now())
		order.Status = utils.Pointer(exec["exec_type"].(string))
		err = b.dao.UpdateOrder(context.TODO(), order) // Update order in database
		if err != nil {
			b.stopChan <- fmt.Errorf("failed to update order[%s] in database: %w", orderID, err)
			return
		}
		b.logger.Info("Order update",
			zap.String("order_id", *order.OrderID),
			zap.String("status", *order.Status),
			zap.String("side", *order.Side),
			zap.Float64("price", *order.Price),
			zap.String("pair", *order.Pair),
			zap.Int("multiplier", *order.Multiplier),
		)

		switch *order.Status {
		case "filled":
			b.handleOrderFilled(order)
		default: // "new", "cancelled", "pending"
		}
	}
}

func (b *GridBot) handleOrderFilled(order *model.Order) {
	var price float64
	if *order.Side == OrderBuy {
		price = *order.Price + b.config.step*float64(*order.Multiplier)
	} else {
		price = *order.Price - b.config.step*float64(*order.Multiplier)
	}
	b.PlaceOrder(b.NewOrder(Opposite(*order.Side), price, *order.Multiplier))
}
