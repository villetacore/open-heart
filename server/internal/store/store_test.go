package store

import (
	"errors"
	"path/filepath"
	"testing"
)

func openServer(t *testing.T) *Server {
	t.Helper()
	db, err := OpenServer(filepath.Join(t.TempDir(), "server.db"))
	if err != nil {
		t.Fatalf("не открыть базу: %v", err)
	}
	t.Cleanup(func() { db.Close() })
	return db
}

// Персонаж должен переживать закрытие базы: ради этого он там и лежит.
func TestCharacterSurvivesReopen(t *testing.T) {
	path := filepath.Join(t.TempDir(), "server.db")
	db, err := OpenServer(path)
	if err != nil {
		t.Fatalf("не открыть базу: %v", err)
	}
	user, err := db.CreateUser("local:vasya", "Вася", "hash")
	if err != nil {
		t.Fatalf("не завести игрока: %v", err)
	}
	id, err := db.SaveCharacter(&Character{
		UserID: user.ID, Preset: "core", Name: "Вася",
		ClassID: "medic", Level: 13, XP: 77, SaveVer: 4, SaveJSON: `{"level":13}`,
	})
	if err != nil {
		t.Fatalf("не сохранить персонажа: %v", err)
	}
	db.Close()

	db, err = OpenServer(path)
	if err != nil {
		t.Fatalf("не переоткрыть базу: %v", err)
	}
	defer db.Close()

	list, err := db.Characters(user.ID, "core")
	if err != nil {
		t.Fatalf("не прочитать персонажей: %v", err)
	}
	if len(list) != 1 {
		t.Fatalf("персонажей %d, ожидался один", len(list))
	}
	if list[0].ID != id || list[0].Level != 13 || list[0].SaveJSON != `{"level":13}` {
		t.Fatalf("персонаж вернулся другим: %+v", list[0])
	}
}

// Повторное сохранение обновляет ту же строку, а не плодит персонажей.
func TestCharacterUpdatesInPlace(t *testing.T) {
	db := openServer(t)
	user, err := db.CreateUser("local:petya", "Петя", "hash")
	if err != nil {
		t.Fatal(err)
	}
	character := &Character{UserID: user.ID, Preset: "core", Name: "Петя", Level: 1, SaveJSON: "{}"}
	id, err := db.SaveCharacter(character)
	if err != nil {
		t.Fatal(err)
	}
	character.ID = id
	character.Level = 7
	character.SaveJSON = `{"level":7}`
	if _, err := db.SaveCharacter(character); err != nil {
		t.Fatal(err)
	}

	list, err := db.Characters(user.ID, "core")
	if err != nil {
		t.Fatal(err)
	}
	if len(list) != 1 {
		t.Fatalf("персонажей %d, ожидался один", len(list))
	}
	if list[0].Level != 7 {
		t.Fatalf("уровень %d, ожидался 7", list[0].Level)
	}
}

// Персонажи разных пресетов не путаются: это разные игры.
func TestCharactersAreSeparatedByPreset(t *testing.T) {
	db := openServer(t)
	user, err := db.CreateUser("local:kolya", "Коля", "hash")
	if err != nil {
		t.Fatal(err)
	}
	for _, preset := range []string{"core", "arena"} {
		if _, err := db.SaveCharacter(&Character{
			UserID: user.ID, Preset: preset, Name: "Коля", SaveJSON: `{"p":"` + preset + `"}`,
		}); err != nil {
			t.Fatal(err)
		}
	}
	list, err := db.Characters(user.ID, "arena")
	if err != nil {
		t.Fatal(err)
	}
	if len(list) != 1 || list[0].SaveJSON != `{"p":"arena"}` {
		t.Fatalf("пресеты перемешались: %+v", list)
	}
}

// Временная личность хоста живёт по тем же правилам, что и обычный сервер,
// но без владельца — и опознаётся тем же CheckServer.
func TestPartyHostChecksLikeAServer(t *testing.T) {
	db, err := OpenMaster(filepath.Join(t.TempDir(), "master.db"))
	if err != nil {
		t.Fatalf("не открыть базу мастера: %v", err)
	}
	defer db.Close()

	id, secret, err := db.NewPartyHost()
	if err != nil {
		t.Fatalf("не выдать личность: %v", err)
	}
	row, err := db.CheckServer(id, secret)
	if err != nil {
		t.Fatalf("личность не опознана: %v", err)
	}
	if row.Listed {
		t.Error("хост игры с другом не должен считаться опубликованным")
	}
	if _, err := db.CheckServer(id, "не тот секрет"); err == nil {
		t.Error("чужой секрет пустили")
	}
	if _, err := db.CheckServer("нет-такого", secret); !errors.Is(err, ErrNotFound) {
		t.Errorf("ожидался ErrNotFound, получено %v", err)
	}
}
