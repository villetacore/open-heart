package main

import (
	"context"
	"io"
	"net/http"
	"strings"
	"time"

	"github.com/coder/websocket"
	"github.com/hashicorp/yamux"

	"github.com/villetacore/open-heart/server/internal/auth"
	"github.com/villetacore/open-heart/server/internal/relay"
	"github.com/villetacore/open-heart/server/internal/wire"
)

// relayConnect — сюда игровой сервер приходит сам, если у него нет белого IP.
//
// Соединение исходящее, поэтому проходит любой NAT и большинство фаерволов:
// снаружи это обычный WebSocket на 443 (docs/MULTIPLAYER.md §8.3).
func (m *master) relayConnect(w http.ResponseWriter, r *http.Request) {
	serverID := r.URL.Query().Get("server_id")
	secret := r.URL.Query().Get("secret")
	if serverID == "" || secret == "" {
		httpErr(w, http.StatusBadRequest, "нужны server_id и secret")
		return
	}
	if _, err := m.db.CheckServer(serverID, secret); err != nil {
		httpErr(w, http.StatusUnauthorized, "сервер не опознан")
		return
	}

	socket, err := websocket.Accept(w, r, &websocket.AcceptOptions{
		InsecureSkipVerify: true,
		CompressionMode:    websocket.CompressionDisabled,
	})
	if err != nil {
		m.log.Warn("релей: не поднять туннель", "server_id", serverID, "err", err)
		return
	}
	// Туннель живёт долго и качает много: снимаем лимит на размер сообщения,
	// границы кадров внутри держит сам протокол релея.
	socket.SetReadLimit(-1)

	ctx := context.Background()
	netConn := websocket.NetConn(ctx, socket, websocket.MessageBinary)

	// Хост звонил нам, поэтому он клиент мультиплексора, а мы — сервер:
	// стримы открываем мы, когда приходит игрок.
	session, err := yamux.Server(netConn, yamuxConfig())
	if err != nil {
		m.log.Warn("релей: не поднять мультиплексор", "server_id", serverID, "err", err)
		_ = socket.Close(websocket.StatusInternalError, "yamux")
		return
	}

	token, err := auth.RandomToken(18)
	if err != nil {
		_ = session.Close()
		return
	}
	host := m.relay.Add(serverID, token, session)
	m.log.Info("релей: сервер на связи", "server_id", serverID, "точка", m.joinURL(token))

	// Держим соединение, пока туннель жив.
	<-session.CloseChan()
	m.relay.Remove(host)
	m.log.Info("релей: сервер отключился", "server_id", serverID)
}

// relayJoin — сюда приходит игрок по ссылке, выданной мастером.
//
// Мастер не разбирает игровой протокол: он перекладывает сообщения между
// игроком и стримом хоста, и только считает трафик.
func (m *master) relayJoin(w http.ResponseWriter, r *http.Request) {
	token := r.PathValue("token")
	host, ok := m.relay.ByToken(token)
	if !ok {
		httpErr(w, http.StatusNotFound, "сервер не на связи")
		return
	}

	stream, err := m.relay.Open(host)
	if err != nil {
		m.log.Warn("релей: не открыть стрим", "server_id", host.ServerID, "err", err)
		httpErr(w, http.StatusServiceUnavailable, "туннель занят")
		return
	}
	defer stream.Close()

	socket, err := websocket.Accept(w, r, &websocket.AcceptOptions{
		InsecureSkipVerify: true,
		CompressionMode:    websocket.CompressionDisabled,
	})
	if err != nil {
		return
	}
	defer socket.Close(websocket.StatusNormalClosure, "bye")
	socket.SetReadLimit(relay.MaxFrame)

	// Сообщаем хосту, кто к нему пришёл: через туннель настоящий адрес игрока
	// иначе не виден, а он нужен и в логах, и в рейт-лимитах.
	if err := relay.WriteFrame(stream, []byte(wire.RemoteAddr(r))); err != nil {
		return
	}

	ctx := r.Context()
	done := make(chan struct{}, 2)

	// Игрок → хост.
	go func() {
		defer func() { done <- struct{}{} }()
		for {
			_, data, err := socket.Read(ctx)
			if err != nil {
				return
			}
			if err := relay.WriteFrame(stream, data); err != nil {
				return
			}
		}
	}()

	// Хост → игрок.
	go func() {
		defer func() { done <- struct{}{} }()
		for {
			payload, err := relay.ReadFrame(stream)
			if err != nil {
				return
			}
			if err := socket.Write(ctx, websocket.MessageText, payload); err != nil {
				return
			}
		}
	}()

	<-done
}

// joinURL — адрес, по которому игрок попадает на сервер через релей.
func (m *master) joinURL(token string) string {
	base := strings.TrimRight(m.cfg.PublicURL, "/")
	if base == "" {
		// Без public_url отдаём относительный путь: в логах он всё равно
		// понятен, а владелец сервера пропишет адрес в конфиге.
		return "/j/" + token
	}
	// Игрок ходит по ws/wss, а не по http/https.
	base = strings.Replace(base, "https://", "wss://", 1)
	base = strings.Replace(base, "http://", "ws://", 1)
	return base + "/j/" + token
}

func yamuxConfig() *yamux.Config {
	cfg := yamux.DefaultConfig()
	// Тик игры — 20 Гц, и пустой канал между кадрами это норма: держим
	// keep-alive пореже, чтобы не будить соединение зря.
	cfg.KeepAliveInterval = 15 * time.Second
	cfg.ConnectionWriteTimeout = 10 * time.Second
	cfg.LogOutput = io.Discard
	return cfg
}
