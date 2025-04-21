package postgres

type Config struct {
	driver       string // Driver is the database driver
	dsn          string // Data Source Name
	maxOpenConns int
	maxIdleConns int
}

type Option func(*Config)

func WithDSN(dsn string) Option {
	return func(c *Config) {
		c.dsn = dsn
	}
}

func WithDriver(driver string) Option {
	return func(c *Config) {
		c.driver = driver
	}
}

func WithMaxOpenConns(maxOpenConns int) Option {
	return func(c *Config) {
		c.maxOpenConns = maxOpenConns
	}
}

func WithMaxIdleConns(maxIdleConns int) Option {
	return func(c *Config) {
		c.maxIdleConns = maxIdleConns
	}
}

func NewConfig(opts ...Option) *Config {
	cfg := &Config{
		dsn:    "",
		driver: "postgres",
	}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}
