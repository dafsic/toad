package utils_test

import (
	"sync/atomic"
	"testing"

	utils "github.com/dafsic/toad/utils"
)

func TestSwitch(t *testing.T) {
	var s atomic.Int32

	if utils.SwitcherStatus(&s) != utils.Off {
		t.Error("expected Off")
	}

	utils.TurnOn(&s)
	if utils.SwitcherStatus(&s) != utils.On {
		t.Error("expected On")
	}
}
