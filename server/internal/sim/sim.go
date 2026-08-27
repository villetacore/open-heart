// Package sim — мост к авторитетной симуляции.
//
// Симуляция живёт в openheart-core (Rust) и подключается как core.wasm через wazero:
// один и тот же код считает игру на клиенте и на сервере (docs/MULTIPLAYER.md §4).
// Пока ядро не собрано, работает встроенная заглушка Stub — она умеет только держать
// игроков и проверять правдоподобие их движения, врагов и урона в ней нет.
//
// Экземпляр Sim не потокобезопасен: им владеет горутина своей комнаты.
package sim

import (
	"fmt"
	"log/slog"
	"os"

	"github.com/villetacore/open-heart/server/internal/proto"
)

// Config — параметры создания симуляции комнаты.
type Config struct {
	Preset    string `json:"preset"`
	Kind      string `json:"kind"` // hub | delve
	Seed      uint64 `json:"seed"`
	Depth     int    `json:"depth"`
	PartySize int    `json:"party_size"`
	// Каталог пресета: ядро читает оттуда enemies.json и остальные данные.
	PresetBase string `json:"preset_base"`
}

// Sim — авторитетная симуляция одной комнаты.
type Sim interface {
	// AddPlayer заводит игрока с уже выданным peer-идентификатором.
	AddPlayer(id uint16, nickname string) error
	RemovePlayer(id uint16)

	// Input принимает ввод и заявленную клиентом позицию.
	Input(id uint16, in proto.Input) []proto.Event
	Fire(id uint16, f proto.Fire) []proto.Event
	Hit(id uint16, h proto.HitClaim) []proto.Event
	Cmd(id uint16, c proto.Cmd) []proto.Event

	// Tick продвигает мир на dt секунд.
	Tick(dt float64) (proto.Snapshot, []proto.Event)

	// Save сериализует состояние комнаты для персиста.
	Save() ([]byte, error)
	Close() error
}

// New создаёт симуляцию: ядро из corePath, либо заглушку, если путь пуст.
func New(cfg Config, corePath string, log *slog.Logger) (Sim, error) {
	if corePath == "" {
		log.Warn("симуляция: core.wasm не задан, работает заглушка (нет врагов и урона)")
		return NewStub(cfg), nil
	}
	data, err := os.ReadFile(corePath)
	if err != nil {
		return nil, fmt.Errorf("sim: не прочитать ядро %s: %w", corePath, err)
	}
	return NewWasm(data, cfg, log)
}
