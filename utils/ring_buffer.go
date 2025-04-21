package utils

import (
	"sync"
)

// RingBuffer 是一个并发安全的环形缓冲区
type RingBuffer struct {
	data     []any
	size     int
	start    int
	end      int
	count    int
	mutex    sync.Mutex
	notEmpty *sync.Cond
	notFull  *sync.Cond
}

// NewRingBuffer 创建一个指定大小的RingBuffer
func NewRingBuffer(size int) *RingBuffer {
	rb := &RingBuffer{
		data:  make([]any, size),
		size:  size,
		start: 0,
		end:   0,
		count: 0,
	}
	rb.notEmpty = sync.NewCond(&rb.mutex)
	rb.notFull = sync.NewCond(&rb.mutex)
	return rb
}

// Enqueue 将元素添加到环形缓冲区
func (rb *RingBuffer) Enqueue(value any) {
	rb.mutex.Lock()
	defer rb.mutex.Unlock()

	for rb.count == rb.size {
		rb.notFull.Wait()
	}

	rb.data[rb.end] = value
	rb.end = (rb.end + 1) % rb.size
	rb.count++
	rb.notEmpty.Signal() // 通知有新数据可供读取
}

// Dequeue 从环形缓冲区中删除并返回一个元素
func (rb *RingBuffer) Dequeue() any {
	rb.mutex.Lock()
	defer rb.mutex.Unlock()

	for rb.count == 0 {
		rb.notEmpty.Wait()
	}

	value := rb.data[rb.start]
	rb.start = (rb.start + 1) % rb.size
	rb.count--
	rb.notFull.Signal() // 通知有新的空位可供写入

	return value
}

// Size 返回环形缓冲区中元素的数量
func (rb *RingBuffer) Size() int {
	rb.mutex.Lock()
	defer rb.mutex.Unlock()
	return rb.count
}
