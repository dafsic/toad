package model

import "time"

type Order struct {
	ID         *int       `db:"id"` // order_userref
	OrderID    *string    `db:"order_id"`
	Bot        *string    `db:"bot"`
	Exchange   *string    `db:"exchange"`
	Pair       *string    `db:"pair"`
	Price      *float64   `db:"price"`
	Amount     *float64   `db:"amount"`
	Side       *string    `db:"side"`
	Multiplier *int       `db:"multiplier"`
	Status     *string    `db:"order_status"`
	CreatedAt  *time.Time `db:"created_at"`
	UpdatedAt  *time.Time `db:"updated_at"`
}
