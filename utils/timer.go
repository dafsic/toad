package utils

import "time"

type Timer struct {
	interval time.Duration
	from     time.Time
	flag     bool
}

func NewTimer(interval int64) *Timer {
	return &Timer{
		interval: time.Duration(interval) * time.Second,
	}
}
func (t *Timer) Start() {
	if t.flag {
		return
	}
	t.flag = true
	t.from = time.Now()
}

func (t *Timer) Reset() {
	t.flag = false
}

func (t *Timer) IsExpired() bool {
	if !t.flag {
		return false
	}
	return time.Since(t.from) > t.interval
}
