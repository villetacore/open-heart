//! Загрузка игровых конфигов пресета: presets/<id>/{enemies,items,level,npcs,quests}.json.

use serde::Deserialize;
use godot::classes::{FileAccess, file_access::ModeFlags};

fn default_scale() -> f32 { 1.0 }

// ── Serde helpers: accept both JSON integers and floats for integer fields ────
pub(crate) fn de_u32<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    f64::deserialize(d).map(|v| v as u32)
}
pub(crate) fn de_i32<'de, D: serde::Deserializer<'de>>(d: D) -> Result<i32, D::Error> {
    f64::deserialize(d).map(|v| v as i32)
}

// ── Структуры конфигов ────────────────────────────────────────────────────────

fn default_xp() -> f32 { 15.0 }

/// Резисты урона: 0.0 = нет резиста, 1.0 = полный иммунитет, <0 = уязвимость.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct Resist {
    #[serde(default)] pub physical: f32,
    #[serde(default)] pub fire:     f32,
    #[serde(default)] pub energy:   f32,
    #[serde(default)] pub void:     f32,
}

impl Resist {
    /// [physical, fire, energy, void] — в порядке DmgType::idx.
    pub fn arr(&self) -> [f32; 4] {
        [self.physical, self.fire, self.energy, self.void]
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct EnemyCfg {
    pub id:              String,
    pub name:            String,
    pub hp:              f32,
    pub speed:           f32,
    pub attack_damage:   f32,
    pub attack_range:    f32,
    pub attack_cooldown: f32,
    pub chase_range:     f32,
    pub patrol_radius:   f32,
    pub color_r:         f32,
    pub color_g:         f32,
    pub color_b:         f32,
    #[serde(default = "default_xp")]
    pub xp:              f32,
    #[serde(default)]
    pub resist:          Resist,
    /// Имя спрайт-листа (enemy_<sprite>.png); по умолчанию = id.
    #[serde(default)]
    pub sprite:          Option<String>,
    /// Масштаб спрайта/коллайдера (1.0 = обычный, >1.2 = «крупный»).
    #[serde(default = "default_scale")]
    pub scale:           f32,
    /// Боевое поведение: "melee" (в контакт, по умолчанию) | "ranged" (держит дистанцию).
    #[serde(default)]
    pub behavior:        Option<String>,
    /// id способностей из abilities.json (кастуются по кулдауну при видимости).
    #[serde(default)]
    pub abilities:       Vec<String>,
    /// Шанс (0..1) прервать врага стаггером при получении урона.
    #[serde(default)]
    pub pain_chance:     f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ItemCfg {
    pub id:       String,
    pub name_ru:  String,
    pub name_en:  String,
    pub desc_ru:  String,
    pub desc_en:  String,
    pub value:    f64,
    pub category: String,
    pub heal:     Option<f32>,
    pub color_r:  f32,
    pub color_g:  f32,
    pub color_b:  f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EnemySpawn { pub kind: String, pub x: f32, pub z: f32 }

#[derive(Debug, Deserialize, Clone)]
pub struct ItemSpawn  { pub kind: String, pub x: f32, pub z: f32 }

#[derive(Debug, Deserialize, Clone)]
pub struct AmmoSpawn {
    pub kind: String,
    #[serde(deserialize_with = "de_u32")] pub amount: u32,
    pub x: f32, pub z: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WeaponSpawn { pub kind: String, pub x: f32, pub z: f32 }

#[derive(Debug, Deserialize, Clone, Default)]
pub struct LevelCfg {
    #[serde(default)]
    pub spawn_enemies: Vec<EnemySpawn>,
    #[serde(default)]
    pub spawn_items:   Vec<ItemSpawn>,
    #[serde(default)]
    pub spawn_ammo:    Vec<AmmoSpawn>,
    #[serde(default)]
    pub spawn_weapons: Vec<WeaponSpawn>,
}

// ── NPC ───────────────────────────────────────────────────────────────────────

/// NPC из npcs.json пресета. `scene`: "story" — динамические сцены из story.rs
/// (для 8 исходных персонажей), пусто/нет — сгенерированный квест-диалог по `quest`.
#[derive(Debug, Deserialize, Clone)]
pub struct NpcCfg {
    pub id:       String,
    pub name_ru:  String,
    #[serde(default)] pub sprite: String,       // имя файла в characters/ (npc_vale...)
    pub pos:      [f32; 2],                      // x, z на карте мира
    #[serde(default)] pub color:  Option<[f32; 3]>,
    #[serde(default)] pub scene:  Option<String>,
    #[serde(default)] pub quest:  Option<String>,
}

// ── Квесты ────────────────────────────────────────────────────────────────────

/// Квест из quests.json. kind: "kill" (target=id врага), "collect" (target=id предмета),
/// "clear_dungeon" (count = требуемая глубина).
#[derive(Debug, Deserialize, Clone)]
pub struct QuestCfg {
    pub id:        String,
    pub title_ru:  String,
    pub desc_ru:   String,
    pub giver:     String,
    pub kind:      String,
    #[serde(default)] pub target: String,
    #[serde(deserialize_with = "de_u32")] pub count:       u32,
    #[serde(default, deserialize_with = "de_u32")] pub reward_xp:   u32,
    #[serde(default, deserialize_with = "de_i32")] pub reward_gold: i32,
}

// ── Способности врагов (abilities.json) ──────────────────────────────────────

fn d_ab_cd() -> f32 { 4.0 }
fn d_ab_maxr() -> f32 { 14.0 }

/// Способность врага. kind определяет, какие поля значимы:
/// projectile_burst — count/spread/proj_speed/damage;
/// charge — speed_mult/duration/damage; summon — minion/count;
/// heal_pulse — heal/radius. Общие: cooldown, telegraph (подсветка перед
/// эффектом — окно на уворот), min_range/max_range (когда кастовать), color.
#[derive(Debug, Deserialize, Clone)]
pub struct AbilityCfg {
    pub id:   String,
    pub kind: String,
    #[serde(default = "d_ab_cd")]   pub cooldown:  f32,
    #[serde(default)]               pub telegraph: f32,
    #[serde(default)]               pub min_range: f32,
    #[serde(default = "d_ab_maxr")] pub max_range: f32,
    #[serde(default)]               pub color: Option<[f32; 3]>,
    // projectile_burst
    #[serde(default, deserialize_with = "de_u32")] pub count: u32,
    #[serde(default)] pub spread:     f32,
    #[serde(default)] pub proj_speed: f32,
    #[serde(default)] pub damage:     f32,
    // charge
    #[serde(default)] pub speed_mult: f32,
    #[serde(default)] pub duration:   f32,
    // summon
    #[serde(default)] pub minion: Option<String>,
    // heal_pulse
    #[serde(default)] pub heal:   f32,
    #[serde(default)] pub radius: f32,
}

// ── Данж (dungeon.json): темы, пулы врагов, настройки ────────────────────────

/// Тема данжа. Текстуры — короткими именами (dtile_* → textures/dungeon,
/// см. map::tex_path) или полными res://-путями.
#[derive(Debug, Deserialize, Clone)]
pub struct ThemeCfg {
    pub name_ru: String,
    pub wall:    String,
    pub accent:  String,
    pub floor:   String,
    pub ceil:    String,
    pub lava:    String,
    pub light:   [f32; 3],
}

/// Пул врагов: действует с глубины min_depth (берётся самый глубокий из подходящих).
#[derive(Debug, Deserialize, Clone)]
pub struct PoolCfg {
    #[serde(deserialize_with = "de_u32")]
    pub min_depth: u32,
    pub enemies:   Vec<String>,
}

fn d_boss()       -> String { "brute".into() }
fn d_boss_mult()  -> f32 { 1.25 }
fn d_mult_depth() -> f32 { 0.18 }

#[derive(Debug, Deserialize, Clone)]
pub struct DungeonSettings {
    /// id босса (enemies.json)
    #[serde(default = "d_boss")]       pub boss:           String,
    #[serde(default = "d_boss_mult")]  pub boss_mult:      f32,
    /// свита босса (спавнится по бокам алтаря)
    #[serde(default)]                  pub boss_guards:    Vec<String>,
    /// награда в боссовой комнате (id предметов)
    #[serde(default)]                  pub boss_items:     Vec<String>,
    /// прирост множителя hp/урона/XP за глубину
    #[serde(default = "d_mult_depth")] pub mult_per_depth: f32,
    /// пул оружейного тайника (id оружия)
    #[serde(default)]                  pub weapon_cache:   Vec<String>,
}

impl Default for DungeonSettings {
    fn default() -> Self {
        serde_json::from_str("{}").expect("DungeonSettings defaults")
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct DungeonCfg {
    #[serde(default)] pub themes:   Vec<ThemeCfg>,
    #[serde(default)] pub pools:    Vec<PoolCfg>,
    #[serde(default)] pub settings: DungeonSettings,
}

// ── Лут (loot.json) ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Clone)]
pub struct LootEntry {
    pub id:     String,
    pub chance: f32,
}

/// Дроп с убитого врага. Записи проверяются по порядку ОДНИМ броском
/// (кумулятивно): сумма chance ≤ 1.0, остаток — «ничего».
#[derive(Debug, Deserialize, Clone)]
pub struct KillDrop {
    pub kind:   String,           // "ammo" (случайный тип) | "item"
    #[serde(default)] pub id: Option<String>,   // id предмета для kind=item
    pub chance: f32,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct LootSettings {
    /// Шансы точек патронов в комнате данжа (каждая — своя позиция).
    #[serde(default)] pub room_ammo_chances: Vec<f32>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct LootCfg {
    #[serde(default)] pub room_items: Vec<LootEntry>,
    #[serde(default)] pub kill_drops: Vec<KillDrop>,
    #[serde(default)] pub settings:   LootSettings,
}

#[derive(Deserialize, Default)] pub(crate) struct EnemiesFile { pub(crate) enemies: Vec<EnemyCfg> }
#[derive(Deserialize, Default)] pub(crate) struct ItemsFile   { pub(crate) items:   Vec<ItemCfg>  }

// ── GameConfig ────────────────────────────────────────────────────────────────

pub struct GameConfig {
    pub enemies: Vec<EnemyCfg>,
    pub items:   Vec<ItemCfg>,
    pub level:   LevelCfg,
    pub npcs:    Vec<NpcCfg>,
    pub quests:  Vec<QuestCfg>,
    /// Data-driven сцены диалогов (dialogues.json); приоритетнее story.rs.
    pub dialogues: Vec<crate::dialogue::Scene>,
    /// Способности врагов (abilities.json); нет файла → встроенные core.
    pub abilities: Vec<AbilityCfg>,
    /// Генерация данжей (dungeon.json); нет файла → встроенные настройки core.
    pub dungeon: DungeonCfg,
    /// Таблицы лута (loot.json); нет файла → встроенные настройки core.
    pub loot:    LootCfg,
    /// npcs.json существует (пустой список ≠ отсутствие файла: пустой — это
    /// осознанное «в этом пресете NPC нет», отсутствие — legacy-фолбэк).
    pub npcs_file_present: bool,
}

fn read(path: &str) -> Option<String> {
    FileAccess::open(path, ModeFlags::READ).map(|f| f.get_as_text().to_string())
}

// Встроенные копии core-пресета: битый JSON контент-мейкера не должен молча
// превращаться в пустой мир (без врагов/предметов/квестов) — падаем на них
// с предупреждением в лог (тот же принцип, что в weapon.rs/classes.rs/perk.rs).
const EMBEDDED_ENEMIES: &str = include_str!("../../godot/presets/core/enemies.json");
const EMBEDDED_ITEMS:   &str = include_str!("../../godot/presets/core/items.json");
const EMBEDDED_LEVEL:   &str = include_str!("../../godot/presets/core/level.json");
const EMBEDDED_NPCS:    &str = include_str!("../../godot/presets/core/npcs.json");
const EMBEDDED_QUESTS:  &str = include_str!("../../godot/presets/core/quests.json");
const EMBEDDED_DUNGEON: &str = include_str!("../../godot/presets/core/dungeon.json");
const EMBEDDED_LOOT:    &str = include_str!("../../godot/presets/core/loot.json");
const EMBEDDED_ABILITIES: &str = include_str!("../../godot/presets/core/abilities.json");

/// Распарсить текст конфига; при ошибке — громкое предупреждение и встроенная копия.
fn parse_loud<T: serde::de::DeserializeOwned + Default>(text: &str, file: &str, embedded: &str) -> T {
    match serde_json::from_str::<T>(text) {
        Ok(v) => v,
        Err(e) => {
            godot::global::godot_warn!("[preset] {file}: ошибка разбора ({e}) — использую встроенную копию core");
            serde_json::from_str(embedded).unwrap_or_else(|e2| {
                godot::global::godot_warn!("[preset] встроенная копия {file} тоже не парсится: {e2}");
                T::default()
            })
        }
    }
}

/// Для cargo test: встроенные копии core обязаны парситься (последний рубеж фолбэков).
#[cfg(test)]
pub(crate) fn embedded_configs_parse_for_test() {
    serde_json::from_str::<EnemiesFile>(EMBEDDED_ENEMIES).expect("embedded enemies.json");
    serde_json::from_str::<ItemsFile>(EMBEDDED_ITEMS).expect("embedded items.json");
    serde_json::from_str::<LevelCfg>(EMBEDDED_LEVEL).expect("embedded level.json");
    serde_json::from_str::<Vec<NpcCfg>>(EMBEDDED_NPCS).expect("embedded npcs.json");
    serde_json::from_str::<Vec<QuestCfg>>(EMBEDDED_QUESTS).expect("embedded quests.json");
    let d = serde_json::from_str::<DungeonCfg>(EMBEDDED_DUNGEON).expect("embedded dungeon.json");
    assert!(!d.themes.is_empty() && !d.pools.is_empty(), "embedded dungeon.json: пустые themes/pools");
    serde_json::from_str::<LootCfg>(EMBEDDED_LOOT).expect("embedded loot.json");
    serde_json::from_str::<Vec<AbilityCfg>>(EMBEDDED_ABILITIES).expect("embedded abilities.json");
}

impl GameConfig {
    /// Загрузить конфиги из корня пресета (например "res://presets/core").
    ///
    /// Отсутствие файла и битый файл — разные состояния: отсутствие quests/level —
    /// осознанное «в этом пресете этого нет», отсутствие npcs — legacy-фолбэк на
    /// NPC_DATA; битый JSON всегда даёт предупреждение и встроенную копию core.
    pub fn load_from(base: &str) -> Self {
        use godot::global::godot_warn;

        let enemies = match read(&format!("{base}/enemies.json")) {
            Some(t) => parse_loud::<EnemiesFile>(&t, "enemies.json", EMBEDDED_ENEMIES),
            None => {
                godot_warn!("[preset] {base}/enemies.json не найден — использую встроенную копию core");
                parse_loud::<EnemiesFile>(EMBEDDED_ENEMIES, "enemies.json", EMBEDDED_ENEMIES)
            }
        }.enemies;

        let items = match read(&format!("{base}/items.json")) {
            Some(t) => parse_loud::<ItemsFile>(&t, "items.json", EMBEDDED_ITEMS),
            None => {
                godot_warn!("[preset] {base}/items.json не найден — использую встроенную копию core");
                parse_loud::<ItemsFile>(EMBEDDED_ITEMS, "items.json", EMBEDDED_ITEMS)
            }
        }.items;

        // level.json опционален: карты пресета (maps/*.json) несут свои спавны.
        let level = match read(&format!("{base}/level.json")) {
            Some(t) => parse_loud::<LevelCfg>(&t, "level.json", EMBEDDED_LEVEL),
            None => LevelCfg::default(),
        };

        let npcs_raw = read(&format!("{base}/npcs.json"));
        let npcs_file_present = npcs_raw.is_some();
        let npcs: Vec<NpcCfg> = match npcs_raw {
            Some(t) => parse_loud(&t, "npcs.json", EMBEDDED_NPCS),
            None => Vec::new(),
        };

        // квестов может осознанно не быть (например, чистая арена)
        let quests: Vec<QuestCfg> = match read(&format!("{base}/quests.json")) {
            Some(t) => parse_loud(&t, "quests.json", EMBEDDED_QUESTS),
            None => Vec::new(),
        };

        // генерация данжей и лут: нет файла — молча встроенные core-настройки
        // (данжи должны работать в любом пресете); битый файл — предупреждение.
        let mut dungeon = match read(&format!("{base}/dungeon.json")) {
            Some(t) => parse_loud::<DungeonCfg>(&t, "dungeon.json", EMBEDDED_DUNGEON),
            None => serde_json::from_str(EMBEDDED_DUNGEON).unwrap_or_default(),
        };
        // семантические минимумы: без тем/пулов генератор не сможет работать
        if dungeon.themes.is_empty() || dungeon.pools.is_empty() {
            godot_warn!("[preset] dungeon.json: пустые themes/pools — использую встроенные core");
            let emb: DungeonCfg = serde_json::from_str(EMBEDDED_DUNGEON).unwrap_or_default();
            if dungeon.themes.is_empty() { dungeon.themes = emb.themes; }
            if dungeon.pools.is_empty()  { dungeon.pools  = emb.pools; }
        }
        let loot = match read(&format!("{base}/loot.json")) {
            Some(t) => parse_loud::<LootCfg>(&t, "loot.json", EMBEDDED_LOOT),
            None => serde_json::from_str(EMBEDDED_LOOT).unwrap_or_default(),
        };

        // способности врагов: нет файла — молча встроенные core (как dungeon/loot)
        let abilities: Vec<AbilityCfg> = match read(&format!("{base}/abilities.json")) {
            Some(t) => parse_loud(&t, "abilities.json", EMBEDDED_ABILITIES),
            None => serde_json::from_str(EMBEDDED_ABILITIES).unwrap_or_default(),
        };

        // диалоги: отсутствие файла — норма (story.rs остаётся встроенным контентом)
        let dialogues = match read(&format!("{base}/dialogues.json")) {
            None => Vec::new(),
            Some(t) => match crate::dialogue::parse_scenes(&t) {
                Ok((scenes, errors)) => {
                    for e in errors {
                        godot_warn!("[preset] dialogues.json: {e} — сцена пропущена");
                    }
                    scenes
                }
                Err(e) => {
                    godot_warn!("[preset] dialogues.json: ошибка разбора ({e}) — диалоги пресета не загружены");
                    Vec::new()
                }
            },
        };

        Self { enemies, items, level, npcs, quests, dialogues, abilities, dungeon, loot, npcs_file_present }
    }

    /// JSON-сцена диалога по id (приоритетнее story.rs — см. game.rs).
    pub fn ability(&self, id: &str) -> Option<&AbilityCfg> {
        self.abilities.iter().find(|a| a.id == id)
    }

    pub fn dialogue(&self, id: &str) -> Option<&crate::dialogue::Scene> {
        self.dialogues.iter().find(|s| s.id == id)
    }

    pub fn enemy(&self, id: &str) -> Option<&EnemyCfg> {
        self.enemies.iter().find(|e| e.id == id)
    }

    pub fn item(&self, id: &str) -> Option<&ItemCfg> {
        self.items.iter().find(|i| i.id == id)
    }

    pub fn quest(&self, id: &str) -> Option<&QuestCfg> {
        self.quests.iter().find(|q| q.id == id)
    }
}
