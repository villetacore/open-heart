# Справочник форматов данных

Все игровые данные — JSON в `game/presets/<пресет>/`. Этот документ описывает каждый
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
  "name_en": "Shotgun",
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
  "fire_frames": [3, 1, 2, 0],  // вспышка → отдача → восстановление
  "fire_fps": 13.0,
  "magazine": 8,
  "reload_time": 1.65,
  "reload_frames": [2, 4, 5, 6, 5, 2, 0],
  "reload_fps": 8.0,
  "switch_time": 0.26,
  "switch_frames": [7, 6, 2, 0],
  "switch_fps": 12.0,
  "recoil": 48.0,              // отдача FP-спрайта и реальный подброс прицела
  "feedback": {
    "muzzle_color": [1.0, 0.45, 0.14], "muzzle_energy": 2.8,
    "muzzle_range": 9.0, "muzzle_duration": 0.13,
    "impact_color": [1.0, 0.32, 0.12], "impact_scale": 1.15,
    "impact_energy": 1.0, "impact_duration": 0.22,
    "tracer_color": [1.0, 0.7, 0.3], "tracer_scale": 0.9,
    "tracer_duration": 0.06
  },
  "audio": {
    "fire_sfx": ["res://assets/sounds/Plasma Gun1.wav"],
    "impact_sfx": ["res://assets/sounds/Plasma Sword Strike1.wav"],
    "reload_sfx": ["res://assets/sounds/An Evil Robot Is Walking1.wav"],
    "fire_pitch": [0.68, 0.76], "impact_pitch": [0.74, 0.84],
    "reload_pitch": [0.82, 0.9],
    "fire_volume_db": 0.0, "impact_volume_db": -6.0, "reload_volume_db": -10.0
  },
  "accuracy": {
    "bloom_per_shot": 0.55, "max_bloom": 1.15, "recovery": 1.7,
    "move_penalty": 0.25, "crosshair_scale": 1.25
  },
  "alt_fire": {
    "name_ru": "Пулевой снаряд", "name_en": "Slug",
    "fire": { "kind": "hitscan", "pellets": 1, "spread": 0.004 },
    "damage_mult": 7.5, "cooldown_mult": 1.25, "range_mult": 2.0,
    "recoil_mult": 1.3, "bloom_mult": 1.4,
    "animation_frames": [3, 4, 2, 1, 2, 0],
    "animation_fps": 10.0, "hit_ratio": 0.08
  }
}
```
`feedback` управляет вспышкой, спрайтом и светом попадания, а также коротким трассером.
Профиль сохраняется в снаряде при выстреле, поэтому смена оружия не меняет визуальный стиль
уже летящего снаряда.
`audio` задаёт варианты WAV, случайный диапазон высоты тона и громкость отдельно для
выстрела, попадания и перезарядки. Для оружия с магазином `reload_sfx` обязателен;
у melee-оружия он может быть пустым.
`accuracy` задаёт накопление разброса за выстрел, его предел, скорость восстановления,
штраф движения и масштаб динамического прицела. Первый выстрел использует базовый `spread`,
а последующие получают множитель `1 + bloom`.
`alt_fire` задаёт локализованный вторичный режим для ПКМ. Он может менять тип атаки,
независимо масштабировать урон, кулдаун, дальность, отдачу и bloom, а также имеет отдельный
клип `animation_frames`/`animation_fps`. `hit_ratio` задаёт момент активного кадра melee;
при отсутствии новых полей старый пресет безопасно использует primary-клип.
Слоты жёстко соответствуют `id` (клавиши 1–8). Изменить можно всё, кроме набора slug'ов
(они завязаны на спрайты и `WeaponId` в Rust).

Номера кадров в `idle_frames`, `fire_frames`, `reload_frames` и `switch_frames` — целые
от `0` до `7`. Если `switch_frames` отсутствует в старом пресете, загрузчик безопасно
использует первый idle-кадр; для включённого production-пресета требуется отдельная
многокадровая последовательность смены оружия.
Первый кадр `fire_frames` показывается непосредственно при выстреле, поэтому для огнестрела
там обычно находится вспышка. У мили попадание происходит в середине `fire_frames`, что
синхронизирует урон с визуальным взмахом.

## weapon_mods.json — взаимоисключающие модификации оружия

```jsonc
{
  "id": "shotgun_choke", "weapon": "shotgun", "branch": 1,
  "name_ru": "Удушающий чок", "name_en": "Tight Choke",
  "desc_ru": "+25% дальность, -30% bloom",
  "desc_en": "+25% range, -30% bloom",
  "range_mult": 1.25, "bloom_mult": 0.7,
  "feedback": {
    "tint": [0.28, 0.72, 1.0], "color_mix": 0.78,
    "muzzle_mult": 0.9, "tracer_mult": 0.68,
    "impact_mult": 1.08, "pitch_mult": 1.08
  }
}
```
Для каждого из восьми оружий обязательны ветви `1` и `2`. Одновременно активна только одна
ветвь; выбор или смена в инвентаре стоит одно ядро, гарантированно выдаваемое за босса.
Незаполненные множители `damage/cooldown/range/recoil/bloom_mult` равны `1.0`.
Опциональный `feedback` задаёт стиль ветви: `tint` смешивается с базовыми цветами через
`color_mix`, а остальные множители меняют вспышку, трассер, попадание и pitch звука.
Без блока `feedback` все значения нейтральны и старые пресеты продолжают работать.

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
  "chase_range": 11.0,          // радиус агра — по ЗРЕНИЮ (сквозь стены не видит; шум будит)
  "patrol_radius": 4.0,
  "color_r": 1.0, "color_g": 0.45, "color_b": 0.25,   // тинт спрайта
  "xp": 28.0,                   // опыт за убийство (умножается на mult данжа)
  "sprite": "cultist",          // лист enemy_<sprite>.png: grunt|fast|heavy|brute|sniper|cultist
  "behavior": "ranged",         // melee (в контакт, по умолчанию) | ranged (держит дистанцию)
  "role": "controller",         // pursuer | tank | artillery | support | summoner | controller | commander
  "abilities": ["fire_volley"], // id из abilities.json (кастуются по кулдауну при видимости)
  "pain_chance": 0.35,          // шанс стаггера при уроне (прерывает каст/рывок)
  "attack_status": { "id": "burning", "chance": 0.5 },  // статус на игрока при атаке (опц.)
  "scale": 1.0,                 // масштаб спрайта и коллайдера (босс ≥1.35)
  "weak_point": { "height": 0.7, "multiplier": 1.6 }, // верхние 30% тела; множитель точного попадания
  "resist": { "fire": 0.6, "void": -0.4 }   // 0..1 = резист, <0 = уязвимость; ключи:
}                                            // physical | fire | energy | void
```
`animation` настраивает восемь состояний атласа 1024×2048: `idle`, `move`,
`alert`, `attack`, `pain`, `cast`, `charge`, `death`. Для каждого доступны `*_frames`,
`*_row` и отдельные `attack_fps`, `pain_fps`, `alert_fps`, `cast_fps`, `death_fps`;
отсутствующие значения наследуют `action_fps`. `attack_hit_ratio` (0.1..0.9) задаёт
момент фактического попадания внутри attack-анимации. Удар, боль, тревога и смерть
проигрываются как one-shot, движение/idle/cast — циклически. Роль врага подбирает
различимый timing-профиль, если точные FPS не заданы в пресете.

`weak_point.height` задаёт начало слабой зоны как долю высоты коллайдера (`0.5..0.95`),
а `weak_point.multiplier` — множитель урона (`1.0..3.0`). Слабые зоны учитываются у
hitscan и прямых попаданий снарядов; взрывы и автонаведение мили не получают критический бонус.

Новый «вид» врага = существующий спрайт + тинт + масштаб + статы/резисты.
Роль влияет на AI, а не служит тегом: дальние роли автоматически держат дистанцию,
`tank` хуже прерывается, `support`/`summoner` чаще используют способности, а `commander`
быстрее кастует и дольше сохраняет тревогу. Включённый production-пресет обязан покрывать
все семь ролей.

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
      "requires": { "stat": "int", "min": 7,     // все указанные условия должны выполняться
                    "flag": "boss_defeated_x",
                    "not_flag": "refused_x",
                    "quest_done": "previous_quest" },
      "effects": [ /* см. ниже */ ],
      "next": "other_scene" }    // опц.: id следующей сцены (JSON или story.rs)
  ] }
```

`requires` может содержать любое сочетание `stat`/`min`, обязательного `flag`,
запрещающего `not_flag` и завершённого `quest_done`. Неуказанные проверки игнорируются;
пустой объект считается ошибкой сцены. Недоступные ответы скрываются, а номера оставшихся
вариантов пересчитываются без дырок.

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
{ "id": "cull_grunts", "title_ru": "Прореживание", "title_en": "Thinning the Ranks",
  "desc_ru": "Сократи поголовье: восемь боевиков.", "desc_en": "Eliminate eight raiders.",
  "giver": "hunter",
  "kind": "kill",               // + interact | discover_weapon | boss
  "target": "grunt",            // enemy/item/NPC/weapon id; discover_weapon accepts "*"
  "count": 8,                   // для clear_dungeon — требуемая глубина
  "reward_xp": 260, "reward_gold": 90,
  "reward_items": [{ "id": "ration", "qty": 2 }],
  "chain_ru": "Охота квартала", "chain_en": "Quarter Hunt", "stage": 1,
  "requires": ["previous_quest"],
  "offer_scene": "hunter_offer", "progress_scene": "hunter_progress",
  "complete_scene": "hunter_complete",
  "world_change": {
    "id": "hunter_beacon", "pattern": "beacon",
    "pos": [-34.0, 0.0, 14.0], "color": [1.0, 0.24, 0.18],
    "scale": 1.15,
    "sprite": "res://assets/sprites/characters/enemy_blood_hound.png",
    "activity": "patrol", "activity_count": 4,
    "activity_radius": 3.2, "activity_speed": 1.15
  } }
```
`world_change` is optional and belongs on a chain finale. Once that quest is completed, the hub
builds the configured persistent landmark immediately and rebuilds it after loading a save. Supported
patterns are `beacon`, `garden`, `gallery`, `archive`, and `shrine`; `scale` accepts `0.5..=3.0`.
Activity profiles (`patrol`, `support`, `crowd`, `echo`, `guardian`) spawn 2–8 animated actors around
the landmark. Their bob/spin animation respects pause mode and is recreated from completed quests.
Гивер выдаёт свои квесты **по цепочке** (первый не завершённый); сдача — в диалоге.
Прогресс collect считается по подборам (расход из инвентаря его не откатывает).
`chain_ru/en` и `stage` группируют задания в журнале и показывают текущий этап на обоих
языках. `reward_items` выдаётся через общий dialogue-effect при сдаче, поэтому одинаково
работает для автоматически собранных и кастомных completion-сцен и сохраняется в инвентаре.
Автотест пресетов запрещает повторяющиеся ID и зависимости, циклы цепочек, нулевой `count`,
отсутствующую награду и ссылки на несуществующие цели, NPC или диалоговые сцены.
Если рядом существуют `.json` и `.ron`, тест также требует их полного семантического совпадения:
runtime читает RON первым, поэтому устаревшая миграция больше не сможет скрыть изменения JSON.

## statuses.json — статусы урона (массив)

Эффекты, накладываемые оружием/способностями/атаками врагов на цель (врага ИЛИ
игрока). Нет файла → встроенные core. Категория редактора: «Статусы».

```jsonc
{ "id": "burning", "name_ru": "Горение",
  "kind": "dot",              // dot | slow | stun | vulnerable
  "duration": 3.0,
  // dot: периодический урон
  "damage": 6.0, "dmg_type": "fire", "tick": 0.5,
  // slow: amount=0.45 → −45% скорости; vulnerable: amount=0.35 → +35% входящего урона
  "amount": 0.0,
  "tint": [1.0, 0.5, 0.2],    // подсветка спрайта врага под статусом
  "icon": "🔥" }              // символ для HUD
```

Ссылки на статус — поле `status: { "id": "...", "chance": 0.5 }`:
- **оружие** (weapons.json) → накладывает на врага при попадании;
- **способность** (abilities.json, projectile_burst) → на игрока снарядом;
- **враг** (enemies.json `attack_status`) → на игрока при мили/рывке.
DoT учитывает резисты врага; уязвимость усиливает урон; оглушение прерывает касты.

## abilities.json — способности врагов (массив)

Кастуются при прямой видимости в диапазоне min_range..max_range по кулдауну.
Перед эффектом — **телеграф**: враг стоит и подсвечивается цветом способности
(окно на уворот/прерывание — стаггер по pain_chance сбрасывает каст).
Круг телеграфа масштабируется по механике: рывок и призыв заметнее обычного выстрела,
а `heal_pulse` показывает фактический радиус лечения.
Нет файла → встроенные core-способности. Категория редактора: «Способности».

```jsonc
{ "id": "fire_volley",
  "kind": "projectile_burst",   // projectile_burst | charge | summon | heal_pulse
  "cooldown": 3.5, "telegraph": 0.6,
  "min_range": 3.0, "max_range": 14.0,
  "color": [1.0, 0.5, 0.2],     // цвет телеграфа и снарядов
  // projectile_burst: веер снарядов в игрока (можно увернуться; стены и «свои» гасят)
  "count": 3, "spread": 0.22, "proj_speed": 11.0, "damage": 9.0 }
// charge:     "speed_mult": 3.4, "duration": 0.9, "damage": 24.0  (рывок с контакт-уроном)
// summon:     "minion": "vermin", "count": 2                       (призыв врагов)
// heal_pulse: "heal": 25.0, "radius": 7.0                          (лечит союзников вокруг)
```
Урон способностей умножается на множитель силы врага (глубина × сложность).

## affixes.json — элитные аффиксы (массив)

Элита = базовый враг + 1–2 аффикса: комбинаторный рост числа противников без
новых ассетов. Спавнится в данже с шансом `elite_chance + elite_per_depth×глубина`
(настройки dungeon.json). Тинт подмешивается в цвет спрайта, имя — префиксом
(«Быстрый grunt», ⭐ в прицеле). Нет файла → встроенные core. Категория: «Аффиксы».

```jsonc
{ "id": "armored", "name_ru": "Бронированный",
  "tint": [0.85, 0.8, 0.5],     // подсветка элиты
  "hp_mult": 1.7, "dmg_mult": 1.0, "speed_mult": 0.9,
  "xp_mult": 1.5,               // больше опыта за элиту
  "pain_mult": 0.25,            // сложнее прервать стаггером
  "lifesteal": 0.0,             // «вампирический»: доля урона → своё HP
  "death_blast": null }         // «взрывной»: [урон, радиус] при смерти
```

## dungeon.json — генерация данжей

Темы, пулы врагов и настройки процедурного генератора. Нет файла → встроенные
настройки core (данжи работают в любом пресете). Пустые `themes`/`pools` тоже
подменяются core-значениями (с предупреждением). Категории редактора: «Данж: …».

`room_archetypes` поддерживает формы `rect`, `round`, `octagon`, `cross` и роли
`safe`, `arena`, `gallery`, `ritual`, `treasure`, `ambush`, `traversal`, `puzzle`,
`story`, `antechamber`. Включённый основной пресет должен содержать все десять ролей
и не менее шестнадцати архетипов. Роль независимо от формы и декора задаёт set-piece,
боевое давление и предпочтительные тактические роли противников.

Каждый архетип также задаёт некollision focal-композицию: `focal_pattern` принимает
`spire|ring|cross|aisle|altar|well|archive|gate`, а `focal_color`, `focal_scale` и
`focal_height` меняют её палитру и пропорции. Основной пресет покрывает все восемь
паттернов и требует уникальный focal-профиль для каждого из шестнадцати архетипов.

`hazard_kind` задаёт не только цвет зоны. `blood` наносит постоянный урон и вызывает
кровотечение; `void` пульсирует и накладывает уязвимость; `electric` даёт короткий
предупреждённый разряд с оглушением; `embers` циклически разгорается и поджигает.
Перед активными импульсами создаётся световой телеграф, поэтому ловушки можно читать
и обходить, а не только лечить полученный урон.

Миникарта строится из тех же данных генератора: контур `floor_map`, центры и роли комнат,
позиции и типы ловушек, входной и глубинный порталы. Поэтому новые архетипы автоматически
получают семантический маркер без ручного изменения HUD.

Роли `puzzle` и `story` также создают runtime-события. Первая формирует группу из трёх
последовательных реле с наградой и событием квеста `solve_puzzle`; вторая — одноразовое
локализованное эхо с событием `discover_lore`. Позиции автоматически привязываются к
гарантированно проходимому внутреннему кольцу комнаты.

Поверх геометрии архетипа генератор применяет seeded dressing-вариант `0..2`: ролевые
сигилы пола, тонкие световые стелы, локальные lights и души для safe/ritual/story.
Этот слой не имеет коллизий, поэтому A*-сетка и фактические маршруты не расходятся;
вариант зависит от seed, глубины и индекса комнаты и воспроизводится при том же забеге.

Квесты поддерживают виды `solve_puzzle` и `discover_lore`. Их `target` может быть `*`
для любой глубины либо строковым номером конкретной глубины; `count` задаёт количество
решённых групп или найденных эхо. Прогресс учитывается только для активного задания.

```jsonc
{
  "themes": [            // ротируются по глубине (depth % количество)
    { "name_ru": "Катакомбы Сердец",
      "wall": "dtile_00", "accent": "dtile_06", "floor": "dtile_02",
      "ceil": "dtile_65", "lava": "dtile_74",     // короткие имена (см. tex-роутинг) или res://-пути
      "light": [1.0, 0.45, 0.72] }                // цвет света комнат
  ],
  "pools": [             // пул врагов: берётся самый глубокий с min_depth ≤ глубине
    { "min_depth": 1, "enemies": ["grunt", "fast", "cultist"] },
    { "min_depth": 2, "enemies": ["grunt", "fast", "cultist", "heavy", "sniper"] }
  ],
  "settings": {
    "boss": "brute",                    // id врага-стража
    "boss_mult": 1.25,                  // множитель поверх глубинного
    "boss_guards": ["cultist","cultist"],  // свита по бокам алтаря
    "boss_items": ["ancient_ruby", "gold_stack", "heart_1up"],
    "boss_roster": [                        // самый глубокий min_depth ≤ текущей глубине
      { "min_depth": 1, "boss": "crypt_warden", "mult": 1.0,
        "guards": ["grunt", "fast"], "items": ["medkit"] },
      { "min_depth": 4, "boss": "heart_tyrant", "mult": 1.12,
        "guards": ["void_priest"], "items": ["heart_1up"] }
    ],                                      // пусто → legacy boss/boss_guards/boss_items
    "mult_per_depth": 0.18,             // прирост hp/урона/XP за глубину
    "weapon_cache": ["shotgun", "rifle", "nailgun", "plasma", "rocket"],
    "elite_chance": 0.08,               // шанс элиты (+ elite_per_depth за глубину)
    "elite_per_depth": 0.03,
    "elite_affixes_max": 2              // максимум аффиксов на элиту
  }
}
```
Для включённого основного пресета валидатор требует минимум четыре boss-tier, уникальные
`min_depth`, существующие ссылки и не менее двух фаз в профиле каждого босса.

## loot.json — таблицы лута

Нет файла → встроенные core-таблицы. Категории редактора: «Лут: …».

```jsonc
{
  "room_items": [                         // каждая запись бросается отдельно;
    { "id": "medkit",    "chance": 0.65 },//  позиции в комнате чередуются
    { "id": "gold_coin", "chance": 0.50 }
  ],
  "kill_drops": [                         // дроп с врага: ОДИН бросок, записи
    { "kind": "ammo",                   "chance": 0.30 },  // кумулятивны,
    { "kind": "item", "id": "medkit",   "chance": 0.14 },  // сумма ≤ 1.0,
    { "kind": "item", "id": "heart_1up","chance": 0.03 }   // остаток — ничего
  ],
  "settings": {
    "room_ammo_chances": [0.85, 0.40]     // точки патронов в комнате данжа
  }
}
```

## maps/*.json — карты мира

> Хаб core-пресета (`maps/hub.json`) **собирается генератором** —
> `tools/build_hub_map.py`. Ручные правки в нём переживут ровно до следующего
> прогона: план города лежит в скрипте. Остальные карты пишутся руками.

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
  "districts": [ {
    "id": "night_market", "name_ru": "Ночной рынок", "name_en": "Night Market",
    "center": [-34, 14], "radius": 23, "color": [1.0, 0.48, 0.2],
    "outline_tex": "liquid_red",
    "landmark": { "tex": "neon_femboy_club", "pos": [-34, 1.2, 18], "px": 0.024 }
  } ],
  "decor_clusters": [ {                    // кольцо пропсов, пилонов и локальных lights
    "id": "market_stalls", "center": [-34,14], "radius": 17, "density": 16,
    "color": [1.0,0.46,0.18], "glow_tex": "liquid_red",
    "bob": 0.34, "spin": -9.0, "speed": 1.35,
    "props": ["street_vending","street_bags","neon_kawaii"]
  } ],
  "route_layers": [ {                      // световая полилиния без коллизии
    "id": "market_spur", "points": [[0,0,7],[-15,0,10],[-34,0,14]],
    "width": 1.1, "color": [1.0,0.46,0.18], "glow_tex": "liquid_red", "uv": 1.0
  } ],
  "skyline_beacons": [ {                   // высокий вторичный ориентир
    "id": "heart_tower", "pos": [0,-3], "height": 18,
    "color": [1.0,0.28,0.7], "glow_tex": "liquid_pink",
    "crown": "neon_heart", "crown_px": 0.03
  } ],
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
Район строит световую границу и локальную цветовую точку, показывает landmark и меняет
название локации в HUD при входе игрока. Production-хаб должен иметь минимум шесть районов;
каждый район обязан иметь уникальный `id`, RU/EN-название, радиус не меньше 8 и landmark.
`decor_clusters` детерминированно распределяет `density` (3..32) тематических пропсов по
кольцу, добавляет вертикальные световые пилоны и локальные источники света. Это слой
визуальной плотности без коллизий; основные маршруты и силуэты по-прежнему задаются вручную.
`bob` задаёт амплитуду парения центральной вывески, `spin` — градусы вращения в секунду,
`speed` — частоту её ритма. Ambient-анимация останавливается вместе с паузой игры.
`route_layers` создаёт световые полосы и узлы поверх земли без коллизии, связывая площадь,
районы и врата в читаемую сеть. `skyline_beacons` создаёт высокие световые оси, короны и
локальный свет: они формируют силуэт квартала, не меняя игровые проходы и навигацию.

## level.json — legacy-спавны

Тот же формат, что `spawns` карты. Используется **только если** у пресета нет
`maps/hub.json`. В новых пресетах предпочитай спавны внутри карты.
