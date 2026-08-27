package store

import (
	"database/sql"
	"errors"
	"time"
)

// ErrNotFound — записи нет.
var ErrNotFound = errors.New("store: не найдено")

var serverSchema = []string{
	`CREATE TABLE users (
	   id          INTEGER PRIMARY KEY,
	   account_ref TEXT UNIQUE NOT NULL,          -- master:1234 | local:vasya | guest:<uuid>
	   nickname    TEXT NOT NULL,
	   pass_hash   TEXT,                          -- argon2id, только для local
	   role        TEXT NOT NULL DEFAULT 'player',
	   created_at  INTEGER NOT NULL,
	   last_seen   INTEGER
	 );
	 CREATE TABLE characters (
	   id         INTEGER PRIMARY KEY,
	   user_id    INTEGER NOT NULL REFERENCES users(id),
	   preset     TEXT NOT NULL,
	   name       TEXT NOT NULL,
	   class_id   TEXT,
	   spec_id    TEXT,
	   level      INTEGER NOT NULL DEFAULT 1,
	   xp         INTEGER NOT NULL DEFAULT 0,
	   save_ver   INTEGER NOT NULL,
	   save_json  TEXT NOT NULL,
	   updated_at INTEGER NOT NULL
	 );
	 CREATE INDEX characters_by_user ON characters(user_id, preset);
	 CREATE TABLE bans (
	   account_ref TEXT PRIMARY KEY,
	   reason      TEXT,
	   until       INTEGER,                       -- 0 = навсегда
	   by          TEXT
	 );
	 CREATE TABLE kv (key TEXT PRIMARY KEY, value TEXT NOT NULL);`,
}

// Роли пользователей сервера.
const (
	RolePlayer = "player"
	RoleMod    = "mod"
	RoleAdmin  = "admin"
)

// User — учётка игрока на конкретном игровом сервере.
type User struct {
	ID         int64
	AccountRef string
	Nickname   string
	PassHash   string
	Role       string
	CreatedAt  int64
	LastSeen   int64
}

// Character — персонаж; save_json хранится ровно в том виде, что пишет client/src/state/save.rs.
type Character struct {
	ID        int64
	UserID    int64
	Preset    string
	Name      string
	ClassID   string
	SpecID    string
	Level     int
	XP        int
	SaveVer   int
	SaveJSON  string
	UpdatedAt int64
}

// Server — база игрового сервера.
type Server struct{ db *sql.DB }

func OpenServer(path string) (*Server, error) {
	db, err := open(path)
	if err != nil {
		return nil, err
	}
	if err := migrate(db, serverSchema); err != nil {
		db.Close()
		return nil, err
	}
	return &Server{db: db}, nil
}

func (s *Server) Close() error { return s.db.Close() }

// UserByRef ищет пользователя по account_ref.
func (s *Server) UserByRef(ref string) (*User, error) {
	row := s.db.QueryRow(
		`SELECT id, account_ref, nickname, COALESCE(pass_hash,''), role, created_at,
		        COALESCE(last_seen,0)
		 FROM users WHERE account_ref = ?`, ref)
	var u User
	err := row.Scan(&u.ID, &u.AccountRef, &u.Nickname, &u.PassHash, &u.Role, &u.CreatedAt, &u.LastSeen)
	if errors.Is(err, sql.ErrNoRows) {
		return nil, ErrNotFound
	}
	if err != nil {
		return nil, err
	}
	return &u, nil
}

// CreateUser заводит учётку. passHash пустой для master/guest.
func (s *Server) CreateUser(ref, nickname, passHash string) (*User, error) {
	now := time.Now().Unix()
	res, err := s.db.Exec(
		`INSERT INTO users (account_ref, nickname, pass_hash, role, created_at, last_seen)
		 VALUES (?, ?, NULLIF(?, ''), ?, ?, ?)`,
		ref, nickname, passHash, RolePlayer, now, now)
	if err != nil {
		return nil, err
	}
	id, err := res.LastInsertId()
	if err != nil {
		return nil, err
	}
	return &User{ID: id, AccountRef: ref, Nickname: nickname, PassHash: passHash,
		Role: RolePlayer, CreatedAt: now, LastSeen: now}, nil
}

// TouchUser отмечает вход и обновляет ник.
func (s *Server) TouchUser(id int64, nickname string) error {
	_, err := s.db.Exec(`UPDATE users SET last_seen = ?, nickname = ? WHERE id = ?`,
		time.Now().Unix(), nickname, id)
	return err
}

// Ban запрещает вход. until = 0 — навсегда.
func (s *Server) Ban(ref, reason, by string, until int64) error {
	_, err := s.db.Exec(
		`INSERT INTO bans (account_ref, reason, until, by) VALUES (?, ?, ?, ?)
		 ON CONFLICT(account_ref) DO UPDATE SET reason = excluded.reason,
		   until = excluded.until, by = excluded.by`,
		ref, reason, until, by)
	return err
}

func (s *Server) Unban(ref string) error {
	_, err := s.db.Exec(`DELETE FROM bans WHERE account_ref = ?`, ref)
	return err
}

// BanOf возвращает причину бана либо "" если бана нет (истёкшие снимаются).
func (s *Server) BanOf(ref string) (string, error) {
	var reason string
	var until int64
	err := s.db.QueryRow(`SELECT COALESCE(reason,''), COALESCE(until,0) FROM bans WHERE account_ref = ?`,
		ref).Scan(&reason, &until)
	if errors.Is(err, sql.ErrNoRows) {
		return "", nil
	}
	if err != nil {
		return "", err
	}
	if until > 0 && time.Now().Unix() > until {
		return "", s.Unban(ref)
	}
	return reason, nil
}

// Characters — персонажи игрока в пресете.
func (s *Server) Characters(userID int64, preset string) ([]Character, error) {
	rows, err := s.db.Query(
		`SELECT id, user_id, preset, name, COALESCE(class_id,''), COALESCE(spec_id,''),
		        level, xp, save_ver, save_json, updated_at
		 FROM characters WHERE user_id = ? AND preset = ? ORDER BY updated_at DESC`,
		userID, preset)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var out []Character
	for rows.Next() {
		var c Character
		if err := rows.Scan(&c.ID, &c.UserID, &c.Preset, &c.Name, &c.ClassID, &c.SpecID,
			&c.Level, &c.XP, &c.SaveVer, &c.SaveJSON, &c.UpdatedAt); err != nil {
			return nil, err
		}
		out = append(out, c)
	}
	return out, rows.Err()
}

// SaveCharacter создаёт (ID == 0) или обновляет персонажа, возвращая его id.
func (s *Server) SaveCharacter(c *Character) (int64, error) {
	now := time.Now().Unix()
	if c.ID == 0 {
		res, err := s.db.Exec(
			`INSERT INTO characters (user_id, preset, name, class_id, spec_id, level, xp,
			                         save_ver, save_json, updated_at)
			 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
			c.UserID, c.Preset, c.Name, c.ClassID, c.SpecID, c.Level, c.XP,
			c.SaveVer, c.SaveJSON, now)
		if err != nil {
			return 0, err
		}
		return res.LastInsertId()
	}
	_, err := s.db.Exec(
		`UPDATE characters SET name = ?, class_id = ?, spec_id = ?, level = ?, xp = ?,
		        save_ver = ?, save_json = ?, updated_at = ?
		 WHERE id = ? AND user_id = ?`,
		c.Name, c.ClassID, c.SpecID, c.Level, c.XP, c.SaveVer, c.SaveJSON, now,
		c.ID, c.UserID)
	return c.ID, err
}

// KVGet/KVSet — мелкие настройки сервера (состояние мира, счётчики).
func (s *Server) KVGet(key string) (string, error) {
	var v string
	err := s.db.QueryRow(`SELECT value FROM kv WHERE key = ?`, key).Scan(&v)
	if errors.Is(err, sql.ErrNoRows) {
		return "", ErrNotFound
	}
	return v, err
}

func (s *Server) KVSet(key, value string) error {
	_, err := s.db.Exec(
		`INSERT INTO kv (key, value) VALUES (?, ?)
		 ON CONFLICT(key) DO UPDATE SET value = excluded.value`, key, value)
	return err
}
