package room

import (
	"testing"

	"github.com/villetacore/open-heart/server/internal/proto"
)

// Лут в кооперативе персональный: чужие предметы игроку не уходят даже по проводу.
func TestOwnedItems(t *testing.T) {
	items := []proto.Ent{
		{ID: 1, Kind: proto.KindItem, Owner: 1},
		{ID: 2, Kind: proto.KindItem, Owner: 2},
		{ID: 3, Kind: proto.KindItem, Owner: 0}, // общий
	}

	first := ownedItems(items, 1)
	if len(first) != 2 {
		t.Fatalf("игроку 1 ушло %d предметов, ожидалось 2 (свой + общий)", len(first))
	}
	for _, item := range first {
		if item.Owner == 2 {
			t.Errorf("чужой предмет %d просочился игроку 1", item.ID)
		}
	}

	second := ownedItems(items, 2)
	if len(second) != 2 {
		t.Fatalf("игроку 2 ушло %d предметов, ожидалось 2", len(second))
	}

	third := ownedItems(items, 3)
	if len(third) != 1 || third[0].ID != 3 {
		t.Errorf("постороннему должен уйти только общий предмет, ушло: %+v", third)
	}

	if ownedItems(nil, 1) != nil {
		t.Error("пустой список должен оставаться пустым")
	}
}
