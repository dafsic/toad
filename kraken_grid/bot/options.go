package bot

import (
	"slices"
	"strconv"
	"strings"

	"github.com/dafsic/toad/utils"
)

type Config struct {
	baseCoin     string  // Base coin for the grid
	quoteCoin    string  // Quote coin for the grid
	step         float64 // Step size for the grid
	amount       float64 // Amount of BaseCoin per grid order
	multipliers  []int   // Multipliers for the grid orders
	basePrice    float64 // Base price for the grid(optional, default is current price)
	currentPrice float64 // Current price of the market
	// If the current price stays above the base price plus step size for the full interval duration,
	// the base price will be adjusted and orders will be rebalanced.
	interval int64
	// Timer for the grid bot
	// It will be used to check if the current price is above the base price plus step size
	// for the full interval duration
	timer *utils.Timer
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

func WithBasePrice(price float64) Option {
	return func(c *Config) {
		c.basePrice = price
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

func WithInterval(interval int64) Option {
	return func(c *Config) {
		c.interval = interval
	}
}

func NewConfig(opts ...Option) *Config {
	cfg := &Config{
		step:        0.00005,
		amount:      1,
		baseCoin:    "XMR",
		quoteCoin:   "BTC",
		multipliers: []int{1, 1, 8},
		interval:    600,
	}
	for _, opt := range opts {
		opt(cfg)
	}
	cfg.timer = utils.NewTimer(cfg.interval)
	// Sort the multipliers in ascending order
	slices.SortFunc(cfg.multipliers, func(a, b int) int {
		return a - b
	})

	return cfg
}
