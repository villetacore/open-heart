# 📇 Индекс проекта OpenHeart

Достоверная карта репозитория: разделение на крейты, каждый подпакет с зоной
ответственности, данные пресета, ассеты, сервер и инструменты.
Обновлён 2026-09-27 под текущее дерево кода.

> Структурная карта «файл → за что отвечает», не полный список сигнатур. Точные
> пути каталогов — в [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md); связи в рантайме
> — в [ARCHITECTURE.md](ARCHITECTURE.md).

**Сводка:** `core/` ≈ 14 500 строк Rust (**111 тестов**) · `client/` ≈ 20 000 строк
Rust (GDExtension) · `server/` ≈ 6 500 строк Go · пресет `core` = 19 контент-стемов
(JSON+RON) + карта · ~230 PNG-ассетов · Python/PS-пайплайн ассетов в `tools/`.

Три крейта, одно ядро правды:

| Крейт | Что | Собирается в |
|---|---|---|
| `core/` (`openheart-core`) | Чистая симуляция и данные без движка; общее для клиента и сервера | rlib + `core.wasm` для сервера |
| `client/` (`openheart`) | GDExtension: ноды, рендер, ввод, HUD, сеть | `openheart.dll/.so/.dylib` |
| `server/` | Go: `oh-server`, `oh-master`, `oh-probe` | бинарники ([server/README.md](../server/README.md)) |

---

## 1. Корень

| Файл | Назначение |
|---|---|
| `README.md`, `CONTRIBUTING.md`, `CHANGELOG.md`, `LICENSE` | Витрина, правила вклада, история, MIT |
| `scripts/run.ps1`, `watch.ps1`, `build.{bat,ps1,sh}` | Сборка Rust + запуск/экспорт (знают winget-путь Godot) |
| `scripts/core-wasm.{ps1,sh}` | Сборка ядра в `core.wasm` для сервера |
| `.github/workflows/pipeline.yml` | Тесты → смоук-барьер (runtime+content+evening) → авто-тег → сборка платформ → релиз |

## 2. `core/` — симуляция и данные (без движка)

Точка входа — `lib.rs`; `abi.rs` — C-ABI для WASM-хоста (сервер).

### 2.1 Верхний уровень / `data/`

| Модуль | Отвечает за |
|---|---|
| `lib.rs`, `abi.rs`, `content.rs` | Дерево модулей; C-ABI симуляции; чтение файлов пресета |
| `math.rs`, `rng.rs`, `log.rs`, `craft.rs` | `Vec3`/утилиты; детерминированный RNG; лог; логика крафта |
| `protocol.rs` (+`protocol_fixtures_tests.rs`) | Сетевой протокол — источник правды с Go |
| `data/format.rs` | Форматы пресета: приоритет **RON → JSON → TOML** |
| `data/hash.rs` (+`hash_disk_tests.rs`) | Отпечаток пресета (sha256), обязан совпасть с Go; пин-хеш в тесте |
| `data/config.rs` | `GameConfig`: enemies/items/level/npcs/quests/abilities/статусы/данж/лут/рецепты/brains/player_abilities; цены магазина (`shop_price`/`sell_price`) |
| `data/classes.rs` | Классы/спеки; `Loadout` (+`charge_slot`, `crit_bonus`); `compute_loadout` |
| `data/preset.rs` (+`preset_tests.rs`) | Загрузка/линковка пресета; тесты «парсится и связывается», JSON↔RON без дрейфа, паспорт баланса оружия |
| `data/item.rs`, `quest.rs`, `story.rs`, `dialogue.rs`, `character.rs` | Инвентарь; журнал; авторские/data-driven сцены; VN-статы |
| `data/recipe_tests.rs`, `evening_tests.rs` | Тесты крафта и контент-пака соседей «Свет для общего стола» |

### 2.2 `combat/` — боевые определения

| Модуль | Отвечает за |
|---|---|
| `weapon.rs` | 8 оружий, типы урона/боезапаса, магазины+релоад, **крит (crit_chance/crit_mult)**, арсенал |
| `ability.rs` | Умения игрока: `AbilityDef`, `AbilityState` (кулдауны, **заряды**, guard/speed), валидация 2×3 |
| `perk.rs` | Перки, синергии, `PerkMods` (hp/speed/dmg/cd/lifesteal/ammo/**crit**) |
| `status.rs`, `mod.rs` | Статусы урона (dot/slow/stun/vulnerable); общая математика |

### 2.3 `sim/` — авторитетная симуляция (offline и сервер)

| Модуль | Отвечает за |
|---|---|
| `mod.rs` | `State`: игроки, враги, тик, движение, античит-лимиты |
| `damage.rs` | Расчёт урона: резисты и **крит** (слабые точки — на клиенте, `combat.rs`) |
| `abilities.rs` | Серверная валидация каста умения игрока |
| `enemy.rs`, `loot.rs`, `world.rs` | ИИ врагов; дроп; проходимость |
| `rescue.rs`, `*_tests.rs` (coop/party/threat/brain/rescue/tests) | Воскрешение, кооп, агро, поведения — с тестами |

### 2.4 `ai/`, `state/`, `worldgen/`

| Модуль | Отвечает за |
|---|---|
| `ai/policy.rs`, `net.rs`, `mod.rs` | `Brain` (обученные веса поведения), валидация, инференс |
| `state/game_state.rs` | Состояние игрока/мира: уровень/XP/перки/флаги/сердца/seed; экономика (`spend_gold`, сброс/крафт перков) |
| `state/save.rs` | Сериализация `user://save.json`; новые поля через `#[serde(default)]` |
| `worldgen/dungeon.rs` (+`dungeon_tests.rs`) | Процедурный данж: комнаты/высоты/темы/босс/`floor_map` |
| `worldgen/map_def.rs`, `nav.rs` | Схема карты (районы/маяки/маршруты/**станции**); A*-навигация |

## 3. `client/` — GDExtension (Godot/Redot)

Точка входа расширения — `lib.rs` (`ExtensionLibrary`); классы `Game3D`/`Player`/
`Enemy`/`MainMenu`/`Npc` регистрируются через `#[derive(GodotClass)]`.

### 3.1 `nodes/game/` — узел `Game3D` (разбит на модули; каждый — `impl Game3D` через `use super::*`)

| Модуль | Отвечает за |
|---|---|
| `mod.rs` | `struct Game3D`, enum-ы, аксессоры игроков, диспетчеры `#[func]` |
| `lifecycle.rs` | Godot-виртуальные `init`/`ready`/`input`/`process` |
| `tables.rs` | Статические таблицы: `NPC_DATA` (фолбэк), лукапы спрайтов/сцен |
| `gameplay.rs` | Ввод, взаимодействия, пикапы, порталы, привязка умений F/G |
| `combat.rs` | Боёвка, **крит (offline)**, эффекты, урон врагов, статусы игрока |
| `abilities.rs` | Умения игрока: каст (offline+сеть), заряды, FX, HUD |
| `hud.rs`, `hud_update.rs`, `interface.rs` | HUD; обновление; общие UI-компоненты панелей |
| `items.rs`, `crafting.rs`, `shop.rs` | Инвентарь; крафт (рецепты+станции); **магазин Торговца** (купля/продажа) |
| `class_select.rs`, `conversation.rs` | Выбор класса/спека; диалоги, квест-раннер |
| `environment.rs`, `delve.rs` | Построение мира/освещения/станций; данж-цикл, гарантия хазарда |
| `campaign.rs` | Финал: эпилог, титры, флаг завершения |
| `creative.rs` | Креатив-режим (F2/меню): всё доступно + **боевая арена** (волны врагов) |
| `presentation.rs` | Декор/арт соседей (контент-пак), загрузка `.tres` |
| `net.rs` | Сетевой клиент игры |
| `runtime_smoke.rs`, `content_smoke.rs` | Debug-only смоук-хуки (`#[godot_api(secondary)]`) |

### 3.2 `nodes/`, `net/`, `data/`, `state/`, `support/`, `worldgen/`, `bin/`

| Модуль | Отвечает за |
|---|---|
| `nodes/player.rs`, `enemy.rs`, `npc.rs`, `main_menu.rs` | FPS-контроллер; нода врага; NPC; меню (+кнопки Креатив/Арена) |
| `net/{mod,session,host,master}.rs` | Сетевая сессия, хост, мастер-сервер/релей |
| `data/content.rs` | ContentDb: загрузка пресета, поиск в `res://`+`user://` |
| `state/{save,settings}.rs` | Обёртки сейва/настроек (сложность, язык, звук) |
| `support/{gfx,convert,locale}.rs` | Меши/спрайты/свет; Vec3↔Vector3; локализация |
| `worldgen/{world,map,dungeon}.rs` | Legacy-мир (фолбэк); сборка карты (+станции) и данжа в сцену |
| `bin/preset_migrate.rs` | CLI-миграция контента в RON |

## 4. `server/` — Go-бэкенд

| Путь | Отвечает за |
|---|---|
| `cmd/oh-server/` | Игровой сервер комнаты: сессии, персонажи, партия, релей, анонс |
| `cmd/oh-master/`, `oh-probe/` | Мастер-сервер (список/лимиты/релей туннелей); диагностика |
| `internal/sim/` | Хост симуляции через `core.wasm` |
| `internal/room/`, `relay/`, `proto/`, `content/` | Комната+лут; `StreamConn` (race `SendRaw`/`Close` починен); протокол; `Hash` пресета |
| `internal/auth/`, `store/`, `registry/`, `config/`, `wire/` | Аутентификация, хранилище, реестр, конфиг, транспорт |

## 5. `game/` — проект движка

| Путь | Назначение |
|---|---|
| `project.godot` | Godot 4.7, рендерер mobile/Vulkan, стретч 1920×1080, input-actions (incl. `ability_primary/secondary`) |
| `OpenHeart.gdextension` | Платформа → `res://bin/openheart.{dll,so,dylib}`; `reloadable=true` |
| `addons/oh_editor/` | Редактор игры (вкладка OpenHeart): схемы категорий, CRUD, копия пресета, «замок ядра» |
| `tools/*.gd` | Смоуки: `runtime_smoke`, `content_smoke`, `evening_smoke`, `release_smoke`, `presentation_smoke` |

### 5.1 `game/presets/core` — контент (RON авторский + JSON)

19 стемов (у большинства RON+JSON, тест `json_and_ron_sources_do_not_drift` следит за
совпадением): `weapons` (+крит), `weapon_mods`, `classes` (+charge_slot), `perks` (+crit),
`synergies`, `enemies`, `items`, `npcs`, `quests` (цепочки + соседи), `dialogues`,
`abilities` (враги), `player_abilities`, `affixes`, `statuses`, `dungeon`, `loot`, `level`,
`recipes`, `preset` (+ `maps/hub.json`). Форматы — [DATA_FORMATS.md](DATA_FORMATS.md).

### 5.2 `game/assets/`

Спрайты/эффекты/UI/текстуры/небо; иконки атласами (`icons/{abilities,items,weapons,perks,cosmetics}_atlas`),
портреты (`portraits/classes*`), **соседи** (`illustrations/neighbors`, `portraits/neighbors`,
`decor/femboy_quarter`), музыка/звуки. Индекс в данных задаёт регион атласа; `.tres` — AtlasTexture.

## 6. `tools/` — пайплайн ассетов (Python/PS + Pillow)

`aigen.py` (ИИ-генерация), `slice_*`/`process_sprites.py` (нарезка), `build_hub_map.py`
(карта хаба + станции), `build_neighbors_atlases.py`/`add_neighbors_content.py`/
`add_evening_content.py` (контент-пак соседей), `repair_generated_assets.py`, `ASSET_GUIDE.md`.

## 7. Планы и производство

Актуальные: [RELEASE_PLAN_2026-09-20.md](RELEASE_PLAN_2026-09-20.md) (доведение до релиза),
[ART_AND_CONTENT_PLAN_2026-09-27.md](ART_AND_CONTENT_PLAN_2026-09-27.md),
[WEAPON_BALANCE.md](WEAPON_BALANCE.md) (паспорт оружия+крит),
[CONTENT_PACK_NEIGHBORHOOD_01.md](CONTENT_PACK_NEIGHBORHOOD_01.md) (соседи).
Историческое: [OPEN_QUESTIONS.md](OPEN_QUESTIONS.md) (частично устарел — см. баннер).

## 8. Быстрые ответы («мне нужно…»)

| Вопрос | Ответ |
|---|---|
| Точка входа логики? | `client/src/lib.rs` → `Game3D` в `main.tscn` → `nodes/game/lifecycle.rs::ready()` |
| Где урон/крит? | offline: `nodes/game/combat.rs::hit_enemy` (крит+слабые точки); online: `core/sim/damage.rs` (крит+резисты) |
| Где умения игрока? | данные `player_abilities.*` → `core/combat/ability.rs` (+заряды) → `nodes/game/abilities.rs` |
| Где прокачка? | `core/state/game_state.rs::add_xp` → `classes::compute_loadout` + `perk::mods_for` |
| Магазин/крафт/арена? | `nodes/game/shop.rs`; `crafting.rs` (+станции карты); `creative.rs` (арена) |
| Где сейв? | `core/state/save.rs` ↔ `user://save.json` |
| Клиента не пускает на сервер? | Разошёлся `content_hash` — сверь `core/data/hash.rs` ↔ `server/internal/content`; при смене контента обнови пин в `hash_disk_tests.rs` |
| Как сделать «свою игру»? | Скопировать пресет (вкладка OpenHeart → «Создать копию») — [EDITOR.md](EDITOR.md) |
