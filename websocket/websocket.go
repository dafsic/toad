// refer: https://github.com/sacOO7/GoWebsocket/blob/master/gowebsocket.go
package websocket

import (
	"crypto/tls"
	"net"
	"net/http"
	"net/url"
	"reflect"
	"sync"
	"time"

	"github.com/gorilla/websocket"
	"go.uber.org/zap"
)

type Socket struct {
	Conn              *websocket.Conn
	WebsocketDialer   *websocket.Dialer
	Url               string
	ConnectionOptions ConnectionOptions
	RequestHeader     http.Header
	OnConnected       func(socket *Socket)
	OnTextMessage     func(message string, socket *Socket)
	OnBinaryMessage   func(data []byte, socket *Socket)
	OnPingReceived    func(data string, socket *Socket)
	OnPongReceived    func(data string, socket *Socket)
	mux               *sync.Mutex // for locking the connection
	logger            *zap.Logger
	reconnectCounter  int
}

type ConnectionOptions struct {
	UseCompression bool
	UseSSL         bool
	Proxy          func(*http.Request) (*url.URL, error)
	Subprotocols   []string
}

func New(url string, l *zap.Logger) *Socket {
	return &Socket{
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
}

func (socket *Socket) Connect() {
	var err error
	socket.setConnectionOptions()

	socket.mux.Lock()
	socket.Conn, _, err = socket.WebsocketDialer.Dial(socket.Url, socket.RequestHeader)
	socket.mux.Unlock()
	if err != nil {
		socket.logger.Panic("WebSocket connection error", zap.String("url", socket.Url), zap.Error(err))
		return
	}

	socket.reconnectCounter = 0
	socket.logger.Info("Connected to server", zap.String("url", socket.Url))

	if socket.OnConnected != nil {
		socket.OnConnected(socket)
	}

	defaultPingHandler := socket.Conn.PingHandler()
	socket.Conn.SetPingHandler(func(appData string) error {
		if socket.OnPingReceived != nil {
			socket.OnPingReceived(appData, socket)
		}
		return defaultPingHandler(appData)
	})

	defaultPongHandler := socket.Conn.PongHandler()
	socket.Conn.SetPongHandler(func(appData string) error {
		if socket.OnPongReceived != nil {
			socket.OnPongReceived(appData, socket)
		}
		return defaultPongHandler(appData)
	})

	go func() {
		for {
			messageType, message, err := socket.Conn.ReadMessage()
			if err != nil {
				socket.handleReadError(err)
				return
			}
			//socket.logger.Info("socket recv", zap.ByteString("message", message))

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
	}()
}

func (socket *Socket) SendText(message string) error {
	return socket.send(websocket.TextMessage, []byte(message))

}

func (socket *Socket) SendBinary(data []byte) error {
	return socket.send(websocket.BinaryMessage, data)
}

func (socket *Socket) send(messageType int, data []byte) error {
	socket.mux.Lock()
	err := socket.Conn.WriteMessage(messageType, data)
	socket.mux.Unlock()
	return err
}

func (socket *Socket) Close() {
	socket.mux.Lock()
	if socket.Conn == nil {
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
	switch e := err.(type) {
	case *websocket.CloseError:
		if websocket.IsCloseError(err, websocket.CloseNormalClosure, websocket.CloseGoingAway) {
			socket.logger.Info("WebSocket closed normally", zap.Int("code", e.Code))
			socket.Conn.Close()
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
	socket.reconnectCounter++
	socket.logger.Info("Reconnecting to server", zap.Int("attempt", socket.reconnectCounter))
	time.Sleep(time.Duration(socket.reconnectCounter) * time.Second)
	socket.Connect()
}
