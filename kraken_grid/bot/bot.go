package bot

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"math"
	"sync/atomic"
	"unsafe"

	"github.com/dafsic/toad/kraken"
	"github.com/dafsic/toad/kraken_grid/dao"
	"github.com/dafsic/toad/kraken_grid/model"
	"github.com/dafsic/toad/utils"
	"github.com/dafsic/toad/websocket"
	"go.uber.org/zap"
)

const (
	statusRunning string = "running"
	statusStopped string = "stopped"
	statusError   string = "error"
)

type Bot interface {
	Name() string
	Run() error
	Status() (string, error)
	Stop()
}

type Grid struct {
	status *atomic.Int32
	config *Config
	logger *zap.Logger
	// orders
	dao dao.Dao
	// websockets
	publicWS  *websocket.Socket
	privateWS *websocket.Socket
	// kraken
	krakenAPI kraken.Kraken
	token     string
	// controller
	stopChan chan error
	err      error
}

var _ Bot = (*Grid)(nil)

// NewBot creates a new bot
func NewBot(logger *zap.Logger, config *Config, krakenAPI kraken.Kraken, dao dao.Dao) *Grid {
	return &Grid{
		status:    new(atomic.Int32),
		config:    config,
		logger:    logger,
		krakenAPI: krakenAPI,
		dao:       dao,
		stopChan:  make(chan error),
	}
}

func (b *Grid) Name() string {
	return b.config.name
}

func (b *Grid) Status() (string, error) {
	status := b.status.Load()
	if status == utils.On {
		return statusRunning, nil
	} else {
		if b.err != nil {
			return statusError, b.err
		} else {
			return statusStopped, nil
		}
	}
}

func (b *Grid) Err() error {
	return b.err
}

func (b *Grid) Run() error {
	b.logger.Info("Starting bot...", zap.String("name", b.config.name))
	utils.TurnOn(b.status)
	go b.listenStop()
	go b.mainloop()
	return nil
}

func (b *Grid) Stop() {
	b.stopChan <- errors.New("bot stopped by user")
}

func (b *Grid) listenStop() {
	err := <-b.stopChan
	b.logger.Error("Stopping bot...", zap.String("name", b.config.name), zap.Error(err))
	utils.TurnOff(b.status)
	b.privateWS.Close()
	b.publicWS.Close()
	close(b.stopChan)
}

func (b *Grid) mainloop() {
	b.logger.Info("Starting main loop...")

	// Get websocket token
	token, err := b.krakenAPI.GetWebsocketToken()
	if err != nil {
		b.stopChan <- err
		return
	}
	b.token = token.Token

	// Initialize websockets
	b.publicWS = b.newSocket(kraken.PublicWSURL)
	b.privateWS = b.newSocket(kraken.PrivateWSURL)

	// Subscribe to necessary channels
	if err := b.krakenAPI.SubscribeTickers(b.publicWS, b.config.baseCoin+"/"+b.config.quoteCoin); err != nil {
		b.stopChan <- err
		return
	}
	if err := b.krakenAPI.SubscribeExecutions(b.privateWS, b.token); err != nil {
		b.stopChan <- err
		return
	}
}

func (b *Grid) newSocket(url string) *websocket.Socket {
	socket := websocket.New(url, b.logger)
	socket.OnPingReceived = func(appData string, s *websocket.Socket) {
		b.logger.Info("WebSocket ping received", zap.String("url", s.Url), zap.String("data", appData))
	}
	socket.OnPongReceived = func(appData string, s *websocket.Socket) {
		b.logger.Info("WebSocket pong received", zap.String("url", s.Url), zap.String("data", appData))
	}
	socket.OnConnected = func(s *websocket.Socket) {
		b.logger.Info("WebSocket connected", zap.String("url", s.Url))
	}

	socket.OnBinaryMessage = b.OnBinaryMessage
	socket.OnTextMessage = b.OnTextMessage

	socket.Connect()
	return socket
}

func (b *Grid) OnBinaryMessage(data []byte, socket *websocket.Socket) {
	b.logger.Info("WebSocket binary message received", zap.ByteString("message", data))
}

func (b *Grid) OnTextMessage(data string, socket *websocket.Socket) {
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

func (b *Grid) handleMapMessage(message map[string]any) {
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

func (b *Grid) handleMethodResponse(message map[string]any) {
	b.logger.Info("WebSocket method response", zap.Any("method", message["method"]), zap.Any("result", message["result"]), zap.Bool("success", message["success"].(bool)))
	if success, ok := message["success"].(bool); !ok || !success {
		b.stopChan <- errors.New("WebSocket method response not successful")
		return
	}
}

func (b *Grid) handleTickerChannel(message map[string]any) {
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
		zap.Float64("center price", b.getCenterPrice()),
	)

	centerPrice := b.getCenterPrice()
	if math.Abs(b.config.currentPrice-centerPrice) > float64(b.config.multipliers[len(b.config.multipliers)-2])*b.config.step {
		b.logger.Info("Price exceeded threshold",
			zap.Float64("current price", b.config.currentPrice),
			zap.Float64("center price", centerPrice),
		)
		b.config.timer.Start()
		if b.config.timer.IsExpired() {
			b.config.timer.Reset()
			b.setCenterPrice(b.config.currentPrice)
			b.rebaseOrders()
		}
	} else {
		b.config.timer.Reset()
	}

}

func (b *Grid) handleExecutionsChannel(message map[string]any) {
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

		order.Status = exec["exec_type"].(string)
		err = b.dao.UpdateOrder(context.TODO(), order.ID, map[string]any{"order_id": orderID, "order_status": order.Status}) // Update order in database
		if err != nil {
			b.stopChan <- fmt.Errorf("failed to update order[%s] in database: %w", orderID, err)
			return
		}
		b.logger.Info("Order update",
			zap.String("order_id", order.OrderID),
			zap.String("status", order.Status),
			zap.String("side", order.Side),
			zap.Float64("price", order.Price),
			zap.String("pair", order.Pair),
			zap.Int("multiplier", order.Multiplier),
		)

		switch order.Status {
		case "filled":
			b.handleOrderFilled(order)
		default: // "new", "cancelled", "pending"
		}
	}
}

func (b *Grid) rebaseOrders() {
	// cancel all orders
	orders, err := b.dao.GetOpenOrders(context.TODO(), b.config.name)
	if err != nil && err != dao.ErrNotFound {
		b.stopChan <- fmt.Errorf("failed to get open orders from database: %w", err)
		return
	}

	orderIDs := make([]string, len(orders))
	for i, order := range orders {
		orderIDs[i] = order.OrderID
	}

	if len(orders) > 0 {
		err := b.krakenAPI.CancelOrderWithWebsocket(b.privateWS, b.token, orderIDs)
		if err != nil {
			b.stopChan <- fmt.Errorf("failed to cancel orders: %w", err)
			return
		}
	}

	// place new orders
	for _, v := range b.config.multipliers {
		b.addOrder(kraken.Buy, b.config.centerPrice, v)
		b.addOrder(kraken.Sell, b.config.centerPrice, v)
	}
}

func (b *Grid) addOrder(side kraken.Side, basePrice float64, multiplier int) {
	price := basePrice - (b.config.step * float64(multiplier))
	if side == kraken.Sell {
		price = basePrice + (b.config.step * float64(multiplier))
	}

	order := b.newOrder()
	order.Pair = b.config.baseCoin + "/" + b.config.quoteCoin
	order.Price = utils.FormatFloat(price, 6)
	order.Amount = b.config.amount
	order.Side = side.String()
	order.Multiplier = multiplier
	order.Status = "pending"

	// Save order to database
	err := b.dao.CreateOrder(context.TODO(), order)
	if err != nil {
		b.stopChan <- fmt.Errorf("failed to create order in database: %w", err)
		return
	}

	err = b.krakenAPI.AddOrderWithWebsocket(
		b.privateWS,
		order.Pair,
		b.token,
		side,
		order.Amount,
		order.Price,
		order.ID,
	)
	if err != nil {
		b.stopChan <- fmt.Errorf("failed to place new order: %w", err)
	}
}

func (b *Grid) handleOrderFilled(order *model.Order) {
	b.addOrder(kraken.NewSide(order.Side).Opposite(), order.Price, order.Multiplier)
}

func (b *Grid) newOrder() *model.Order {
	return &model.Order{
		Bot:      b.config.name,
		Exchange: "kraken",
	}
}

func (b *Grid) getCenterPrice() float64 {
	return math.Float64frombits(atomic.LoadUint64((*uint64)(unsafe.Pointer(&b.config.centerPrice))))
}

func (b *Grid) setCenterPrice(new float64) {
	atomic.StoreUint64((*uint64)(unsafe.Pointer(&b.config.centerPrice)), math.Float64bits(new))
}
