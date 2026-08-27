# Бэкенд OpenHeart

Три бинарника на Go. Архитектура и план — [../docs/MULTIPLAYER.md](../docs/MULTIPLAYER.md),
формат сообщений — [../protocol/schema.md](../protocol/schema.md).

| Бинарник | Что делает |
|---|---|
| `oh-server` | игровой сервер: комнаты, тик 20 Гц, вход игроков, персонажи в SQLite |
| `oh-master` | реестр публичных серверов, аккаунты и тикеты входа, пати-коды, релей (N4) |
| `oh-probe` | бот-клиент для смоук-проверок и нагрузочных прогонов |

Никаких внешних сервисов: SQLite в файле, реестр в памяти, всё остальное — стандартная
библиотека. Зависимости — websocket, wazero, sqlite (чистый Go), argon2.

## Сборка

```bash
go build ./...
```

Симуляция живёт не здесь, а в `openheart-core` (Rust) и подключается как `core.wasm` —
один и тот же код считает игру на клиенте и на сервере:

```bash
cd .. && cargo build -p openheart-core --target wasm32-unknown-unknown --features abi --release
```

Готовый модуль лежит в `target/wasm32-unknown-unknown/release/openheart_core.wasm`;
путь к нему передаётся сервером через `core = "…"` в конфиге или флагом `-core`.
Без него сервер поднимется на встроенной заглушке: игроки видят друг друга, но врагов
и урона нет.

## Быстрый старт: два игрока на одной машине

```bash
go run ./cmd/oh-server -listen 127.0.0.1:7777 -preset core -presets ../game/presets -auth open
```

В другом терминале:

```bash
go run ./cmd/oh-probe -addr ws://127.0.0.1:7777/ws -nick bot1 -seconds 5
```

Оба бота видят друг друга в снапшотах, сервер логирует вход и выход.

## Проверить бой

Комната-данж вместо хаба — врагов раздаёт сервер:

```bash
go run ./cmd/oh-server -world delve -depth 2 -preset core -presets ../game/presets -auth open -core core.wasm
```

Бот, который стреляет по ближайшему врагу:

```bash
go run ./cmd/oh-probe -addr ws://127.0.0.1:7777/ws -nick gunner -seconds 6 -attack -quiet
```

В выводе видно `enemy_died` и суммарный урон: заявки о попадании проверяет сервер,
а не клиент.

## Публичный сервер и список

```bash
# 1. Мастер
go run ./cmd/oh-master -listen 127.0.0.1:7780

# 2. Аккаунт владельца
curl -X POST http://127.0.0.1:7780/v1/auth/register -d '{"login":"admin","password":"secret123"}'

# 3. Личность сервера (печатает server_id и secret — секрет показывается один раз)
go run ./cmd/oh-master server-add -account 1 -name "Мой сервер"

# 4. Прописать server_id/secret в server.toml, поднять сервер с public = true
# 5. Проверить, что он в списке
curl "http://127.0.0.1:7780/v1/servers?preset=core"
```

## Конфигурация

`server.example.toml` и `master.example.toml` — образцы со всеми ключами.
Приоритет: значения по умолчанию → файл → переменные `OH_*` → флаги.

## Эксплуатация

- TLS не встроен: наружу ставится Caddy (`reverse_proxy 127.0.0.1:7777`), сервер слушает
  локальный порт. Для WebSocket никакой особой настройки не нужно.
- Бэкап — копия файла базы: `sqlite3 server.db "VACUUM INTO 'backup.db'"` либо просто `cp`
  при остановленном сервере.
- Здоровье: `GET /healthz` у обоих бинарников.
- Логи — `slog` в stdout, `-v` включает debug.
