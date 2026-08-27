package main

import (
	"context"
	"errors"
	"io"
	"net/url"
	"strings"
	"time"

	"github.com/coder/websocket"
	"github.com/hashicorp/yamux"

	"github.com/villetacore/open-heart/server/internal/relay"
)

const (
	// Реконнект: сначала быстро, потом реже — мастер может лежать долго,
	// а сервер при этом продолжает обслуживать тех, кто пришёл напрямую.
	relayRetryMin = 2 * time.Second
	relayRetryMax = 60 * time.Second
)

// relayLoop держит туннель к мастеру, пока сервер жив.
//
// Соединение исходящее: так сервер доступен игрокам, даже если у него нет
// белого IP и проброшенных портов (docs/MULTIPLAYER.md §8.3).
func (s *server) relayLoop(ctx context.Context) {
	wait := relayRetryMin
	for ctx.Err() == nil {
		err := s.relayOnce(ctx)
		if ctx.Err() != nil {
			return
		}
		if err != nil {
			s.log.Warn("релей: туннель оборвался", "err", err, "повтор через", wait)
		} else {
			s.log.Info("релей: туннель закрыт мастером", "повтор через", wait)
		}

		select {
		case <-ctx.Done():
			return
		case <-time.After(wait):
		}
		if wait *= 2; wait > relayRetryMax {
			wait = relayRetryMax
		}
	}
}

// relayOnce поднимает один туннель и обслуживает его до обрыва.
func (s *server) relayOnce(ctx context.Context) error {
	endpoint, err := relayURL(s.cfg.Master, s.cfg.ServerID, s.cfg.Secret)
	if err != nil {
		return err
	}

	dialCtx, cancel := context.WithTimeout(ctx, 15*time.Second)
	socket, _, err := websocket.Dial(dialCtx, endpoint, nil)
	cancel()
	if err != nil {
		return err
	}
	defer socket.CloseNow()
	socket.SetReadLimit(-1)

	netConn := websocket.NetConn(ctx, socket, websocket.MessageBinary)

	// Звоним мы, значит мы клиент мультиплексора: стримы открывает мастер,
	// когда к нам приходит игрок.
	session, err := yamux.Client(netConn, relayYamuxConfig())
	if err != nil {
		return err
	}
	defer session.Close()

	s.log.Info("релей: туннель поднят", "master", s.cfg.Master)
	s.markRelayUp()

	// Туннель рвём вместе с сервером, иначе Accept повиснет на остановке.
	go func() {
		<-ctx.Done()
		_ = session.Close()
	}()

	for {
		stream, err := session.AcceptStream()
		if err != nil {
			if ctx.Err() != nil {
				return nil
			}
			return err
		}
		go s.serveRelayStream(ctx, stream)
	}
}

// serveRelayStream обслуживает одного игрока, пришедшего через туннель.
func (s *server) serveRelayStream(ctx context.Context, stream io.ReadWriteCloser) {
	// Первым кадром мастер сообщает адрес игрока: через туннель настоящий
	// адрес иначе не виден, а он нужен в логах и рейт-лимитах.
	head, err := relay.ReadFrame(stream)
	if err != nil {
		_ = stream.Close()
		return
	}
	addr := string(head)
	if addr == "" {
		addr = "relay"
	}

	s.log.Debug("релей: новый игрок", "addr", addr)
	s.serve(ctx, relay.NewStreamConn(stream, addr))
}

// relayURL собирает адрес точки входа туннеля из базового URL мастера.
func relayURL(master, serverID, secret string) (string, error) {
	if serverID == "" || secret == "" {
		return "", errors.New("релей: нужны server_id и secret")
	}
	base, err := url.Parse(strings.TrimRight(master, "/"))
	if err != nil {
		return "", err
	}
	switch base.Scheme {
	case "http":
		base.Scheme = "ws"
	case "https", "":
		base.Scheme = "wss"
	}
	base.Path = strings.TrimRight(base.Path, "/") + "/v1/relay/connect"
	base.RawQuery = url.Values{
		"server_id": {serverID},
		"secret":    {secret},
	}.Encode()
	return base.String(), nil
}

func relayYamuxConfig() *yamux.Config {
	cfg := yamux.DefaultConfig()
	cfg.KeepAliveInterval = 15 * time.Second
	cfg.ConnectionWriteTimeout = 10 * time.Second
	cfg.LogOutput = io.Discard
	return cfg
}

// markRelayUp разово сообщает остальным, что туннель поднят.
func (s *server) markRelayUp() {
	select {
	case <-s.relayUp:
	default:
		close(s.relayUp)
	}
}
