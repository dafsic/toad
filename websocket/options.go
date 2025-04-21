package websocket

import "time"

type Config struct {
	WriteWait time.Duration
	ReadWait  time.Duration
}

type Option func(*Config)

func WithWriteWait(wait time.Duration) Option {
	return func(c *Config) {
		c.WriteWait = wait
	}
}

func WithReadWait(wait time.Duration) Option {
	return func(c *Config) {
		c.ReadWait = wait
	}
}

func NewConfig(opts ...Option) *Config {
	cfg := &Config{
		WriteWait: 1000 * 1000 * 1000,
		ReadWait:  0,
	}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}
