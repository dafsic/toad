package dao

import (
	"context"
	"errors"

	sq "github.com/Masterminds/squirrel"
	"github.com/dafsic/toad/kraken_grid/model"
	pg "github.com/dafsic/toad/postgres"
	"github.com/jmoiron/sqlx"
	"go.uber.org/zap"
)

var (
	ErrNotFound      = errors.New("not found")
	ErrAlreadyExists = errors.New("already exists")
)

// CreateOrder creates a new order in the database
func (dao *DaoImpl) CreateOrder(ctx context.Context, order *model.Order) error {
	query, args := pg.ToSQL(pg.Insert(orderTable).
		Columns("order_id", "bot", "exchange", "pair", "price", "amount", "side", "multiplier", "order_status").
		Values("", order.Bot, order.Exchange, order.Pair, order.Price, order.Amount, order.Side, order.Multiplier, order.Status).
		Suffix("RETURNING \"id\""))

	dao.logger.Debug("Creating order", zap.String("query", query), zap.Any("args", args))

	// Execute the query
	return dao.Transact(ctx, func(tx *sqlx.Tx) error {
		return tx.GetContext(ctx, &order.ID, query, args...)
	})
}

// GetOrder retrieves an order from the database by its ID
func (dao *DaoImpl) GetOrder(ctx context.Context, id int) (*model.Order, error) {
	var order model.Order

	query, args := pg.ToSQL(pg.
		Select("id", "order_id", "bot", "exchange", "pair", "price", "amount", "side", "multiplier", "order_status", "created_at", "updated_at").
		From(orderTable).
		Where(sq.Eq{"id": id}))

	dao.logger.Debug("GetOrder query", zap.String("query", query), zap.Any("args", args))

	err := dao.Transact(ctx, func(tx *sqlx.Tx) error {
		return tx.GetContext(ctx, &order, query, args...)
	})

	return &order, err
}

// GetOrdersByBot retrieves all orders from the database for a specific bot
func (dao *DaoImpl) GetOrdersByBot(ctx context.Context, bot string) ([]*model.Order, error) {
	var orders []*model.Order

	query, args := pg.ToSQL(pg.
		Select("id", "order_id", "bot", "exchange", "pair", "price", "amount", "side", "multiplier", "order_status", "created_at", "updated_at").
		From(orderTable).
		Where(sq.Eq{"bot": bot}))

	dao.logger.Debug("GetOrdersByBot query", zap.String("query", query), zap.Any("args", args))

	err := dao.Transact(ctx, func(tx *sqlx.Tx) error {
		return tx.SelectContext(ctx, &orders, query, args...)
	})

	return orders, err
}

// GetOpenOrders retrieves all open orders from the database for a specific bot
func (db *DaoImpl) GetOpenOrders(ctx context.Context, bot string) ([]*model.Order, error) {
	var orders []*model.Order

	query, args := pg.ToSQL(pg.
		Select("id", "order_id", "bot", "exchange", "pair", "price", "amount", "side", "multiplier", "order_status", "created_at", "updated_at").
		From(orderTable).
		Where(sq.Eq{"bot": bot, "order_status": "new"}))

	db.logger.Debug("GetOpenOrders query", zap.String("query", query), zap.Any("args", args))
	err := db.Transact(ctx, func(tx *sqlx.Tx) error {
		return tx.SelectContext(ctx, &orders, query, args...)
	})

	return orders, err
}

// UpdateOrder updates an existing order in the database
func (db *DaoImpl) UpdateOrder(ctx context.Context, id int, fieldsMap map[string]any) error {
	query, args := pg.ToSQL(pg.Update(orderTable).
		SetMap(fieldsMap).
		Where(sq.Eq{"id": id}))

	db.logger.Debug("UpdateOrder query", zap.String("query", query), zap.Any("args", args))

	return db.Transact(ctx, func(tx *sqlx.Tx) error {
		_, err := tx.ExecContext(ctx, query, args...)
		return err
	})
}
