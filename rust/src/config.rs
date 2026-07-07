//! Загрузка игровых конфигов пресета: presets/<id>/{enemies,items,level,npcs,quests}.json.

use serde::Deserialize;
use godot::classes::{FileAccess, file_access::ModeFlags};

fn default_scale() -> f32 { 1.0 }

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
}

#[derive(Debug, Deserialize, Clone)]
pub struct ItemCfg {
    pub id:       String,
    pub name_ru:  String,
    pub name_en:  String,
    pub desc_ru:  String,
    pub desc_en:  String,
    pub value:    i32,
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
pub struct AmmoSpawn { pub kind: String, pub amount: u32, pub x: f32, pub z: f32 }

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
    pub count:     u32,
    #[serde(default)] pub reward_xp:   u32,
    #[serde(default)] pub reward_gold: i32,
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

        Self { enemies, items, level, npcs, quests, dialogues, npcs_file_present }
    }

    /// JSON-сцена диалога по id (приоритетнее story.rs — см. game.rs).
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
