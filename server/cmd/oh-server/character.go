package main

import (
	"encoding/json"
	"strconv"

	"github.com/villetacore/open-heart/server/internal/proto"
	"github.com/villetacore/open-heart/server/internal/room"
	"github.com/villetacore/open-heart/server/internal/store"
)

// loadCharacter достаёт персонажа, которого сервер помнит за этим игроком.
//
// У гостя (`auth=open`) `UserID` нулевой: прогресс такому серверу хранить не
// за кем, и это честно — вход без имени и пароля персонажа не заводит.
func (s *server) loadCharacter(player *room.Player) *proto.Save {
	if player.UserID == 0 {
		return nil
	}
	list, err := s.db.Characters(player.UserID, s.cfg.Preset)
	if err != nil {
		s.log.Error("не прочитать персонажа", "user", player.UserID, "err", err)
		return nil
	}
	if len(list) == 0 {
		return nil
	}
	// Свежий первый: Characters сортирует по updated_at.
	character := list[0]
	player.CharacterID = character.ID
	return &proto.Save{Ver: character.SaveVer, Data: character.SaveJSON}
}

// storeCharacter сохраняет присланное клиентом состояние.
//
// Сервер в блоб не заглядывает — только вынимает уровень и класс, чтобы их
// было видно в списке персонажей и в логах. Разбор появится, когда прогресс
// переедет в ядро (docs/MULTIPLAYER.md §12, N7).
func (s *server) storeCharacter(player *room.Player, save proto.Save) {
	if player.UserID == 0 {
		return
	}
	if len(save.Data) > proto.MaxSaveBytes {
		s.log.Warn("сейв больше допустимого", "peer", player.Peer, "байт", len(save.Data))
		return
	}
	if save.Data == "" {
		return
	}

	character := store.Character{
		ID:       player.CharacterID,
		UserID:   player.UserID,
		Preset:   s.cfg.Preset,
		Name:     player.Nickname,
		SaveVer:  save.Ver,
		SaveJSON: save.Data,
	}
	fillSummary(&character, save.Data)

	id, err := s.db.SaveCharacter(&character)
	if err != nil {
		s.log.Error("не сохранить персонажа", "user", player.UserID, "err", err)
		return
	}
	player.CharacterID = id
}

// fillSummary вытаскивает из сейва то немногое, что серверу полезно видеть.
//
// Битый или чужой формат — не повод отказать в сохранении: блоб принадлежит
// клиенту, а сводка нужна только для показа.
func fillSummary(character *store.Character, data string) {
	var summary struct {
		Level    int    `json:"level"`
		XP       int    `json:"xp"`
		ClassID  string `json:"class_id"`
		SpecID   string `json:"spec_id"`
		ClassIdx *int   `json:"class_idx"`
		SpecIdx  *int   `json:"spec_idx"`
	}
	if err := json.Unmarshal([]byte(data), &summary); err != nil {
		return
	}
	character.Level = summary.Level
	character.XP = summary.XP
	character.ClassID = summary.ClassID
	character.SpecID = summary.SpecID
	// Старый формат хранит класс индексом — сохраняем хоть его.
	if character.ClassID == "" && summary.ClassIdx != nil {
		character.ClassID = strconv.Itoa(*summary.ClassIdx)
	}
	if character.SpecID == "" && summary.SpecIdx != nil {
		character.SpecID = strconv.Itoa(*summary.SpecIdx)
	}
}
