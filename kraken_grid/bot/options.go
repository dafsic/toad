package bot

import (
	"slices"
	"strconv"
	"strings"
)

type Config struct {
	baseCoin     string  // Base coin for the grid
	quoteCoin    string  // Quote coin for the grid
	step         float64 // Step size for the grid
	amount       float64 // Amount of BaseCoin per grid order
	currentPrice float64 // Current price of the market
	// Multipliers for the grid orders
	// ---not used yet---
	multipliers []int
}

type Option func(*Config)

func WithSetp(step float64) Option {
	return func(c *Config) {
		c.step = step
	}
}

func WithGridAmount(amount float64) Option {
	return func(c *Config) {
		c.amount = amount
	}
}

func WithBaseCoin(coin string) Option {
	return func(c *Config) {
		c.baseCoin = coin
	}
}

func WithQuoteCoin(coin string) Option {
	return func(c *Config) {
		c.quoteCoin = coin
	}
}

func WithMultipliers(multipliers string) Option {
	return func(c *Config) {
		multipliersList := strings.SplitSeq(multipliers, ",")
		for multiplier := range multipliersList {
			m, err := strconv.Atoi(multiplier)
			if err != nil || m <= 0 {
				continue
			}
			c.multipliers = append(c.multipliers, m)
		}
	}
}

func NewConfig(opts ...Option) *Config {
	cfg := &Config{
		step:        0.00001,
		amount:      1,
		baseCoin:    "XMR",
		quoteCoin:   "BTC",
		multipliers: []int{5, 5, 10, 40},
	}
	for _, opt := range opts {
		opt(cfg)
	}
	// Sort the multipliers in ascending order
	slices.SortFunc(cfg.multipliers, func(a, b int) int {
		return a - b
	})

	return cfg
}
