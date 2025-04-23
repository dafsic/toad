package middlewares

import (
	"net/http"

	"github.com/gin-gonic/gin"
)

const (
	corsAllowOrigin      = "*"
	corsAllowMethods     = "GET, POST, OPTIONS, PUT, PATCH, DELETE"
	corsAllowHeaders     = "Authorization, Content-Length, X-CSRF-Token, Token,session, X_Requested_With, Accept, Origin, Host, Connection, Accept-Encoding, Accept-Language, DNT, X-CustomHeader, Keep-Alive, User-Agent, X-Requested-With, If-Modified-Since, Cache-Control, Content-Type, Pragma"
	corsAllowCredentials = "true"
	corsExposeHeaders    = "Content-Length, Access-Control-Allow-Origin, Access-Control-Allow-Headers, Cache-Control, Content-Language, Content-Type, Expires, Last-Modified, Pragma, FooBar"
	corsMaxAge           = "86400"
	corsContentType      = "application/json"
)

// CORS
func CORS() gin.HandlerFunc {
	return func(c *gin.Context) {
		if c.Request.Header.Get("Origin") != "" {
			c.Writer.Header().Set("Access-Control-Allow-Origin", corsAllowOrigin)
			c.Writer.Header().Set("Access-Control-Allow-Methods", corsAllowMethods)
			c.Writer.Header().Set("Access-Control-Allow-Headers", corsAllowHeaders)
			c.Writer.Header().Set("Access-Control-Allow-Credentials", corsAllowCredentials)
			c.Writer.Header().Set("Access-Control-Expose-Headers", corsExposeHeaders)
			c.Writer.Header().Set("Access-Control-Max-Age", corsMaxAge)
			c.Writer.Header().Set("content-type", corsContentType)
		}
		if c.Request.Method == http.MethodOptions {
			c.AbortWithStatus(http.StatusNoContent)
		}
		c.Next()
	}
}
