package room

import (
	"context"
	"fmt"
	"log/slog"
	"sync"
	"time"

	"github.com/villetacore/open-heart/server/internal/sim"
)

// Manager держит комнаты сервера: общий хаб и делв-инстансы под забеги.
//
// Делв — это отдельный мир: свой сид, свои враги, свой тик. Инстанс создаётся,
// когда игрок уходит в забег, и закрывается, когда из него ушли все
// (docs/MULTIPLAYER.md §11, пункт 6).
type Manager struct {
	mu    sync.Mutex
	rooms map[string]*Room

	hub *Room
	log *slog.Logger
	ctx context.Context

	// newDelve собирает симуляцию для нового забега: комната не должна знать,
	// как именно устроено ядро.
	newDelve func(seed uint64, depth int) (sim.Sim, Config, error)

	// Сколько инстанс живёт пустым, прежде чем закрыться: игрок мог отвалиться
	// и вернуться, терять забег из-за обрыва связи обидно.
	linger time.Duration
}

// NewManager создаёт менеджер с уже запущенным хабом.
func NewManager(
	ctx context.Context,
	hub *Room,
	log *slog.Logger,
	newDelve func(seed uint64, depth int) (sim.Sim, Config, error),
) *Manager {
	return &Manager{
		rooms:    map[string]*Room{hub.cfg.ID: hub},
		hub:      hub,
		log:      log,
		ctx:      ctx,
		newDelve: newDelve,
		linger:   60 * time.Second,
	}
}

// Hub — общая комната, куда попадают при входе на сервер.
func (m *Manager) Hub() *Room { return m.hub }

// Room возвращает комнату по id.
func (m *Manager) Room(id string) *Room {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.rooms[id]
}

// Rooms — снимок списка комнат (для логов и /healthz).
func (m *Manager) Rooms() []*Room {
	m.mu.Lock()
	defer m.mu.Unlock()
	out := make([]*Room, 0, len(m.rooms))
	for _, r := range m.rooms {
		out = append(out, r)
	}
	return out
}

// Players — сколько людей на сервере суммарно.
func (m *Manager) Players() int {
	total := 0
	for _, r := range m.Rooms() {
		total += r.Count()
	}
	return total
}

// DelveID — id инстанса забега, который ведёт игрок `owner`.
func DelveID(owner uint16, depth int) string {
	return fmt.Sprintf("delve-%d-%d", owner, depth)
}

// OpenDelve находит или создаёт инстанс забега.
//
// `owner` — тот, кто ведёт забег: его id и делает инстанс адресуемым, чтобы
// товарищи могли зайти в тот же самый данж, а не в свои копии.
func (m *Manager) OpenDelve(owner uint16, depth int, seed uint64) (*Room, error) {
	id := DelveID(owner, depth)

	m.mu.Lock()
	if existing, ok := m.rooms[id]; ok {
		m.mu.Unlock()
		return existing, nil
	}
	m.mu.Unlock()

	world, cfg, err := m.newDelve(seed, depth)
	if err != nil {
		return nil, err
	}
	cfg.ID = id
	cfg.Kind = "delve"
	cfg.Depth = depth
	cfg.Seed = seed

	created := New(cfg, world, m.log)

	m.mu.Lock()
	// Пока мы собирали мир, инстанс мог создать кто-то ещё.
	if existing, ok := m.rooms[id]; ok {
		m.mu.Unlock()
		_ = world.Close()
		return existing, nil
	}
	m.rooms[id] = created
	m.mu.Unlock()

	go created.Run(m.ctx)
	go m.closeWhenEmpty(created)
	m.log.Info("создан инстанс забега", "room", id, "depth", depth, "seed", seed)
	return created, nil
}

// closeWhenEmpty закрывает инстанс, из которого все ушли.
func (m *Manager) closeWhenEmpty(target *Room) {
	ticker := time.NewTicker(10 * time.Second)
	defer ticker.Stop()

	var emptySince time.Time
	for {
		select {
		case <-m.ctx.Done():
			return
		case <-target.Done():
			m.forget(target.cfg.ID)
			return
		case <-ticker.C:
			if target.Count() > 0 {
				emptySince = time.Time{}
				continue
			}
			if emptySince.IsZero() {
				emptySince = time.Now()
				continue
			}
			if time.Since(emptySince) >= m.linger {
				m.log.Info("инстанс забега закрыт", "room", target.cfg.ID)
				target.Stop()
				m.forget(target.cfg.ID)
				return
			}
		}
	}
}

func (m *Manager) forget(id string) {
	m.mu.Lock()
	delete(m.rooms, id)
	m.mu.Unlock()
}
