package utils

import (
	"math"
	"math/rand"
	"net"
	"regexp"
	"runtime"
	"strconv"
	"strings"
	"unsafe"

	"github.com/google/uuid"
)

// ConcatStrings concatenates multiple strings into a slice of strings.
func ConcatStrings(elems ...string) []string {
	return elems
}

// ConcatString concatenates multiple strings into a single string.
func StrSplit(s string, p ...rune) []string {
	return strings.FieldsFunc(s, func(c rune) bool { return c == ',' || c == ';' })
}

// StringToBytes converts a string to a byte slice without copying the data.
func StringToBytes(s string) []byte {
	stringHeader := unsafe.StringData(s)
	return unsafe.Slice(stringHeader, len(s))
}

// BytesToString converts a byte slice to a string without copying the data.
func BytesToString(b []byte) string {
	return *(*string)(unsafe.Pointer(&b))
}

// CompressStr removes all whitespace characters from a string.
func CompressStr(str string) string {
	if str == "" {
		return ""
	}
	// \s matches any whitespace character (space, tab, newline, etc.)
	reg := regexp.MustCompile(`\\s+`)
	return reg.ReplaceAllString(str, "")
}

func StringToInt32(numStr string) int32 {
	num, _ := strconv.Atoi(numStr)
	return int32(num)
}

func StringToFloat64(num string) float64 {
	fnum, err := strconv.ParseFloat(num, 64)
	if err != nil {
		return 0
	}
	return fnum
}

func Float64ToString(f float64) string {
	return strconv.FormatFloat(f, 'f', -1, 64)
}

func FormatFloat(value float64, precision float64) float64 {
	precision = math.Pow(10, precision)
	return math.Round(value*precision) / precision
}

func GenerateUUID() string {
	return uuid.New().String()
}

// GenerateRandomString generates a random string of the specified length.
func GenerateRandomString(length int) string {
	chars := "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
	result := make([]byte, length)
	for i := 0; i < length; i++ {
		result[i] = chars[rand.Intn(len(chars))]
	}
	return string(result)
}

// GetAllLocalIPs retrieves all local IP addresses of the machine.
func GetAllLocalIPs() ([]string, error) {
	ipArr := make([]string, 0)
	addrs, err := net.InterfaceAddrs()
	if err != nil {
		return ipArr, err
	}
	for _, address := range addrs {
		// Check if the address is an IP address and not a loopback address
		// and if it is an IPv4 address
		if ipnet, ok := address.(*net.IPNet); ok && !ipnet.IP.IsLoopback() {
			if ipnet.IP.To4() != nil {
				ipArr = append(ipArr, ipnet.IP.String())
			}
		}
	}
	return ipArr, nil
}

// LineInfo returns the function name, file name and line number of the caller function.
func LineInfo() string {
	function := "xxx"
	pc, file, line, ok := runtime.Caller(1)
	if !ok {
		file = "???"
		line = 0
	}
	function = runtime.FuncForPC(pc).Name()

	return strings.Join(ConcatStrings(file, "(", function, "):", strconv.Itoa(line)), "")
}
