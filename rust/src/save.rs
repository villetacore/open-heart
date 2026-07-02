//! Сохранение / загрузка игры в user://save.json.
//! Обходимся без serde-деривов на GameState — маппим вручную.

use serde::{Deserialize, Serialize};
use godot::classes::{FileAccess, file_access::ModeFlags};
use std::collections::{HashMap, HashSet};
use crate::game_state::GameState;
use crate::item::Item;
use crate::quest::QuestState;

const SAVE_PATH: &str = "user://save.json";

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
    pub quests:     Vec<(String, String, bool)>, // (id, title, completed)
    pub inventory:  Vec<(String, String, u32)>,  // (id, name, qty)
    pub player_hp:  f32,
}

impl SaveData {
    pub fn from_game(state: &GameState, player_hp: f32) -> Self {
        Self {
            version:   1,
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
        }
    }

    pub fn into_game(self) -> (GameState, f32) {
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
        (s, self.player_hp)
    }
}

pub fn save(state: &GameState, player_hp: f32) -> bool {
    let data = SaveData::from_game(state, player_hp);
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

pub fn load() -> Option<(GameState, f32)> {
    let f    = FileAccess::open(SAVE_PATH, ModeFlags::READ)?;
    let text = f.get_as_text().to_string();
    let data: SaveData = serde_json::from_str(&text).ok()?;
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
