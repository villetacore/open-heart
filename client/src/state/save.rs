//! Файл сохранения: user://save.json.
//!
//! Модель (`SaveData`, версия формата, конверсии) живёт в ядре — она нужна и
//! серверу; здесь остался только ввод-вывод через движок.

use godot::classes::{file_access::ModeFlags, DirAccess, FileAccess};

use crate::game_state::GameState;
use crate::weapon::Arsenal;

pub use openheart_core::state::save::{SaveData, SAVE_VERSION};

const SAVE_PATH: &str = "user://save.json";
/// Куда откладывается нечитаемый сейв (вместо тихой потери «Продолжить»).
const CORRUPT_PATH: &str = "user://save.corrupt.json";

pub fn save(state: &GameState, player_hp: f32, ars: &Arsenal) -> bool {
    let data = SaveData::from_game(state, player_hp, ars);
    match data.to_json() {
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
    let f = FileAccess::open(SAVE_PATH, ModeFlags::READ)?;
    let text = f.get_as_text().to_string();
    drop(f); // закрыть до возможного переименования
    let data = match SaveData::from_json(&text) {
        Ok(d) => d,
        Err(e) => {
            // Битый сейв НЕ считается «сейва нет» (иначе новая игра молча его
            // перезапишет): откладываем в save.corrupt.json и громко сообщаем.
            godot_warn!("[save] save.json не читается ({e}) — отложен в {CORRUPT_PATH}");
            if let Some(mut dir) = DirAccess::open("user://") {
                let _ = dir.rename(SAVE_PATH, CORRUPT_PATH);
            }
            return None;
        }
    };
    if data.version > SAVE_VERSION {
        godot_warn!(
            "[save] сейв версии {} новее поддерживаемой {} — читаю, что смогу",
            data.version,
            SAVE_VERSION
        );
    }
    Some(data.into_game())
}

pub fn exists() -> bool {
    FileAccess::file_exists(SAVE_PATH)
}

pub fn delete() {
    if let Some(mut dir) = DirAccess::open("user://") {
        let _ = dir.remove(SAVE_PATH);
    }
}
