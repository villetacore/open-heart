package main

import (
	"encoding/json"
	"errors"
	"net/http"
	"strconv"
	"strings"
	"time"

	"github.com/villetacore/open-heart/server/internal/auth"
	"github.com/villetacore/open-heart/server/internal/registry"
	"github.com/villetacore/open-heart/server/internal/store"
	"github.com/villetacore/open-heart/server/internal/wire"
)

func (m *master) routes() http.Handler {
	mux := http.NewServeMux()

	mux.HandleFunc("POST /v1/auth/register", m.authRegister)
	mux.HandleFunc("POST /v1/auth/login", m.authLogin)
	mux.HandleFunc("POST /v1/auth/ticket", m.authTicket)
	mux.HandleFunc("GET /v1/auth/pubkey", m.authPubkey)

	mux.HandleFunc("POST /v1/servers/register", m.serversRegister)
	mux.HandleFunc("POST /v1/servers/announce", m.serversAnnounce)
	mux.HandleFunc("POST /v1/servers/remove", m.serversRemove)
	mux.HandleFunc("GET /v1/servers", m.serversList)

	// Релей: сервер приходит сам, игрок заходит по выданной ссылке.
	mux.HandleFunc("GET /v1/relay/connect", m.relayConnect)
	mux.HandleFunc("GET /j/{token}", m.relayJoin)

	mux.HandleFunc("POST /v1/party/host", m.partyHost)
	mux.HandleFunc("POST /v1/party/create", m.partyCreate)
	mux.HandleFunc("GET /v1/party/{code}", m.partyResolve)

	mux.HandleFunc("GET /healthz", func(w http.ResponseWriter, r *http.Request) {
		list := m.list.List(registry.Filter{})
		players := 0
		for _, e := range list {
			players += e.Players
		}
		w.Header().Set("Content-Type", "text/plain; charset=utf-8")
		w.Write([]byte("ok servers=" + strconv.Itoa(len(list)) +
			" players=" + strconv.Itoa(players) +
			" relay=" + strconv.Itoa(m.relay.Count()) + "\n"))
	})
	return mux
}

// ── аккаунты ─────────────────────────────────────────────────────────────────

func (m *master) authRegister(w http.ResponseWriter, r *http.Request) {
	if !m.allow(w, r) {
		return
	}
	var req struct {
		Login    string `json:"login"`
		Password string `json:"password"`
		Email    string `json:"email"`
	}
	if !readJSON(w, r, &req) {
		return
	}
	acc, err := m.db.CreateAccount(req.Login, req.Password, req.Email)
	if errors.Is(err, store.ErrLoginTaken) {
		httpErr(w, http.StatusConflict, "логин занят")
		return
	}
	if err != nil {
		httpErr(w, http.StatusBadRequest, err.Error())
		return
	}
	m.log.Info("новый аккаунт", "login", acc.Login, "id", acc.ID)
	writeJSON(w, map[string]any{"account_id": acc.ID})
}

func (m *master) authLogin(w http.ResponseWriter, r *http.Request) {
	if !m.allow(w, r) {
		return
	}
	var req struct {
		Login    string `json:"login"`
		Password string `json:"password"`
	}
	if !readJSON(w, r, &req) {
		return
	}
	acc, err := m.db.Login(req.Login, req.Password)
	if err != nil {
		httpErr(w, http.StatusUnauthorized, "неверный логин или пароль")
		return
	}
	if acc.Banned {
		httpErr(w, http.StatusForbidden, "аккаунт заблокирован")
		return
	}
	token, err := m.db.NewRefresh(acc.ID)
	if err != nil {
		httpErr(w, http.StatusInternalServerError, "не выдать токен")
		return
	}
	writeJSON(w, map[string]any{
		"refresh_token": token,
		"account_id":    acc.ID,
		"nickname":      acc.Login,
	})
}

// authTicket выдаёт подписанный пропуск на конкретный игровой сервер.
// Пароль в этом обмене не участвует — в этом весь смысл (docs/MULTIPLAYER.md §10).
func (m *master) authTicket(w http.ResponseWriter, r *http.Request) {
	var req struct {
		RefreshToken string `json:"refresh_token"`
		ServerID     string `json:"server_id"`
	}
	if !readJSON(w, r, &req) {
		return
	}
	acc, err := m.db.ResolveRefresh(req.RefreshToken)
	if err != nil {
		httpErr(w, http.StatusUnauthorized, "токен недействителен")
		return
	}
	if acc.Banned {
		httpErr(w, http.StatusForbidden, "аккаунт заблокирован")
		return
	}
	ticket, err := auth.Sign(m.key, auth.Ticket{
		AccountID: acc.ID,
		Nickname:  acc.Login,
		Aud:       req.ServerID,
		Exp:       time.Now().Add(auth.TicketTTL).Unix(),
	})
	if err != nil {
		httpErr(w, http.StatusInternalServerError, "не подписать тикет")
		return
	}
	writeJSON(w, map[string]any{"ticket": ticket, "expires_in": int(auth.TicketTTL.Seconds())})
}

func (m *master) authPubkey(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, map[string]any{"pubkey": auth.PublicKeyString(m.key)})
}

// ── сервера ──────────────────────────────────────────────────────────────────

func (m *master) serversRegister(w http.ResponseWriter, r *http.Request) {
	token := bearer(r)
	if token == "" {
		httpErr(w, http.StatusUnauthorized, "нужен Bearer-токен")
		return
	}
	acc, err := m.db.ResolveRefresh(token)
	if err != nil {
		httpErr(w, http.StatusUnauthorized, "токен недействителен")
		return
	}
	var req struct {
		Name string `json:"name"`
	}
	if !readJSON(w, r, &req) {
		return
	}
	if strings.TrimSpace(req.Name) == "" {
		httpErr(w, http.StatusBadRequest, "нужно имя сервера")
		return
	}
	id, secret, err := m.db.RegisterServer(acc.ID, req.Name)
	if err != nil {
		httpErr(w, http.StatusInternalServerError, "не зарегистрировать сервер")
		return
	}
	m.log.Info("зарегистрирован сервер", "id", id, "owner", acc.Login, "name", req.Name)
	writeJSON(w, map[string]any{"server_id": id, "server_secret": secret})
}

func (m *master) serversAnnounce(w http.ResponseWriter, r *http.Request) {
	var req struct {
		registry.Entry
		Secret string `json:"secret"`
	}
	if !readJSON(w, r, &req) {
		return
	}
	row, err := m.db.CheckServer(req.ServerID, req.Secret)
	if err != nil {
		httpErr(w, http.StatusUnauthorized, "сервер не опознан")
		return
	}
	if !row.Listed {
		httpErr(w, http.StatusForbidden, "сервер снят с публикации")
		return
	}

	entry := req.Entry
	// Имя владельца из базы важнее самоназвания в анонсе только при модерации;
	// остальное сервер сообщает о себе сам.
	if entry.Name == "" {
		entry.Name = row.Name
	}
	if entry.MaxPlayers <= 0 {
		entry.MaxPlayers = 1
	}
	if entry.Players < 0 {
		entry.Players = 0
	}
	if entry.Players > entry.MaxPlayers {
		entry.Players = entry.MaxPlayers
	}
	if entry.Relayed {
		// У релейного сервера точку входа знает мастер, а не он сам.
		host, ok := m.relay.ByServer(entry.ServerID)
		if !ok {
			httpErr(w, http.StatusPreconditionFailed, "туннель не поднят")
			return
		}
		entry.Endpoint = m.joinURL(host.Token)
	}
	if entry.Endpoint == "" {
		httpErr(w, http.StatusBadRequest, "нужен endpoint или relay")
		return
	}
	m.list.Announce(entry)
	writeJSON(w, map[string]any{"ok": true, "ttl": int(registry.DefaultTTL.Seconds())})
}

func (m *master) serversRemove(w http.ResponseWriter, r *http.Request) {
	var req struct {
		ServerID string `json:"server_id"`
		Secret   string `json:"secret"`
	}
	if !readJSON(w, r, &req) {
		return
	}
	if _, err := m.db.CheckServer(req.ServerID, req.Secret); err != nil {
		httpErr(w, http.StatusUnauthorized, "сервер не опознан")
		return
	}
	m.list.Remove(req.ServerID)
	writeJSON(w, map[string]any{"ok": true})
}

func (m *master) serversList(w http.ResponseWriter, r *http.Request) {
	q := r.URL.Query()
	protocol, _ := strconv.Atoi(q.Get("protocol"))
	list := m.list.List(registry.Filter{
		Preset:      q.Get("preset"),
		ContentHash: q.Get("content_hash"),
		Region:      q.Get("region"),
		Protocol:    protocol,
		OnlyFree:    q.Get("free") == "1",
	})
	if list == nil {
		list = []registry.Entry{}
	}
	writeJSON(w, list)
}

// ── пати-коды ────────────────────────────────────────────────────────────────

// partyHost выдаёт временную личность хоста для игры с другом.
//
// Без аккаунта: «позвать друга» не должно упираться в регистрацию. Взамен
// такой хост не попадает в список серверов — ему доступны только релей и
// пати-код (docs/MULTIPLAYER.md §8.4).
func (m *master) partyHost(w http.ResponseWriter, r *http.Request) {
	if !m.allow(w, r) {
		return
	}
	id, secret, err := m.db.NewPartyHost()
	if err != nil {
		httpErr(w, http.StatusInternalServerError, "не выдать личность хоста")
		return
	}
	m.log.Info("выдана личность для игры с другом", "server_id", id)
	writeJSON(w, map[string]any{
		"server_id":     id,
		"server_secret": secret,
		"expires_in":    int(store.PartyHostTTL.Seconds()),
	})
}

func (m *master) partyCreate(w http.ResponseWriter, r *http.Request) {
	var req struct {
		ServerID string `json:"server_id"`
		Secret   string `json:"secret"`
		Endpoint string `json:"endpoint"`
	}
	if !readJSON(w, r, &req) {
		return
	}
	if _, err := m.db.CheckServer(req.ServerID, req.Secret); err != nil {
		httpErr(w, http.StatusUnauthorized, "сервер не опознан")
		return
	}
	endpoint := req.Endpoint
	if host, ok := m.relay.ByServer(req.ServerID); ok {
		endpoint = m.joinURL(host.Token)
	}
	code, exp, err := m.codes.Create(req.ServerID, endpoint)
	if err != nil {
		httpErr(w, http.StatusInternalServerError, "не выдать код")
		return
	}
	// Точку входа возвращаем и хозяину: он показывает её тем, кому код не нужен.
	writeJSON(w, map[string]any{"code": code, "expires_at": exp.Unix(), "endpoint": endpoint})
}

func (m *master) partyResolve(w http.ResponseWriter, r *http.Request) {
	if !m.allow(w, r) {
		return
	}
	serverID, endpoint, ok := m.codes.Resolve(r.PathValue("code"))
	if !ok {
		httpErr(w, http.StatusNotFound, "код не найден или истёк")
		return
	}
	writeJSON(w, map[string]any{"server_id": serverID, "endpoint": endpoint})
}

// ── вспомогательное ──────────────────────────────────────────────────────────

// allow применяет рейт-лимит к чувствительным ручкам (вход, регистрация, коды).
func (m *master) allow(w http.ResponseWriter, r *http.Request) bool {
	if m.limit.allow(wire.RemoteAddr(r)) {
		return true
	}
	httpErr(w, http.StatusTooManyRequests, "слишком много запросов")
	return false
}

func bearer(r *http.Request) string {
	head := r.Header.Get("Authorization")
	if after, ok := strings.CutPrefix(head, "Bearer "); ok {
		return strings.TrimSpace(after)
	}
	return ""
}

func readJSON(w http.ResponseWriter, r *http.Request, v any) bool {
	defer r.Body.Close()
	dec := json.NewDecoder(http.MaxBytesReader(w, r.Body, 64*1024))
	if err := dec.Decode(v); err != nil {
		httpErr(w, http.StatusBadRequest, "битый JSON: "+err.Error())
		return false
	}
	return true
}

func writeJSON(w http.ResponseWriter, v any) {
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	if err := json.NewEncoder(w).Encode(v); err != nil {
		return
	}
}

func httpErr(w http.ResponseWriter, code int, msg string) {
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	w.WriteHeader(code)
	json.NewEncoder(w).Encode(map[string]string{"error": msg})
}
