//! Настройки игры: язык, громкость, чувствительность.
//! Сохраняются в user://settings.json через FileAccess.

use serde::{Deserialize, Serialize};
use godot::classes::{FileAccess, file_access::ModeFlags};

fn default_preset() -> String { "core".into() }
fn default_difficulty() -> String { "normal".into() }
fn t_true() -> bool { true }
fn d_music() -> f32 { 0.7 }
fn d_sfx() -> f32 { 0.9 }
fn d_fov() -> f32 { 75.0 }
fn d_one() -> f32 { 1.0 }

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

    // ── Аудио ──
    #[serde(default = "d_music")] pub music_vol: f32,
    #[serde(default = "d_sfx")]   pub sfx_vol:   f32,

    // ── Видео ──
    #[serde(default)]             pub fullscreen: bool,
    #[serde(default = "t_true")]  pub vsync:      bool,
    #[serde(default = "d_fov")]   pub fov:        f32,

    // ── Графика ──
    #[serde(default = "t_true")]  pub post_fx:        bool,
    #[serde(default = "d_one")]   pub post_intensity: f32,
    #[serde(default = "t_true")]  pub glow:           bool,
    #[serde(default = "t_true")]  pub shadows:        bool,

    // ── Геймплей ──
    /// Реактивная «тряска»/пульсация пост-процесса на события (урон/убийство).
    #[serde(default = "t_true")]  pub screen_shake:   bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            lang: "ru".into(), master_vol: 0.8, mouse_sens: 0.002,
            preset: "core".into(), difficulty: "normal".into(),
            music_vol: 0.7, sfx_vol: 0.9,
            fullscreen: false, vsync: true, fov: 75.0,
            post_fx: true, post_intensity: 1.0, glow: true, shadows: true,
            screen_shake: true,
        }
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

/// Линейная громкость (0..1) → децибелы.
fn lin_to_db(v: f32) -> f32 {
    if v <= 0.001 { -60.0 } else { 20.0 * v.log10() }
}

impl Settings {
    /// Применить видео-настройки: режим окна (фуллскрин/окно) + вертикальная синхр.
    pub fn apply_video(&self) {
        use godot::classes::DisplayServer;
        use godot::classes::display_server::{WindowMode, VSyncMode};
        use godot::obj::Singleton;
        let mut ds = DisplayServer::singleton();
        ds.window_set_mode(if self.fullscreen { WindowMode::FULLSCREEN } else { WindowMode::WINDOWED });
        ds.window_set_vsync_mode(if self.vsync { VSyncMode::ENABLED } else { VSyncMode::DISABLED });
    }

    /// Применить громкость к аудио-шинам (Master, а при наличии — Music/SFX).
    pub fn apply_audio(&self) {
        use godot::classes::AudioServer;
        use godot::obj::Singleton;
        let mut a = AudioServer::singleton();
        a.set_bus_volume_db(0, lin_to_db(self.master_vol));
        // Отдельные шины Music/SFX, если заведены (иначе тихо игнорируем).
        for (name, vol) in [("Music", self.music_vol), ("SFX", self.sfx_vol)] {
            let idx = a.get_bus_index(name);
            if idx >= 0 { a.set_bus_volume_db(idx, lin_to_db(vol)); }
        }
    }

    /// Применить всё, что можно применить без узлов сцены (видео + аудио).
    pub fn apply_global(&self) {
        self.apply_video();
        self.apply_audio();
    }
}
