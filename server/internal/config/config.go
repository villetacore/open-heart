// Package config — конфигурация серверов.
//
// Порядок приоритета: значения по умолчанию → файл (плоский TOML) → env (OH_*) → флаги.
// Парсер намеренно свой и крошечный: конфиг плоский, ради дюжины ключей тянуть
// библиотеку незачем (см. docs/MULTIPLAYER.md §2).
package config

import (
	"bufio"
	"errors"
	"fmt"
	"os"
	"strconv"
	"strings"
)

// Server — конфиг игрового сервера (oh-server).
type Server struct {
	Name       string // имя в списке серверов
	MOTD       string
	Preset     string // активный пресет контента
	Region     string
	MaxPlayers int
	PartySize  int
	Auth       string // master | local | open
	Password   string // пароль на вход, пусто = без пароля
	Listen     string
	Master     string // базовый URL мастера, пусто = не объявляться
	Public     bool   // объявляться в списке
	Relay      bool   // работать через релей мастера (нет белого IP)
	DB         string
	Core       string // путь к core.wasm; пусто = встроенная заглушка симуляции
	World      string // вид главной комнаты: hub | delve (delve — для проверки боя)
	Depth      int    // глубина делва, если World = delve
	Presets    string // каталог пресетов для расчёта content_hash
	PublicURL  string // как сервер виден снаружи: wss://host:port/ws
	ServerID   string // выдаётся мастером при /v1/servers/register
	Secret     string // секрет сервера, показывается мастером один раз
}

// Master — конфиг мастер-сервера (oh-master).
type Master struct {
	Listen    string
	DB        string
	Key       string // файл с ed25519-ключом, создаётся при первом запуске
	PublicURL string // как мастер виден снаружи (для relay-эндпоинтов)
	Relay     bool
}

// Режимы авторизации.
const (
	AuthMaster = "master"
	AuthLocal  = "local"
	AuthOpen   = "open"
)

func DefaultServer() Server {
	return Server{
		Name:       "OpenHeart server",
		MOTD:       "",
		Preset:     "core",
		Region:     "eu",
		MaxPlayers: 16,
		PartySize:  4,
		Auth:       AuthOpen,
		Listen:     ":7777",
		Public:     false,
		Relay:      false,
		DB:         "server.db",
		World:      "hub",
		Depth:      1,
		Presets:    "../game/presets",
	}
}

func DefaultMaster() Master {
	return Master{
		Listen:    ":7780",
		DB:        "master.db",
		Key:       "master.key",
		PublicURL: "",
		Relay:     true,
	}
}

// LoadServer читает конфиг сервера. Отсутствие файла — не ошибка.
func (c *Server) LoadServer(path string) error {
	kv, err := readFlat(path)
	if err != nil || kv == nil {
		return err
	}
	for k, v := range kv {
		var err error
		switch k {
		case "name":
			c.Name = v
		case "motd":
			c.MOTD = v
		case "preset":
			c.Preset = v
		case "region":
			c.Region = v
		case "max_players":
			c.MaxPlayers, err = strconv.Atoi(v)
		case "party_size":
			c.PartySize, err = strconv.Atoi(v)
		case "auth":
			c.Auth = v
		case "password":
			c.Password = v
		case "listen":
			c.Listen = v
		case "master":
			c.Master = v
		case "public":
			c.Public, err = strconv.ParseBool(v)
		case "relay":
			c.Relay, err = strconv.ParseBool(v)
		case "db":
			c.DB = v
		case "core":
			c.Core = v
		case "world":
			c.World = v
		case "depth":
			c.Depth, err = strconv.Atoi(v)
		case "presets":
			c.Presets = v
		case "public_url":
			c.PublicURL = v
		case "server_id":
			c.ServerID = v
		case "secret":
			c.Secret = v
		default:
			return fmt.Errorf("config %s: неизвестный ключ %q", path, k)
		}
		if err != nil {
			return fmt.Errorf("config %s: ключ %q: %w", path, k, err)
		}
	}
	return nil
}

// LoadMaster читает конфиг мастера. Отсутствие файла — не ошибка.
func (c *Master) LoadMaster(path string) error {
	kv, err := readFlat(path)
	if err != nil || kv == nil {
		return err
	}
	for k, v := range kv {
		var err error
		switch k {
		case "listen":
			c.Listen = v
		case "db":
			c.DB = v
		case "key":
			c.Key = v
		case "public_url":
			c.PublicURL = v
		case "relay":
			c.Relay, err = strconv.ParseBool(v)
		default:
			return fmt.Errorf("config %s: неизвестный ключ %q", path, k)
		}
		if err != nil {
			return fmt.Errorf("config %s: ключ %q: %w", path, k, err)
		}
	}
	return nil
}

// Validate проверяет конфиг сервера на очевидные противоречия.
func (c *Server) Validate() error {
	switch c.Auth {
	case AuthMaster, AuthLocal, AuthOpen:
	default:
		return fmt.Errorf("auth: ожидается master|local|open, получено %q", c.Auth)
	}
	if c.Auth == AuthMaster && c.Master == "" {
		return errors.New("auth=master требует адрес мастера (ключ master)")
	}
	if (c.Public || c.Relay) && c.Master == "" {
		return errors.New("public/relay требуют адрес мастера (ключ master)")
	}
	if (c.Public || c.Relay) && (c.ServerID == "" || c.Secret == "") {
		return errors.New("public/relay требуют server_id и secret (получить: oh-master register)")
	}
	if c.Public && !c.Relay && c.PublicURL == "" {
		return errors.New("public без relay требует public_url (wss://host:port/ws)")
	}
	if c.MaxPlayers < 1 || c.MaxPlayers > 256 {
		return fmt.Errorf("max_players: ожидается 1..256, получено %d", c.MaxPlayers)
	}
	if c.PartySize < 1 || c.PartySize > c.MaxPlayers {
		return fmt.Errorf("party_size: ожидается 1..max_players, получено %d", c.PartySize)
	}
	if c.World != "hub" && c.World != "delve" {
		return fmt.Errorf("world: ожидается hub|delve, получено %q", c.World)
	}
	return nil
}

// Env применяет переменные окружения вида OH_LISTEN, OH_DB, OH_MASTER…
func Env(get func(string) string, apply map[string]*string) {
	for name, dst := range apply {
		if v := get(name); v != "" {
			*dst = v
		}
	}
}

// readFlat разбирает файл вида `key = value`, `# комментарий`, значения в кавычках.
// Возвращает nil, nil если файла нет.
func readFlat(path string) (map[string]string, error) {
	if path == "" {
		return nil, nil
	}
	f, err := os.Open(path)
	if errors.Is(err, os.ErrNotExist) {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}
	defer f.Close()

	kv := make(map[string]string)
	sc := bufio.NewScanner(f)
	line := 0
	for sc.Scan() {
		line++
		s := strings.TrimSpace(sc.Text())
		if s == "" || strings.HasPrefix(s, "#") || strings.HasPrefix(s, "[") {
			continue
		}
		k, v, ok := strings.Cut(s, "=")
		if !ok {
			return nil, fmt.Errorf("config %s:%d: ожидается `ключ = значение`", path, line)
		}
		k = strings.TrimSpace(k)
		v = strings.TrimSpace(v)
		if i := strings.Index(v, " #"); i >= 0 {
			v = strings.TrimSpace(v[:i])
		}
		if len(v) >= 2 && (v[0] == '"' && v[len(v)-1] == '"') {
			unq, err := strconv.Unquote(v)
			if err != nil {
				return nil, fmt.Errorf("config %s:%d: %w", path, line, err)
			}
			v = unq
		}
		kv[k] = v
	}
	return kv, sc.Err()
}
