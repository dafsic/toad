package kraken

import (
	"crypto/hmac"
	"crypto/sha256"
	"crypto/sha512"
	"crypto/tls"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"io"
	"mime"
	"net/http"
	"net/url"
	"strings"
	"time"

	"github.com/dafsic/toad/utils"
	"github.com/dafsic/toad/websocket"
	"go.uber.org/zap"
)

const (
	// APIURL is the official Kraken API Endpoint
	APIURL = "https://api.kraken.com"
	// APIVersion is the official Kraken API Version Number
	APIVersion = "0"
	// APIUserAgent identifies this library with the Kraken API
	APIUserAgent = "Kraken GO API Agent"

	PublicWSURL  = "wss://ws.kraken.com/v2"
	PrivateWSURL = "wss://ws-auth.kraken.com/v2"
)

type Kraken interface {
	// Ticker gets the ticker for a given symbol
	Ticker(pairs ...string) (*TickerResponse, error)
	// Balance gets the balance of the account
	Balance() (*BalanceResponse, error)
	// OpenOrders() (*OpenOrdersResponse, error)
	GetWebsocketToken() (*WebsocketTokenResponse, error)
	SubscribeTickers(socket *websocket.Socket, pairs ...string) error
	SubscribeBalances(socket *websocket.Socket, token string) error
	SubscribeExecutions(socket *websocket.Socket, token string) error
	AddOrderWithWebsocket(socket *websocket.Socket, pair, token, side string, orderQty, price float64, userref int) error
	CancelOrderWithWebsocket(socket *websocket.Socket, token string, orderIDs []string) error
}

type krakenAPI struct {
	config *Config
	client *http.Client
	logger *zap.Logger
}

var _ Kraken = (*krakenAPI)(nil)

func NewKraken(logger *zap.Logger, config *Config) *krakenAPI {
	return &krakenAPI{
		config: config,
		logger: logger,
		client: &http.Client{
			Transport: &http.Transport{
				TLSClientConfig: &tls.Config{InsecureSkipVerify: true},
			},
		},
	}
}

// Ticker returns the ticker for given comma separated pairs
func (api *krakenAPI) Ticker(pairs ...string) (*TickerResponse, error) {
	resp, err := api.queryPublicGet("Ticker", url.Values{
		"pair": {strings.Join(pairs, ",")},
	}, &TickerResponse{})
	if err != nil {
		return nil, err
	}

	return resp.(*TickerResponse), nil
}

// Balance returns all account asset balances
func (api *krakenAPI) Balance() (*BalanceResponse, error) {
	resp, err := api.queryPrivate("Balance", url.Values{}, &BalanceResponse{})
	if err != nil {
		return nil, err
	}

	return resp.(*BalanceResponse), nil
}

func (api *krakenAPI) GetWebsocketToken() (*WebsocketTokenResponse, error) {
	resp, err := api.queryPrivate("GetWebSocketsToken", url.Values{}, &WebsocketTokenResponse{})
	if err != nil {
		return nil, err
	}

	return resp.(*WebsocketTokenResponse), nil
}

func (api *krakenAPI) SubscribeTickers(socket *websocket.Socket, pairs ...string) error {
	req := &WebsocketRequest{
		Method: "subscribe",
		Params: map[string]any{
			"channel": "ticker",
			"symbol":  pairs,
		},
	}

	return api.requestWithWebsocket(socket, req)
}

func (api *krakenAPI) SubscribeBalances(socket *websocket.Socket, token string) error {
	req := &WebsocketRequest{
		Method: "subscribe",
		Params: map[string]any{
			"channel": "balances",
			"token":   token,
		},
	}

	return api.requestWithWebsocket(socket, req)
}

func (api *krakenAPI) SubscribeExecutions(socket *websocket.Socket, token string) error {
	req := &WebsocketRequest{
		Method: "subscribe",
		Params: map[string]any{
			"channel":      "executions",
			"token":        token,
			"snap_orders":  true,
			"snap_trades":  false,
			"order_status": false,
			"ratecounter":  true,
		},
	}

	return api.requestWithWebsocket(socket, req)
}

func (api *krakenAPI) AddOrderWithWebsocket(
	socket *websocket.Socket,
	pair, token, side string,
	orderQty, price float64,
	userref int,
) error {
	req := &WebsocketRequest{
		Method: "add_order",
		Params: map[string]any{
			"order_type":    "limit",
			"side":          side,
			"limit_price":   price,
			"order_qty":     orderQty,
			"symbol":        pair,
			"token":         token,
			"order_userref": userref,
			// Post only" refers to a special order execution instruction that ensures your order
			// is only placed on the order book as a "maker" order (adding liquidity)
			// and will be automatically canceled if it would execute immediately as a "taker" order
			// (removing liquidity).
			"post_only": true,
		},
	}

	return api.requestWithWebsocket(socket, req)
}

func (api *krakenAPI) CancelOrderWithWebsocket(socket *websocket.Socket, token string, orderID []string) error {
	req := &WebsocketRequest{
		Method: "cancel_order",
		Params: map[string]any{
			"token":    token,
			"order_id": orderID,
		},
	}

	return api.requestWithWebsocket(socket, req)
}

func (api *krakenAPI) requestWithWebsocket(socket *websocket.Socket, req *WebsocketRequest) error {
	payload, err := json.Marshal(req)
	if err != nil {
		return fmt.Errorf("%w%s", err, utils.LineInfo())
	}

	err = socket.SendBinary(payload)
	if err != nil {
		return fmt.Errorf("%w%s", err, utils.LineInfo())
	}

	return nil
}

// Execute a public method query
func (api *krakenAPI) queryPublicPost(method string, values url.Values, typ any) (any, error) {
	url := fmt.Sprintf("%s/%s/public/%s", APIURL, APIVersion, method)
	resp, err := api.doPost(url, values, nil, typ)

	return resp, err
}

func (api *krakenAPI) queryPublicGet(reqURL string, values url.Values, typ any) (any, error) {
	url := fmt.Sprintf("%s/%s/public/%s", APIURL, APIVersion, reqURL)
	return api.doGet(url, values, nil, typ)
}

// queryPrivate executes a private method query
func (api *krakenAPI) queryPrivate(method string, values url.Values, typ any) (any, error) {
	urlPath := fmt.Sprintf("/%s/private/%s", APIVersion, method)
	reqURL := fmt.Sprintf("%s%s", APIURL, urlPath)
	secret, _ := base64.StdEncoding.DecodeString(api.config.Secret)
	values.Set("nonce", fmt.Sprintf("%d", time.Now().UnixNano()))

	// Create signature
	signature := createSignature(urlPath, values, secret)

	// Add Key and signature to request headers
	headers := map[string]string{
		"API-Key":  api.config.Key,
		"API-Sign": signature,
	}

	resp, err := api.doPost(reqURL, values, headers, typ)

	return resp, err
}

func (api *krakenAPI) doGet(reqURL string, values url.Values, headers map[string]string, typ any) (any, error) {
	encodedValues := values.Encode()
	fullURL := reqURL + "?" + encodedValues

	req, err := http.NewRequest("GET", fullURL, nil)
	if err != nil {
		return nil, fmt.Errorf("Could not execute request! #1 (%s)", err.Error())
	}

	return api.doAPIRequest(req, headers, typ)
}

// doPost executes a HTTP Request to the Kraken API and returns the result
func (api *krakenAPI) doPost(reqURL string, values url.Values, headers map[string]string, typ any) (any, error) {

	// Create request
	req, err := http.NewRequest("POST", reqURL, strings.NewReader(values.Encode()))
	if err != nil {
		return nil, fmt.Errorf("Could not execute request! #1 (%s)", err.Error())
	}

	return api.doAPIRequest(req, headers, typ)
}

func (api *krakenAPI) doAPIRequest(req *http.Request, headers map[string]string, typ any) (any, error) {
	req.Header.Add("User-Agent", APIUserAgent)
	for key, value := range headers {
		req.Header.Add(key, value)
	}

	// Execute request
	resp, err := api.client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("could not execute request! #2 (%s)", err.Error())
	}
	defer resp.Body.Close()

	// Read request
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("Could not execute request! #3 (%s)", err.Error())
	}

	// Check mime type of response
	mimeType, _, err := mime.ParseMediaType(resp.Header.Get("Content-Type"))
	if err != nil {
		return nil, fmt.Errorf("Could not execute request #4! (%s)", err.Error())
	}
	if mimeType != "application/json" {
		return nil, fmt.Errorf("Could not execute request #5! (%s)", fmt.Sprintf("Response Content-Type is '%s', but should be 'application/json'.", mimeType))
	}

	// Parse request
	var jsonData KrakenResponse

	// Set the KrakenResponse.Result to typ so `json.Unmarshal` will
	// unmarshal it into given type, instead of `any`.
	if typ != nil {
		jsonData.Result = typ
	}

	err = json.Unmarshal(body, &jsonData)
	if err != nil {
		return nil, fmt.Errorf("Could not execute request! #6 (%s)", err.Error())
	}

	// Check for Kraken API error
	if len(jsonData.Error) > 0 {
		return nil, fmt.Errorf("Could not execute request! #7 (%s)", jsonData.Error)
	}

	return jsonData.Result, nil
}

// getSha256 creates a sha256 hash for given []byte
func getSha256(input []byte) []byte {
	sha := sha256.New()
	sha.Write(input)
	return sha.Sum(nil)
}

// getHMacSha512 creates a hmac hash with sha512
func getHMacSha512(message, secret []byte) []byte {
	mac := hmac.New(sha512.New, secret)
	mac.Write(message)
	return mac.Sum(nil)
}

func createSignature(urlPath string, values url.Values, secret []byte) string {
	// See https://www.kraken.com/help/api#general-usage for more information
	shaSum := getSha256([]byte(values.Get("nonce") + values.Encode()))
	macSum := getHMacSha512(append([]byte(urlPath), shaSum...), secret)
	return base64.StdEncoding.EncodeToString(macSum)
}
