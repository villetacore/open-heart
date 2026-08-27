package proto

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

// Те же образцы сообщений разбирает Rust-тест
// (core/src/protocol_fixtures_tests.rs). Разъедется формат — упадёт одна из сторон,
// а не игрок на живом сервере.
func fixture(t *testing.T, name string) Envelope {
	t.Helper()
	path := filepath.Join("..", "..", "..", "protocol", "fixtures", name)
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("не прочитать %s: %v", path, err)
	}
	var env Envelope
	if err := json.Unmarshal(raw, &env); err != nil {
		t.Fatalf("%s: битый конверт: %v", name, err)
	}
	return env
}

func TestClientFixtures(t *testing.T) {
	env := fixture(t, "hello.json")
	if env.T != THello {
		t.Fatalf("тег %q, ожидался %q", env.T, THello)
	}
	var hello Hello
	if err := env.Into(&hello); err != nil {
		t.Fatal(err)
	}
	if hello.Protocol != Version {
		t.Errorf("protocol = %d, ожидался %d", hello.Protocol, Version)
	}
	if hello.Auth.Mode != "open" || hello.Auth.Nickname != "vasya" {
		t.Errorf("auth разобрался неверно: %+v", hello.Auth)
	}

	env = fixture(t, "input.json")
	var input Input
	if err := env.Into(&input); err != nil {
		t.Fatal(err)
	}
	if input.Tick != 42 || !input.OnFloor || input.Buttons != 6 {
		t.Errorf("ввод разобрался неверно: %+v", input)
	}
	if input.Pos[1] != 1.1 {
		t.Errorf("позиция разобралась неверно: %v", input.Pos)
	}

	env = fixture(t, "hit.json")
	var hit HitClaim
	if err := env.Into(&hit); err != nil {
		t.Fatal(err)
	}
	if hit.Target != 1000 || hit.Weapon != "pistol" || hit.Part != 1 {
		t.Errorf("заявка о попадании разобралась неверно: %+v", hit)
	}
}

func TestServerFixtures(t *testing.T) {
	env := fixture(t, "welcome.json")
	var welcome Welcome
	if err := env.Into(&welcome); err != nil {
		t.Fatal(err)
	}
	if welcome.Peer != 3 || welcome.World.Kind != "delve" || len(welcome.Players) != 2 {
		t.Errorf("welcome разобрался неверно: %+v", welcome)
	}
	if welcome.World.Seed != 11951638873787587581 {
		t.Errorf("сид потерял точность: %d", welcome.World.Seed)
	}

	env = fixture(t, "snap.json")
	var snap Snapshot
	if err := env.Into(&snap); err != nil {
		t.Fatal(err)
	}
	if snap.Tick != 181 || snap.Ack != 42 || len(snap.Players) != 1 {
		t.Errorf("снапшот разобрался неверно: %+v", snap)
	}
	enemy := snap.Enemies[0]
	if enemy.ID != 1000 || enemy.TypeID != 3 || enemy.Flags != FlagElite {
		t.Errorf("враг разобрался неверно: %+v", enemy)
	}

	env = fixture(t, "event.json")
	var event Event
	if err := env.Into(&event); err != nil {
		t.Fatal(err)
	}
	if event.Kind != EvEnemyDied || event.Target != 1000 {
		t.Errorf("событие разобралось неверно: %+v", event)
	}

	env = fixture(t, "reject.json")
	var reject Reject
	if err := env.Into(&reject); err != nil {
		t.Fatal(err)
	}
	if reject.Reason != RejContent || reject.Detail == "" {
		t.Errorf("отказ разобрался неверно: %+v", reject)
	}
}
