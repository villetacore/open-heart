//! Настройки игры: язык, громкость, чувствительность.
//! Сохраняются в user://settings.json через FileAccess.

use serde::{Deserialize, Serialize};
use godot::classes::{FileAccess, file_access::ModeFlags};

fn default_preset() -> String { "core".into() }
fn default_difficulty() -> String { "normal".into() }

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub lang:        String,
    pub master_vol:  f32,
    pub mouse_sens:  f32,
    /// Активный пресет игры (папка в res://presets или user://presets).
    #[serde(default = "default_preset")]
    pub preset:      String,
    /// Сложность: easy | normal | hard (множитель hp/урона врагов).
    #[serde(default = "default_difficulty")]
    pub difficulty:  String,
}

impl Default for Settings {
    fn default() -> Self {
        Self { lang: "ru".into(), master_vol: 0.8, mouse_sens: 0.002,
               preset: "core".into(), difficulty: "normal".into() }
    }
}

impl Settings {
    /// Множитель силы врагов по сложности.
    pub fn difficulty_mult(&self) -> f32 {
        match self.difficulty.as_str() {
            "easy" => 0.75,
            "hard" => 1.35,
            _      => 1.0,
        }
    }

    /// Название сложности для UI.
    pub fn difficulty_ru(&self) -> &'static str {
        match self.difficulty.as_str() {
            "easy" => "Лёгкая",
            "hard" => "Тяжёлая",
            _      => "Обычная",
        }
    }

    /// Циклическое переключение easy → normal → hard.
    pub fn cycle_difficulty(&mut self) {
        self.difficulty = match self.difficulty.as_str() {
            "easy"   => "normal".into(),
            "normal" => "hard".into(),
            _        => "easy".into(),
        };
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
