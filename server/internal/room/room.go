// Package room — игровая комната: одна горутина, тик 20 Гц, каналы наружу.
//
// Состояние мира живёт внутри sim.Sim и трогается только этой горутиной —
// мьютексов на геймплее нет (docs/MULTIPLAYER.md §7.1).
package room

import (
	"context"
	"encoding/json"
	"errors"
	"log/slog"
	"sync"
	"sync/atomic"
	"time"

	"github.com/villetacore/open-heart/server/internal/proto"
	"github.com/villetacore/open-heart/server/internal/sim"
	"github.com/villetacore/open-heart/server/internal/wire"
)

// ErrFull — в комнате нет мест.
var ErrFull = errors.New("room: комната заполнена")

// ErrClosed — комната остановлена.
var ErrClosed = errors.New("room: комната закрыта")

const (
	inboxSize   = 1024
	chatMaxLen  = 256
	inputPerSec = 60 // жёсткий потолок входящих сообщений от игрока
)

// Config — параметры комнаты.
type Config struct {
	ID          string
	Kind        string // hub | delve
	Preset      string
	ContentHash string
	Seed        uint64
	Depth       int
	MaxPlayers  int
	TickRate    int // Гц
}

// Player — подключённый игрок.
type Player struct {
	Peer       uint16
	Nickname   string
	UserID     int64
	AccountRef string
	Conn       wire.Conn

	budget  int // оставшийся лимит сообщений в текущую секунду
	dropped int
}

// Room — комната. Создаётся через New, работает в Run.
type Room struct {
	cfg Config
	sim sim.Sim
	log *slog.Logger

	joins  chan joinReq
	leaves chan uint16
	msgs   chan inbound
	done   chan struct{}
	stop   chan struct{}
	once   sync.Once

	players  map[uint16]*Player
	nextPeer uint16
	tick     uint32
	dead     []uint16 // отвалившиеся за текущий тик, вычищаются после рассылки

	count atomic.Int32 // число игроков, читается извне (анонс, /healthz)
}

type inbound struct {
	peer uint16
	env  proto.Envelope
}

type joinReq struct {
	player *Player
	resp   chan error
}

func New(cfg Config, s sim.Sim, log *slog.Logger) *Room {
	if cfg.TickRate <= 0 {
		cfg.TickRate = 20
	}
	if cfg.MaxPlayers <= 0 {
		cfg.MaxPlayers = 16
	}
	return &Room{
		cfg:      cfg,
		sim:      s,
		log:      log.With("room", cfg.ID),
		joins:    make(chan joinReq),
		leaves:   make(chan uint16, 32),
		msgs:     make(chan inbound, inboxSize),
		done:     make(chan struct{}),
		stop:     make(chan struct{}),
		players:  make(map[uint16]*Player),
		nextPeer: 1,
	}
}

// Stop просит комнату завершиться (инстанс забега опустел).
func (r *Room) Stop() {
	r.once.Do(func() { close(r.stop) })
}

// Done закрывается, когда комната завершила работу.
func (r *Room) Done() <-chan struct{} { return r.done }

// ID комнаты.
func (r *Room) ID() string { return r.cfg.ID }

// Count — сколько игроков сейчас в комнате.
func (r *Room) Count() int { return int(r.count.Load()) }

// Join добавляет игрока и возвращает выданный ему peer.
// Welcome отправляет сама комната — чтобы он гарантированно ушёл до первого снапшота.
func (r *Room) Join(ctx context.Context, p *Player) error {
	resp := make(chan error, 1)
	select {
	case <-r.done:
		return ErrClosed
	case <-ctx.Done():
		return ctx.Err()
	case r.joins <- joinReq{player: p, resp: resp}:
	}
	select {
	case <-ctx.Done():
		return ctx.Err()
	case err := <-resp:
		return err
	}
}

// Leave убирает игрока (вызывается при обрыве соединения).
func (r *Room) Leave(peer uint16) {
	select {
	case <-r.done:
	case r.leaves <- peer:
	}
}

// Post кладёт входящее сообщение в очередь комнаты. Не блокирует:
// переполнение очереди означает, что клиент шлёт больше, чем комната успевает.
func (r *Room) Post(peer uint16, env proto.Envelope) {
	select {
	case <-r.done:
	case r.msgs <- inbound{peer: peer, env: env}:
	default:
		r.log.Warn("очередь комнаты переполнена, сообщение отброшено", "peer", peer)
	}
}

// Run крутит тик до отмены контекста.
func (r *Room) Run(ctx context.Context) {
	interval := time.Second / time.Duration(r.cfg.TickRate)
	ticker := time.NewTicker(interval)
	defer ticker.Stop()

	budget := time.NewTicker(time.Second)
	defer budget.Stop()

	defer close(r.done)
	defer r.sim.Close()

	dt := interval.Seconds()
	r.log.Info("комната запущена", "kind", r.cfg.Kind, "tick", r.cfg.TickRate, "seed", r.cfg.Seed)

	for {
		select {
		case <-ctx.Done():
			r.log.Info("комната остановлена")
			return

		case <-r.stop:
			r.log.Info("комната закрыта")
			return

		case req := <-r.joins:
			req.resp <- r.doJoin(req.player)

		case peer := <-r.leaves:
			r.doLeave(peer)

		case m := <-r.msgs:
			r.handle(m)

		case <-budget.C:
			for _, p := range r.players {
				p.budget = inputPerSec
			}

		case <-ticker.C:
			r.step(dt)
		}
	}
}

func (r *Room) doJoin(p *Player) error {
	if len(r.players) >= r.cfg.MaxPlayers {
		return ErrFull
	}
	p.Peer = r.nextPeer
	r.nextPeer++
	if r.nextPeer == 0 { // 0 зарезервирован под «нет игрока»
		r.nextPeer = 1
	}
	p.budget = inputPerSec

	if err := r.sim.AddPlayer(p.Peer, p.Nickname); err != nil {
		return err
	}
	r.players[p.Peer] = p
	r.count.Store(int32(len(r.players)))
	r.tellPartySize()

	welcome := proto.Welcome{
		Peer:        p.Peer,
		Tick:        r.tick,
		PresetID:    r.cfg.Preset,
		ContentHash: r.cfg.ContentHash,
		World:       proto.World{Kind: r.cfg.Kind, Seed: r.cfg.Seed, Depth: r.cfg.Depth},
		Players:     r.roster(),
	}
	if err := p.Conn.Send(proto.TWelcome, welcome); err != nil {
		r.log.Warn("не отправить welcome", "peer", p.Peer, "err", err)
	}
	r.broadcastEvent(proto.Event{Kind: proto.EvJoin, Actor: p.Peer, Text: p.Nickname})
	r.log.Info("игрок вошёл", "peer", p.Peer, "nick", p.Nickname, "всего", len(r.players))
	return nil
}

func (r *Room) doLeave(peer uint16) {
	p, ok := r.players[peer]
	if !ok {
		return
	}
	r.sim.RemovePlayer(peer)
	delete(r.players, peer)
	r.count.Store(int32(len(r.players)))
	r.tellPartySize()
	// Соединение здесь НЕ закрываем: игрок мог просто перейти в другую комнату,
	// а сокет у него один. Закрывает его сессия, когда обрывается чтение.
	r.broadcastEvent(proto.Event{Kind: proto.EvLeave, Actor: peer, Text: p.Nickname})
	r.log.Info("игрок вышел", "peer", peer, "nick", p.Nickname, "всего", len(r.players))
}

// tellPartySize сообщает ядру, сколько людей сейчас в комнате: от этого
// зависит сложность забега (docs/MULTIPLAYER.md §11, пункт 5).
func (r *Room) tellPartySize() {
	args, err := json.Marshal(map[string]int{"size": len(r.players)})
	if err != nil {
		return
	}
	for _, event := range r.sim.Cmd(0, proto.Cmd{Kind: "party", Args: args}) {
		r.broadcastEvent(event)
	}
}

func (r *Room) roster() []proto.PlayerInfo {
	out := make([]proto.PlayerInfo, 0, len(r.players))
	for _, p := range r.players {
		out = append(out, proto.PlayerInfo{Peer: p.Peer, Nickname: p.Nickname})
	}
	return out
}

func (r *Room) handle(m inbound) {
	p, ok := r.players[m.peer]
	if !ok {
		return
	}
	if p.budget <= 0 {
		p.dropped++
		if p.dropped == 1 {
			r.log.Warn("превышен лимит сообщений", "peer", p.Peer)
		}
		return
	}
	p.budget--

	var events []proto.Event
	switch m.env.T {
	case proto.TInput:
		var in proto.Input
		if err := m.env.Into(&in); err != nil {
			return
		}
		events = r.sim.Input(p.Peer, in)

	case proto.TFire:
		var f proto.Fire
		if err := m.env.Into(&f); err != nil {
			return
		}
		events = r.sim.Fire(p.Peer, f)

	case proto.THitClaim:
		var h proto.HitClaim
		if err := m.env.Into(&h); err != nil {
			return
		}
		events = r.sim.Hit(p.Peer, h)

	case proto.TInteract:
		var in proto.Interact
		if err := m.env.Into(&in); err != nil {
			return
		}
		args, _ := json.Marshal(in)
		events = r.sim.Cmd(p.Peer, proto.Cmd{Kind: "interact", Args: args})

	case proto.TCmd:
		var c proto.Cmd
		if err := m.env.Into(&c); err != nil {
			return
		}
		events = r.sim.Cmd(p.Peer, c)

	case proto.TChat:
		var c proto.Chat
		if err := m.env.Into(&c); err != nil {
			return
		}
		text := sanitizeChat(c.Text)
		if text == "" {
			return
		}
		r.broadcastEvent(proto.Event{Kind: proto.EvChat, Actor: p.Peer, Text: text})
		return

	case proto.TPing:
		var ping proto.Ping
		if err := m.env.Into(&ping); err != nil {
			return
		}
		_ = p.Conn.Send(proto.TPong, proto.Pong{T: ping.T})
		return

	default:
		// Неизвестные теги игнорируем: так старый сервер переживает новый клиент.
		return
	}

	for _, e := range events {
		r.broadcastEvent(e)
	}
}

func (r *Room) step(dt float64) {
	r.tick++
	snap, events := r.sim.Tick(dt)
	snap.Tick = r.tick

	for _, e := range events {
		r.broadcastEvent(e)
	}

	acker, _ := r.sim.(interface{ AckOf(uint16) uint32 })
	allItems := snap.Items
	for _, p := range r.players {
		if acker != nil {
			snap.Ack = acker.AckOf(p.Peer)
		}
		// Лут в кооперативе персональный: чужие предметы игроку даже не шлём —
		// так их нельзя ни увидеть, ни узнать о них из трафика.
		snap.Items = ownedItems(allItems, p.Peer)
		if err := p.Conn.Send(proto.TSnapshot, snap); err != nil {
			r.log.Warn("снапшот не ушёл", "peer", p.Peer, "err", err)
			r.dead = append(r.dead, p.Peer)
		}
	}
	r.reapDead()
}

// reapDead убирает игроков, отвалившихся во время рассылки: удалять их прямо
// в цикле нельзя, а слать в собственный канал leaves — риск дедлока.
func (r *Room) reapDead() {
	for _, peer := range r.dead {
		r.doLeave(peer)
	}
	r.dead = r.dead[:0]
}

func (r *Room) broadcastEvent(e proto.Event) {
	data, err := proto.Marshal(proto.TEvent, e)
	if err != nil {
		r.log.Error("не сериализовать событие", "kind", e.Kind, "err", err)
		return
	}
	for _, p := range r.players {
		if err := p.Conn.SendRaw(data); err != nil {
			r.dead = append(r.dead, p.Peer)
		}
	}
}

// ownedItems оставляет предметы без владельца и предметы этого игрока.
func ownedItems(items []proto.Ent, peer uint16) []proto.Ent {
	if len(items) == 0 {
		return nil
	}
	out := make([]proto.Ent, 0, len(items))
	for _, item := range items {
		if item.Owner == 0 || item.Owner == peer {
			out = append(out, item)
		}
	}
	return out
}

func sanitizeChat(text string) string {
	out := make([]rune, 0, len(text))
	for _, ch := range text {
		if ch == '\n' || ch == '\r' || ch == '\t' {
			ch = ' '
		}
		if ch < 0x20 {
			continue
		}
		out = append(out, ch)
		if len(out) >= chatMaxLen {
			break
		}
	}
	return string(out)
}
