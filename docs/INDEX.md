# 📇 Индекс проекта OpenHeart

Достоверная карта репозитория для быстрого анализа: разделение на крейты, каждый
подпакет с зоной ответственности, данные пресета, ассеты, сервер и инструменты.
Обновлён 2026-09-20 под текущее дерево кода.

> Это структурная карта «файл → за что отвечает», а не полный список сигнатур.
> Точные пути каталогов — в [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md); как что
> связано в рантайме — в [ARCHITECTURE.md](ARCHITECTURE.md).

**Сводка:** `core/` ≈ 13 800 строк Rust (94 теста) · `client/` ≈ 18 800 строк Rust
(GDExtension) · `server/` ≈ 6 400 строк Go · пресет `core` = 19 контент-стемов
(JSON+RON) + карты · ~200 PNG-ассетов · Python-пайплайн ассетов в `tools/`.

Три крейта, одно ядро правды:

| Крейт | Что | Собирается в |
|---|---|---|
| `core/` (`openheart-core`) | Чистая симуляция и данные без движка; общее для клиента и сервера | rlib + `core.wasm` для сервера |
| `client/` (`openheart`) | GDExtension: ноды, рендер, ввод, HUD, сеть | `openheart.dll/.so/.dylib` |
| `server/` | Go: `oh-server`, `oh-master`, `oh-probe` | бинарники ([server/README.md](../server/README.md)) |

---

## 1. Корень репозитория

| Файл | Назначение |
|---|---|
| `README.md` | Витрина проекта: фичи, быстрый старт, управление |
| `CONTRIBUTING.md` | Правила вклада, смоук-чеклист, стиль |
| `CHANGELOG.md` | История версий (Keep a Changelog) |
| `LICENSE` | MIT |
| `scripts/run.ps1` | Сборка Rust + запуск редактора (знает winget-путь Godot) |
| `scripts/watch.ps1` | Автопересборка DLL при сохранении `.rs` |
| `scripts/build.{bat,ps1,sh}` | Сборка под платформы + экспорт |
| `scripts/core-wasm.{ps1,sh}` | Сборка ядра в `core.wasm` для сервера |

## 2. `core/` — симуляция и данные (без движка)

Точка входа — `lib.rs`. Компилируется и в `core.wasm` (`abi.rs` — C-ABI для Go-хоста).
Разложено по подпакетам:

### 2.1 Верхний уровень

| Модуль | Отвечает за |
|---|---|
| `lib.rs` | Дерево модулей, реэкспорты, регистрация |
| `abi.rs` | C-ABI поверхности симуляции для WASM-хоста (сервер) |
| `content.rs` | Абстракция чтения файлов пресета (хост подставляет источник) |
| `craft.rs` | Логика крафта по рецептам |
| `math.rs`, `rng.rs`, `log.rs` | `Vec3`/утилиты, детерминированный RNG, логирование |
| `protocol.rs` + `protocol_fixtures_tests.rs` | Сетевой протокол (источник правды с Go) + фикстуры |

### 2.2 `data/` — data-driven контент

| Модуль | Отвечает за |
|---|---|
| `format.rs` | Форматы пресета: приоритет **RON → JSON → TOML**, общий разбор |
| `hash.rs` + `hash_disk_tests.rs` | Отпечаток пресета (sha256), обязан совпадать с Go; пин-хеш в тесте |
| `config.rs` | `GameConfig`: enemies/items/level/npcs/quests/abilities/статусы/данж/лут/рецепты/brains + **player_abilities** |
| `classes.rs`, `perk.rs` (в `combat/`) | Классы/спеки; дерево перков и синергии |
| `item.rs`, `quest.rs`, `recipe_tests.rs` | Инвентарь, журнал квестов, тесты рецептов |
| `dialogue.rs`, `story.rs`, `character.rs` | Data-driven сцены (приоритет над `story.rs`); авторские сцены; VN-статы |
| `preset.rs` + `preset_tests.rs` | Загрузка/линковка пресета; тесты «парсится и связывается», JSON↔RON без дрейфа |

### 2.3 `combat/` — боевые определения

| Модуль | Отвечает за |
|---|---|
| `weapon.rs` | 8 оружий, типы урона/боезапаса, магазины и релоад, арсенал |
| `ability.rs` | **Умения игрока**: `AbilityDef`, `AbilityState` (кулдауны, guard/speed), прицеливание, валидация (2 умения × 3 класса) |
| `perk.rs` | Перки, синергии, агрегация модификаторов |
| `status.rs` | Статусы урона (dot/slow/stun/vulnerable) |
| `mod.rs` | Общая боевая математика/связки |

### 2.4 `sim/` — авторитетная симуляция (offline и сервер)

| Модуль | Отвечает за |
|---|---|
| `mod.rs` | `State`: игроки, враги, тик, урон, движение, античит-лимиты скорости |
| `abilities.rs` | Серверная валидация каста умения игрока (кулдаун/смерть/оглушение/видимость/цели/лечение) |
| `enemy.rs`, `damage.rs`, `loot.rs` | ИИ врагов, расчёт урона/резистов, дроп |
| `world.rs` | Проходимость/геометрия для симуляции |
| `rescue.rs` + `*_tests.rs`, `coop_tests.rs`, `party_tests.rs`, `threat_tests.rs`, `brain_tests.rs` | Воскрешение, кооп, партия, агро, обученные поведения — с тестами |

### 2.5 `ai/`, `state/`, `worldgen/`

| Модуль | Отвечает за |
|---|---|
| `ai/policy.rs`, `ai/net.rs`, `ai/mod.rs` | `Brain` (обученные веса поведения), валидация, инференс |
| `state/game_state.rs` | Всё «чистое» состояние игрока/мира: уровень/XP/перки/флаги/сердца/seed (+ VN-наследие для диалогов) |
| `state/save.rs` | Сериализация `user://save.json`; новые поля через `#[serde(default)]` |
| `worldgen/dungeon.rs` + `dungeon_tests.rs` | Процедурный данж: комнаты/высоты/темы/босс/`floor_map`, тесты связности |
| `worldgen/map_def.rs`, `nav.rs`, `mod.rs` | Схема карты из данных; A*-навигация врагов |

## 3. `client/` — GDExtension (Godot/Redot)

Точка входа расширения — `lib.rs` (`impl ExtensionLibrary`); классы `Game3D`, `Player`,
`Enemy`, `MainMenu`, `Npc` регистрируются автоматически через `#[derive(GodotClass)]`.

### 3.1 `nodes/` — игровые ноды

| Модуль | Отвечает за |
|---|---|
| `nodes/player.rs` | FPS-контроллер (WASD/мышь/прыжок/спринт), статы задаёт `Game3D` |
| `nodes/enemy.rs` | Нода врага: AI-обёртка, резисты, анимация листа, hurt-flash |
| `nodes/npc.rs`, `nodes/main_menu.rs` | NPC-обёртка; меню (новая игра/продолжить/кооп/пресеты/настройки) |

### 3.2 `nodes/game/` — главный узел `Game3D` (разбит на модули)

`mod.rs` держит `struct Game3D`, константы, диспетчеры `#[func]` и объявления подмодулей.
Каждый подмодуль — `impl Game3D` через `use super::*;`.

| Модуль | Отвечает за |
|---|---|
| `mod.rs` | Определение `Game3D`, enum-ы режимов/полезной нагрузки, аксессоры игроков, диспетчеры UI |
| `lifecycle.rs` | Godot-виртуальные `init`/`ready`/`input`/`process` |
| `tables.rs` | Статические таблицы: `NPC_DATA` (фолбэк), лукапы спрайтов/сцен |
| `gameplay.rs` | Ввод в игре, взаимодействия, пикапы, порталы, привязка умений F/G |
| `combat.rs` | Боёвка, эффекты, урон от врагов, статусы игрока |
| `abilities.rs` | **Умения игрока**: каст (offline и сеть), FX, HUD способностей |
| `hud.rs`, `hud_update.rs` | Построение и обновление HUD (здоровье/оружие/цель/миникарта) |
| `items.rs`, `crafting.rs`, `interface.rs` | Инвентарь, крафт, общие UI-компоненты панелей |
| `class_select.rs`, `conversation.rs` | Выбор класса/спека; диалоги и квест-раннер |
| `environment.rs`, `delve.rs` | Построение мира/освещения; данж-цикл |
| `campaign.rs` | Финал кампании: завершение, эпилог, титры, флаг завершения |
| `net.rs` | Сетевой клиент игры: применение событий сервера к сцене |
| `runtime_smoke.rs` | Debug-only смоук-хуки для `game/tools/runtime_smoke.gd` (`#[godot_api(secondary)]`) |

### 3.3 `net/`, `data/`, `state/`, `support/`, `worldgen/`, `bin/`

| Модуль | Отвечает за |
|---|---|
| `net/mod.rs`, `session.rs`, `host.rs`, `master.rs` | Сетевая сессия, хост, связь с мастер-сервером/релеем |
| `data/content.rs` | ContentDb: загрузка пресета в рантайме, поиск в `res://`+`user://` |
| `state/save.rs`, `settings.rs` | Обёртки сейва/настроек над ядром (`user://`) |
| `support/gfx.rs`, `convert.rs`, `locale.rs` | Построение мешей/спрайтов/света; конвертация Vec3↔Vector3; локализация HUD/меню |
| `worldgen/world.rs`, `map.rs`, `dungeon.rs` | Legacy-мир кодом (фолбэк), сборка карты и данжа в сцену |
| `bin/preset_migrate.rs` | CLI-миграция контента пресета в RON |

## 4. `server/` — Go-бэкенд

| Путь | Отвечает за |
|---|---|
| `cmd/oh-server/` | Игровой сервер комнаты: сессии, персонажи, партия, релей, анонс |
| `cmd/oh-master/` | Мастер-сервер: список серверов, лимиты, релей туннелей |
| `cmd/oh-probe/` | Диагностическая утилита |
| `internal/sim/` | Хост симуляции через `core.wasm` (`sim.go`/`wasm.go`/`stub.go`) |
| `internal/room/` | Логика комнаты и лута (+ тесты) |
| `internal/relay/` | `StreamConn` поверх туннеля (`conn.go` — исправлен race `SendRaw`/`Close`), кадры, хаб |
| `internal/proto/` | Протокол на Go — зеркало `core/protocol.rs` (+ фикстуры) |
| `internal/content/` | `Hash(presetDir)` — совпадает с `core/src/data/hash.rs` |
| `internal/auth/`, `store/`, `registry/`, `config/`, `wire/` | Аутентификация, хранилище (сервер/мастер/персонажи), реестр, конфиг, транспорт |

## 5. `game/` — проект движка

| Путь | Назначение |
|---|---|
| `project.godot` | Godot 4.7, рендерер mobile (Forward Mobile/Vulkan), стретч 1920×1080, input-actions (в т.ч. `ability_primary`/`ability_secondary`) |
| `OpenHeart.gdextension` | Платформа → `res://bin/openheart.{dll,so,dylib}`; `reloadable=true` |
| `main_menu.tscn`, `main.tscn` | Стартовая и игровая сцены |
| `addons/oh_editor/` | Редактор игры (вкладка OpenHeart): схемы категорий, CRUD, копия пресета, «замок ядра» |
| `tools/runtime_smoke.gd`, `presentation_smoke.gd` | Смоук-сценарии (дёргают `runtime_smoke_*` из клиента) |

### 5.1 `game/presets/core` — контент («данные = игра»)

Каждый контент-стем есть в **RON (авторский) и JSON**; при наличии RON игра читает его
первым (см. `data/format.rs`). Тест `json_and_ron_sources_do_not_drift` следит за совпадением.
Форматы — [DATA_FORMATS.md](DATA_FORMATS.md).

Стемы: `weapons`, `weapon_mods`, `classes`, `perks`, `synergies`, `enemies`, `items`,
`npcs`, `quests`, `dialogues`, `abilities` (враги), **`player_abilities`** (умения игрока),
`affixes`, `statuses`, `dungeon`, `loot`, `level`, `recipes`, `preset` (+ `maps/hub.json`).

### 5.2 `game/assets/`

Спрайты персонажей/оружия/пикапов/пропсов, эффекты, UI, текстуры хаба и тем данжа, небо.
Иконки контента — атласами: `icons/abilities_atlas.png`, `icons/items_atlas.png`,
`portraits/classes_atlas.png` (индекс задаёт `icon` в данных). Исходники — в `*_raw/`.

## 6. `tools/` — пайплайн ассетов (Python + Pillow)

| Скрипт | Назначение |
|---|---|
| `aigen.py` | ИИ-генерация ассетов (клиент к SD-серверу) + постобработка |
| `slice_atlases.py`, `slice_props.py`, `slice_fix.py` | Нарезка атласов/пропсов, правочные проходы |
| `build_hub_map.py` | Генерация `maps/hub.json` |
| `ASSET_GUIDE.md` | Промпты и спецификации форматов |

## 7. Быстрые ответы («мне нужно…»)

| Вопрос | Ответ |
|---|---|
| Точка входа логики? | `client/src/lib.rs` → `Game3D` в `main.tscn` → `nodes/game/lifecycle.rs::ready()` |
| Где загружаются данные? | `content::load_preset()` + `core::data::config::GameConfig::load_from()` |
| Как выбирается карта? | `worldgen/map::load_map(preset,"hub")`; нет файла → `worldgen/world::build_world()` (legacy) |
| Где урон? | `nodes/game/combat.rs` → `Enemy::take_damage`; в сети — `core::sim` (авторитетно) |
| Где умения игрока? | Данные `player_abilities.*` → `core/combat/ability.rs` (+ `sim/abilities.rs` на сервере) → `nodes/game/abilities.rs` (ввод F/G в `gameplay.rs`) |
| Где прокачка? | `core/state/game_state.rs::add_xp` → `classes::compute_loadout` + `perk::mods_for` |
| Где сейв? | `core/state/save.rs` ↔ `user://save.json` |
| Финал кампании? | `nodes/game/campaign.rs` (проверка на `dungeon_depth == 4`) |
| Почему клиента не пускает на сервер? | Разошёлся `content_hash` пресета — сверь `core/data/hash.rs` ↔ `server/internal/content`; при изменении контента обнови пин-хеш в `core/src/data/hash_disk_tests.rs` |
| Как сделать «свою игру»? | Скопировать пресет (вкладка OpenHeart → «Создать копию») — [EDITOR.md](EDITOR.md) |
