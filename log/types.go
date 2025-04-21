package log

import (
	"go.uber.org/zap/zapcore"
)

type Level string

func StringToLevel(lvl Level) zapcore.Level {
	switch lvl {
	case "debug", "DEBUG":
		return zapcore.DPanicLevel
	case "info", "INFO":
		return zapcore.InfoLevel
	case "warn", "WARN":
		return zapcore.WarnLevel
	case "error", "ERROR":
		return zapcore.ErrorLevel
	case "panic", "PANIC":
		return zapcore.PanicLevel
	case "fatal", "FATAL":
		return zapcore.FatalLevel
	default:
		return zapcore.InfoLevel
	}
}
