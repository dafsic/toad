package dao

import (
	"context"

	"github.com/dafsic/toad/kraken_grid/model"
	"github.com/dafsic/toad/postgres"
	"go.uber.org/zap"
)

const (
	orderTable = `orders`
)

type Dao interface {
	// CreateOrder creates a new order in the database
	CreateOrder(ctx context.Context, order *model.Order) error
	// GetOrder retrieves an order from the database by its OrderID
	GetOrder(ctx context.Context, id int) (*model.Order, error)
	// GetOpenOrders retrieves all open orders from the database
	GetOpenOrders(ctx context.Context, bot string) ([]*model.Order, error)
	// UpdateOrder updates an existing order in the database
	UpdateOrder(ctx context.Context, order *model.Order) error
}

type DaoImpl struct {
	logger *zap.Logger
	postgres.Database
}

var _ Dao = (*DaoImpl)(nil)

func NewDao(logger *zap.Logger, db postgres.Database) *DaoImpl {
	return &DaoImpl{
		logger:   logger,
		Database: db,
	}
}
