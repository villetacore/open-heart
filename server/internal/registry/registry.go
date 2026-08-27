// Package registry — список публичных серверов и пати-коды.
//
// Всё живёт в памяти: анонс держится TTL, после рестарта мастера сервера
// переанонсятся сами (docs/MULTIPLAYER.md §8.1). В базу писать нечего.
package registry

import (
	"context"
	"crypto/rand"
	"sort"
	"strings"
	"sync"
	"time"
)

// TTL по умолчанию: сервер шлёт heartbeat раз в 20 с, запись живёт 60 с.
const (
	DefaultTTL      = 60 * time.Second
	HeartbeatPeriod = 20 * time.Second
)

// Entry — то, что сервер сообщает о себе и что видит клиент в списке.
type Entry struct {
	ServerID    string `json:"server_id"`
	Name        string `json:"name"`
	MOTD        string `json:"motd,omitempty"`
	Region      string `json:"region,omitempty"`
	Players     int    `json:"players"`
	MaxPlayers  int    `json:"max_players"`
	PresetID    string `json:"preset_id"`
	ContentHash string `json:"content_hash"`
	GameVersion string `json:"game_version"`
	Protocol    int    `json:"protocol"`
	Mode        string `json:"mode,omitempty"`
	Passworded  bool   `json:"passworded"`
	AuthMode    string `json:"auth_mode"`
	Endpoint    string `json:"endpoint"`
	Relayed     bool   `json:"relayed"`

	UpdatedAt time.Time `json:"-"`
}

// Filter — параметры выборки из списка.
type Filter struct {
	Preset      string
	ContentHash string
	Region      string
	Protocol    int
	OnlyFree    bool
}

// Registry — потокобезопасный список анонсов с TTL.
type Registry struct {
	mu  sync.RWMutex
	ttl time.Duration
	m   map[string]Entry
}

func New(ttl time.Duration) *Registry {
	if ttl <= 0 {
		ttl = DefaultTTL
	}
	return &Registry{ttl: ttl, m: make(map[string]Entry)}
}

// Announce добавляет или обновляет запись.
func (r *Registry) Announce(e Entry) {
	e.UpdatedAt = time.Now()
	r.mu.Lock()
	r.m[e.ServerID] = e
	r.mu.Unlock()
}

// Remove снимает сервер со списка (корректное завершение).
func (r *Registry) Remove(serverID string) {
	r.mu.Lock()
	delete(r.m, serverID)
	r.mu.Unlock()
}

// List возвращает живые записи, отсортированные по заполненности (живые сервера выше).
func (r *Registry) List(f Filter) []Entry {
	now := time.Now()
	r.mu.RLock()
	out := make([]Entry, 0, len(r.m))
	for _, e := range r.m {
		if now.Sub(e.UpdatedAt) > r.ttl {
			continue
		}
		if f.Preset != "" && !strings.EqualFold(e.PresetID, f.Preset) {
			continue
		}
		if f.ContentHash != "" && e.ContentHash != f.ContentHash {
			continue
		}
		if f.Region != "" && !strings.EqualFold(e.Region, f.Region) {
			continue
		}
		if f.Protocol != 0 && e.Protocol != f.Protocol {
			continue
		}
		if f.OnlyFree && e.Players >= e.MaxPlayers {
			continue
		}
		out = append(out, e)
	}
	r.mu.RUnlock()

	sort.Slice(out, func(i, j int) bool {
		if out[i].Players != out[j].Players {
			return out[i].Players > out[j].Players
		}
		return out[i].Name < out[j].Name
	})
	return out
}

// Sweep выбрасывает протухшие записи, возвращая их число.
func (r *Registry) Sweep() int {
	now := time.Now()
	r.mu.Lock()
	defer r.mu.Unlock()
	n := 0
	for id, e := range r.m {
		if now.Sub(e.UpdatedAt) > r.ttl {
			delete(r.m, id)
			n++
		}
	}
	return n
}

// Run периодически чистит список, пока жив контекст.
func (r *Registry) Run(ctx context.Context, every time.Duration) {
	t := time.NewTicker(every)
	defer t.Stop()
	for {
		select {
		case <-ctx.Done():
			return
		case <-t.C:
			r.Sweep()
		}
	}
}

// ── пати-коды ────────────────────────────────────────────────────────────────

// Алфавит без похожих друг на друга символов: ни 0/O, ни 1/I.
const codeAlphabet = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789"

// CodeLen — длина пати-кода.
const CodeLen = 6

// Codes — временные коды приглашения («играть с другом»).
type Codes struct {
	mu  sync.Mutex
	ttl time.Duration
	m   map[string]codeEntry
}

type codeEntry struct {
	ServerID string
	Endpoint string
	Expires  time.Time
}

func NewCodes(ttl time.Duration) *Codes {
	if ttl <= 0 {
		ttl = 2 * time.Hour
	}
	return &Codes{ttl: ttl, m: make(map[string]codeEntry)}
}

// Create выдаёт свободный код на эндпоинт сервера.
func (c *Codes) Create(serverID, endpoint string) (string, time.Time, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.sweepLocked()
	for attempt := 0; attempt < 16; attempt++ {
		code, err := randomCode()
		if err != nil {
			return "", time.Time{}, err
		}
		if _, busy := c.m[code]; busy {
			continue
		}
		exp := time.Now().Add(c.ttl)
		c.m[code] = codeEntry{ServerID: serverID, Endpoint: endpoint, Expires: exp}
		return code, exp, nil
	}
	return "", time.Time{}, errNoFreeCode
}

// Resolve возвращает эндпоинт по коду.
func (c *Codes) Resolve(code string) (serverID, endpoint string, ok bool) {
	code = strings.ToUpper(strings.TrimSpace(code))
	c.mu.Lock()
	defer c.mu.Unlock()
	e, ok := c.m[code]
	if !ok || time.Now().After(e.Expires) {
		delete(c.m, code)
		return "", "", false
	}
	return e.ServerID, e.Endpoint, true
}

// Drop удаляет код (сервер выключился).
func (c *Codes) Drop(code string) {
	c.mu.Lock()
	delete(c.m, strings.ToUpper(code))
	c.mu.Unlock()
}

func (c *Codes) sweepLocked() {
	now := time.Now()
	for code, e := range c.m {
		if now.After(e.Expires) {
			delete(c.m, code)
		}
	}
}

var errNoFreeCode = errCode("registry: не удалось выдать свободный код")

type errCode string

func (e errCode) Error() string { return string(e) }

func randomCode() (string, error) {
	buf := make([]byte, CodeLen)
	if _, err := rand.Read(buf); err != nil {
		return "", err
	}
	out := make([]byte, CodeLen)
	for i, b := range buf {
		out[i] = codeAlphabet[int(b)%len(codeAlphabet)]
	}
	return string(out), nil
}
