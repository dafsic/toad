package kraken

type Config struct {
	Key    string
	Secret string
}

type Option func(*Config)

func WithKey(key string) Option {
	return func(c *Config) {
		c.Key = key
	}
}

func WithSecret(secret string) Option {
	return func(c *Config) {
		c.Secret = secret
	}
}

func NewConfig(opts ...Option) *Config {
	cfg := &Config{
		Key:    "not set",
		Secret: "not set",
	}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}
