//! Загрузка игровых конфигов из godot/data/*.json.

use serde::Deserialize;
use godot::classes::{FileAccess, file_access::ModeFlags};

// ── Структуры конфигов ────────────────────────────────────────────────────────

fn default_xp() -> f32 { 15.0 }

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

#[derive(Deserialize, Default)] struct EnemiesFile { enemies: Vec<EnemyCfg> }
#[derive(Deserialize, Default)] struct ItemsFile   { items:   Vec<ItemCfg>  }

// ── GameConfig ────────────────────────────────────────────────────────────────

pub struct GameConfig {
    pub enemies: Vec<EnemyCfg>,
    pub items:   Vec<ItemCfg>,
    pub level:   LevelCfg,
}

fn read(path: &str) -> String {
    FileAccess::open(path, ModeFlags::READ)
        .map(|f| f.get_as_text().to_string())
        .unwrap_or_default()
}

impl GameConfig {
    pub fn load() -> Self {
        let enemies = serde_json::from_str::<EnemiesFile>(&read("res://data/enemies.json"))
            .unwrap_or_default().enemies;
        let items   = serde_json::from_str::<ItemsFile>(&read("res://data/items.json"))
            .unwrap_or_default().items;
        let level   = serde_json::from_str::<LevelCfg>(&read("res://data/level.json"))
            .unwrap_or_default();
        Self { enemies, items, level }
    }

    pub fn enemy(&self, id: &str) -> Option<&EnemyCfg> {
        self.enemies.iter().find(|e| e.id == id)
    }

    pub fn item(&self, id: &str) -> Option<&ItemCfg> {
        self.items.iter().find(|i| i.id == id)
    }
}
