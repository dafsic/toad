package utils

import (
	"bytes"
	"io"
	"sync"
)

type BufferedWriter struct {
	writer io.Writer
	buf    *bytes.Buffer
	size   int
	mu     sync.Mutex
}

func NewBufferedWriter(writer io.Writer, size int) *BufferedWriter {
	return &BufferedWriter{
		writer: writer,
		buf:    bytes.NewBuffer(make([]byte, 0, size)),
		size:   size,
	}
}

func (bw *BufferedWriter) Write(p []byte) (int, error) {
	bw.mu.Lock()
	defer bw.mu.Unlock()

	n := 0

	for len(p) > 0 {
		remaining := bw.size - bw.buf.Len()
		if len(p) <= remaining {
			// If the data fits into the remaining buffer space
			bw.buf.Write(p)
			n += len(p)
			break
		} else {
			// If the data is larger than the remaining buffer space
			bw.buf.Write(p[:remaining])
			n += remaining
			p = p[remaining:]
			if err := bw.Flush(); err != nil {
				return n, err
			}
		}
	}

	return n, nil
}

func (bw *BufferedWriter) Flush() error {
	if bw.buf.Len() == 0 {
		return nil
	}

	_, err := bw.writer.Write(bw.buf.Bytes())
	bw.buf.Reset()
	return err
}
