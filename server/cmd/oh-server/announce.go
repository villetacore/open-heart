package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"strings"
	"time"

	"github.com/villetacore/open-heart/server/internal/proto"
	"github.com/villetacore/open-heart/server/internal/registry"
)

// announceLoop держит запись сервера в списке мастера живой.
// Запись протухает за 60 с, поэтому heartbeat идёт раз в 20 с.
func (s *server) announceLoop(ctx context.Context) {
	// У релейного сервера точку входа выдаёт мастер, и только когда туннель
	// поднят: анонс раньше этого момента мастер отклонит.
	if s.cfg.Relay {
		select {
		case <-s.relayUp:
		case <-ctx.Done():
			return
		}
	}

	client := &http.Client{Timeout: 10 * time.Second}
	url := strings.TrimRight(s.cfg.Master, "/") + "/v1/servers/announce"

	t := time.NewTicker(registry.HeartbeatPeriod)
	defer t.Stop()

	fails := 0
	for {
		if err := s.announceOnce(ctx, client, url); err != nil {
			fails++
			// Не спамим логом: мастер может лежать, игру это не ломает.
			if fails == 1 || fails%15 == 0 {
				s.log.Warn("анонс не прошёл", "err", err, "подряд", fails)
			}
		} else if fails > 0 {
			s.log.Info("анонс восстановлен", "после", fails)
			fails = 0
		}

		select {
		case <-ctx.Done():
			s.removeFromList(client)
			return
		case <-t.C:
		}
	}
}

func (s *server) announceOnce(ctx context.Context, client *http.Client, url string) error {
	entry := registry.Entry{
		ServerID:    s.cfg.ServerID,
		Name:        s.cfg.Name,
		MOTD:        s.cfg.MOTD,
		Region:      s.cfg.Region,
		Players:     s.rooms.Players(),
		MaxPlayers:  s.cfg.MaxPlayers,
		PresetID:    s.cfg.Preset,
		ContentHash: s.contentHash,
		GameVersion: Version,
		Protocol:    proto.Version,
		Mode:        "pve",
		Passworded:  s.cfg.Password != "",
		AuthMode:    s.cfg.Auth,
		Endpoint:    s.cfg.PublicURL,
		Relayed:     s.cfg.Relay,
	}
	body, err := json.Marshal(struct {
		registry.Entry
		Secret string `json:"secret"`
	}{Entry: entry, Secret: s.cfg.Secret})
	if err != nil {
		return err
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodPost, url, bytes.NewReader(body))
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/json")

	resp, err := client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("мастер ответил %s", resp.Status)
	}
	return nil
}

// removeFromList снимает сервер со списка при штатной остановке.
func (s *server) removeFromList(client *http.Client) {
	url := strings.TrimRight(s.cfg.Master, "/") + "/v1/servers/remove"
	body, err := json.Marshal(map[string]string{
		"server_id": s.cfg.ServerID,
		"secret":    s.cfg.Secret,
	})
	if err != nil {
		return
	}
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, url, bytes.NewReader(body))
	if err != nil {
		return
	}
	req.Header.Set("Content-Type", "application/json")
	resp, err := client.Do(req)
	if err != nil {
		return
	}
	resp.Body.Close()
}
