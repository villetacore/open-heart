# 📇 Индекс проекта OpenHeart

Полная индексация репозитория для быстрого анализа: каждый модуль кода с его публичным
API, каждый файл данных, ассеты и инструменты. Числа актуальны на v0.4.0 (2026-07-04);
при больших изменениях обновляй вместе с кодом.

**Сводка:** ~8 100 строк Rust в 24 модулях · 2 GDScript-файла редактора (~660 строк) ·
2 пресета × 10 JSON-файлов данных · 197 PNG-ассетов · 14 Python-скриптов пайплайна.

---

## 1. Корень репозитория

| Файл | Назначение |
|---|---|
| `README.md` | Витрина проекта: фичи, быстрый старт, управление, ссылки |
| `CONTRIBUTING.md` | Правила вклада, смоук-чеклист, стиль кода |
| `CHANGELOG.md` | История версий (Keep a Changelog) |
| `CODE_OF_CONDUCT.md` | Кодекс поведения |
| `LICENSE` | MIT |
| `run.ps1` | Сборка Rust + запуск редактора одной командой (знает winget-путь Godot) |
| `watch.ps1` | Автопересборка DLL при сохранении `.rs` (cargo-watch) |
| `build.bat` | Просто `cargo build` |
| `.gitignore` | rust/target, .godot, *.import, tools/preview, __pycache__ |

## 2. `rust/` — игровая логика (все 24 модуля)

Компилируется в `openheart.dll` (GDExtension). Зависимости: `godot 0.5.4 (api-4-3)`,
`serde`, `serde_json`. Точка входа — `lib.rs` (регистрация классов `Game3D`, `Player`,
`Enemy`, `MainMenu`, `Npc`).

### 2.1 Оркестрация

| Модуль | Строк | Ключевой API | Отвечает за |
|---|---|---|---|
| `game.rs` | ~3070 | `Game3D` (Node3D) | **Главный узел**: режимы (ClassSelect/SpecSelect/Explore/Dialogue/Inventory/Perks/Dead), весь HUD, боёвка (hitscan/мили/снаряды/взрывы), порталы и данж-цикл, спавны, пикапы, квест-раннер (`bump_quests`, `make_giver_scene`), миникарта, пост-шейдер, автосейв. *Кандидат на разбиение (план M0)* |
| `content.rs` | 86 | `load_preset(id)`, `preset_base`, `preset_info`, `discover_presets` | ContentDb: загрузка всех данных пресета при старте, поиск пресетов в `res://`+`user://` |
| `main_menu.rs` | 377 | `MainMenu` (Control) | Меню: новая игра/продолжить/**выбор пресета**/настройки; хит-тест кликов в дизайн-координатах |

### 2.2 Data-driven контент (паттерн: RwLock + `include_str!`-фолбэк)

| Модуль | Строк | Ключевой API | Отвечает за |
|---|---|---|---|
| `weapon.rs` | 328 | `WeaponId`, `DmgType`, `AmmoType`, `FireKind`, `WeaponDef`, `Arsenal`, `load()`, `weapon_def(id)` | 8 оружий из `weapons.json`; боезапас 4 типов; арсенал игрока (owned/ammo/current) |
| `classes.rs` | 175 | `ClassDef`, `SpecDef`, `Loadout`, `load()`, `classes()`, `compute_loadout`, `xp_to_next` | Классы/спеки из `classes.json`; расчёт итоговых статов (класс+спек+уровень+сердца+перки) |
| `perk.rs` | 172 | `PerkDef`, `SynergyDef`, `PerkMods`, `load()`, `perks()`, `synergies()`, `mods_for`, `available`, `reqs_met` | Дерево перков и синергии из JSON; агрегация модификаторов |
| `config.rs` | 173 | `GameConfig::load_from(base)`, `EnemyCfg`, `ItemCfg`, `NpcCfg`, `QuestCfg`, `LevelCfg`, `Resist` | Загрузка enemies/items/npcs/quests/level пресета (per-run, без глобального стейта) |
| `map.rs` | 393 | `MapDef` (+`BlockDef`/`BuildingDef`/`PropDef`/`FlatDef`/`LightDef`/`GlowDef`/`GroundDef`), `load_map`, `build_map` → `BuiltMap` | Карты из JSON: геометрия (box/**ramp**/stairs/cylinder), здания+вывески, glow-каналы, спавны, врата, env |

### 2.3 Игровые сущности (Godot-ноды)

| Модуль | Строк | Ключевой API | Отвечает за |
|---|---|---|---|
| `player.rs` | 136 | `Player` (CharacterBody3D): `take_damage`, `heal`, `aim_dir`, `eye_pos`, `teleport`; поля `hp/max_hp/speed/frozen` | FPS-контроллер: WASD/мышь/прыжок/спринт; статы задаёт Game3D |
| `enemy.rs` | 326 | `Enemy` (CharacterBody3D): `configure(…, resist, sprite, scale)`, `take_damage(amount, DmgType) -> dealt`, `set_player` | AI патруль→погоня→атака; резисты; LOS для дальнобойных; hurt-flash; анимация 4-кадрового листа |
| `world.rs` | 226 | `build_world(cache) -> WorldPlan`, `GATE_POS` | **Legacy-фолбэк** мира кодом (используется, если у пресета нет `maps/hub.json`) |
| `dungeon.rs` | 525 | `generate(depth, seed, cache) -> DungeonPlan`; const `GRID/CELL/WALL_H` | Процедурный данж: комнаты на высотах 0/0.8/1.6, переменные потолки, ступени, платформы, темы по глубине, босс, лут, `floor_map` для миникарты |
| `gfx.rs` | 180 | `TexCache`, `make_box(_rot)`, `make_glow_slab`, `make_billboard`, `make_flat_sprite`, `make_light`, `Rng` (xorshift64\*) | Утилиты построения: меши+коллизия, спрайты, свет (аттенюация 0.5), детерминированный RNG |

### 2.4 Состояние, сейвы, настройки

| Модуль | Строк | Ключевой API | Отвечает за |
|---|---|---|---|
| `game_state.rs` | 316 | `GameState`: `add_xp`, `apply(&[Effect])`, `rel`, `has`; поля `level/xp/perks/perk_points/quest_kills/preset/hearts/dungeon_seed` | Всё «чистое» состояние игрока/мира. ⚠️ Содержит и VN-наследие (`Period`, `Location`, `Action`) — используется только диалогами |
| `save.rs` | 166 | `save(state, hp, arsenal)`, `load() -> (GameState, hp, Arsenal)`, `exists`, `delete`; `SaveData` v2 | Сериализация в `user://save.json`; все новые поля с `#[serde(default)]` |
| `settings.rs` | 42 | `Settings { lang, master_vol, mouse_sens, preset }` | `user://settings.json` |

### 2.5 Диалоги и квесты

| Модуль | Строк | Ключевой API | Отвечает за |
|---|---|---|---|
| `dialogue.rs` | 56 | `Scene`, `Line`, `Choice`, `Effect` (Stat/Rel/Flag/Gold/**Xp**/Quest/QuestDone/Flash) | Структуры диалоговой системы |
| `story.rs` | 1023 | `get_scene(id, state)` | Авторские сцены 8 story-NPC (хардкод; план — вынести в JSON, M2/§12) |
| `quest.rs` | 59 | `Quest`, `QuestLog`, `QuestState` | Журнал квестов (актив/завершён) |
| `item.rs` | 55 | `Item`, `Inventory` | Стековый инвентарь |
| `npc.rs` | 70 | `Npc` (нода; вспомогательная) | Легаси-обёртка NPC |
| `character.rs` | 53 | `StatKind` (INT/CHR/FIT/REP/WIL), `Stats` | VN-статы для гейтинга реплик («нужен ИНТ 7») |
| `locale.rs` | 84 | `t(key, lang)` | Строки HUD/меню ru/en |

## 3. `godot/` — проект движка

| Путь | Назначение |
|---|---|
| `project.godot` | Godot 4.7, рендерер **mobile** (Forward Mobile/Vulkan), стретч canvas_items 1920×1080, все input-actions, включённый плагин oh_editor |
| `OpenHeart.gdextension` | Маппинг платформа → `rust/target/{debug,release}/openheart.{dll,so,dylib}`; `reloadable=true` |
| `main_menu.tscn` | Стартовая сцена (нода `MainMenu`) |
| `main.tscn` | Игровая сцена: `Game3D` + `Player`(+Camera3D, капсула) |

### 3.1 `godot/addons/oh_editor/` — редактор игры (GDScript, только в редакторе)

| Файл | Назначение |
|---|---|
| `plugin.cfg`, `plugin.gd` | Регистрация main-screen вкладки «OpenHeart» |
| `editor_main.gd` (~600 строк) | Вся панель: `SCHEMAS` (декларативные схемы 14 категорий), список записей, генератор форм (str/text/float/int/bool/enum/json), CRUD, сохранение пресета, «Создать копию» пресета, «Замок ядра» (attrib ±R на фундаментальные файлы) |

### 3.2 `godot/presets/` — контент (данные = игры)

Каждый пресет содержит одинаковый набор файлов (форматы: [DATA_FORMATS.md](DATA_FORMATS.md)):

| Файл | core («Неоновое Сердце») | arena («Кровавая арена») |
|---|---|---|
| `preset.json` | манифест | манифест |
| `weapons.json` | 8 стволов | 8 (копия core) |
| `classes.json` | 3×3 класса | боезапас ×2.5 |
| `perks.json` / `synergies.json` | 18 / 8 | копия |
| `enemies.json` | **14 типов** | копия |
| `items.json` | 18 предметов | копия |
| `npcs.json` | 12 NPC | пусто (осознанно) |
| `quests.json` | 7 квестов | пусто |
| `level.json` | legacy-спавны (фолбэк) | пусто |
| `maps/hub.json` | многоярусный квартал | компактный колизей |

### 3.3 `godot/assets/` — 197 PNG

| Папка | Файлов | Что |
|---|---|---|
| `sprites/characters/` | 14 | 8 NPC + 6 врагов, листы 512×256 (idle×2, walk×2) |
| `sprites/weapons_fp/` | 8 | FP-оружие, стрипы 8 кадров ×84px |
| `sprites/pickups/` | 8 | патроны ×4, сердце, душа, свиток, граната |
| `sprites/projectiles/` | 1 | ракета |
| `sprites/props/` | 35 | неон-вывески ×14, улица ×10, мебель ×7, ванная ×4 |
| `sprites/items/` | 7 | иконки предметов (128×64) |
| `sprites/` (корень) | 3+спека | legacy femboy-листы (фолбэк NPC) + `SPRITE_SPEC.md` |
| `effects/` | 8 | взрыв, трейсер, энергия, кровь, дым, лечение, мана, телепорт |
| `ui/` | 8 | иконки HUD 64×64 |
| `textures/` | 8 | стены/пол/потолок хаба (512×512) |
| `textures/dungeon/` | 81 | 77 тайлов тем данжа + 4 жидкости |
| `textures/sky/` | 4 | панорамы неба |
| `sprites_raw/`, `textures_raw/` | 6 | исходники до нарезки (рантаймом world.rs использует `world_complete.png`) |

### 3.4 `godot/data/`

| Файл | Назначение |
|---|---|
| `weapons_fp.json` | Манифест нарезанных стрипов оружия (генерируется `tools/slice_atlases.py`; информационный — рантайм читает `weapons.json` пресета) |

## 4. `tools/` — пайплайн ассетов (Python + Pillow)

| Скрипт | Назначение |
|---|---|
| `slice_atlases.py` | Главная нарезка атласов: оружие FP, пикапы, эффекты, UI, небо, тайлы, пропсы; контактные листы в `tools/preview/` |
| `slice_fix.py` | Правочный проход: подписи, слипшиеся ячейки, ровная сетка неба |
| `slice_props.py` | Пропсы по вручную измеренным боксам (финальная версия) |
| `analyze_weapons.py`, `analyze_regions.py` | Поиск координат секций атласа по плотности пикселей |
| `ASSET_GUIDE.md` | Промпты генерации новых атласов + спецификации форматов |
| остальные (`extract_atlas.py`, `process_sprites.py`, `debug_cells.py`, …) | Ранние/одноразовые инструменты, оставлены для справки |
| `ChatGPT Image 30 июн….png` | Исходный мастер-атлас (эффекты/UI/небо/жидкости) |

## 5. `docs/` — документация

См. [docs/README.md](README.md) — навигация по всем документам с назначением каждого.

## 6. Быстрые ответы («мне нужно…»)

| Вопрос | Ответ |
|---|---|
| Где точка входа логики? | `rust/src/lib.rs` → нода `Game3D` в `main.tscn` → `game.rs::ready()` |
| Где загружаются данные? | `game.rs::ready()` → `content::load_preset()` + `GameConfig::load_from()` |
| Как игра выбирает карту? | `map::load_map(preset, "hub")`; нет файла → `world::build_world()` (legacy) |
| Где урон считается? | `game.rs::try_fire/fire_ray/fire_melee/explode` → `Enemy::take_damage(amount, DmgType)` с резистами |
| Где прокачка? | `game_state.rs::add_xp` (уровни+очки) → `classes::compute_loadout` + `perk::mods_for` → `game.rs::apply_loadout` |
| Где квесты трекаются? | `game.rs::bump_quests` (вызовы из `process_kills` и `pick_up_item`) |
| Где сейв? | `save.rs` ↔ `user://save.json` (v2) |
| Почему в `game_state.rs` «школьные» локации? | VN-наследие для диалогов — см. [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md#наследие-визуальной-новеллы-важно-понимать) |
| Какой рендерер? | Forward Mobile (Vulkan); glow/ACES включаются в `game.rs::build_environment` |
| Как сделать «свою игру»? | Скопировать пресет (вкладка OpenHeart → «Создать копию») — [EDITOR.md](EDITOR.md) |
