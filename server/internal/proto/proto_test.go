package proto

import (
	"encoding/json"
	"strings"
	"testing"
)

// Имена полей на проводе фиксированы в protocol/schema.md; та же проверка есть
// на стороне Rust (core/src/sim.rs). Если тест упал — обнови обе стороны.
func TestWireFieldNames(t *testing.T) {
	data, err := Marshal(TInput, Input{Tick: 5, Move: [2]float32{1, 0}, Yaw: 1.5, OnFloor: true})
	if err != nil {
		t.Fatal(err)
	}
	s := string(data)
	for _, want := range []string{`"t":"input"`, `"tick":5`, `"move":[1,0]`, `"on_floor":true`} {
		if !strings.Contains(s, want) {
			t.Errorf("в %s нет %s", s, want)
		}
	}
}

func TestEnvelopeRoundTrip(t *testing.T) {
	data, err := Marshal(TWelcome, Welcome{
		Peer:     3,
		PresetID: "core",
		World:    World{Kind: "delve", Seed: 42, Depth: 2},
		Players:  []PlayerInfo{{Peer: 3, Nickname: "vasya"}},
	})
	if err != nil {
		t.Fatal(err)
	}

	var env Envelope
	if err := json.Unmarshal(data, &env); err != nil {
		t.Fatal(err)
	}
	if env.T != TWelcome {
		t.Fatalf("тег %q, ожидался %q", env.T, TWelcome)
	}

	var got Welcome
	if err := env.Into(&got); err != nil {
		t.Fatal(err)
	}
	if got.Peer != 3 || got.World.Seed != 42 || len(got.Players) != 1 {
		t.Fatalf("развалилось: %+v", got)
	}
}

// Неизвестные поля игнорируются: новый клиент не должен ронять старый сервер.
func TestUnknownFieldsIgnored(t *testing.T) {
	raw := `{"t":"hello","d":{"protocol":1,"game_version":"9.9","новое_поле":true}}`
	var env Envelope
	if err := json.Unmarshal([]byte(raw), &env); err != nil {
		t.Fatal(err)
	}
	var hello Hello
	if err := env.Into(&hello); err != nil {
		t.Fatalf("hello с лишним полем не разобрался: %v", err)
	}
	if hello.Protocol != Version {
		t.Fatalf("protocol = %d", hello.Protocol)
	}
}

func TestEntIsCompact(t *testing.T) {
	data, err := json.Marshal(Ent{ID: 41, Kind: KindEnemy, Pos: Vec3{12.5, 0, -3.25}, Yaw: 1.5, HP: 220, Flags: 3})
	if err != nil {
		t.Fatal(err)
	}
	// Ориентир из schema.md: одна сущность в JSON — около 70 байт.
	if len(data) > 90 {
		t.Errorf("запись сущности разрослась до %d байт: %s", len(data), data)
	}
}
