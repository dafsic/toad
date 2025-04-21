package utils

import "sync/atomic"

const (
	On  = 1
	Off = 0
)

func TurnOn(s *atomic.Int32) bool {
	return s.CompareAndSwap(Off, On)
}

func TurnOff(s *atomic.Int32) bool {
	return s.CompareAndSwap(On, Off)
}

func SwitcherStatus(s *atomic.Int32) int32 {
	return s.Load()
}
