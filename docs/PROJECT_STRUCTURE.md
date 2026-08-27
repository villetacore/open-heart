# OpenHeart — структура проекта

Карта того, что где лежит и зачем. Более детальный справочник (публичные API модулей,
счётчики, «где что искать») — в **[INDEX.md](INDEX.md)**; сетевая часть подробно
разобрана в **[MULTIPLAYER.md](MULTIPLAYER.md)**.

## Как это всё связано

```
        ┌────────────────── core (openheart-core) ───────────────────┐
        │  симуляция и данные БЕЗ движка: worldgen, боевые формулы,  │
        │  rng, модели контента                                     │
        └────────┬─────────────────────────────────┬────────────────┘
        линкуется│                                 │ --target wasm32
                 ▼                                 ▼
    ┌────────────────────────┐         ┌──────────────────────────┐
    │ client → openheart.dll │         │ core.wasm                │
    │ — GDExtension:         │         │ грузится server/ (wazero)│
    │ узлы, рендер, ввод, HUD│         │ авторитетный тик сервера │
    └───────────┬────────────┘         └──────────────────────────┘
                │ грузит через game/OpenHeart.gdextension
                ▼
    Godot/Redot (окно, рендер, физика, звук)
                │ при старте читает
                ▼
    game/presets/<id>/*.json  (оружие, классы, враги, перки, квесты, карты)
```

Три правила, из которых следует вся раскладка:

1. **Логика — в Rust, контент — в JSON, сцены и ассеты — в Godot.** Новый враг или оружие
   добавляется правкой JSON, а не кода ([DESIGN_PLAN §2](DESIGN_PLAN.md)).
2. **Всё, что считает игру, живёт в `core` и не знает про движок.** Тогда клиент и сервер
   считают одинаково по построению ([MULTIPLAYER.md §4](MULTIPLAYER.md)).
3. **Go — платформа вокруг ядра, а не вторая реализация игры**: сеть, комнаты, аккаунты,
   персист, реестр серверов.

---

## Корень репозитория

```
open-heart/
├─ README.md            — описание проекта, фичи, управление
├─ CHANGELOG.md         — история версий
├─ CONTRIBUTING.md      — правила вклада, смоук-чеклист
├─ LICENSE
│
├─ Cargo.toml           — воркспейс Rust: core + client
│
├─ core/                — правила игры: симуляция, генерация, формулы. Без движка
├─ client/              — клиент: расширение движка (GDExtension)
├─ server/              — бэкенд: игровой сервер, мастер-сервер, бот-клиент
├─ protocol/            — схема сетевого протокола (источник правды для обеих сторон)
├─ game/                — проект движка: сцены, ассеты, пресеты контента
├─ scripts/             — сборка и запуск (см. ниже)
├─ tools/               — Python-пайплайн нарезки ассетов
└─ docs/                — вся проектная документация
```

---

## `scripts/` — сборка и запуск

| Скрипт | Что делает |
|---|---|
| `run.ps1` | собрать клиентскую DLL и открыть/запустить проект в движке — самый частый способ |
| `build.ps1` / `build.sh` | сборка под все платформы (desktop, Android, Web), с экспортом |
| `build.bat` | просто `cargo build -p openheart` + копия DLL в `game/bin/` |
| `watch.ps1` | авто-пересборка DLL при сохранении `.rs` (нужен `cargo-watch`) |
| `core-wasm.ps1` / `core-wasm.sh` | собрать ядро в `core.wasm` и положить рядом с сервером |

Все скрипты вычисляют корень репозитория сами, запускать можно откуда угодно:

```powershell
.\scripts\run.ps1          # игра
.\scripts\core-wasm.ps1    # ядро для сервера
```

---

## `core/` и `client/` — код игры

```
Cargo.toml              — [workspace]: members = core, client
.cargo/config.toml      — флаги кросс-сборки (Web/Emscripten)

core/                   — openheart-core: правила игры БЕЗ движка (клиент + сервер)
└─ src/
   ├─ lib.rs            — модули и плоские имена (crate::weapon, crate::config…)
   ├─ math.rs           — Vec2/Vec3 вместо godot::builtin
   ├─ rng.rs            — детерминированный xorshift64* (общий сид → общий мир)
   ├─ log.rs            — предупреждения через хук хоста (движок / slog / stderr)
   ├─ content.rs        — чтение файлов через хук + ContentSource для наборов данных
   ├─ sim/              — авторитетная симуляция: мир, враги, урон, снапшоты
   ├─ protocol.rs       — сообщения протокола (зеркало protocol/schema.md)
   ├─ abi.rs            — C-ABI для wasm-хоста (feature = "abi")
   ├─ ai/               — net.rs (инференс сети) · policy.rs (признаки → намерение)
   ├─ craft.rs          — рецепты: проверка и выполнение
   ├─ combat/           — weapon.rs · perk.rs · status.rs: урон, резисты, перки
   ├─ data/             — модели контента и разбор форматов
   │  ├─ format.rs      RON / JSON / TOML
   │  ├─ config.rs      enemies/items/level/abilities/affixes/loot + резисты
   │  ├─ classes.rs · character.rs · item.rs · quest.rs · dialogue.rs · story.rs
   │  ├─ preset.rs      манифест пресета
   │  ├─ hash.rs        отпечаток данных пресета (сверяется с сервером)
   │  └─ preset_tests.rs проверки целостности всех пресетов (без движка)
   ├─ state/            — game_state.rs (прогресс) · save.rs (модель сейва)
   └─ worldgen/         — dungeon.rs (план данжа) · map_def.rs (формат карты)
                          · nav.rs (A* по сетке данжа)

client/                 — openheart: GDExtension (собирается в openheart.dll)
   └─ src/
      ├─ lib.rs          — регистрация классов, реэкспорт модулей
      │
      ├─ nodes/          — узлы Godot (весь слой представления)
      │  ├─ game/        — Game3D, разбитый по подсистемам:
      │  │  ├─ mod.rs        главный узел, режимы, игроки комнаты, id сущностей
      │  │  ├─ combat.rs     hitscan/мили/снаряды, урон, смерти, FX и звук
      │  │  ├─ environment.rs спавн врагов, NPC, окружение, ambient
      │  │  ├─ delve.rs      цикл данжа: генерация, вход/выход, глубина
      │  │  ├─ items.rs      пикапы, инвентарь, применение предметов
      │  │  ├─ hud.rs        построение HUD и экранов
      │  │  ├─ hud_update.rs обновление HUD по состоянию
      │  │  ├─ gameplay.rs   режимы, смерть, респавн, автосейв
      │  │  ├─ conversation.rs диалоги и квест-гиверы
      │  │  └─ class_select.rs выбор класса и спека
      │  ├─ player.rs    — Player: контроллер по PlayerInput (ввод развязан ради сети)
      │  ├─ enemy.rs     — Enemy: AI патруль→погоня→атака, способности, entity_id
      │  └─ main_menu.rs — главное меню, настройки, выбор пресета
      │
      ├─ net/            — сеть: session.rs (разговор с сервером) + сокет
      │
      ├─ data/content.rs — поиск пресетов и загрузка их файлов движком
      │
      ├─ worldgen/       — построение мира в сцене
      │  ├─ dungeon.rs   геометрия данжа по плану из ядра (меши, свет, декор)
      │  ├─ map.rs       сборка карты: меши, коллайдеры, свет (формат — в ядре)
      │  └─ world.rs     открытый мир: хаб и пустоши
      │
      ├─ state/          — файловый ввод-вывод
      │  ├─ save.rs      чтение и запись user://save.json (модель — в ядре)
      │  └─ settings.rs  язык, громкость, чувствительность мыши
      │
      ├─ support/        — утилиты
      │  ├─ gfx.rs       боксы, биллборды, свет, TexCache
      │  ├─ convert.rs   единственное место перевода Vec3/Color движок ↔ ядро
      │  └─ locale.rs    строки ru/en
      │
      └─ bin/preset_migrate.rs — миграция пресетов в новый формат
```

Оба крейта переэкспортируют модули плоскими именами, поэтому `crate::weapon`,
`crate::config`, `crate::game_state` работают одинаково — независимо от того, в каком
крейте лежит файл. Правило деления простое: **трогает движок — в `client/`, считает
игру — в `core/`.**

### Наследие визуальной новеллы

Проект начинался как визуальная новелла, и часть кода оттуда осталась:
`core/src/data/character.rs` (статы INT/CHR/FIT/REP/WIL), Period/Location/Action
в `core/src/state/game_state.rs`, весь `core/src/data/story.rs`. Диалоги NPC в хабе до сих пор работают на этой системе — она рабочая
и нужная, но **не связана** с RPG-прогрессией (уровень/XP/класс/перки). План — свести
диалоги на data-driven систему ([DESIGN_PLAN §12](DESIGN_PLAN.md)).

---

## `server/` — бэкенд

```
server/
├─ cmd/
│  ├─ oh-server/     — игровой сервер: комнаты, тик 20 Гц, вход игроков, SQLite
│  ├─ oh-master/     — реестр публичных серверов, аккаунты, тикеты, пати-коды, релей
│  └─ oh-probe/      — бот-клиент для смоук-проверок и нагрузочных прогонов
├─ internal/
│  ├─ proto/         — сообщения протокола (зеркало protocol/schema.md)
│  ├─ wire/          — соединение с игроком: интерфейс Conn и WebSocket-реализация
│  ├─ relay/         — туннель для серверов без белого IP: кадры, стримы, хаб хостов
│  ├─ room/          — комната: одна горутина, тик, рассылка снапшотов
│  ├─ sim/           — мост в core.wasm через wazero (+ заглушка до готовности ядра)
│  ├─ store/         — SQLite: юзеры, персонажи, аккаунты, сервера
│  ├─ auth/          — argon2id и ed25519-тикеты входа
│  ├─ registry/      — список серверов и пати-коды в памяти
│  ├─ content/       — content_hash пресета
│  └─ config/        — плоский TOML + env + флаги
├─ server.example.toml · master.example.toml
└─ README.md         — быстрый старт бэкенда
```

Подробности — [MULTIPLAYER.md](MULTIPLAYER.md) и [../server/README.md](../server/README.md).

## `protocol/` — протокол

`schema.md` — единственный источник правды по формату сообщений. Обе реализации
(`server/internal/proto`, `client/src/net` на этапе N1) пишутся руками по нему,
совместимость проверяется тестами с обеих сторон.

---

## `game/` — проект движка

```
game/
├─ project.godot            — настройки: инпуты, рендерер Forward Mobile (Vulkan),
│                             stretch UI (база 1920×1080), плагин-редактор oh_editor
├─ OpenHeart.gdextension    — связывает движок с openheart.dll из game/bin/
├─ main_menu.tscn           — стартовая сцена
├─ main.tscn                — сцена игры: Game3D + Player + камера
├─ bin/                     — сюда скрипты кладут собранную библиотеку (в git не хранится)
│
├─ addons/oh_editor/        — РЕДАКТОР ИГРЫ (вкладка «OpenHeart» в Godot)
│  └─ editor_main.gd        — панели контента, создание пресетов, «замок ядра»
│
├─ presets/                 — ПРЕСЕТЫ: каждый = самодостаточный набор данных = отдельная игра
│  ├─ core/                 — кампания «Неоновое Сердце»
│  │  ├─ preset.json        манифест
│  │  ├─ weapons.json · classes.json · perks.json · synergies.json
│  │  ├─ enemies.json · abilities.json · affixes.json · items.json · loot.json
│  │  ├─ npcs.json · quests.json · dialogues.json · statuses.json
│  │  ├─ level.json         legacy-спавны (если у пресета нет карты)
│  │  └─ maps/hub.json      карта мира: блоки, здания, свет, неон, спавны, врата
│  └─ arena/                — «Кровавая арена»: другой набор данных = другая игра
│
├─ data/weapons_fp.json     — манифест спрайт-листов оружия (генерируется tools/)
└─ assets/                  — спрайты, текстуры, эффекты, UI, небо
   ├─ sprites/{characters,weapons_fp,pickups,projectiles,props,items}
   ├─ textures/{dungeon,sky} · textures_raw/ · sprites_raw/
   ├─ effects/ · ui/
   └─ sprites/SPRITE_SPEC.md — спецификация форматов спрайтов
```

Пресет и есть игра: сервер публикует `content_hash` по этим файлам, и клиент с другим
набором данных к нему не подключится ([MULTIPLAYER.md §8.1](MULTIPLAYER.md)).

---

## `tools/` — генераторы и пайплайн ассетов

Python-скрипты, которые не входят в игру, но собирают то, что руками не поддержать.
Полный гайд по промптам генерации ассетов — `tools/ASSET_GUIDE.md`.

`build_hub_map.py` собирает `game/presets/core/maps/hub.json` из плана города: улицы,
кварталы, фасады, неон и свет. Он же следит за тем, чтобы дома не встали на NPC и
точки спавна, а ламп на плитку земли не стало больше, чем тянет мобильный рендер.
Править карту руками можно, но следующий прогон генератора это затрёт — меняй план
в скрипте. `--plan out.png` рисует вид сверху, `--check` проверяет, что файл в репозитории
совпадает с генератором.

Игру глазами движка снимают две отладочные сцены в `game/`: `dev_shot.tscn` грузит основную
сцену и сохраняет кадры с заданных точек (первый — от лица игрока, поэтому годится и для
проверки сетевого режима), `dev_menu.tscn` прокликивает меню и снимает экран сетевой игры.
Оба кладут PNG в `user://`.

```bash
python tools/build_hub_map.py --plan plan.png
godot --path game --resolution 1600x900 res://dev_shot.tscn
godot --path game --resolution 1600x900 res://dev_menu.tscn
```

Нарезка атласов:

```
1. slice_atlases.py  — основная нарезка: оружие, пикапы, эффекты, UI, небо, тайлы, пропсы
2. slice_fix.py      — правочный проход: слипшиеся ячейки, подписи, чистка
3. slice_props.py    — точная нарезка пропсов по вручную измеренным координатам
```

Результат кладётся в `game/assets/…`, контрольные листы — в `tools/preview/` (в `.gitignore`).
Остальные скрипты (`analyze_*.py`, `extract_atlas.py`, `debug_*.py`, `scan_*.py`) —
одноразовые инструменты из начала пайплайна, оставлены для справки.

---

## Где что искать, если нужно…

| Хочу… | Файл(ы) |
|---|---|
| **Редактировать контент без файлов** | Godot-редактор → вкладка **OpenHeart** |
| Добавить/поменять оружие | `game/presets/<id>/weapons.json` |
| Добавить/поменять врага | `game/presets/<id>/enemies.json` |
| Добавить перк или синергию | `game/presets/<id>/perks.json`, `synergies.json` |
| Поменять класс или спек | `game/presets/<id>/classes.json` |
| Поменять карту мира | `tools/build_hub_map.py` (хаб генерируется), затем `game/presets/<id>/maps/hub.json` |
| Добавить NPC или квест | `game/presets/<id>/npcs.json`, `quests.json` |
| Сделать «другую игру» | вкладка OpenHeart → «Создать копию» пресета |
| Поменять реплику NPC | `core/src/data/story.rs` (пока хардкод) |
| Поменять HUD | `client/src/nodes/game/hud.rs`, `hud_update.rs` |
| Поменять движение игрока | `client/src/nodes/player.rs` |
| Поменять AI врага | `client/src/nodes/enemy.rs` (движение), `core/src/worldgen/nav.rs` (путь) |
| Поменять планировку данжа | `core/src/worldgen/dungeon.rs` (комнаты, коридоры, сетка) |
| Поменять внешний вид данжа | `client/src/worldgen/dungeon.rs` (меши, свет, декор) |
| Поменять геометрию хаба | `client/src/worldgen/world.rs` |
| Добавить тип урона или статус | `core/src/combat/weapon.rs`, `core/src/combat/status.rs` |
| Поменять то, что считает **сервер** | `core/src/sim.rs` (и пересобрать `core.wasm`) |
| Поменять формат сетевых сообщений | `protocol/schema.md` + `server/internal/proto` + `core/src/sim.rs` |
| Поднять сервер или мастер | `server/README.md` |
| Добавить спрайт | `tools/ASSET_GUIDE.md` → `tools/slice_*.py` |
| Понять планы | `docs/DESIGN_PLAN.md`, `docs/MULTIPLAYER.md` |
| Понять открытые вопросы геймдизайна | `docs/OPEN_QUESTIONS.md` |
