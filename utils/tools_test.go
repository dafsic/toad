package utils

import (
	"strings"
	"testing"
)

func TestStrLink(t *testing.T) {
	s := ConcatStrings("123", "456", "789")

	if strings.Join(s, "|") != "123|456|789" {
		t.Error(s)
	}
}

func TestStringToBytes(t *testing.T) {
	s := "0123456789"

	bs := StringToBytes(s)

	if bs[0] != '0' || bs[9] != '9' {
		t.Error(bs)
	}
}

func TestBytesToString(t *testing.T) {
	b := []byte{48, 49, 50, 51, 52, 53, 54, 55, 56, 57}

	s := BytesToString(b)

	if s != "0123456789" {
		t.Error(s)
	}
}
