package main

import (
	"context"
	"crypto/ed25519"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"strings"
	"time"

	"github.com/villetacore/open-heart/server/internal/auth"
	"github.com/villetacore/open-heart/server/internal/config"
	"github.com/villetacore/open-heart/server/internal/proto"
	"github.com/villetacore/open-heart/server/internal/room"
	"github.com/villetacore/open-heart/server/internal/store"
	"github.com/villetacore/open-heart/server/internal/wire"
)

const (
	handshakeTimeout = 10 * time.Second
	readTimeout      = 15 * time.Second
)

// rejectErr — отказ, который уходит клиенту в сообщении reject.
type rejectErr struct {
	reason string
	detail string
}

func (e rejectErr) Error() string { return e.reason + ": " + e.detail }

func reject(reason, format string, args ...any) rejectErr {
	return rejectErr{reason: reason, detail: fmt.Sprintf(format, args...)}
}

func (s *server) handleWS(w http.ResponseWriter, r *http.Request) {
	conn, err := wire.Accept(w, r)
	if err != nil {
		s.log.Warn("не поднять websocket", "addr", wire.RemoteAddr(r), "err", err)
		return
	}
	s.serve(r.Context(), conn)
}

// serve ведёт игрока от рукопожатия до выхода.
//
// Как игрок пришёл — напрямую по WebSocket или стримом через релей мастера —
// отсюда не видно и не важно (docs/MULTIPLAYER.md §8.3).
func (s *server) serve(ctx context.Context, conn wire.Conn) {
	// Соединение закрывает сессия: комнаты игрок может менять, соединение — нет.
	defer conn.Close("bye")

	player, err := s.handshake(ctx, conn)
	if err != nil {
		var rej rejectErr
		if !errors.As(err, &rej) {
			rej = rejectErr{reason: proto.RejBadRequest, detail: err.Error()}
		}
		s.log.Info("вход отклонён", "addr", conn.Addr(), "reason", rej.reason, "detail", rej.detail)
		conn.Reject(rej.reason, rej.detail)
		return
	}

	current := s.rooms.Hub()
	if err := current.Join(ctx, player); err != nil {
		if errors.Is(err, room.ErrFull) {
			conn.Reject(proto.RejFull, "нет свободных мест")
		} else {
			conn.Reject(proto.RejBadRequest, err.Error())
		}
		return
	}
	defer func() { current.Leave(player.Peer) }()

	for {
		readCtx, cancel := context.WithTimeout(ctx, readTimeout)
		env, err := conn.Read(readCtx)
		cancel()
		if err != nil {
			s.log.Debug("соединение закрыто", "peer", player.Peer, "room", current.ID(), "err", err)
			return
		}

		// Переход между комнатами обрабатывает сессия: комната про соседей не знает.
		if next := s.roomSwitch(ctx, current, player, env); next != nil {
			current = next
			continue
		}
		current.Post(player.Peer, env)
	}
}

// / Команды входа и выхода из забега.
const (
	cmdEnterDelve = "enter_delve"
	cmdLeaveDelve = "leave_delve"
)

// roomSwitch переносит игрока в другую комнату, если сообщение об этом просит.
// Возвращает новую комнату либо nil, если сообщение обычное.
func (s *server) roomSwitch(
	ctx context.Context,
	current *room.Room,
	player *room.Player,
	env proto.Envelope,
) *room.Room {
	if env.T != proto.TCmd {
		return nil
	}
	var command proto.Cmd
	if err := env.Into(&command); err != nil {
		return nil
	}

	switch command.Kind {
	case cmdEnterDelve:
		if current != s.rooms.Hub() {
			return nil // уже в забеге
		}
		var args struct {
			Depth int    `json:"depth"`
			Owner uint16 `json:"owner"`
		}
		_ = json.Unmarshal(command.Args, &args)
		depth := args.Depth
		if depth <= 0 {
			depth = 1
		}
		// Забег адресуется по ведущему: товарищи заходят в тот же данж,
		// а не каждый в свою копию.
		owner := args.Owner
		if owner == 0 {
			owner = player.Peer
		}
		seed := uint64(time.Now().UnixNano()) ^ (uint64(owner) << 32) ^ uint64(depth)

		target, err := s.rooms.OpenDelve(owner, depth, seed)
		if err != nil {
			s.log.Error("не создать инстанс забега", "err", err)
			return nil
		}
		return s.transfer(ctx, current, target, player)

	case cmdLeaveDelve:
		if current == s.rooms.Hub() {
			return nil
		}
		return s.transfer(ctx, current, s.rooms.Hub(), player)
	}
	return nil
}

// transfer выводит игрока из одной комнаты и заводит в другую.
// Новый peer выдаёт принимающая комната — клиент узнает его из `welcome`.
func (s *server) transfer(
	ctx context.Context,
	from, to *room.Room,
	player *room.Player,
) *room.Room {
	from.Leave(player.Peer)
	if err := to.Join(ctx, player); err != nil {
		s.log.Warn("переход не удался, возвращаю в исходную комнату",
			"from", from.ID(), "to", to.ID(), "err", err)
		if back := from.Join(ctx, player); back != nil {
			player.Conn.Reject(proto.RejBadRequest, "не удалось вернуться в комнату")
			return nil
		}
		return from
	}
	s.log.Info("игрок сменил комнату", "nick", player.Nickname, "from", from.ID(), "to", to.ID())
	return to
}

// handshake принимает hello, проверяет совместимость и авторизует игрока.
func (s *server) handshake(ctx context.Context, conn wire.Conn) (*room.Player, error) {
	hsCtx, cancel := context.WithTimeout(ctx, handshakeTimeout)
	defer cancel()

	env, err := conn.Read(hsCtx)
	if err != nil {
		return nil, reject(proto.RejBadRequest, "нет hello: %v", err)
	}
	if env.T != proto.THello {
		return nil, reject(proto.RejBadRequest, "первым сообщением должен быть hello, получено %q", env.T)
	}
	var hello proto.Hello
	if err := env.Into(&hello); err != nil {
		return nil, reject(proto.RejBadRequest, "битый hello: %v", err)
	}

	if hello.Protocol != proto.Version {
		return nil, reject(proto.RejProtocol, "сервер говорит на протоколе %d, клиент на %d",
			proto.Version, hello.Protocol)
	}
	if hello.PresetID != "" && hello.PresetID != s.cfg.Preset {
		return nil, reject(proto.RejContent, "на сервере пресет %q", s.cfg.Preset)
	}
	if s.contentHash != "" && hello.ContentHash != "" && hello.ContentHash != s.contentHash {
		return nil, reject(proto.RejContent, "данные пресета отличаются от серверных")
	}
	if s.cfg.Password != "" && hello.Password != s.cfg.Password {
		return nil, reject(proto.RejAuth, "неверный пароль сервера")
	}

	ref, nickname, userID, err := s.authorize(hello.Auth)
	if err != nil {
		return nil, err
	}

	if banned, err := s.db.BanOf(ref); err != nil {
		s.log.Error("проверка бана", "ref", ref, "err", err)
	} else if banned != "" {
		return nil, reject(proto.RejBanned, "%s", banned)
	}

	return &room.Player{
		Nickname:   nickname,
		UserID:     userID,
		AccountRef: ref,
		Conn:       conn,
	}, nil
}

// authorize разбирает блок auth в соответствии с режимом сервера.
func (s *server) authorize(a proto.Auth) (ref, nickname string, userID int64, err error) {
	if a.Mode != "" && a.Mode != s.cfg.Auth {
		return "", "", 0, reject(proto.RejAuth, "сервер принимает вход в режиме %q", s.cfg.Auth)
	}

	switch s.cfg.Auth {
	case config.AuthOpen:
		nickname = sanitizeNick(a.Nickname)
		if nickname == "" {
			return "", "", 0, reject(proto.RejBadRequest, "нужен ник")
		}
		token, err := auth.RandomToken(9)
		if err != nil {
			return "", "", 0, err
		}
		// Гостя в базе не заводим: на open-сервере прогресс не хранится.
		return "guest:" + token, nickname, 0, nil

	case config.AuthLocal:
		login := strings.ToLower(strings.TrimSpace(a.Login))
		if len(login) < 3 || len(login) > 24 {
			return "", "", 0, reject(proto.RejBadRequest, "логин должен быть 3..24 символа")
		}
		ref = "local:" + login
		user, err := s.db.UserByRef(ref)
		switch {
		case errors.Is(err, store.ErrNotFound):
			if !a.Register {
				return "", "", 0, reject(proto.RejAuth, "нет такого игрока")
			}
			hash, err := auth.HashPassword(a.Password)
			if err != nil {
				return "", "", 0, reject(proto.RejBadRequest, "%v", err)
			}
			nickname = sanitizeNick(a.Nickname)
			if nickname == "" {
				nickname = login
			}
			user, err = s.db.CreateUser(ref, nickname, hash)
			if err != nil {
				return "", "", 0, err
			}
			s.log.Info("создан локальный аккаунт", "login", login)
		case err != nil:
			return "", "", 0, err
		default:
			if !auth.VerifyPassword(user.PassHash, a.Password) {
				return "", "", 0, reject(proto.RejAuth, "неверный пароль")
			}
		}
		nickname = user.Nickname
		if nick := sanitizeNick(a.Nickname); nick != "" {
			nickname = nick
		}
		if err := s.db.TouchUser(user.ID, nickname); err != nil {
			s.log.Error("обновление пользователя", "err", err)
		}
		return ref, nickname, user.ID, nil

	case config.AuthMaster:
		if s.masterKey == nil {
			return "", "", 0, reject(proto.RejAuth, "сервер не знает ключа мастера")
		}
		ticket, err := auth.Verify(s.masterKey, a.Ticket, s.cfg.ServerID, time.Now())
		if err != nil {
			return "", "", 0, reject(proto.RejAuth, "%v", err)
		}
		ref = fmt.Sprintf("master:%d", ticket.AccountID)
		nickname = sanitizeNick(ticket.Nickname)
		if nickname == "" {
			nickname = fmt.Sprintf("player%d", ticket.AccountID)
		}
		user, err := s.db.UserByRef(ref)
		if errors.Is(err, store.ErrNotFound) {
			user, err = s.db.CreateUser(ref, nickname, "")
		}
		if err != nil {
			return "", "", 0, err
		}
		if err := s.db.TouchUser(user.ID, nickname); err != nil {
			s.log.Error("обновление пользователя", "err", err)
		}
		return ref, nickname, user.ID, nil
	}
	return "", "", 0, reject(proto.RejAuth, "неизвестный режим входа")
}

// fetchMasterKey забирает публичный ключ мастера для офлайн-проверки тикетов.
func fetchMasterKey(masterURL string) (ed25519.PublicKey, error) {
	client := &http.Client{Timeout: 10 * time.Second}
	resp, err := client.Get(strings.TrimRight(masterURL, "/") + "/v1/auth/pubkey")
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("мастер ответил %s", resp.Status)
	}
	var body struct {
		PubKey string `json:"pubkey"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		return nil, err
	}
	return auth.ParsePublicKey(body.PubKey)
}

// sanitizeNick оставляет только печатные символы и режет длину.
func sanitizeNick(nick string) string {
	nick = strings.TrimSpace(nick)
	out := make([]rune, 0, 24)
	for _, ch := range nick {
		if ch < 0x20 || ch == 0x7f {
			continue
		}
		out = append(out, ch)
		if len(out) >= 24 {
			break
		}
	}
	return strings.TrimSpace(string(out))
}
