package postgres

import (
	"context"
	"database/sql"
	"fmt"

	"github.com/jmoiron/sqlx"
	_ "github.com/lib/pq"
	"go.uber.org/zap"
)

type Database interface {
	Ping() error
	Session() *sqlx.DB

	Transact(ctx context.Context, txFunc func(tx *sqlx.Tx) error) error
	TransactReadOnly(ctx context.Context, txFunc func(tx *sqlx.Tx) error) error
}

type DatabaseImpl struct {
	logger *zap.Logger
	db     *sqlx.DB
	driver string
	dsn    string
}

func NewDatabase(logger *zap.Logger, config *Config) *DatabaseImpl {
	db, _ := sqlx.Open(config.driver, config.dsn)
	return &DatabaseImpl{db: db, dsn: config.dsn, driver: config.driver}
}

func (impl *DatabaseImpl) Ping() error {
	if err := impl.db.Ping(); err != nil {
		return fmt.Errorf("failed to ping database(%s): %w", impl.dsn, err)
	}
	return nil
}

func (impl *DatabaseImpl) Session() *sqlx.DB {
	return impl.db
}

func (impl *DatabaseImpl) Transact(ctx context.Context, txFunc func(tx *sqlx.Tx) error) (err error) {
	return impl.transact(ctx, txFunc, nil)
}

func (impl *DatabaseImpl) TransactReadOnly(ctx context.Context, txFunc func(tx *sqlx.Tx) error) (err error) {
	return impl.transact(ctx, txFunc, &sql.TxOptions{ReadOnly: true})
}

func (impl *DatabaseImpl) transact(ctx context.Context, txFunc func(tx *sqlx.Tx) error, options *sql.TxOptions) (err error) {
	tx, err := impl.Session().BeginTxx(ctx, options)
	if err != nil {
		return
	}
	defer func() {
		if p := recover(); p != nil {
			rollbackErr := tx.Rollback()
			if rollbackErr != nil {
				impl.logger.Error("error during rollback", zap.Error(rollbackErr))
			}
			panic(p) // re-throw panic after Rollback
		} else if err != nil {
			rollbackErr := tx.Rollback() // err is non-nil; don't change it
			if rollbackErr != nil {
				err = fmt.Errorf("error during rollback: %s: %w", rollbackErr, err)
			}
		} else {
			err = tx.Commit() // err is nil; if Commit returns error update err
		}
	}()
	err = txFunc(tx)
	return
}
