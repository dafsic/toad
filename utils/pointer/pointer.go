package pointer

func Get[T any](val T) *T {
	return &val
}
