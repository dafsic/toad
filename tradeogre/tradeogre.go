package tradeogre

import (
	"crypto/tls"
	"encoding/json"
	"fmt"
	"io"
	"mime"
	"net/http"
	"net/url"
	"strings"

	"go.uber.org/zap"
)

const (
	APIURL       = "https://tradeogre.com"
	APIVersion   = "api/v1"
	APIUserAgent = "Ogre GO API Agent"
	WSURL        = "wss://tradeogre.com:8443"
)

type Ogre interface {
	AddOrder(pair, token, side string, orderQty, price float64, userref int) (string, error)
	//CancelOrder(token string, orderIDs []string) error
}

type OgreAPI struct {
	key    string
	secret string
	client *http.Client
	logger *zap.Logger
}

var _ Ogre = (*OgreAPI)(nil)

func NewOgreAPI(logger *zap.Logger, key, secret string) *OgreAPI {
	return &OgreAPI{
		key:    key,
		secret: secret,
		logger: logger,
		client: &http.Client{
			Transport: &http.Transport{
				TLSClientConfig: &tls.Config{InsecureSkipVerify: true},
			},
		},
	}
}

func (api *OgreAPI) AddOrder(pair, token, side string, orderQty, price float64, userref int) (string, error) {
	method := "order/buy"
	if side == "sell" {
		method = "order/sell"
	}
	urlPath := fmt.Sprintf("/%s/%s", APIVersion, method)
	reqURL := fmt.Sprintf("%s%s", APIURL, urlPath)

	headers := map[string]string{
		api.key: api.secret,
	}

	data := url.Values{
		"market":   {pair},
		"price":    {fmt.Sprintf("%f", price)},
		"quantity": {fmt.Sprintf("%f", orderQty)},
	}

	resp, err := api.doPost(reqURL, data, headers, &AddOrderResponse{})
	if err != nil {
		return "", fmt.Errorf("failed to add order: %v", err)
	}

	addOrderResp := resp.(*AddOrderResponse)
	if !addOrderResp.Success {
		return "", fmt.Errorf("failed to add order: %s", addOrderResp.UUID)
	}

	api.logger.Info("AddOrder",
		zap.String("uuid", addOrderResp.UUID),
		zap.String("pair", pair),
		zap.String("side", side),
		zap.Float64("price", price),
		zap.Float64("orderQty", orderQty),
		zap.String("bnewbalavail", addOrderResp.BNewBalAvail),
		zap.String("snewbalavail", addOrderResp.SNewBalAvail),
	)
	return addOrderResp.UUID, nil
}

func (api *OgreAPI) doGet(reqURL string, values url.Values, headers map[string]string, typ any) (any, error) {
	encodedValues := values.Encode()
	fullURL := reqURL + "?" + encodedValues

	req, err := http.NewRequest("GET", fullURL, nil)
	if err != nil {
		return nil, fmt.Errorf("Could not execute request! #1 (%s)", err.Error())
	}

	return api.doAPIRequest(req, headers, typ)
}

func (api *OgreAPI) doPost(reqURL string, values url.Values, headers map[string]string, typ any) (any, error) {
	// Create request
	req, err := http.NewRequest("POST", reqURL, strings.NewReader(values.Encode()))
	if err != nil {
		return nil, fmt.Errorf("Could not execute request! #1 (%s)", err.Error())
	}

	return api.doAPIRequest(req, headers, typ)
}

func (api *OgreAPI) doAPIRequest(req *http.Request, headers map[string]string, typ any) (any, error) {
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
	var jsonData any

	if typ != nil {
		jsonData = typ
	}

	err = json.Unmarshal(body, &jsonData)
	if err != nil {
		return nil, fmt.Errorf("Could not execute request! #6 (%s)", err.Error())
	}

	return jsonData, nil
}
