package middlewares

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"net/http"
	"strconv"
	"time"

	"github.com/gin-gonic/gin"
)

func Abs(n int64) int64 {
	if n < 0 {
		return -n
	}
	return n
}

func generateHMAC(message string, key string) string {
	h := hmac.New(sha256.New, []byte(key))
	h.Write([]byte(message))
	return hex.EncodeToString(h.Sum(nil))
}

func Auth(key string) gin.HandlerFunc {
	return func(c *gin.Context) {
		now := time.Now().Unix()
		ts := c.Query("ts")

		if ts == "" {
			c.JSON(http.StatusBadRequest, gin.H{
				"error": "ts is required",
			})
			c.Abort()
			return
		}

		tsInt, _ := strconv.ParseInt(ts, 10, 64)

		if Abs(now-tsInt) > 60 {
			c.JSON(http.StatusBadRequest, gin.H{
				"error": "ts is expired",
			})
			c.Abort()
			return
		}

		sign := c.Query("sign")
		if sign == "" {
			c.JSON(http.StatusBadRequest, gin.H{
				"error": "sign is required",
			})
			c.Abort()
			return
		}

		expectedSign := generateHMAC(ts, key)

        if sign != expectedSign {
            c.JSON(http.StatusUnauthorized, gin.H{
                "error": "invalid signature",
            })
            c.Abort()
            return
        }

        c.Next()

	}
}
