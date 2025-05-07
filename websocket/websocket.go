// refer: https://github.com/sacOO7/GoWebsocket/blob/master/gowebsocket.go
package websocket

import (
	"crypto/tls"
	"net"
	"net/http"
	"net/url"
	"reflect"
	"sync"
	"sync/atomic"
	"time"

	"github.com/dafsic/toad/utils"
	"github.com/gorilla/websocket"
	"go.uber.org/zap"
)

type Socket struct {
	Conn              *websocket.Conn
	ConnStatus        *atomic.Int32
	WebsocketDialer   *websocket.Dialer
	Url               string
	ConnectionOptions ConnectionOptions
	RequestHeader     http.Header
	OnTextMessage     func(message string, socket *Socket)
	OnBinaryMessage   func(data []byte, socket *Socket)
	mux               *sync.Mutex // for locking the connection
	logger            *zap.Logger
}

type ConnectionOptions struct {
	UseCompression bool
	UseSSL         bool
	Proxy          func(*http.Request) (*url.URL, error)
	Subprotocols   []string
}

func New(url string, l *zap.Logger) *Socket {
	return &Socket{
		ConnStatus:    new(atomic.Int32),
		Url:           url,
		RequestHeader: http.Header{},
		ConnectionOptions: ConnectionOptions{
			UseCompression: false,
			UseSSL:         true,
		},
		WebsocketDialer: &websocket.Dialer{},
		mux:             &sync.Mutex{},
		logger:          l,
	}
}

func (socket *Socket) setConnectionOptions() {
	socket.WebsocketDialer.EnableCompression = socket.ConnectionOptions.UseCompression
	socket.WebsocketDialer.TLSClientConfig = &tls.Config{InsecureSkipVerify: socket.ConnectionOptions.UseSSL}
	socket.WebsocketDialer.Proxy = socket.ConnectionOptions.Proxy
	socket.WebsocketDialer.Subprotocols = socket.ConnectionOptions.Subprotocols
	socket.WebsocketDialer.HandshakeTimeout = 30 * time.Second
}

func (socket *Socket) Connect() error {
	var err error
	socket.setConnectionOptions()

	socket.Conn, _, err = socket.WebsocketDialer.Dial(socket.Url, socket.RequestHeader)
	if err != nil {
		// socket.logger.Panic("WebSocket connection error", zap.String("url", socket.Url), zap.Error(err))
		return err
	}

	socket.logger.Info("Connected to server", zap.String("url", socket.Url), zap.String("addr", socket.Conn.LocalAddr().String()))
	utils.TurnOn(socket.ConnStatus)

	go socket.readloop()

	return nil
}

func (socket *Socket) SendText(message string) error {
	return socket.send(websocket.TextMessage, []byte(message))

}

func (socket *Socket) SendBinary(data []byte) error {
	return socket.send(websocket.BinaryMessage, data)
}

func (socket *Socket) readloop() {
	for {
		messageType, message, err := socket.Conn.ReadMessage()
		if err != nil {
			socket.handleReadError(err)
			break
		}

		switch messageType {
		case websocket.TextMessage:
			if socket.OnTextMessage != nil {
				socket.OnTextMessage(string(message), socket)
			}
		case websocket.BinaryMessage:
			if socket.OnBinaryMessage != nil {
				socket.OnBinaryMessage(message, socket)
			}
		}
	}
}

func (socket *Socket) send(messageType int, data []byte) error {
	if utils.SwitcherStatus(socket.ConnStatus) == utils.Off {
		socket.logger.Error("WebSocket connection is closed, cannot send message", zap.String("url", socket.Url))
		return nil
	}

	socket.mux.Lock()
	err := socket.Conn.WriteMessage(messageType, data)
	socket.mux.Unlock()
	return err
}

func (socket *Socket) Close() {
	socket.mux.Lock()
	if socket.Conn == nil {
		socket.mux.Unlock()
		return
	}
	socket.mux.Unlock()

	err := socket.send(websocket.CloseMessage, websocket.FormatCloseMessage(websocket.CloseNormalClosure, ""))
	if err != nil {
		socket.logger.Error("socket write close error", zap.Error(err))
	}
	// Don't call socket.Conn.Close() here, as it will be called in the close handler
	// socket.Conn.Close()
}

func (socket *Socket) handleReadError(err error) {
	utils.TurnOff(socket.ConnStatus)
	socket.mux.Lock()
	defer socket.mux.Unlock()
	socket.Conn.Close()
	socket.Conn = nil
	switch e := err.(type) {
	case *websocket.CloseError:
		if websocket.IsCloseError(err, websocket.CloseNormalClosure) {
			socket.logger.Info("WebSocket closed normally", zap.Int("code", e.Code), zap.String("url", socket.Url))
			return
		}
		socket.logger.Error("WebSocket read error, reconnecting...", zap.Error(err), zap.Int("code", e.Code))
		socket.reconnect()

	case *net.OpError:
		socket.logger.Error("Network read error, reconnecting...", zap.Error(err), zap.String("op", e.Op), zap.String("net", e.Net))
		socket.reconnect()
	default:
		socket.logger.Error("WebSocket read error, reconnecting...", zap.Error(err), zap.String("type", reflect.TypeOf(err).String()))
		socket.reconnect()
	}
}

func (socket *Socket) reconnect() {
	for i := range 5 {
		time.Sleep(time.Minute)
		socket.logger.Info("Reconnecting to server", zap.Int("count", i), zap.String("url", socket.Url))
		err := socket.Connect()
		if err == nil {
			return
		}
	}
}
