package log

import (
	"go.uber.org/zap"
)

type Option func(*zap.Config)

func WithLevel(level Level) Option {
	return func(c *zap.Config) {
		c.Level = zap.NewAtomicLevelAt(StringToLevel(level))
	}
}

func NewConfig(opts ...Option) *zap.Config {
	// encoderConfig := zapcore.EncoderConfig{
	//     TimeKey:        "timestamp",
	//     LevelKey:       "level",
	//     NameKey:        "logger",
	//     CallerKey:      "caller",
	//     MessageKey:     "message",
	//     StacktraceKey:  "stacktrace",
	//     LineEnding:     zapcore.DefaultLineEnding,
	//     EncodeLevel:    zapcore.CapitalLevelEncoder,
	//     EncodeTime:     zapcore.ISO8601TimeEncoder,
	//     EncodeDuration: zapcore.StringDurationEncoder,
	//     EncodeCaller:   zapcore.ShortCallerEncoder,
	// }

	cfg := &zap.Config{
		Level:            zap.NewAtomicLevelAt(zap.InfoLevel),
		Development:      false,
		Encoding:         "console",
		EncoderConfig:    zap.NewDevelopmentEncoderConfig(),
		OutputPaths:      []string{"stdout"},
		ErrorOutputPaths: []string{"stderr"},
	}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}
