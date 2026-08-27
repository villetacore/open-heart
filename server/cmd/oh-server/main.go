// Команда oh-server — игровой сервер OpenHeart.
//
// Один и тот же бинарник работает и как публичный дедик, и как «игра с другом»
// (клиент запускает его дочерним процессом с --embedded). docs/MULTIPLAYER.md §7.
package main

import (
	"context"
	"crypto/ed25519"
	"crypto/rand"
	"encoding/binary"
	"errors"
	"flag"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"path/filepath"
	"strconv"
	"syscall"
	"time"

	"github.com/villetacore/open-heart/server/internal/config"
	"github.com/villetacore/open-heart/server/internal/content"
	"github.com/villetacore/open-heart/server/internal/room"
	"github.com/villetacore/open-heart/server/internal/sim"
	"github.com/villetacore/open-heart/server/internal/store"
)

// Version — версия игры, которую сервер объявляет и требует от клиентов.
const Version = "0.1.0-dev"

type server struct {
	cfg         config.Server
	log         *slog.Logger
	db          *store.Server
	hub         *room.Room
	rooms       *room.Manager
	contentHash string
	masterKey   ed25519.PublicKey // для auth=master

	// relayUp закрывается, когда туннель поднят: до этого мастер не знает
	// точки входа, и анонс релейного сервера отклонит.
	relayUp chan struct{}
}

func main() {
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, "oh-server:", err)
		os.Exit(1)
	}
}

func run() error {
	cfg := config.DefaultServer()

	fs := flag.NewFlagSet("oh-server", flag.ExitOnError)
	cfgPath := fs.String("config", "server.toml", "файл конфигурации")
	listen := fs.String("listen", cfg.Listen, "адрес прослушивания")
	db := fs.String("db", cfg.DB, "файл SQLite")
	preset := fs.String("preset", cfg.Preset, "активный пресет")
	presets := fs.String("presets", cfg.Presets, "каталог пресетов")
	master := fs.String("master", cfg.Master, "базовый URL мастера")
	name := fs.String("name", cfg.Name, "имя сервера в списке")
	auth := fs.String("auth", cfg.Auth, "режим входа: master|local|open")
	core := fs.String("core", cfg.Core, "путь к core.wasm")
	public := fs.Bool("public", cfg.Public, "объявляться в списке серверов")
	relay := fs.Bool("relay", cfg.Relay, "работать через релей мастера")
	world := fs.String("world", cfg.World, "главная комната: hub|delve")
	depth := fs.Int("depth", cfg.Depth, "глубина делва при -world delve")
	embedded := fs.Bool("embedded", false, "запущен игровым клиентом")
	party := fs.Bool("party", false, "игра с другом: взять личность хоста у мастера и включить релей")
	status := fs.String("status", "", "файл состояния для запустившего клиента")
	idle := fs.Duration("idle-exit", 0, "выключиться, если столько никого нет (0 — никогда)")
	verbose := fs.Bool("v", false, "подробные логи")
	if err := fs.Parse(os.Args[1:]); err != nil {
		return err
	}

	if err := cfg.LoadServer(*cfgPath); err != nil {
		return err
	}
	config.Env(os.Getenv, map[string]*string{
		"OH_LISTEN": &cfg.Listen, "OH_DB": &cfg.DB, "OH_MASTER": &cfg.Master,
		"OH_PRESET": &cfg.Preset, "OH_PRESETS": &cfg.Presets, "OH_AUTH": &cfg.Auth,
		"OH_CORE": &cfg.Core, "OH_SERVER_ID": &cfg.ServerID, "OH_SECRET": &cfg.Secret,
		"OH_PUBLIC_URL": &cfg.PublicURL,
	})
	// Явно заданные флаги перекрывают файл и окружение.
	fs.Visit(func(f *flag.Flag) {
		switch f.Name {
		case "listen":
			cfg.Listen = *listen
		case "db":
			cfg.DB = *db
		case "preset":
			cfg.Preset = *preset
		case "presets":
			cfg.Presets = *presets
		case "master":
			cfg.Master = *master
		case "name":
			cfg.Name = *name
		case "auth":
			cfg.Auth = *auth
		case "core":
			cfg.Core = *core
		case "world":
			cfg.World = *world
		case "depth":
			cfg.Depth = *depth
		case "public":
			cfg.Public = *public
		case "relay":
			cfg.Relay = *relay
		}
	})
	if *party {
		// Игра с другом: личность и релей сервер добывает сам, чтобы клиенту
		// не пришлось хранить чужие секреты и просить у игрока регистрацию.
		if cfg.Master == "" {
			return errors.New("-party требует адрес мастера (ключ master)")
		}
		cfg.Relay = true
		cfg.Public = false
		if *idle == 0 {
			// Клиент запускает нас и забывает: без этого после каждой игры с
			// другом на машине остаётся живой сервер с открытым туннелем.
			*idle = 5 * time.Minute
		}
		if cfg.ServerID == "" || cfg.Secret == "" {
			ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
			id, secret, err := fetchPartyIdentity(ctx, cfg.Master)
			cancel()
			if err != nil {
				return fmt.Errorf("не получить личность хоста: %w", err)
			}
			cfg.ServerID, cfg.Secret = id, secret
		}
	}

	if err := cfg.Validate(); err != nil {
		return err
	}

	level := slog.LevelInfo
	if *verbose {
		level = slog.LevelDebug
	}
	log := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: level}))

	hash, err := content.Hash(filepath.Join(cfg.Presets, cfg.Preset))
	if err != nil {
		return fmt.Errorf("пресет %q: %w", cfg.Preset, err)
	}
	if hash == "" {
		log.Warn("пресет не найден, сверка content_hash отключена",
			"путь", filepath.Join(cfg.Presets, cfg.Preset))
	}

	sdb, err := store.OpenServer(cfg.DB)
	if err != nil {
		return err
	}
	defer sdb.Close()

	srv := &server{cfg: cfg, log: log, db: sdb, contentHash: hash, relayUp: make(chan struct{})}

	if cfg.Auth == config.AuthMaster {
		key, err := fetchMasterKey(cfg.Master)
		if err != nil {
			return fmt.Errorf("не получить ключ мастера: %w", err)
		}
		srv.masterKey = key
		log.Info("ключ мастера получен", "master", cfg.Master)
	}

	seed, err := hubSeed(sdb)
	if err != nil {
		return err
	}
	presetBase := filepath.Join(cfg.Presets, cfg.Preset)
	simCfg := sim.Config{
		Preset:     cfg.Preset,
		Kind:       cfg.World,
		Seed:       seed,
		Depth:      cfg.Depth,
		PartySize:  cfg.PartySize,
		PresetBase: filepath.ToSlash(presetBase),
	}
	worldSim, err := sim.New(simCfg, cfg.Core, log)
	if err != nil {
		return err
	}

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	srv.hub = room.New(room.Config{
		ID:          cfg.World,
		Kind:        cfg.World,
		Preset:      cfg.Preset,
		ContentHash: hash,
		Seed:        seed,
		Depth:       cfg.Depth,
		MaxPlayers:  cfg.MaxPlayers,
		TickRate:    20,
	}, worldSim, log)
	go srv.hub.Run(ctx)

	// Забеги живут в отдельных комнатах: свой сид, свои враги, свой тик.
	srv.rooms = room.NewManager(ctx, srv.hub, log, func(seed uint64, depth int) (sim.Sim, room.Config, error) {
		delveCfg := sim.Config{
			Preset: cfg.Preset,
			Kind:   "delve",
			Seed:   seed,
			Depth:  depth,
			// Забег начинается на одного: реальный состав комната сообщит
			// ядру сама, когда игроки зайдут.
			PartySize:  1,
			PresetBase: filepath.ToSlash(presetBase),
		}
		world, err := sim.New(delveCfg, cfg.Core, log)
		if err != nil {
			return nil, room.Config{}, err
		}
		return world, room.Config{
			Preset:      cfg.Preset,
			ContentHash: hash,
			MaxPlayers:  cfg.PartySize,
			TickRate:    20,
		}, nil
	})

	mux := http.NewServeMux()
	mux.HandleFunc("/ws", srv.handleWS)
	mux.HandleFunc("/healthz", func(w http.ResponseWriter, r *http.Request) {
		fmt.Fprintf(w, "ok players=%d rooms=%d max=%d preset=%s\n",
			srv.rooms.Players(), len(srv.rooms.Rooms()), cfg.MaxPlayers, cfg.Preset)
	})

	httpSrv := &http.Server{
		Addr:              cfg.Listen,
		Handler:           mux,
		ReadHeaderTimeout: 10 * time.Second,
	}

	if cfg.Public {
		go srv.announceLoop(ctx)
	}
	if cfg.Relay {
		go srv.relayLoop(ctx)
	}
	if *status != "" {
		// Клиент ждёт этот файл, чтобы показать код и подключиться.
		go srv.publishParty(ctx, *status)
	}
	if *idle > 0 {
		go srv.idleWatch(ctx, *idle, stop)
	}

	log.Info("сервер запущен",
		"listen", cfg.Listen, "preset", cfg.Preset, "auth", cfg.Auth,
		"world", cfg.World, "content_hash", short(hash), "core", coreLabel(cfg.Core))
	if *embedded {
		// Строка для игрового клиента, запустившего нас дочерним процессом.
		fmt.Printf("OH-READY listen=%s preset=%s hash=%s\n", cfg.Listen, cfg.Preset, hash)
		os.Stdout.Sync()
	}

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

// hubSeed — сид общего мира: постоянен для сервера, чтобы хаб не менялся между рестартами.
func hubSeed(db *store.Server) (uint64, error) {
	v, err := db.KVGet("hub_seed")
	if err == nil {
		return strconv.ParseUint(v, 10, 64)
	}
	if !errors.Is(err, store.ErrNotFound) {
		return 0, err
	}
	var buf [8]byte
	if _, err := rand.Read(buf[:]); err != nil {
		return 0, err
	}
	seed := binary.LittleEndian.Uint64(buf[:])
	return seed, db.KVSet("hub_seed", strconv.FormatUint(seed, 10))
}

func coreLabel(path string) string {
	if path == "" {
		return "заглушка"
	}
	return path
}

func short(hash string) string {
	if len(hash) <= 12 {
		return hash
	}
	return hash[:12]
}
