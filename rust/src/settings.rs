//! Настройки игры: язык, громкость, чувствительность.
//! Сохраняются в user://settings.json через FileAccess.

use serde::{Deserialize, Serialize};
use godot::classes::{FileAccess, file_access::ModeFlags};

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub lang:        String,
    pub master_vol:  f32,
    pub mouse_sens:  f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self { lang: "ru".into(), master_vol: 0.8, mouse_sens: 0.002 }
    }
}

impl Settings {
    pub fn load() -> Self {
        if let Some(f) = FileAccess::open("user://settings.json", ModeFlags::READ) {
            if let Ok(s) = serde_json::from_str(&f.get_as_text().to_string()) {
                return s;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            if let Some(mut f) = FileAccess::open("user://settings.json", ModeFlags::WRITE) {
                f.store_string(&json);
            }
        }
    }
}
