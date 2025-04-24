package postgres

import (
	"fmt"
	"reflect"
	"sort"

	sq "github.com/Masterminds/squirrel"
	"github.com/jmoiron/sqlx/reflectx"
	"github.com/samber/lo"
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

func SliceStructSorted(strct any, includeNilFields bool, includeIdField bool) ([]string, []any) {
	v := reflect.ValueOf(strct)
	keys, values := FieldValueList(v, includeNilFields, includeIdField, nil)

	sort.Sort(&dualSorter{cols: keys, vals: values})
	return keys, values
}

func FieldValueList(v reflect.Value, includeNilFields bool, includeIdField bool, excludedFields []string) ([]string, []any) {
	v = reflect.Indirect(v)
	if k := v.Kind(); k != reflect.Struct {
		panic("expecting struct")
	}
	tm := mapper.TypeMap(v.Type())
	fields := []string{}
	values := []any{}
	excludedFieldsSet := lo.SliceToMap(excludedFields, func(s string) (string, struct{}) {
		return s, struct{}{}
	})
	for tagName, fi := range tm.Names {
		if !includeIdField && tagName == "id" {
			continue
		}

		if _, ok := excludedFieldsSet[tagName]; ok {
			continue
		}

		val := reflectx.FieldByIndexesReadOnly(v, fi.Index)
		if !includeNilFields && val.Kind() != reflect.Struct && val.IsNil() {
			continue
		}
		fields = append(fields, tagName)
		values = append(values, val.Interface())
	}
	return fields, values
}

func MapStruct(strct any, includeNilFields bool, includeIdField bool) map[string]any {
	fields := BuildFieldMap(strct)
	pairs := map[string]any{}
	for key := range fields {
		f := fields[key]
		if (includeIdField || key != "id") && (includeNilFields || f.Kind() == reflect.Struct || !f.IsNil()) {
			pairs[key] = f.Interface()
		}
	}
	return pairs
}

func BuildFieldMap(strct any) map[string]reflect.Value {
	v := reflect.ValueOf(strct)
	return FieldMap(v)
}

func FieldMap(v reflect.Value) map[string]reflect.Value {
	v = reflect.Indirect(v)
	if k := v.Kind(); k != reflect.Struct {
		panic("expecting struct")
	}
	r := map[string]reflect.Value{}
	tm := mapper.TypeMap(v.Type())
	for tagName, fi := range tm.Names {
		r[tagName] = reflectx.FieldByIndexesReadOnly(v, fi.Index)
	}
	return r
}

type dualSorter struct {
	cols []string
	vals []any
}

func (d *dualSorter) Len() int {
	return len(d.cols)
}

func (d *dualSorter) Less(i, j int) bool {
	return d.cols[i] <= d.cols[j]
}

func (d *dualSorter) Swap(i, j int) {
	d.cols[i], d.cols[j] = d.cols[j], d.cols[i]
	d.vals[i], d.vals[j] = d.vals[j], d.vals[i]
}
