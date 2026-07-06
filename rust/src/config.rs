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
    pub spawn_enemies: Vec<EnemySpawn>,
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

#[derive(Deserialize, Default)] struct EnemiesFile { enemies: Vec<EnemyCfg> }
#[derive(Deserialize, Default)] struct ItemsFile   { items:   Vec<ItemCfg>  }

// ── GameConfig ────────────────────────────────────────────────────────────────

pub struct GameConfig {
    pub enemies: Vec<EnemyCfg>,
    pub items:   Vec<ItemCfg>,
    pub level:   LevelCfg,
    pub npcs:    Vec<NpcCfg>,
    pub quests:  Vec<QuestCfg>,
    /// npcs.json существует (пустой список ≠ отсутствие файла: пустой — это
    /// осознанное «в этом пресете NPC нет», отсутствие — legacy-фолбэк).
    pub npcs_file_present: bool,
}

fn read(path: &str) -> String {
    FileAccess::open(path, ModeFlags::READ)
        .map(|f| f.get_as_text().to_string())
        .unwrap_or_default()
}

impl GameConfig {
    /// Загрузить конфиги из корня пресета (например "res://presets/core").
    pub fn load_from(base: &str) -> Self {
        let enemies = serde_json::from_str::<EnemiesFile>(&read(&format!("{base}/enemies.json")))
            .unwrap_or_default().enemies;
        let items   = serde_json::from_str::<ItemsFile>(&read(&format!("{base}/items.json")))
            .unwrap_or_default().items;
        let level   = serde_json::from_str::<LevelCfg>(&read(&format!("{base}/level.json")))
            .unwrap_or_default();
        let npcs_raw = read(&format!("{base}/npcs.json"));
        let npcs_file_present = !npcs_raw.is_empty();
        let npcs    = serde_json::from_str::<Vec<NpcCfg>>(&npcs_raw).unwrap_or_default();
        let quests  = serde_json::from_str::<Vec<QuestCfg>>(&read(&format!("{base}/quests.json")))
            .unwrap_or_default();
        Self { enemies, items, level, npcs, quests, npcs_file_present }
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
