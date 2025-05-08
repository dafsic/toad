package bot

import (
	"errors"
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
	StatusRunning string = "running"
	StatusStopped string = "stopped"
	StatusError   string = "error"
	OrderBuy      string = "buy"
	OrderSell     string = "sell"
)

type Bot interface {
	Run() error
	Stop(reason string)
	Status() (string, string)
	//Pair() string
	GetStep() float64
	PlaceOrder(order *model.Order)
	NewOrder(side string, price float64, multiplier int) *model.Order
}

type GridBot struct {
	status *atomic.Int32
	logger *zap.Logger
	config *Config
	// orders
	dao dao.Dao
	// websockets
	//publicWS  *websocket.Socket
	privateWS *websocket.Socket
	// kraken
	krakenAPI kraken.Kraken
	token     string
	// controller
	stopChan chan error
	err      error
}

var _ Bot = (*GridBot)(nil)

func Opposite(side string) string {
	if side == OrderBuy {
		return "sell"
	} else if side == OrderSell {
		return "buy"
	}
	return ""
}

// NewBot creates a new bot
func NewBot(logger *zap.Logger, config *Config, krakenAPI kraken.Kraken, dao dao.Dao) *GridBot {
	return &GridBot{
		status:    new(atomic.Int32),
		config:    config,
		logger:    logger,
		krakenAPI: krakenAPI,
		dao:       dao,
		stopChan:  make(chan error),
	}
}

func (b *GridBot) Pair() string {
	return b.config.baseCoin + "/" + b.config.quoteCoin
}

func (b *GridBot) GetStep() float64 {
	return b.config.step
}

func (b *GridBot) Status() (s string, e string) {
	s, e = StatusRunning, ""
	if utils.Off == b.status.Load() {
		s, e = StatusStopped, ""
		if b.err != nil {
			s, e = StatusError, b.err.Error()
		}
	}
	return
}

func (b *GridBot) Run() error {
	b.logger.Info("Starting bot...", zap.String("pair", b.Pair()))
	if len(b.config.multipliers) < 2 {
		return errors.New("at least 2 multipliers are required")
	}

	utils.TurnOn(b.status)
	go b.listenStop()
	go b.mainloop()
	return nil
}

func (b *GridBot) Stop(reason string) {
	b.stopChan <- errors.New(reason)
}

func (b *GridBot) listenStop() {
	err := <-b.stopChan
	b.logger.Error("Stopping bot...", zap.String("pair", b.Pair()), zap.Error(err))
	utils.TurnOff(b.status)
	b.privateWS.Close()
	//b.publicWS.Close()
	close(b.stopChan)
}

func (b *GridBot) mainloop() {
	b.logger.Info("Starting main loop...")

	// Get websocket token
	token, err := b.krakenAPI.GetWebsocketToken()
	if err != nil {
		b.stopChan <- err
		return
	}
	b.token = token.Token

	// Initialize websockets
	b.privateWS, err = b.newSocket(kraken.PrivateWSURL)
	if err != nil {
		b.stopChan <- err
		return
	}

	// Subscribe to necessary channels
	// if err := b.krakenAPI.SubscribeTickers(b.publicWS, b.Pair()); err != nil {
	// 	b.stopChan <- err
	// 	return
	// }
	if err := b.krakenAPI.SubscribeExecutions(b.privateWS, b.token); err != nil {
		b.stopChan <- err
		return
	}
}

func (b *GridBot) GetPrice() float64 {
	return math.Float64frombits(atomic.LoadUint64((*uint64)(unsafe.Pointer(&b.config.currentPrice))))
}

func (b *GridBot) SetPrice(new float64) {
	atomic.StoreUint64((*uint64)(unsafe.Pointer(&b.config.currentPrice)), math.Float64bits(new))
}
