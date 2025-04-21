package websocket

import (
	"crypto/tls"
	"net"
	"net/http"
	"net/url"
	"reflect"
	"sync"

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
	sendMu            *sync.Mutex // Prevent "concurrent write to websocket connection"
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
		Url:           url,
		RequestHeader: http.Header{},
		ConnectionOptions: ConnectionOptions{
			UseCompression: false,
			UseSSL:         true,
		},
		WebsocketDialer: &websocket.Dialer{},
		sendMu:          &sync.Mutex{},
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

	socket.Conn, _, err = socket.WebsocketDialer.Dial(socket.Url, socket.RequestHeader)
	if err != nil {
		socket.logger.Panic("WebSocket connection error", zap.String("url", socket.Url), zap.Error(err))
		return
	}

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
	socket.sendMu.Lock()
	err := socket.Conn.WriteMessage(messageType, data)
	socket.sendMu.Unlock()
	return err
}

func (socket *Socket) Close() {
	if socket.Conn == nil {
		return
	}

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
		socket.Connect()

	case *net.OpError:
		socket.logger.Error("Network read error, reconnecting...", zap.Error(err), zap.String("op", e.Op), zap.String("net", e.Net))
		socket.Connect()
	default:
		socket.logger.Error("WebSocket read error, reconnecting...", zap.Error(err), zap.String("type", reflect.TypeOf(err).String()))
		socket.Connect()
	}
}
