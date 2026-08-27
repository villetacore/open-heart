package main

import (
	"context"
	"sync"
	"time"
)

// limiter — простой счётчик запросов на IP в скользящем окне фиксированной длины.
// Задача скромная: не дать перебирать пароли и коды. Ничего умнее не нужно.
type limiter struct {
	mu     sync.Mutex
	hits   map[string]int
	limit  int
	window time.Duration
}

func newLimiter(limit int, window time.Duration) *limiter {
	return &limiter{hits: make(map[string]int), limit: limit, window: window}
}

// allow уменьшает остаток для адреса; false — лимит исчерпан.
func (l *limiter) allow(addr string) bool {
	l.mu.Lock()
	defer l.mu.Unlock()
	if l.hits[addr] >= l.limit {
		return false
	}
	l.hits[addr]++
	return true
}

// run обнуляет счётчики раз в окно.
func (l *limiter) run(ctx context.Context) {
	t := time.NewTicker(l.window)
	defer t.Stop()
	for {
		select {
		case <-ctx.Done():
			return
		case <-t.C:
			l.mu.Lock()
			l.hits = make(map[string]int, len(l.hits))
			l.mu.Unlock()
		}
	}
}
