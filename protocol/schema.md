# Протокол OpenHeart — wire schema v1

Единственный источник правды по формату сообщений. Реализации пишутся руками с обеих
сторон (`server/internal/proto`, `client/src/net`) и обязаны совпадать с этим файлом.
Архитектурный контекст — [../docs/MULTIPLAYER.md](../docs/MULTIPLAYER.md) §6.

## Транспорт

- WebSocket (текстовые кадры), путь `/ws`, TLS обязателен вне localhost.
- Кодек v1 — JSON. Бинарный кодек — этап N7, поля и семантика при переходе не меняются.
- Лимит кадра: 64 КБ. Превышение = разрыв с `reject{too_big}`.

## Конверт

Каждое сообщение — объект с типом и полезной нагрузкой:

```json
{ "t": "input", "d": { ... } }
```

`t` — строковый тег из таблиц ниже, `d` — тело (может отсутствовать).

## Порядок соединения

```
клиент                                    сервер
  │ ── hello ──────────────────────────────▶ │  проверка protocol / content_hash / auth
  │ ◀───────────────────── welcome | reject ─│
  │ ── input (30 Гц) ──────────────────────▶ │
  │ ◀────────────────────── snap (20 Гц) ────│
  │ ◀────────────────────── event (по факту) │
```

`hello` обязан быть первым сообщением, иначе `reject{bad_request}`.
До `welcome` сервер игнорирует всё, кроме `hello` и `ping`.

## Клиент → сервер

| `t` | Тело |
|---|---|
| `hello` | `protocol:int, game_version:str, preset_id:str, content_hash:str, auth:{mode, ticket?, login?, password?, nickname?, register?}` |
| `input` | `tick:u32, move:[f32,f32], yaw:f32, pitch:f32, buttons:u16, pos:[f32;3], vel:[f32;3], on_floor:bool` |
| `fire` | `tick:u32, weapon:str, origin:[f32;3], dir:[f32;3], secondary:bool` |
| `hit` | `tick:u32, weapon:str, target:u16, pos:[f32;3], part:u8` |
| `interact` | `target:u16, kind:str` |
| `cmd` | `kind:str, args:{...}` |
| `chat` | `text:str` (≤ 256 символов) |
| `ping` | `t:i64` (мс клиента) |
| `save` | `ver:int, data:str` — состояние персонажа целиком, ≤ 64 КиБ. Сервер его не разбирает: хранит и отдаёт обратно при следующем входе |

`buttons` — битовая маска:

| Бит | Значение |
|---|---|
| 0 | jump |
| 1 | sprint |
| 2 | fire |
| 3 | alt_fire |
| 4 | reload |
| 5 | use |
| 6 | crouch |

`cmd.kind`: `enter_delve`, `leave_delve`, `switch_weapon`, `reload`, `take_perk`,
`equip`, `use_item`, `respawn`, `revive`, `select_character`.

`enter_delve` принимает `args: { depth: int, owner?: u16 }`. Без `owner` сервер
заводит забег на самого игрока; с `owner` — заводит его в уже открытый инстанс
товарища. После перехода приходит новый `welcome` с новым `peer` и своим миром.

## Сервер → клиент

| `t` | Тело |
|---|---|
| `welcome` | `peer:u16, tick:u32, preset_id:str, content_hash:str, world:{kind, seed:u64, depth:int}, players:[{peer, nickname, class, level}], character?:{ver:int, data:str}` |
| `snap` | `tick:u32, ack:u32, players:[ent], enemies:[ent], projectiles:[ent], items:[ent]` |
| `event` | `kind:str, actor?:u16, target?:u16, amount?:f32, text?:str, pos?:[f32;3], extra?:{...}` |
| `reject` | `reason:str, detail?:str` |
| `pong` | `t:i64` (эхо клиентского) |

`ent` — запись сущности:

```json
{ "id": 41, "kind": 1, "t": 3, "pos": [12.5, 0.0, -3.25], "yaw": 1.57, "hp": 220, "flags": 3 }
```

| Поле | Тип | Смысл |
|---|---|---|
| `id` | u16 | стабильный `entity_id`, уникален в комнате |
| `kind` | u8 | 0 player · 1 enemy · 2 projectile · 3 item · 4 npc |
| `t` | u16 | вид врага: индекс в `enemies.json` пресета +1 (0 — не задан). Пресет один и тот же, он сверяется по `content_hash` |
| `owner` | u16 | чья это сущность: лут в кооперативе персональный, чужой предмет клиент не рисует. 0 — общая |
| `pos` | f32×3 | мировая позиция (в бинарном кодеке — i16, квант 1/64 м) |
| `yaw` | f32 | рад (в бинарном кодеке — u8) |
| `hp` | u8 | процент от максимума, 0 = мёртв |
| `flags` | u8 | биты: 0 downed · 1 stunned · 2 burning · 3 elite · 4 firing · 5 hidden |

`snap` — **дельта** к последнему подтверждённому клиентом тику: массивы содержат только
изменившиеся сущности, исчезнувшие приходят в `event{kind:"despawn"}`. Раз в 2 с
отправляется полный снапшот (кейфрейм), чтобы новый или отставший клиент синхронизировался.

`event.kind`: `damage`, `enemy_died`, `loot`, `xp`, `level_up`, `quest`, `downed`,
`revived`, `player_out`, `wipe`, `join`, `leave`, `chat`, `world_load`, `despawn`,
`pickup`, `notice`.

Про кооператив: `downed` — игрок упал и ждёт помощи, `revived` — его подняли
(`actor` — кто поднял), `player_out` — истёк кровью и выбыл до конца забега,
`wipe` — легли все, забег провален.

`reject.reason`: `protocol` (несовпадение версии протокола), `version` (версия игры),
`content` (не тот `content_hash`), `auth` (плохой тикет/пароль), `banned`, `full`,
`bad_request`, `too_big`, `rate`.

## Совместимость

- `protocol` увеличивается при любом несовместимом изменении этого файла.
- Неизвестные поля получатель **игнорирует**, неизвестные `t` — тоже (кроме `hello`).
- Совместимость проверяется с обеих сторон на одних и тех же образцах:
  `protocol/fixtures/*.json` читают `go test ./internal/proto` и
  `cargo test -p openheart-core fixtures`. Меняешь формат — обнови образцы,
  иначе упадут оба теста.
