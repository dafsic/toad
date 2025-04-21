package postgres

import (
	"fmt"

	sq "github.com/Masterminds/squirrel"
	"github.com/jmoiron/sqlx/reflectx"
)

var (
	builder = sq.StatementBuilder.PlaceholderFormat(sq.Dollar)
	mapper  = reflectx.NewMapper("db")
)

type UpdateOptions struct {
	IncludeNilFields bool
	ExcludeFields    []string
}

func Select(columns ...string) sq.SelectBuilder {
	return builder.Select(columns...)
}

func Insert(into string) sq.InsertBuilder {
	return builder.Insert(into)
}

func Update(table string) sq.UpdateBuilder {
	return builder.Update(table)
}

func Delete(from string) sq.DeleteBuilder {
	return builder.Delete(from)
}

func ToSQL(sqlizer sq.Sqlizer) (string, []any) {
	query, args, err := sqlizer.ToSql()
	if err != nil {
		panic(fmt.Sprintf("ToSQL error: %v", err))
	}
	return query, args
}
