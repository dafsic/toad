package tradeogre

// {"success":true,"uuid":"235f770b-aa3f-4a31-8194-73d9612c2df1","bnewbalavail":"0.10000000","snewbalavail":"0.50000000"}
type AddOrderResponse struct {
	Success      bool   `json:"success"`
	UUID         string `json:"uuid"`
	BNewBalAvail string `json:"bnewbalavail"`
	SNewBalAvail string `json:"snewbalavail"`
}
