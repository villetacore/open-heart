package sim

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"

	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"

	"github.com/villetacore/open-heart/server/internal/proto"
)

// Экспорты ядра. ABI описан в docs/MULTIPLAYER.md §7.2; буферы передаются
// через линейную память, результат — упакованная пара (ptr<<32 | len).
const (
	fnAlloc = "sim_alloc"
	fnFree  = "sim_free"
	fnNew   = "sim_new"
	fnDrop  = "sim_drop"
	fnTick  = "sim_tick"
	fnCmd   = "sim_cmd"
	fnSave  = "sim_save"
	fnLoad  = "sim_load"
)

// Wasm — симуляция, исполняемая ядром openheart-core внутри wazero.
// Не потокобезопасна: принадлежит горутине своей комнаты.
type Wasm struct {
	ctx    context.Context
	rt     wazero.Runtime
	mod    api.Module
	handle uint64
	log    *slog.Logger

	acks    map[uint16]uint32
	inputs  []taggedInput
	pending []proto.Event
}

// taggedInput — ввод одного игрока за тик в том виде, в каком его ждёт ядро.
type taggedInput struct {
	Peer uint16      `json:"peer"`
	In   proto.Input `json:"in"`
}

// tickResult — то, что ядро возвращает из sim_tick.
type tickResult struct {
	Snapshot proto.Snapshot `json:"snapshot"`
	Events   []proto.Event  `json:"events"`
}

// coreCmd — универсальная команда ядру (выстрел, попадание, действие, вход/выход игрока).
type coreCmd struct {
	Kind string          `json:"kind"`
	Peer uint16          `json:"peer"`
	Data json.RawMessage `json:"data,omitempty"`
}

// NewWasm поднимает рантайм и создаёт симуляцию комнаты.
func NewWasm(module []byte, cfg Config, log *slog.Logger) (*Wasm, error) {
	ctx := context.Background()
	rt := wazero.NewRuntime(ctx)

	compiled, err := rt.CompileModule(ctx, module)
	if err != nil {
		rt.Close(ctx)
		return nil, fmt.Errorf("sim: не скомпилировать ядро: %w", err)
	}
	mod, err := rt.InstantiateModule(ctx, compiled,
		wazero.NewModuleConfig().WithName("").WithStartFunctions())
	if err != nil {
		rt.Close(ctx)
		return nil, fmt.Errorf("sim: не создать инстанс ядра: %w", err)
	}

	w := &Wasm{ctx: ctx, rt: rt, mod: mod, log: log, acks: make(map[uint16]uint32)}
	for _, name := range []string{fnAlloc, fnFree, fnNew, fnTick, fnCmd} {
		if mod.ExportedFunction(name) == nil {
			w.Close()
			return nil, fmt.Errorf("sim: ядро не экспортирует %s", name)
		}
	}

	cfgJSON, err := json.Marshal(cfg)
	if err != nil {
		w.Close()
		return nil, err
	}
	ptr, length, err := w.writeBuf(cfgJSON)
	if err != nil {
		w.Close()
		return nil, err
	}
	defer w.freeBuf(ptr, length)

	handle, err := w.call(fnNew, uint64(ptr), uint64(length))
	if err != nil {
		w.Close()
		return nil, err
	}
	if handle == 0 {
		w.Close()
		return nil, fmt.Errorf("sim: ядро отказалось создавать комнату (пресет %q)", cfg.Preset)
	}
	w.handle = handle
	return w, nil
}

func (w *Wasm) AddPlayer(id uint16, nickname string) error {
	data, _ := json.Marshal(map[string]string{"nickname": nickname})
	_, err := w.cmd(coreCmd{Kind: "join", Peer: id, Data: data})
	return err
}

func (w *Wasm) RemovePlayer(id uint16) {
	if _, err := w.cmd(coreCmd{Kind: "leave", Peer: id}); err != nil {
		w.log.Error("sim: leave", "peer", id, "err", err)
	}
	delete(w.acks, id)
}

// Input копится до конца тика: ядро получает весь батч разом.
func (w *Wasm) Input(id uint16, in proto.Input) []proto.Event {
	w.acks[id] = in.Tick
	w.inputs = append(w.inputs, taggedInput{Peer: id, In: in})
	return nil
}

func (w *Wasm) Fire(id uint16, f proto.Fire) []proto.Event {
	return w.cmdEvents("fire", id, f)
}

func (w *Wasm) Hit(id uint16, h proto.HitClaim) []proto.Event {
	return w.cmdEvents("hit", id, h)
}

func (w *Wasm) Cmd(id uint16, c proto.Cmd) []proto.Event {
	data, err := json.Marshal(c)
	if err != nil {
		return nil
	}
	events, err := w.cmd(coreCmd{Kind: "cmd", Peer: id, Data: data})
	if err != nil {
		w.log.Error("sim: cmd", "peer", id, "kind", c.Kind, "err", err)
		return nil
	}
	return events
}

func (w *Wasm) Tick(dt float64) (proto.Snapshot, []proto.Event) {
	payload, err := json.Marshal(w.inputs)
	w.inputs = w.inputs[:0]
	if err != nil {
		w.log.Error("sim: сериализация ввода", "err", err)
		return proto.Snapshot{}, nil
	}
	ptr, length, err := w.writeBuf(payload)
	if err != nil {
		w.log.Error("sim: запись ввода в память ядра", "err", err)
		return proto.Snapshot{}, nil
	}
	defer w.freeBuf(ptr, length)

	packed, err := w.call(fnTick, w.handle, uint64(uint32(dt*1000)), uint64(ptr), uint64(length))
	if err != nil {
		w.log.Error("sim: tick", "err", err)
		return proto.Snapshot{}, nil
	}
	raw, err := w.takeResult(packed)
	if err != nil {
		w.log.Error("sim: чтение результата тика", "err", err)
		return proto.Snapshot{}, nil
	}
	var res tickResult
	if err := json.Unmarshal(raw, &res); err != nil {
		w.log.Error("sim: разбор результата тика", "err", err)
		return proto.Snapshot{}, nil
	}
	events := res.Events
	if len(w.pending) > 0 {
		events = append(w.pending, events...)
		w.pending = w.pending[:0]
	}
	return res.Snapshot, events
}

// AckOf — последний учтённый Input.Tick игрока.
func (w *Wasm) AckOf(id uint16) uint32 { return w.acks[id] }

func (w *Wasm) Save() ([]byte, error) {
	if w.mod.ExportedFunction(fnSave) == nil {
		return nil, fmt.Errorf("sim: ядро не экспортирует %s", fnSave)
	}
	packed, err := w.call(fnSave, w.handle)
	if err != nil {
		return nil, err
	}
	return w.takeResult(packed)
}

func (w *Wasm) Close() error {
	if w.handle != 0 && w.mod.ExportedFunction(fnDrop) != nil {
		if _, err := w.call(fnDrop, w.handle); err != nil {
			w.log.Error("sim: drop", "err", err)
		}
		w.handle = 0
	}
	return w.rt.Close(w.ctx)
}

// ── низкий уровень ───────────────────────────────────────────────────────────

func (w *Wasm) cmdEvents(kind string, peer uint16, payload any) []proto.Event {
	data, err := json.Marshal(payload)
	if err != nil {
		return nil
	}
	events, err := w.cmd(coreCmd{Kind: kind, Peer: peer, Data: data})
	if err != nil {
		w.log.Error("sim: команда ядру", "kind", kind, "peer", peer, "err", err)
		return nil
	}
	return events
}

func (w *Wasm) cmd(c coreCmd) ([]proto.Event, error) {
	payload, err := json.Marshal(c)
	if err != nil {
		return nil, err
	}
	ptr, length, err := w.writeBuf(payload)
	if err != nil {
		return nil, err
	}
	defer w.freeBuf(ptr, length)

	packed, err := w.call(fnCmd, w.handle, uint64(ptr), uint64(length))
	if err != nil {
		return nil, err
	}
	raw, err := w.takeResult(packed)
	if err != nil || len(raw) == 0 {
		return nil, err
	}
	var events []proto.Event
	if err := json.Unmarshal(raw, &events); err != nil {
		return nil, err
	}
	return events, nil
}

func (w *Wasm) call(name string, args ...uint64) (uint64, error) {
	fn := w.mod.ExportedFunction(name)
	if fn == nil {
		return 0, fmt.Errorf("sim: ядро не экспортирует %s", name)
	}
	res, err := fn.Call(w.ctx, args...)
	if err != nil {
		return 0, fmt.Errorf("sim: %s: %w", name, err)
	}
	if len(res) == 0 {
		return 0, nil
	}
	return res[0], nil
}

// writeBuf выделяет буфер в памяти ядра и копирует туда данные.
func (w *Wasm) writeBuf(data []byte) (ptr uint32, length uint32, err error) {
	if len(data) == 0 {
		return 0, 0, nil
	}
	raw, err := w.call(fnAlloc, uint64(len(data)))
	if err != nil {
		return 0, 0, err
	}
	ptr = uint32(raw)
	if ptr == 0 {
		return 0, 0, fmt.Errorf("sim: ядро не выделило %d байт", len(data))
	}
	if !w.mod.Memory().Write(ptr, data) {
		w.freeBuf(ptr, uint32(len(data)))
		return 0, 0, fmt.Errorf("sim: запись %d байт вне памяти ядра", len(data))
	}
	return ptr, uint32(len(data)), nil
}

func (w *Wasm) freeBuf(ptr, length uint32) {
	if ptr == 0 || length == 0 {
		return
	}
	if _, err := w.call(fnFree, uint64(ptr), uint64(length)); err != nil {
		w.log.Error("sim: освобождение буфера", "err", err)
	}
}

// takeResult читает результат вида (ptr<<32 | len) и освобождает буфер ядра.
func (w *Wasm) takeResult(packed uint64) ([]byte, error) {
	ptr := uint32(packed >> 32)
	length := uint32(packed)
	if ptr == 0 || length == 0 {
		return nil, nil
	}
	defer w.freeBuf(ptr, length)
	data, ok := w.mod.Memory().Read(ptr, length)
	if !ok {
		return nil, fmt.Errorf("sim: результат (%d, %d) вне памяти ядра", ptr, length)
	}
	out := make([]byte, len(data))
	copy(out, data)
	return out, nil
}
