// Package store — персистентность на SQLite (modernc, без cgo).
//
// Две базы: server.db у игрового сервера (юзеры, персонажи, баны) и master.db
// у мастера (аккаунты, refresh-токены, владельцы серверов). Схемы — §7.3 и §8.2
// docs/MULTIPLAYER.md. Миграции — через PRAGMA user_version, без фреймворков.
package store

import (
	"database/sql"
	"fmt"

	_ "modernc.org/sqlite"
)

func open(path string) (*sql.DB, error) {
	dsn := path + "?_pragma=busy_timeout(5000)&_pragma=journal_mode(WAL)" +
		"&_pragma=foreign_keys(1)&_pragma=synchronous(NORMAL)"
	db, err := sql.Open("sqlite", dsn)
	if err != nil {
		return nil, err
	}
	// SQLite не любит параллельные писатели; читателей WAL пускает свободно.
	db.SetMaxOpenConns(4)
	if err := db.Ping(); err != nil {
		db.Close()
		return nil, fmt.Errorf("store: не открыть %s: %w", path, err)
	}
	return db, nil
}

// migrate применяет схему и поднимает user_version до len(steps).
// Каждый шаг — DDL, выполняемый ровно один раз за всю жизнь базы.
func migrate(db *sql.DB, steps []string) error {
	var version int
	if err := db.QueryRow(`PRAGMA user_version`).Scan(&version); err != nil {
		return err
	}
	for i := version; i < len(steps); i++ {
		tx, err := db.Begin()
		if err != nil {
			return err
		}
		if _, err := tx.Exec(steps[i]); err != nil {
			tx.Rollback()
			return fmt.Errorf("store: миграция %d: %w", i+1, err)
		}
		// PRAGMA не принимает плейсхолдеры.
		if _, err := tx.Exec(fmt.Sprintf(`PRAGMA user_version = %d`, i+1)); err != nil {
			tx.Rollback()
			return err
		}
		if err := tx.Commit(); err != nil {
			return err
		}
	}
	return nil
}
