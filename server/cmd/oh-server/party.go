package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"time"
)

// partyStatus — то, что сервер сообщает о себе запустившему его клиенту.
//
// Через stdout это не передать: клиент запускает нас фоновым процессом и
// читать его вывод построчно из движка неудобно и небезопасно. Файл проще:
// клиент ждёт, пока он появится, и читает целиком.
type partyStatus struct {
	// Ready — сервер готов принимать игроков.
	Ready bool `json:"ready"`
	// Listen — локальный адрес: хозяин заходит на него напрямую, без релея.
	Listen string `json:"listen"`
	// Endpoint — адрес для друзей (через релей мастера), если он есть.
	Endpoint string `json:"endpoint,omitempty"`
	// Code — шестизначный пати-код; им и делятся.
	Code string `json:"code,omitempty"`
	// Error — почему кода нет: игру всё равно можно играть одному.
	Error       string `json:"error,omitempty"`
	Preset      string `json:"preset"`
	ContentHash string `json:"content_hash"`
	PID         int    `json:"pid"`
}

// fetchPartyIdentity просит у мастера временную личность хоста.
//
// Для игры с другом аккаунт не нужен: иначе «позвать друга» упирается в
// регистрацию (docs/MULTIPLAYER.md §8.4).
func fetchPartyIdentity(ctx context.Context, master string) (id, secret string, err error) {
	url := strings.TrimRight(master, "/") + "/v1/party/host"
	request, err := http.NewRequestWithContext(ctx, http.MethodPost, url, bytes.NewReader([]byte("{}")))
	if err != nil {
		return "", "", err
	}
	request.Header.Set("Content-Type", "application/json")

	client := &http.Client{Timeout: 10 * time.Second}
	response, err := client.Do(request)
	if err != nil {
		return "", "", err
	}
	defer response.Body.Close()
	if response.StatusCode != http.StatusOK {
		return "", "", fmt.Errorf("мастер ответил %s", response.Status)
	}
	var reply struct {
		ServerID string `json:"server_id"`
		Secret   string `json:"server_secret"`
	}
	if err := json.NewDecoder(io.LimitReader(response.Body, 8<<10)).Decode(&reply); err != nil {
		return "", "", err
	}
	if reply.ServerID == "" || reply.Secret == "" {
		return "", "", fmt.Errorf("мастер не выдал личность")
	}
	return reply.ServerID, reply.Secret, nil
}

// requestPartyCode берёт у мастера код, которым хозяин зовёт друзей.
func (s *server) requestPartyCode(ctx context.Context) (code, endpoint string, err error) {
	url := strings.TrimRight(s.cfg.Master, "/") + "/v1/party/create"
	body, err := json.Marshal(map[string]string{
		"server_id": s.cfg.ServerID,
		"secret":    s.cfg.Secret,
		"endpoint":  s.cfg.PublicURL,
	})
	if err != nil {
		return "", "", err
	}
	request, err := http.NewRequestWithContext(ctx, http.MethodPost, url, bytes.NewReader(body))
	if err != nil {
		return "", "", err
	}
	request.Header.Set("Content-Type", "application/json")

	client := &http.Client{Timeout: 10 * time.Second}
	response, err := client.Do(request)
	if err != nil {
		return "", "", err
	}
	defer response.Body.Close()
	if response.StatusCode != http.StatusOK {
		return "", "", fmt.Errorf("мастер ответил %s", response.Status)
	}
	var reply struct {
		Code     string `json:"code"`
		Endpoint string `json:"endpoint"`
	}
	if err := json.NewDecoder(io.LimitReader(response.Body, 8<<10)).Decode(&reply); err != nil {
		return "", "", err
	}
	return reply.Code, reply.Endpoint, nil
}

// publishParty дожидается туннеля, берёт код и пишет файл состояния.
//
// Ошибка кода не мешает играть: сервер уже слушает локально, и хозяин зайдёт
// на него в любом случае — просто друга позвать будет нечем.
func (s *server) publishParty(ctx context.Context, path string) {
	status := partyStatus{
		Ready:       true,
		Listen:      s.cfg.Listen,
		Preset:      s.cfg.Preset,
		ContentHash: s.contentHash,
		PID:         os.Getpid(),
	}

	if s.cfg.Relay {
		select {
		case <-s.relayUp:
		case <-ctx.Done():
			return
		case <-time.After(20 * time.Second):
			status.Error = "мастер не отозвался"
			s.writeStatus(path, status)
			return
		}
	}

	if s.cfg.Master != "" && s.cfg.ServerID != "" {
		code, endpoint, err := s.requestPartyCode(ctx)
		if err != nil {
			status.Error = err.Error()
			s.log.Warn("пати-код не получен", "err", err)
		} else {
			status.Code = code
			status.Endpoint = endpoint
			s.log.Info("игра с другом готова", "код", code, "точка входа", endpoint)
		}
	}
	s.writeStatus(path, status)
}

// writeStatus кладёт файл целиком: клиент читает его без блокировок, поэтому
// пишем во временный и переименовываем — половину файла он увидеть не должен.
func (s *server) writeStatus(path string, status partyStatus) {
	data, err := json.MarshalIndent(status, "", " ")
	if err != nil {
		return
	}
	temporary := path + ".tmp"
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		s.log.Warn("не создать каталог для файла состояния", "путь", path, "err", err)
		return
	}
	if err := os.WriteFile(temporary, data, 0o644); err != nil {
		s.log.Warn("не записать файл состояния", "путь", temporary, "err", err)
		return
	}
	if err := os.Rename(temporary, path); err != nil {
		s.log.Warn("не переименовать файл состояния", "путь", path, "err", err)
	}
}

// idleWatch выключает сервер, если он никому не нужен.
//
// Первого игрока ждём столько же, сколько потом простоя: клиент запускает нас
// заранее и подключается через пару секунд, но если он не пришёл вовсе —
// висеть в памяти незачем.
func (s *server) idleWatch(ctx context.Context, idle time.Duration, stop func()) {
	ticker := time.NewTicker(15 * time.Second)
	defer ticker.Stop()

	empty := time.Now()
	for {
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
		}
		if s.rooms.Players() > 0 {
			empty = time.Now()
			continue
		}
		if time.Since(empty) >= idle {
			s.log.Info("никого нет, выключаюсь", "простой", idle)
			stop()
			return
		}
	}
}
