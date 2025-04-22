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
	Name() string
	Stop()
	Run() error
	Status() (string, error)
	Err() error
	GetBasePrice() float64
	SetBasePrice(new float64)
	PlaceOrder(order *model.Order)
	NewBuyOrder(basePrice float64, multiplier int) *model.Order
	NewSellOrder(basePrice float64, multiplier int) *model.Order
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
		return StatusRunning, nil
	} else {
		if b.err != nil {
			return StatusError, b.err
		} else {
			return StatusStopped, nil
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

func (b *Grid) GetBasePrice() float64 {
	return math.Float64frombits(atomic.LoadUint64((*uint64)(unsafe.Pointer(&b.config.basePrice))))
}

func (b *Grid) SetBasePrice(new float64) {
	atomic.StoreUint64((*uint64)(unsafe.Pointer(&b.config.basePrice)), math.Float64bits(new))
}
