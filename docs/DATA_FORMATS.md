# Справочник форматов данных

Все игровые данные — JSON в `godot/presets/<пресет>/`. Этот документ описывает каждый
формат: поля, типы, допустимые значения, примеры. Правишь руками или через
[редактор OpenHeart](EDITOR.md) — результат один.

Общие правила:
- Кодировка UTF-8, отступ 2 пробела.
- Новые поля движок принимает с дефолтами; **неизвестные id** (оружия/патронов/врагов)
  дают предупреждение в лог и фолбэк — смотри Output-панель на строки `using embedded`.
- Строки для игрока — на русском, суффикс `_ru`.

## preset.json — манифест пресета

```json
{ "id": "core", "name_ru": "Неоновое Сердце",
  "desc_ru": "Основная кампания…", "author": "OpenHeart", "version": 1 }
```
`name_ru`/`desc_ru` показываются в главном меню на кнопке пресета.

## weapons.json — оружие (массив, ровно 8 слотов)

```jsonc
{
  "id": "shotgun",              // slug: sword|chainsaw|pistol|shotgun|rifle|nailgun|plasma|rocket
  "name_ru": "Дробовик",
  "damage": 9.0,                // урон за пеллету/удар/снаряд
  "dmg_type": "physical",       // physical | fire | energy | void
  "cooldown": 0.95,             // сек между выстрелами (модифицируется классом/перками)
  "range": 16.0,                // метры (для projectile — дальность жизни снаряда)
  "fire": { "kind": "hitscan", "pellets": 7, "spread": 0.09 },
  //        kind: melee | hitscan{pellets,spread} | projectile{speed,splash}
  "ammo": { "type": "shells", "per_shot": 1 },   // null = без боеприпасов (мили)
  //        type: bullets | shells | rockets | cells
  "auto": false,                // true = стреляет при удержании ЛКМ
  "sheet": "res://assets/sprites/weapons_fp/wf_shotgun.png",
  "frame_h": 95.0,              // высота кадра стрипа (ширина всегда 84)
  "idle_frames": [0],           // кадры простоя (несколько — цикл, как у пилы)
  "fire_frames": [6, 7, 3, 4],  // последовательность выстрела
  "fire_fps": 10.0
}
```
Слоты жёстко соответствуют `id` (клавиши 1–8). Изменить можно всё, кроме набора slug'ов
(они завязаны на спрайты и `WeaponId` в Rust).

## classes.json — классы (массив из 3)

```jsonc
{
  "id": "berserk", "name_ru": "БЕРСЕРК", "role_ru": "Мили",
  "desc_ru": "Пила и клинок…",
  "base_hp": 150.0, "speed": 5.8, "dmg_mult": 1.0,
  "start_weapons": ["sword", "chainsaw"],       // slug'и оружия
  "start_ammo": [{ "type": "shells", "amount": 8 }],
  "specs": [                                     // ровно 3 спека
    { "id": "bloodreaper", "name_ru": "Кровожнец", "desc_ru": "…",
      "hp_bonus": 0.0, "speed_mult": 1.0, "dmg_mult": 1.0,
      "cd_mult": 1.0,          // <1 = быстрее стреляет
      "lifesteal": 0.25,       // доля нанесённого урона в HP (мили-вампиризм)
      "ammo_mult": 1.0,        // множитель максимума боезапаса
      "extra_weapon": null }   // slug оружия, выдаваемого спеком (или null)
  ]
}
```

## perks.json — перки (массив)

```jsonc
{
  "id": "vampirism",
  "branch": "survival",         // survival | offense | utility (порядок веток на экране)
  "tier": 2,                    // информативно (сортировка/будущий UI)
  "max_ranks": 3,
  "cost": 1,                    // очков за ранг
  "requires": ["thick_skin:1"], // "perk_id:минимальный_ранг"
  "name_ru": "Вампиризм", "desc_ru": "+5% вампиризма за ранг.",
  "effects": [ { "stat": "lifesteal", "add": 0.05 } ]
}
```
`effects[].stat`: `max_hp` (add), `speed` (mult), `dmg` (add к множителю),
`cd` (mult, <1 быстрее), `lifesteal` (add), `ammo` (add к множителю).
`add` умножается на ранг; `mult` возводится в степень ранга.

## synergies.json — комбинации перков (массив)

```jsonc
{ "id": "glass_cannon",
  "needs": ["sharpshooter:2", "fleet_footed:2"],   // все условия сразу
  "name_ru": "Стеклянная пушка", "desc_ru": "+15% урона, но −30 HP.",
  "effects": [ { "stat": "dmg", "add": 0.15 }, { "stat": "max_hp", "add": -30.0 } ] }
```
Синергия активируется автоматически и показывается на экране перков (`P`).

## enemies.json — враги (`{ "enemies": [...] }`)

```jsonc
{
  "id": "pyro_cultist", "name": "Пиро-культист",
  "hp": 77.0, "speed": 3.3,
  "attack_damage": 18.0, "attack_range": 2.0, "attack_cooldown": 1.2,
  "chase_range": 11.0,          // радиус агра (атаки дальше 3 м требуют прямой видимости)
  "patrol_radius": 4.0,
  "color_r": 1.0, "color_g": 0.45, "color_b": 0.25,   // тинт спрайта
  "xp": 28.0,                   // опыт за убийство (умножается на mult данжа)
  "sprite": "cultist",          // лист enemy_<sprite>.png: grunt|fast|heavy|brute|sniper|cultist
  "scale": 1.0,                 // масштаб спрайта и коллайдера (босс ≥1.35)
  "resist": { "fire": 0.6, "void": -0.4 }   // 0..1 = резист, <0 = уязвимость; ключи:
}                                            // physical | fire | energy | void
```
Новый «вид» врага = существующий спрайт + тинт + масштаб + статы/резисты.

## items.json — предметы (`{ "items": [...] }`)

```jsonc
{ "id": "big_potion", "name_ru": "Большое зелье", "name_en": "Big Potion",
  "desc_ru": "Восстанавливает 80 HP", "desc_en": "Restores 80 HP",
  "value": 0,                    // золото при подборе (для category=currency)
  "category": "consumable",      // consumable → в инвентарь (Q — использовать)
                                 // currency  → мгновенно золото
                                 // key       → в инвентарь как квестовый
  "heal": 80.0,                  // для consumable (null у прочих)
  "color_r": 0.95, "color_g": 0.2, "color_b": 0.6 }
```
Спец-предмет `heart_1up` (сердце: +15 макс. HP навсегда) обрабатывается движком отдельно —
в items.json его описывать не нужно, в спавнах используется по id.

## npcs.json — NPC (массив)

```jsonc
{ "id": "hunter", "name_ru": "Охотница Ба",
  "sprite": "npc_guard",        // лист characters/<sprite>.png
  "pos": [-8, -34],             // [x, z] на карте мира (y — пол)
  "color": [1.0, 0.55, 0.45],   // тинт (опц.)
  "scene": null,                // "story" = авторские сцены story.rs (для 8 исходных id);
                                //  "<id сцены>" = сцена из dialogues.json или story.rs; null = квест-гивер
  "quest": "cull_grunts" }      // маркер гивера (цепочка берётся по giver из quests.json)
```
⚠️ Пустой массив = «NPC в этом пресете нет»; legacy-набор подставляется только если
файла **нет вообще**.

## dialogues.json — сцены диалогов (массив)

Data-driven сцены; **приоритетнее story.rs**: сцена с тем же id переопределяет
встроенную, новые id добавляют контент без пересборки. Пример — `presets/core/dialogues.json`
(демо-сцены `demo_dialogue*`). Категория редактора: «Диалоги».

```jsonc
{ "id": "my_scene",
  "lines": [                     // реплики по порядку (E — далее)
    { "speaker": "", "text": "Нарратор (пустой speaker)." },
    { "speaker": "Незнакомец", "portrait": "stranger", "text": "Реплика." }
  ],
  "choices": [                   // пусто → диалог просто закрывается
    { "text": "Вариант ответа",
      "requires": { "stat": "int", "min": 7 },   // опц.: int|chr|fit|rep|wil
      "effects": [ /* см. ниже */ ],
      "next": "other_scene" }    // опц.: id следующей сцены (JSON или story.rs)
  ] }
```

Эффекты выбора (`effects[]`, поле `kind`):

```jsonc
{ "kind": "stat",  "stat": "int", "value": 1 }       // + к стату (int|chr|fit|rep|wil)
{ "kind": "rel",   "npc": "vale", "value": 5 }       // отношения с NPC
{ "kind": "flag",  "flag": "met_x" }                 // поставить флаг
{ "kind": "unflag","flag": "met_x" }                 // снять флаг
{ "kind": "gold",  "value": 50 }                     // золото (может быть <0)
{ "kind": "xp",    "value": 100 }                    // опыт (уровни пересчитаются)
{ "kind": "quest", "id": "q1", "title": "…", "desc": "…" }  // выдать квест
{ "kind": "quest_done", "id": "q1" }                 // завершить квест
{ "kind": "flash", "text": "Сообщение" }             // всплывающий текст
```

Битая сцена пропускается с предупреждением в лог — остальные работают.

## quests.json — квесты (массив)

```jsonc
{ "id": "cull_grunts", "title_ru": "Прореживание", "giver": "hunter",
  "desc_ru": "Сократи поголовье: восемь боевиков.",
  "kind": "kill",               // kill | collect | clear_dungeon
  "target": "grunt",            // kill: id врага · collect: id предмета · clear_dungeon: ""
  "count": 8,                   // для clear_dungeon — требуемая глубина
  "reward_xp": 260, "reward_gold": 90 }
```
Гивер выдаёт свои квесты **по цепочке** (первый не завершённый); сдача — в диалоге.
Прогресс collect считается по подборам (расход из инвентаря его не откатывает).

## maps/*.json — карты мира

Полный формат — в [ARCHITECTURE.md §5](ARCHITECTURE.md#5-карты-и-геометрия); кратко:

```jsonc
{
  "id": "hub", "name_ru": "Неоновый квартал",
  "env": { "sky": "sky_purple",            // файл из textures/sky (без .png)
           "fog_density": 0.011, "ambient": [0.30,0.22,0.34], "ambient_energy": 0.8 },
  "player_spawn": [0, 1.1, 12],
  "gate": [0, 2.6, -58],                    // врата данжа (арка строится автоматически)
  "ground": { "size": 200, "tex": "floor_main", "uv": 4,
              "border_h": 5.0, "border_tex": "wall_arena" },
  "blocks": [                               // геометрия с коллизией
    { "shape": "box",      "pos": [x,y,z], "size": [w,h,d], "rot": 12, "tex": "…", "uv": 2 },
    { "shape": "ramp",     "from": [x,y,z], "to": [x2,y2,z2], "width": 3, "tex": "…" },
    { "shape": "stairs",   "from": …, "to": …, "width": 3, "steps": 8, "tex": "…" },
    { "shape": "cylinder", "pos": [x,y,z], "radius": 2.2, "height": 7, "tex": "…" }
  ],
  "buildings": [ { "pos": [x,z], "size": [w,h,d], "tex": "wall_market",
                   "sign": "neon_femboy_club", "sign_side": "s" } ],   // n|s|e|w
  "props":  [ { "tex": "street_bench", "pos": [x,y,z], "px": 0.020 } ],  // биллборды
  "flats":  [ { "tex": "neon_kawaii", "pos": …, "rot": 90, "px": 0.02 } ], // на стены
  "lights": [ { "pos": …, "color": [r,g,b], "energy": 1.5, "range": 16 } ],
  "glows":  [ { "pos": …, "size": …, "tex": "liquid_pink",
                "emission": [0.95,0.3,0.55], "uv": 4 } ],   // светящиеся плиты-каналы
  "spawns": { "spawn_enemies": [{ "kind": "grunt", "x": -46, "z": 18 }],
              "spawn_items":   [{ "kind": "medkit", "x": -8, "z": -12 }],
              "spawn_ammo":    [{ "kind": "bullets", "amount": 30, "x": -6, "z": -14 }],
              "spawn_weapons": [{ "kind": "shotgun", "x": -52, "z": -48 }] }
}
```

Имена текстур (`tex`) — короткие, папка выводится по префиксу: `dtile_*`/`liquid_*` →
`textures/dungeon`, `sky_*` → `textures/sky`, `neon_*`/`street_*`/`furn_*`/`bath_*` →
`sprites/props`, `effect_*` → `effects`, `item_*` → `sprites/items`, `ammo_*`/`soul`/
`heart_*`/`grenade`/`scroll` → `sprites/pickups`, иначе → `textures/`.

Рампы держат уклон ≤ ~40° (лимит хождения CharacterBody3D — 45°).

## level.json — legacy-спавны

Тот же формат, что `spawns` карты. Используется **только если** у пресета нет
`maps/hub.json`. В новых пресетах предпочитай спавны внутри карты.
