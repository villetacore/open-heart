package store

import (
	"crypto/sha256"
	"database/sql"
	"encoding/base64"
	"errors"
	"strings"
	"time"

	"github.com/villetacore/open-heart/server/internal/auth"
)

var masterSchema = []string{
	`CREATE TABLE accounts (
	   id         INTEGER PRIMARY KEY,
	   login      TEXT UNIQUE NOT NULL,
	   pass_hash  TEXT NOT NULL,
	   email      TEXT,
	   created_at INTEGER NOT NULL,
	   last_seen  INTEGER,
	   banned     INTEGER NOT NULL DEFAULT 0
	 );
	 CREATE TABLE refresh_tokens (
	   token_hash TEXT PRIMARY KEY,
	   account_id INTEGER NOT NULL REFERENCES accounts(id),
	   expires_at INTEGER NOT NULL
	 );
	 CREATE TABLE servers (
	   id          TEXT PRIMARY KEY,
	   owner_id    INTEGER NOT NULL REFERENCES accounts(id),
	   name        TEXT NOT NULL,
	   secret_hash TEXT NOT NULL,
	   created_at  INTEGER NOT NULL,
	   listed      INTEGER NOT NULL DEFAULT 1
	 );`,
	// Хост «игры с другом» — не сервер из списка: у него нет владельца, он
	// живёт часы и в реестр не попадает. Поэтому отдельная таблица, а не
	// колонка в servers с NULL-владельцем.
	`CREATE TABLE party_hosts (
	   id          TEXT PRIMARY KEY,
	   secret_hash TEXT NOT NULL,
	   created_at  INTEGER NOT NULL
	 );`,
}

// PartyHostTTL — сколько живёт временная личность хоста. Игра с другом длится
// вечер, а не неделю.
const PartyHostTTL = 12 * time.Hour

// RefreshTTL — срок жизни refresh-токена мастера.
const RefreshTTL = 30 * 24 * time.Hour

// Account — аккаунт на мастере.
type Account struct {
	ID        int64
	Login     string
	Email     string
	CreatedAt int64
	Banned    bool
}

// ServerRow — зарегистрированная личность игрового сервера.
type ServerRow struct {
	ID        string
	OwnerID   int64
	Name      string
	CreatedAt int64
	Listed    bool
}

// Master — база мастер-сервера.
type Master struct{ db *sql.DB }

func OpenMaster(path string) (*Master, error) {
	db, err := open(path)
	if err != nil {
		return nil, err
	}
	if err := migrate(db, masterSchema); err != nil {
		db.Close()
		return nil, err
	}
	return &Master{db: db}, nil
}

func (m *Master) Close() error { return m.db.Close() }

// ErrLoginTaken — логин занят.
var ErrLoginTaken = errors.New("store: логин занят")

// CreateAccount регистрирует аккаунт; пароль хешируется argon2id.
func (m *Master) CreateAccount(login, password, email string) (*Account, error) {
	login = strings.ToLower(strings.TrimSpace(login))
	if len(login) < 3 || len(login) > 24 {
		return nil, errors.New("store: логин должен быть 3..24 символа")
	}
	hash, err := auth.HashPassword(password)
	if err != nil {
		return nil, err
	}
	now := time.Now().Unix()
	res, err := m.db.Exec(
		`INSERT INTO accounts (login, pass_hash, email, created_at, last_seen)
		 VALUES (?, ?, NULLIF(?, ''), ?, ?)`, login, hash, email, now, now)
	if err != nil {
		if strings.Contains(err.Error(), "UNIQUE") {
			return nil, ErrLoginTaken
		}
		return nil, err
	}
	id, err := res.LastInsertId()
	if err != nil {
		return nil, err
	}
	return &Account{ID: id, Login: login, Email: email, CreatedAt: now}, nil
}

// Login проверяет пару логин/пароль.
func (m *Master) Login(login, password string) (*Account, error) {
	login = strings.ToLower(strings.TrimSpace(login))
	var a Account
	var hash string
	var banned int
	err := m.db.QueryRow(
		`SELECT id, login, COALESCE(email,''), created_at, banned, pass_hash
		 FROM accounts WHERE login = ?`, login).
		Scan(&a.ID, &a.Login, &a.Email, &a.CreatedAt, &banned, &hash)
	if errors.Is(err, sql.ErrNoRows) {
		// Считаем фиктивный хеш, чтобы время ответа не выдавало наличие логина.
		auth.VerifyPassword("$argon2id$v=19$m=65536,t=3,p=4$AAAAAAAAAAAAAAAAAAAAAA$"+
			"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", password)
		return nil, auth.ErrBadPassword
	}
	if err != nil {
		return nil, err
	}
	if !auth.VerifyPassword(hash, password) {
		return nil, auth.ErrBadPassword
	}
	a.Banned = banned != 0
	_, _ = m.db.Exec(`UPDATE accounts SET last_seen = ? WHERE id = ?`, time.Now().Unix(), a.ID)
	return &a, nil
}

// NewRefresh выдаёт refresh-токен; в базе лежит только его хеш.
func (m *Master) NewRefresh(accountID int64) (string, error) {
	token, err := auth.RandomToken(32)
	if err != nil {
		return "", err
	}
	_, err = m.db.Exec(
		`INSERT INTO refresh_tokens (token_hash, account_id, expires_at) VALUES (?, ?, ?)`,
		hashToken(token), accountID, time.Now().Add(RefreshTTL).Unix())
	if err != nil {
		return "", err
	}
	return token, nil
}

// ResolveRefresh проверяет токен и возвращает аккаунт.
func (m *Master) ResolveRefresh(token string) (*Account, error) {
	var a Account
	var banned int
	var expires int64
	err := m.db.QueryRow(
		`SELECT a.id, a.login, COALESCE(a.email,''), a.created_at, a.banned, t.expires_at
		 FROM refresh_tokens t JOIN accounts a ON a.id = t.account_id
		 WHERE t.token_hash = ?`, hashToken(token)).
		Scan(&a.ID, &a.Login, &a.Email, &a.CreatedAt, &banned, &expires)
	if errors.Is(err, sql.ErrNoRows) {
		return nil, ErrNotFound
	}
	if err != nil {
		return nil, err
	}
	if time.Now().Unix() > expires {
		_, _ = m.db.Exec(`DELETE FROM refresh_tokens WHERE token_hash = ?`, hashToken(token))
		return nil, ErrNotFound
	}
	a.Banned = banned != 0
	return &a, nil
}

func (m *Master) RevokeRefresh(token string) error {
	_, err := m.db.Exec(`DELETE FROM refresh_tokens WHERE token_hash = ?`, hashToken(token))
	return err
}

// RegisterServer заводит личность игрового сервера, возвращая id и секрет.
// Секрет показывается один раз — в базе лежит только хеш.
func (m *Master) RegisterServer(ownerID int64, name string) (id, secret string, err error) {
	id, err = auth.RandomToken(9)
	if err != nil {
		return "", "", err
	}
	secret, err = auth.RandomToken(24)
	if err != nil {
		return "", "", err
	}
	_, err = m.db.Exec(
		`INSERT INTO servers (id, owner_id, name, secret_hash, created_at) VALUES (?, ?, ?, ?, ?)`,
		id, ownerID, name, hashToken(secret), time.Now().Unix())
	if err != nil {
		return "", "", err
	}
	return id, secret, nil
}

// NewPartyHost заводит временную личность хоста для игры с другом.
//
// Аккаунт для этого не нужен: иначе «позвать друга» упирается в регистрацию.
// Взамен такой хост не попадает в список серверов — только релей и пати-код.
func (m *Master) NewPartyHost() (id, secret string, err error) {
	id, err = auth.RandomToken(9)
	if err != nil {
		return "", "", err
	}
	secret, err = auth.RandomToken(24)
	if err != nil {
		return "", "", err
	}
	_, err = m.db.Exec(
		`INSERT INTO party_hosts (id, secret_hash, created_at) VALUES (?, ?, ?)`,
		id, hashToken(secret), time.Now().Unix())
	if err != nil {
		return "", "", err
	}
	return id, secret, nil
}

// PurgePartyHosts убирает протухшие временные личности.
func (m *Master) PurgePartyHosts() (int64, error) {
	cutoff := time.Now().Add(-PartyHostTTL).Unix()
	result, err := m.db.Exec(`DELETE FROM party_hosts WHERE created_at < ?`, cutoff)
	if err != nil {
		return 0, err
	}
	return result.RowsAffected()
}

// CheckServer проверяет пару server_id / secret и возвращает запись сервера.
func (m *Master) CheckServer(id, secret string) (*ServerRow, error) {
	var s ServerRow
	var listed int
	var hash string
	err := m.db.QueryRow(
		`SELECT id, owner_id, name, created_at, listed, secret_hash FROM servers WHERE id = ?`, id).
		Scan(&s.ID, &s.OwnerID, &s.Name, &s.CreatedAt, &listed, &hash)
	if errors.Is(err, sql.ErrNoRows) {
		// Может быть временный хост игры с другом — у него нет владельца.
		return m.checkPartyHost(id, secret)
	}
	if err != nil {
		return nil, err
	}
	if hash != hashToken(secret) {
		return nil, auth.ErrBadPassword
	}
	s.Listed = listed != 0
	return &s, nil
}

// checkPartyHost — та же проверка для временной личности.
//
// Такой хост считается «не в списке»: релей и пати-код ему доступны, а анонс
// в реестр — нет.
func (m *Master) checkPartyHost(id, secret string) (*ServerRow, error) {
	var hash string
	var created int64
	err := m.db.QueryRow(
		`SELECT secret_hash, created_at FROM party_hosts WHERE id = ?`, id).
		Scan(&hash, &created)
	if errors.Is(err, sql.ErrNoRows) {
		return nil, ErrNotFound
	}
	if err != nil {
		return nil, err
	}
	if hash != hashToken(secret) {
		return nil, auth.ErrBadPassword
	}
	if time.Since(time.Unix(created, 0)) > PartyHostTTL {
		return nil, ErrNotFound
	}
	return &ServerRow{ID: id, Name: "party", CreatedAt: created}, nil
}

// SetListed включает/выключает право сервера появляться в списке (модерация).
func (m *Master) SetListed(id string, listed bool) error {
	v := 0
	if listed {
		v = 1
	}
	_, err := m.db.Exec(`UPDATE servers SET listed = ? WHERE id = ?`, v, id)
	return err
}

// hashToken — токены хранятся только в виде SHA-256: утечка базы не даёт входа.
func hashToken(token string) string {
	sum := sha256.Sum256([]byte(token))
	return base64.RawStdEncoding.EncodeToString(sum[:])
}
