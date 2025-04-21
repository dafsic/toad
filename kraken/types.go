package kraken

import "strconv"

// KrakenResponse wraps the Kraken API JSON response
type KrakenResponse struct {
	Error  []string `json:"error"`
	Result any      `json:"result"`
}

type TickerResponse map[string]PairTickerInfo

// PairTickerInfo represents ticker information for a pair
type PairTickerInfo struct {
	// Ask array(<price>, <whole lot volume>, <lot volume>)
	Ask []string `json:"a"`
	// Bid array(<price>, <whole lot volume>, <lot volume>)
	Bid []string `json:"b"`
	// Last trade closed array(<price>, <lot volume>)
	Close []string `json:"c"`
	// Volume array(<today>, <last 24 hours>)
	Volume []string `json:"v"`
	// Volume weighted average price array(<today>, <last 24 hours>)
	VolumeAveragePrice []string `json:"p"`
	// Number of trades array(<today>, <last 24 hours>)
	Trades []int `json:"t"`
	// Low array(<today>, <last 24 hours>)
	Low []string `json:"l"`
	// High array(<today>, <last 24 hours>)
	High []string `json:"h"`
	// Today's opening price
	OpeningPrice float64 `json:"o,string"`
}

type BalanceResponse map[string]string

func (b BalanceResponse) Get(coin string) float64 {
	var price float64
	switch coin {
	case "XMR":
		price, _ = strconv.ParseFloat(b["XXMR"], 64)
	case "BTC":
		price, _ = strconv.ParseFloat(b["XBT.F"], 64)
	default:
	}
	return price
}

type WebsocketTokenResponse struct {
	Token   string `json:"token"`
	Expires int    `json:"expires"`
}

type Pair struct {
	Base  string
	Quote string
}

func (p Pair) Symbol() string {
	// return Base/Quote
	return p.Base + "/" + p.Quote
}

type WebsocketRequest struct {
	Method string         `json:"method"`
	Params map[string]any `json:"params"`
	ReqID  int64          `json:"req_id"`
}

type TikerSubscribeParams struct {
	Channel string   `json:"channel"`
	Symbol  []string `json:"symbol"`
}

type Side string

const (
	Buy  Side = "buy"
	Sell Side = "sell"
)

func NewSide(side string) Side {
	switch side {
	case "buy":
		return Buy
	case "sell":
		return Sell
	default:
		return ""
	}
}

func (side Side) String() string {
	switch side {
	case Buy:
		return "buy"
	case Sell:
		return "sell"
	default:
		return ""
	}
}

func (side Side) IsBuy() bool {
	return side == Buy
}
func (side Side) IsSell() bool {
	return side == Sell
}

// Opposite returns the opposite side
func (side Side) Opposite() Side {
	if side == Buy {
		return Sell
	}
	return Buy
}
