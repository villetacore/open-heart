//! Модель сохранения: `SaveData` ↔ `GameState` + арсенал.
//!
//! Здесь только структура и конверсии — читает и пишет файл хост
//! (`client/src/state/save.rs` через движок, сервер — через свою БД).

use crate::game_state::GameState;
use crate::item::Item;
use crate::quest::QuestState;
use crate::weapon::{Arsenal, WeaponId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Текущая версия формата. Поднимать при несовместимых изменениях; чтение
/// старых версий обеспечивают `#[serde(default)]`-поля (см. load()).
pub const SAVE_VERSION: u32 = 4;

fn default_level() -> u32 {
    1
}
fn default_seed() -> u64 {
    0x5EED_0001
}

#[derive(Serialize, Deserialize)]
pub struct SaveData {
    #[serde(default)]
    pub abilities: crate::combat::ability::AbilityState,
    pub version: u32,
    pub day: u32,
    pub gold: i32,
    pub int_: i32,
    pub chr: i32,
    pub fit: i32,
    pub rep: i32,
    pub wil: i32,
    pub relations: HashMap<String, i32>,
    pub flags: Vec<String>,
    pub quests: Vec<(String, String, bool)>,
    pub inventory: Vec<(String, String, u32)>,
    pub player_hp: f32,

    // ── v2 ──
    #[serde(default)]
    pub class_idx: i32, // -1 = класс не выбран
    #[serde(default)]
    pub spec_idx: u32,
    #[serde(default = "default_level")]
    pub level: u32,
    #[serde(default)]
    pub xp: u32,
    #[serde(default)]
    pub clips: Vec<u32>,
    #[serde(default)]
    pub ammo: Vec<u32>, // 4 типа
    #[serde(default)]
    pub weapons: Vec<u32>, // слоты имеющегося оружия
    #[serde(default)]
    pub cur_weapon: u32,
    #[serde(default = "default_seed")]
    pub dungeon_seed: u64,
    #[serde(default)]
    pub dungeons_cleared: u32,
    #[serde(default)]
    pub hearts: u32,
    #[serde(default)]
    pub perks: Vec<(String, u32)>,
    #[serde(default)]
    pub perk_points: u32,
    #[serde(default = "default_preset")]
    pub preset: String,
    #[serde(default)]
    pub quest_kills: Vec<(String, u32)>,
    #[serde(default)]
    pub weapon_mods: Vec<u8>,
    #[serde(default)]
    pub weapon_mod_cores: u32,
    #[serde(default)]
    pub respec_used: bool,
}

fn default_preset() -> String {
    "core".into()
}

impl SaveData {
    pub fn from_game(state: &GameState, player_hp: f32, ars: &Arsenal) -> Self {
        Self {
            abilities: state.abilities.clone(),
            version: SAVE_VERSION,
            day: state.day,
            gold: state.gold,
            int_: state.stats.intelligence,
            chr: state.stats.charm,
            fit: state.stats.fitness,
            rep: state.stats.reputation,
            wil: state.stats.willpower,
            relations: state.relations.clone(),
            flags: state.flags.iter().cloned().collect(),
            quests: state
                .quests
                .quests
                .iter()
                .map(|q| {
                    (
                        q.id.clone(),
                        q.title.clone(),
                        q.state == QuestState::Completed,
                    )
                })
                .collect(),
            inventory: state
                .inventory
                .items
                .iter()
                .map(|i| (i.id.clone(), i.name.clone(), i.qty))
                .collect(),
            player_hp,
            class_idx: state.class_idx.map(|c| c as i32).unwrap_or(-1),
            spec_idx: state.spec_idx as u32,
            level: state.level,
            xp: state.xp,
            clips: ars.clips.to_vec(),
            ammo: ars.ammo.to_vec(),
            weapons: (0..8).filter(|s| ars.owned[*s]).map(|s| s as u32).collect(),
            cur_weapon: ars.current.slot() as u32,
            dungeon_seed: state.dungeon_seed,
            dungeons_cleared: state.dungeons_cleared,
            hearts: state.hearts,
            perks: state.perks.iter().map(|(k, v)| (k.clone(), *v)).collect(),
            perk_points: state.perk_points,
            preset: state.preset.clone(),
            quest_kills: state
                .quest_kills
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect(),
            weapon_mods: state.weapon_mods.to_vec(),
            weapon_mod_cores: state.weapon_mod_cores,
            respec_used: state.respec_used,
        }
    }

    pub fn into_game(self) -> (GameState, f32, Arsenal) {
        let mut s = GameState::new("Игрок");
        s.abilities = self.abilities;
        s.day = self.day;
        s.gold = self.gold;
        s.stats.intelligence = self.int_;
        s.stats.charm = self.chr;
        s.stats.fitness = self.fit;
        s.stats.reputation = self.rep;
        s.stats.willpower = self.wil;
        s.relations = self.relations;
        s.flags = self.flags.into_iter().collect::<HashSet<_>>();
        for (id, title, done) in self.quests {
            s.quests.add(&id, &title, "");
            if done {
                s.quests.complete(&id);
            }
        }
        for (id, name, qty) in self.inventory {
            s.inventory.add(Item::new(&id, &name, "", qty));
        }
        s.class_idx = if self.class_idx >= 0 {
            Some(self.class_idx as usize)
        } else {
            None
        };
        s.spec_idx = self.spec_idx as usize;
        s.level = self.level.max(1);
        s.xp = self.xp;
        s.dungeon_seed = self.dungeon_seed;
        s.dungeons_cleared = self.dungeons_cleared;
        s.hearts = self.hearts;
        s.perks = self.perks.into_iter().collect();
        s.perk_points = self.perk_points;
        s.preset = self.preset;
        s.quest_kills = self.quest_kills.into_iter().collect();
        for (slot, branch) in self.weapon_mods.iter().take(8).enumerate() {
            s.weapon_mods[slot] = (*branch).min(2);
        }
        s.weapon_mod_cores = self.weapon_mod_cores;
        s.respec_used = self.respec_used;

        let mut ars = Arsenal::new();
        for (i, v) in self.ammo.iter().take(4).enumerate() {
            ars.ammo[i] = *v;
        }
        for slot in &self.weapons {
            ars.owned[(*slot as usize).min(7)] = true;
        }
        if self.clips.is_empty() {
            for slot in 0..8 {
                if ars.owned[slot] {
                    ars.clips[slot] = crate::weapon::weapon_def(WeaponId::from_slot(slot)).magazine;
                }
            }
        } else {
            for (slot, value) in self.clips.iter().take(8).enumerate() {
                ars.clips[slot] = *value;
            }
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
impl SaveData {
    /// Разобрать сейв из JSON. Ошибку решает вызывающий: клиент откладывает
    /// битый файл, сервер — отвечает игроку.
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }

    /// Сериализовать для записи (файл у клиента, колонка save_json у сервера).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boss_aftermath_survives_save_round_trip() {
        let mut state = GameState::new("Tester");
        state.flags.insert("boss_defeated_heart_tyrant".into());
        state.inventory.add(Item::new(
            "trophy_tyrant_heart",
            "Tyrant's Heart",
            "Persistent trophy",
            1,
        ));
        let data = SaveData::from_game(&state, 83.0, &Arsenal::new());
        let (restored, hp, _) = data.into_game();

        assert_eq!(hp, 83.0);
        assert!(restored.flags.contains("boss_defeated_heart_tyrant"));
        assert!(restored
            .inventory
            .items
            .iter()
            .any(|item| item.id == "trophy_tyrant_heart" && item.qty == 1));
    }

    #[test]
    fn respec_flag_survives_save_round_trip() {
        let mut state = GameState::new("Tester");
        assert!(!state.respec_used);
        state.respec_used = true;
        let data = SaveData::from_game(&state, 100.0, &Arsenal::new());
        let (restored, _, _) = data.into_game();
        assert!(
            restored.respec_used,
            "первый бесплатный сброс не должен восстанавливаться перезагрузкой"
        );
    }

    #[test]
    fn completed_quest_finales_survive_save_round_trip() {
        let mut state = GameState::new("Tester");
        for quest_id in [
            "district_patrol",
            "relief_supplies",
            "stolen_cores",
            "archive_whispers",
            "heart_execution",
        ] {
            state.quests.add(quest_id, quest_id, "finale");
            state.quests.complete(quest_id);
        }
        let data = SaveData::from_game(&state, 100.0, &Arsenal::new());
        let (restored, _, _) = data.into_game();
        assert_eq!(
            restored
                .quests
                .quests
                .iter()
                .filter(|quest| quest.state == QuestState::Completed)
                .count(),
            5
        );
    }
}
