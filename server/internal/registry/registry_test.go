package registry

import (
	"testing"
	"time"
)

func entry(id, preset, hash string, players int) Entry {
	return Entry{
		ServerID: id, Name: id, PresetID: preset, ContentHash: hash,
		Players: players, MaxPlayers: 4, Protocol: 1, Endpoint: "ws://x/ws",
	}
}

func TestListFiltersAndSorts(t *testing.T) {
	r := New(time.Minute)
	r.Announce(entry("a", "core", "h1", 1))
	r.Announce(entry("b", "core", "h1", 3))
	r.Announce(entry("c", "arena", "h2", 0))

	all := r.List(Filter{})
	if len(all) != 3 {
		t.Fatalf("ожидались 3 сервера, получено %d", len(all))
	}
	if all[0].ServerID != "b" {
		t.Errorf("список должен начинаться с самого живого, а начинается с %q", all[0].ServerID)
	}

	byPreset := r.List(Filter{Preset: "arena"})
	if len(byPreset) != 1 || byPreset[0].ServerID != "c" {
		t.Errorf("фильтр по пресету не сработал: %+v", byPreset)
	}

	// Несовпадение данных пресета = сервер несовместим и не показывается.
	if got := r.List(Filter{ContentHash: "h2"}); len(got) != 1 || got[0].ServerID != "c" {
		t.Errorf("фильтр по content_hash не сработал: %+v", got)
	}
}

func TestEntriesExpire(t *testing.T) {
	r := New(20 * time.Millisecond)
	r.Announce(entry("a", "core", "h1", 1))
	if len(r.List(Filter{})) != 1 {
		t.Fatal("свежий анонс должен быть виден")
	}
	time.Sleep(40 * time.Millisecond)
	if got := r.List(Filter{}); len(got) != 0 {
		t.Errorf("протухший анонс всё ещё виден: %+v", got)
	}
	if n := r.Sweep(); n != 1 {
		t.Errorf("Sweep убрал %d записей, ожидалась 1", n)
	}
}

func TestPartyCodes(t *testing.T) {
	c := NewCodes(time.Minute)
	code, exp, err := c.Create("srv1", "wss://relay/j/tok")
	if err != nil {
		t.Fatal(err)
	}
	if len(code) != CodeLen {
		t.Fatalf("код %q длины %d, ожидалось %d", code, len(code), CodeLen)
	}
	for _, ch := range code {
		// В коде не должно быть символов, которые путают при диктовке.
		if ch == '0' || ch == 'O' || ch == '1' || ch == 'I' {
			t.Errorf("в коде %q неоднозначный символ %q", code, ch)
		}
	}
	if exp.Before(time.Now()) {
		t.Error("код выдан уже просроченным")
	}

	// Регистр вводить необязательно.
	id, endpoint, ok := c.Resolve(lower(code))
	if !ok || id != "srv1" || endpoint != "wss://relay/j/tok" {
		t.Fatalf("код не разрешился: %q %q %v", id, endpoint, ok)
	}

	c.Drop(code)
	if _, _, ok := c.Resolve(code); ok {
		t.Error("снятый код всё ещё работает")
	}
}

func TestExpiredCodeIsRejected(t *testing.T) {
	c := NewCodes(10 * time.Millisecond)
	code, _, err := c.Create("srv1", "wss://relay/j/tok")
	if err != nil {
		t.Fatal(err)
	}
	time.Sleep(30 * time.Millisecond)
	if _, _, ok := c.Resolve(code); ok {
		t.Error("истёкший код должен отвергаться")
	}
}

func lower(s string) string {
	out := []rune(s)
	for i, ch := range out {
		if ch >= 'A' && ch <= 'Z' {
			out[i] = ch + ('a' - 'A')
		}
	}
	return string(out)
}
