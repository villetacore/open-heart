package relay

import (
	"errors"
	"io"
	"sync"
	"time"

	"github.com/hashicorp/yamux"
)

// ErrNoHost — сервера с таким id сейчас нет на связи.
var ErrNoHost = errors.New("relay: сервер не подключён к релею")

// Hub — сторона мастера: держит туннели подключённых серверов и открывает
// в них стримы, когда приходит игрок.
type Hub struct {
	mu    sync.RWMutex
	hosts map[string]*Host

	// Ограничение на комнату: релей не должен превращаться в бесплатный
	// туннель-сервис (docs/MULTIPLAYER.md §8.3).
	maxStreams int
}

// Host — один подключённый игровой сервер.
type Host struct {
	ServerID  string
	Token     string
	Session   *yamux.Session
	Connected time.Time

	streams int
	mu      sync.Mutex
}

func NewHub() *Hub {
	return &Hub{hosts: make(map[string]*Host), maxStreams: 32}
}

// Add регистрирует туннель сервера. Прежний туннель того же сервера
// закрывается: переподключился — значит старый мёртв.
func (h *Hub) Add(serverID, token string, session *yamux.Session) *Host {
	host := &Host{ServerID: serverID, Token: token, Session: session, Connected: time.Now()}

	h.mu.Lock()
	if previous, ok := h.hosts[serverID]; ok {
		_ = previous.Session.Close()
	}
	h.hosts[serverID] = host
	h.mu.Unlock()
	return host
}

// Remove убирает туннель, если это всё ещё он.
func (h *Hub) Remove(host *Host) {
	h.mu.Lock()
	if current, ok := h.hosts[host.ServerID]; ok && current == host {
		delete(h.hosts, host.ServerID)
	}
	h.mu.Unlock()
	_ = host.Session.Close()
}

// ByToken ищет сервер по выданному ему токену входа.
func (h *Hub) ByToken(token string) (*Host, bool) {
	h.mu.RLock()
	defer h.mu.RUnlock()
	for _, host := range h.hosts {
		if host.Token == token {
			return host, true
		}
	}
	return nil, false
}

// ByServer ищет туннель по id сервера.
func (h *Hub) ByServer(serverID string) (*Host, bool) {
	h.mu.RLock()
	defer h.mu.RUnlock()
	host, ok := h.hosts[serverID]
	return host, ok
}

// Count — сколько серверов сейчас на связи.
func (h *Hub) Count() int {
	h.mu.RLock()
	defer h.mu.RUnlock()
	return len(h.hosts)
}

// Open открывает стрим к серверу под нового игрока.
func (h *Hub) Open(host *Host) (io.ReadWriteCloser, error) {
	host.mu.Lock()
	if host.streams >= h.maxStreams {
		host.mu.Unlock()
		return nil, errors.New("relay: слишком много игроков через туннель")
	}
	host.streams++
	host.mu.Unlock()

	stream, err := host.Session.OpenStream()
	if err != nil {
		host.release()
		return nil, err
	}
	return &countedStream{Stream: stream, host: host}, nil
}

func (h *Host) release() {
	h.mu.Lock()
	if h.streams > 0 {
		h.streams--
	}
	h.mu.Unlock()
}

// countedStream возвращает место в квоте, когда игрок ушёл.
type countedStream struct {
	*yamux.Stream
	host *Host
	once sync.Once
}

func (s *countedStream) Close() error {
	s.once.Do(s.host.release)
	return s.Stream.Close()
}
