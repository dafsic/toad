package middlewares

import (
	"bytes"
	"io"
	"time"

	"github.com/dafsic/toad/utils"
	"github.com/gin-gonic/gin"
	"go.uber.org/zap"
)

type responseLogWriter struct {
	gin.ResponseWriter
	responseBuffer *bytes.Buffer
}

func (w responseLogWriter) Write(b []byte) (int, error) {
	w.responseBuffer.Write(b)
	return w.ResponseWriter.Write(b)
}

// log what request and response are
func Logger(l *zap.Logger) gin.HandlerFunc {
	return func(ctx *gin.Context) {
		start := time.Now()

		urlPath := ctx.Request.URL.Path
		raw := ctx.Request.URL.RawQuery
		if raw != "" {
			urlPath = urlPath + "?" + raw
		}

		src := ctx.ClientIP()
		ctx.Set("src", src)

		var buf bytes.Buffer
		tee := io.TeeReader(ctx.Request.Body, &buf)
		requestBody, _ := io.ReadAll(tee)
		ctx.Request.Body = io.NopCloser(&buf)

		blw := responseLogWriter{responseBuffer: bytes.NewBufferString(""), ResponseWriter: ctx.Writer}
		ctx.Writer = &blw
		ctx.Next()

		end := time.Now()
		// gets the status code from the response
		status := ctx.Writer.Status()

		l.Info("HTTP Request",
			zap.String("src", src),
			zap.String("method", ctx.Request.Method),
			zap.String("url", urlPath),
			zap.String("request", utils.CompressStr(string(requestBody))),
			zap.String("response", utils.CompressStr(blw.responseBuffer.String())),
			zap.Int("status", status),
			zap.String("duration", end.Sub(start).String()),
		)
	}
}
