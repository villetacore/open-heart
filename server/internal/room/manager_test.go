package room

import (
	"context"
	"io"
	"log/slog"
	"testing"

	"github.com/villetacore/open-heart/server/internal/sim"
)

func quietLog() *slog.Logger {
	return slog.New(slog.NewTextHandler(io.Discard, nil))
}

func testManager(t *testing.T) (*Manager, context.CancelFunc) {
	t.Helper()
	ctx, cancel := context.WithCancel(context.Background())
	log := quietLog()

	hub := New(Config{ID: "hub", Kind: "hub", MaxPlayers: 8}, sim.NewStub(sim.Config{}), log)
	go hub.Run(ctx)

	manager := NewManager(ctx, hub, log, func(seed uint64, depth int) (sim.Sim, Config, error) {
		return sim.NewStub(sim.Config{Kind: "delve", Seed: seed, Depth: depth}),
			Config{MaxPlayers: 4, TickRate: 20}, nil
	})
	return manager, cancel
}

func TestDelveIDIsStablePerOwnerAndDepth(t *testing.T) {
	if DelveID(3, 2) != DelveID(3, 2) {
		t.Fatal("id забега обязан быть стабильным")
	}
	if DelveID(3, 2) == DelveID(4, 2) {
		t.Error("у разных ведущих должны быть разные забеги")
	}
	if DelveID(3, 2) == DelveID(3, 3) {
		t.Error("разные глубины — разные забеги")
	}
}

// Товарищи должны попадать в тот же данж, а не каждый в свою копию.
func TestOpenDelveReusesInstance(t *testing.T) {
	manager, cancel := testManager(t)
	defer cancel()

	first, err := manager.OpenDelve(1, 2, 42)
	if err != nil {
		t.Fatal(err)
	}
	second, err := manager.OpenDelve(1, 2, 999)
	if err != nil {
		t.Fatal(err)
	}
	if first != second {
		t.Fatal("повторный вход к тому же ведущему обязан дать тот же инстанс")
	}
	if first.ID() != DelveID(1, 2) {
		t.Errorf("id комнаты %q, ожидался %q", first.ID(), DelveID(1, 2))
	}

	other, err := manager.OpenDelve(2, 2, 42)
	if err != nil {
		t.Fatal(err)
	}
	if other == first {
		t.Error("у другого ведущего должен быть свой забег")
	}
}

func TestManagerCountsRoomsAndPlayers(t *testing.T) {
	manager, cancel := testManager(t)
	defer cancel()

	if got := len(manager.Rooms()); got != 1 {
		t.Fatalf("в начале должна быть одна комната, а не %d", got)
	}
	if _, err := manager.OpenDelve(1, 1, 7); err != nil {
		t.Fatal(err)
	}
	if got := len(manager.Rooms()); got != 2 {
		t.Errorf("после создания забега комнат %d, ожидалось 2", got)
	}
	// Игроков нет — счётчик обязан это показывать.
	if got := manager.Players(); got != 0 {
		t.Errorf("игроков %d, ожидалось 0", got)
	}
}
