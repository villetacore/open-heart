// Команда oh-master — реестр публичных серверов, аккаунты и (с этапа N4) релей.
//
// В игровом тике не участвует: если мастер лежит, идущие сессии продолжаются,
// а прямой коннект по IP работает. docs/MULTIPLAYER.md §8.
package main

import (
	"context"
	"crypto/ed25519"
	"errors"
	"flag"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/villetacore/open-heart/server/internal/auth"
	"github.com/villetacore/open-heart/server/internal/config"
	"github.com/villetacore/open-heart/server/internal/registry"
	"github.com/villetacore/open-heart/server/internal/relay"
	"github.com/villetacore/open-heart/server/internal/store"
)

type master struct {
	cfg   config.Master
	log   *slog.Logger
	db    *store.Master
	key   ed25519.PrivateKey
	list  *registry.Registry
	codes *registry.Codes
	relay *relay.Hub
	limit *limiter
}

func main() {
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, "oh-master:", err)
		os.Exit(1)
	}
}

func run() error {
	// Подкоманда server-add — локальная операторская: заводит личность игрового
	// сервера и печатает пару server_id/secret (секрет показывается один раз).
	if len(os.Args) > 1 && os.Args[1] == "server-add" {
		return serverAdd(os.Args[2:])
	}

	cfg := config.DefaultMaster()
	fs := flag.NewFlagSet("oh-master", flag.ExitOnError)
	cfgPath := fs.String("config", "master.toml", "файл конфигурации")
	listen := fs.String("listen", cfg.Listen, "адрес прослушивания")
	db := fs.String("db", cfg.DB, "файл SQLite")
	key := fs.String("key", cfg.Key, "файл ed25519-ключа")
	verbose := fs.Bool("v", false, "подробные логи")
	if err := fs.Parse(os.Args[1:]); err != nil {
		return err
	}
	if err := cfg.LoadMaster(*cfgPath); err != nil {
		return err
	}
	config.Env(os.Getenv, map[string]*string{
		"OH_LISTEN": &cfg.Listen, "OH_DB": &cfg.DB, "OH_KEY": &cfg.Key,
		"OH_PUBLIC_URL": &cfg.PublicURL,
	})
	fs.Visit(func(f *flag.Flag) {
		switch f.Name {
		case "listen":
			cfg.Listen = *listen
		case "db":
			cfg.DB = *db
		case "key":
			cfg.Key = *key
		}
	})

	level := slog.LevelInfo
	if *verbose {
		level = slog.LevelDebug
	}
	log := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: level}))

	mdb, err := store.OpenMaster(cfg.DB)
	if err != nil {
		return err
	}
	defer mdb.Close()

	// Личности «игры с другом» живут часы: чистим их при старте, чтобы база
	// не росла от каждого вечернего кооп-забега.
	if removed, err := mdb.PurgePartyHosts(); err != nil {
		return err
	} else if removed > 0 {
		fmt.Printf("убрано протухших хостов: %d\n", removed)
	}

	priv, err := auth.LoadOrCreateKey(cfg.Key)
	if err != nil {
		return err
	}

	m := &master{
		cfg:   cfg,
		log:   log,
		db:    mdb,
		key:   priv,
		list:  registry.New(registry.DefaultTTL),
		relay: relay.NewHub(),
		codes: registry.NewCodes(2 * time.Hour),
		limit: newLimiter(10, time.Minute),
	}

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()
	go m.list.Run(ctx, 30*time.Second)
	go m.limit.run(ctx)

	httpSrv := &http.Server{
		Addr:              cfg.Listen,
		Handler:           m.routes(),
		ReadHeaderTimeout: 10 * time.Second,
	}

	log.Info("мастер запущен", "listen", cfg.Listen, "db", cfg.DB,
		"pubkey", auth.PublicKeyString(priv))

	errc := make(chan error, 1)
	go func() {
		if err := httpSrv.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
			errc <- err
		}
	}()

	select {
	case err := <-errc:
		return err
	case <-ctx.Done():
		log.Info("остановка…")
		shutCtx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		return httpSrv.Shutdown(shutCtx)
	}
}

// serverAdd — `oh-master server-add -account 1 -name "Мой сервер"`.
func serverAdd(args []string) error {
	fs := flag.NewFlagSet("server-add", flag.ExitOnError)
	dbPath := fs.String("db", "master.db", "файл SQLite")
	account := fs.Int64("account", 0, "id аккаунта-владельца")
	name := fs.String("name", "", "имя сервера")
	if err := fs.Parse(args); err != nil {
		return err
	}
	if *account == 0 || *name == "" {
		return errors.New("нужны -account и -name")
	}
	mdb, err := store.OpenMaster(*dbPath)
	if err != nil {
		return err
	}
	defer mdb.Close()

	id, secret, err := mdb.RegisterServer(*account, *name)
	if err != nil {
		return err
	}
	fmt.Printf("server_id = %q\nsecret    = %q\n", id, secret)
	fmt.Println("\nПропиши обе строки в server.toml — секрет больше не показывается.")
	return nil
}
