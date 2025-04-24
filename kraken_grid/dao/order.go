package dao

import (
	"context"
	"errors"
	"strings"

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
	cols, vals := pg.SliceStructSorted(order, false, false)
	query, args := pg.ToSQL(pg.Insert(orderTable).
		Columns(cols...).
		Values(vals...).
		Suffix(dao.buildReturningColumns()))

	dao.logger.Debug("Creating order", zap.String("query", query), zap.Any("args", args))

	return dao.Transact(ctx, func(tx *sqlx.Tx) error {
		return tx.GetContext(ctx, order, query, args...)
	})
}

// GetOrder retrieves an order from the database by its ID
func (dao *DaoImpl) GetOrder(ctx context.Context, id int) (*model.Order, error) {
	var order model.Order

	query, args := pg.ToSQL(pg.
		Select(dao.buildSelectColumns()...).
		From(orderTable).
		Where(sq.Eq{"id": id}))

	dao.logger.Debug("GetOrder query", zap.String("query", query), zap.Any("args", args))

	err := dao.Transact(ctx, func(tx *sqlx.Tx) error {
		return tx.GetContext(ctx, &order, query, args...)
	})

	return &order, err
}

// GetOpenOrders retrieves all open orders from the database for a specific bot
func (dao *DaoImpl) GetOpenOrders(ctx context.Context, bot string) ([]*model.Order, error) {
	var orders []*model.Order

	query, args := pg.ToSQL(pg.
		Select(dao.buildSelectColumns()...).
		From(orderTable).
		Where(sq.Eq{"bot": bot, "order_status": "new"}))

	dao.logger.Debug("GetOpenOrders query", zap.String("query", query), zap.Any("args", args))
	err := dao.Transact(ctx, func(tx *sqlx.Tx) error {
		return tx.SelectContext(ctx, &orders, query, args...)
	})

	return orders, err
}

// UpdateOrder updates an existing order in the database
func (dao *DaoImpl) UpdateOrder(ctx context.Context, order *model.Order) error {
	query, args := pg.ToSQL(pg.Update(orderTable).
		SetMap(pg.MapStruct(order, false, false)).
		Where(sq.Eq{"id": order.ID}))

	dao.logger.Debug("UpdateOrder query", zap.String("query", query), zap.Any("args", args))

	return dao.Transact(ctx, func(tx *sqlx.Tx) error {
		_, err := tx.ExecContext(ctx, query, args...)
		return err
	})
}

func (dao *DaoImpl) buildReturningColumns() string {
	var builder strings.Builder
	builder.Grow(1024)
	//builder.WriteString(" RETURNING id, created_at, updated_at, order_id, bot, exchange, pair, price, amount, side, multiplier, order_status")
	builder.WriteString("RETURNING ")
	builder.WriteString("id as \"id\",")
	builder.WriteString("created_at as \"created_at\",")
	builder.WriteString("updated_at as \"updated_at\",")
	builder.WriteString("order_id as \"order_id\",")
	builder.WriteString("bot as \"bot\",")
	builder.WriteString("exchange as \"exchange\",")
	builder.WriteString("pair as \"pair\",")
	builder.WriteString("price as \"price\",")
	builder.WriteString("amount as \"amount\",")
	builder.WriteString("side as \"side\",")
	builder.WriteString("multiplier as \"multiplier\",")
	builder.WriteString("order_status as \"order_status\"")
	return builder.String()
}

func (dao *DaoImpl) buildSelectColumns() []string {
	return []string{
		"id as \"id\"",
		"created_at as \"created_at\"",
		"updated_at as \"updated_at\"",
		"order_id as \"order_id\"",
		"bot as \"bot\"",
		"exchange as \"exchange\"",
		"pair as \"pair\"",
		"price as \"price\"",
		"amount as \"amount\"",
		"side as \"side\"",
		"multiplier as \"multiplier\"",
		"order_status as \"order_status\"",
	}
}
