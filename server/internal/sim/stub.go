package sim

import (
	"encoding/json"
	"math"

	"github.com/villetacore/open-heart/server/internal/proto"
)

// MaxSpeed — предел правдоподобной скорости игрока, м/с.
// Базовая скорость класса ~5, спринт ×1.42, запас на рывки и склоны.
const MaxSpeed = 12.0

// speedTolerance — насколько допускаем превышение из-за джиттера сети.
const speedTolerance = 1.25

// Stub — временная симуляция: только игроки, без врагов, урона и лута.
// Существует, чтобы сеть, комнаты и клиент разрабатывались до готовности ядра (этап N1).
type Stub struct {
	cfg     Config
	tick    uint32
	players map[uint16]*stubPlayer
}

type stubPlayer struct {
	id       uint16
	nickname string
	pos      proto.Vec3
	yaw      float32
	hp       uint8
	flags    uint8
	ack      uint32
	// сколько раз клиент заявлял невозможную скорость
	violations int
}

func NewStub(cfg Config) *Stub {
	return &Stub{cfg: cfg, players: make(map[uint16]*stubPlayer)}
}

func (s *Stub) AddPlayer(id uint16, nickname string) error {
	s.players[id] = &stubPlayer{id: id, nickname: nickname, hp: 100}
	return nil
}

func (s *Stub) RemovePlayer(id uint16) { delete(s.players, id) }

func (s *Stub) Input(id uint16, in proto.Input) []proto.Event {
	p, ok := s.players[id]
	if !ok {
		return nil
	}
	p.yaw = in.Yaw
	p.ack = in.Tick

	// Мягкая валидация движения: сервер не пересчитывает шаг, а проверяет,
	// что игрок не прыгнул дальше, чем мог (docs/MULTIPLAYER.md §5).
	limit := MaxSpeed * speedTolerance * tickSeconds
	if dist(p.pos, in.Pos) > limit && p.pos != (proto.Vec3{}) {
		p.violations++
		return nil // позицию не принимаем, клиент увидит расхождение и откатится
	}
	p.pos = in.Pos
	return nil
}

// tickSeconds — шаг сервера; для валидации хватает номинального значения.
const tickSeconds = 0.05

func (s *Stub) Fire(id uint16, f proto.Fire) []proto.Event { return nil }

func (s *Stub) Hit(id uint16, h proto.HitClaim) []proto.Event { return nil }

func (s *Stub) Cmd(id uint16, c proto.Cmd) []proto.Event { return nil }

func (s *Stub) Tick(dt float64) (proto.Snapshot, []proto.Event) {
	s.tick++
	snap := proto.Snapshot{Tick: s.tick, Full: true}
	for _, p := range s.players {
		snap.Players = append(snap.Players, proto.Ent{
			ID:    p.id,
			Kind:  proto.KindPlayer,
			Pos:   p.pos,
			Yaw:   p.yaw,
			HP:    p.hp,
			Flags: p.flags,
		})
	}
	return snap, nil
}

// AckOf — последний учтённый Input.Tick игрока (комната подставляет его в снапшот).
func (s *Stub) AckOf(id uint16) uint32 {
	if p, ok := s.players[id]; ok {
		return p.ack
	}
	return 0
}

func (s *Stub) Save() ([]byte, error) { return json.Marshal(s.cfg) }

func (s *Stub) Close() error { return nil }

func dist(a, b proto.Vec3) float64 {
	dx := float64(a[0] - b[0])
	dy := float64(a[1] - b[1])
	dz := float64(a[2] - b[2])
	return math.Sqrt(dx*dx + dy*dy + dz*dz)
}
