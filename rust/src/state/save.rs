//! Сохранение / загрузка игры в user://save.json (версия 2: класс + арсенал).

use serde::{Deserialize, Serialize};
use godot::classes::{FileAccess, file_access::ModeFlags};
use std::collections::{HashMap, HashSet};
use crate::game_state::GameState;
use crate::item::Item;
use crate::quest::QuestState;
use crate::weapon::{Arsenal, WeaponId};

const SAVE_PATH: &str = "user://save.json";
/// Куда откладывается нечитаемый сейв (вместо тихой потери «Продолжить»).
const CORRUPT_PATH: &str = "user://save.corrupt.json";
/// Текущая версия формата. Поднимать при несовместимых изменениях; чтение
/// старых версий обеспечивают `#[serde(default)]`-поля (см. load()).
pub const SAVE_VERSION: u32 = 2;

fn default_level() -> u32 { 1 }
fn default_seed() -> u64 { 0x5EED_0001 }

#[derive(Serialize, Deserialize)]
pub struct SaveData {
    pub version:    u32,
    pub day:        u32,
    pub gold:       i32,
    pub int_:       i32,
    pub chr:        i32,
    pub fit:        i32,
    pub rep:        i32,
    pub wil:        i32,
    pub relations:  HashMap<String, i32>,
    pub flags:      Vec<String>,
    pub quests:     Vec<(String, String, bool)>,
    pub inventory:  Vec<(String, String, u32)>,
    pub player_hp:  f32,

    // ── v2 ──
    #[serde(default)] pub class_idx:  i32,   // -1 = класс не выбран
    #[serde(default)] pub spec_idx:   u32,
    #[serde(default = "default_level")] pub level: u32,
    #[serde(default)] pub xp:         u32,
    #[serde(default)] pub ammo:       Vec<u32>,     // 4 типа
    #[serde(default)] pub weapons:    Vec<u32>,     // слоты имеющегося оружия
    #[serde(default)] pub cur_weapon: u32,
    #[serde(default = "default_seed")] pub dungeon_seed: u64,
    #[serde(default)] pub dungeons_cleared: u32,
    #[serde(default)] pub hearts: u32,
    #[serde(default)] pub perks: Vec<(String, u32)>,
    #[serde(default)] pub perk_points: u32,
    #[serde(default = "default_preset")] pub preset: String,
    #[serde(default)] pub quest_kills: Vec<(String, u32)>,
}

fn default_preset() -> String { "core".into() }

impl SaveData {
    pub fn from_game(state: &GameState, player_hp: f32, ars: &Arsenal) -> Self {
        Self {
            version:   SAVE_VERSION,
            day:       state.day,
            gold:      state.gold,
            int_:      state.stats.intelligence,
            chr:       state.stats.charm,
            fit:       state.stats.fitness,
            rep:       state.stats.reputation,
            wil:       state.stats.willpower,
            relations: state.relations.clone(),
            flags:     state.flags.iter().cloned().collect(),
            quests:    state.quests.quests.iter()
                .map(|q| (q.id.clone(), q.title.clone(), q.state == QuestState::Completed))
                .collect(),
            inventory: state.inventory.items.iter()
                .map(|i| (i.id.clone(), i.name.clone(), i.qty))
                .collect(),
            player_hp,
            class_idx: state.class_idx.map(|c| c as i32).unwrap_or(-1),
            spec_idx:  state.spec_idx as u32,
            level:     state.level,
            xp:        state.xp,
            ammo:      ars.ammo.to_vec(),
            weapons:   (0..8).filter(|s| ars.owned[*s]).map(|s| s as u32).collect(),
            cur_weapon: ars.current.slot() as u32,
            dungeon_seed:     state.dungeon_seed,
            dungeons_cleared: state.dungeons_cleared,
            hearts:           state.hearts,
            perks:            state.perks.iter().map(|(k, v)| (k.clone(), *v)).collect(),
            perk_points:      state.perk_points,
            preset:           state.preset.clone(),
            quest_kills:      state.quest_kills.iter().map(|(k, v)| (k.clone(), *v)).collect(),
        }
    }

    pub fn into_game(self) -> (GameState, f32, Arsenal) {
        let mut s = GameState::new("Игрок");
        s.day   = self.day;
        s.gold  = self.gold;
        s.stats.intelligence = self.int_;
        s.stats.charm        = self.chr;
        s.stats.fitness      = self.fit;
        s.stats.reputation   = self.rep;
        s.stats.willpower    = self.wil;
        s.relations          = self.relations;
        s.flags              = self.flags.into_iter().collect::<HashSet<_>>();
        for (id, title, done) in self.quests {
            s.quests.add(&id, &title, "");
            if done { s.quests.complete(&id); }
        }
        for (id, name, qty) in self.inventory {
            s.inventory.add(Item::new(&id, &name, "", qty));
        }
        s.class_idx = if self.class_idx >= 0 { Some(self.class_idx as usize) } else { None };
        s.spec_idx  = self.spec_idx as usize;
        s.level     = self.level.max(1);
        s.xp        = self.xp;
        s.dungeon_seed     = self.dungeon_seed;
        s.dungeons_cleared = self.dungeons_cleared;
        s.hearts           = self.hearts;
        s.perks            = self.perks.into_iter().collect();
        s.perk_points      = self.perk_points;
        s.preset           = self.preset;
        s.quest_kills      = self.quest_kills.into_iter().collect();

        let mut ars = Arsenal::new();
        for (i, v) in self.ammo.iter().take(4).enumerate() {
            ars.ammo[i] = *v;
        }
        for slot in &self.weapons {
            ars.owned[(*slot as usize).min(7)] = true;
        }
        ars.current = WeaponId::from_slot(self.cur_weapon as usize);
        if !ars.owned[ars.current.slot()] {
            // подстраховка: хоть какое-то оружие
            if let Some(s0) = (0..8).find(|s| ars.owned[*s]) {
                ars.current = WeaponId::from_slot(s0);
            }
        }
        (s, self.player_hp, ars)
    }
}

pub fn save(state: &GameState, player_hp: f32, ars: &Arsenal) -> bool {
    let data = SaveData::from_game(state, player_hp, ars);
    match serde_json::to_string_pretty(&data) {
        Ok(json) => {
            if let Some(mut f) = FileAccess::open(SAVE_PATH, ModeFlags::WRITE) {
                f.store_string(&json);
                return true;
            }
            false
        }
        Err(_) => false,
    }
}

pub fn load() -> Option<(GameState, f32, Arsenal)> {
    use godot::global::godot_warn;
    let f    = FileAccess::open(SAVE_PATH, ModeFlags::READ)?;
    let text = f.get_as_text().to_string();
    drop(f); // закрыть до возможного переименования
    let data: SaveData = match serde_json::from_str(&text) {
        Ok(d) => d,
        Err(e) => {
            // Битый сейв НЕ считается «сейва нет» (иначе новая игра молча его
            // перезапишет): откладываем в save.corrupt.json и громко сообщаем.
            godot_warn!("[save] save.json не читается ({e}) — отложен в {CORRUPT_PATH}");
            if let Some(mut dir) = godot::classes::DirAccess::open("user://") {
                let _ = dir.rename(SAVE_PATH, CORRUPT_PATH);
            }
            return None;
        }
    };
    if data.version > SAVE_VERSION {
        godot_warn!("[save] сейв версии {} новее поддерживаемой {} — читаю, что смогу",
                    data.version, SAVE_VERSION);
    }
    Some(data.into_game())
}

pub fn exists() -> bool {
    FileAccess::file_exists(SAVE_PATH)
}

pub fn delete() {
    use godot::classes::DirAccess;
    if let Some(mut dir) = DirAccess::open("user://") {
        let _ = dir.remove(SAVE_PATH);
    }
}
