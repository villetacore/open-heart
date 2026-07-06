//! ContentDb — загрузка контента ПРЕСЕТА из `res://presets/<id>/` (или `user://presets/<id>/`).
//!
//! Пресет — самодостаточный набор игровых данных (оружие, классы, перки, враги,
//! предметы, NPC, квесты, карты). Их может быть несколько — в главном меню выбирается
//! активный, и по сути это «разные игры» на одном движке (DESIGN_PLAN, запрос «пресеты»).
//!
//! Каждый тип контента при ошибке разбора падает на встроенную (`include_str!`)
//! копию из core-пресета с предупреждением в лог — игра запускается всегда.

use godot::classes::{DirAccess, FileAccess, file_access::ModeFlags};
use serde::Deserialize;

fn read(path: &str) -> Option<String> {
    FileAccess::open(path, ModeFlags::READ).map(|f| f.get_as_text().to_string())
}

/// Корневая папка пресета: сначала ищем в ресурсах игры, затем в пользовательской.
pub fn preset_base(id: &str) -> String {
    let res = format!("res://presets/{}", id);
    if FileAccess::file_exists(&format!("{res}/preset.json")) {
        return res;
    }
    let user = format!("user://presets/{}", id);
    if FileAccess::file_exists(&format!("{user}/preset.json")) {
        return user;
    }
    res // фолбэк: даже если манифеста нет, читаем res://-путь (сработают embedded-копии)
}

/// Загрузить весь data-driven контент пресета. Вызывать в начале `Game3D::ready`.
pub fn load_preset(id: &str) {
    let base = preset_base(id);
    crate::weapon::load(read(&format!("{base}/weapons.json")).as_deref());
    crate::classes::load(read(&format!("{base}/classes.json")).as_deref());
    crate::perk::load(
        read(&format!("{base}/perks.json")).as_deref(),
        read(&format!("{base}/synergies.json")).as_deref(),
    );
}

// ── Манифест и обнаружение пресетов ──────────────────────────────────────────

#[derive(Deserialize, Clone)]
pub struct PresetInfo {
    pub id:      String,
    #[serde(default)] pub name_ru: String,
    #[serde(default)] pub desc_ru: String,
    #[serde(default)] pub author:  String,
    #[serde(default)] pub version: u32,
}

pub fn preset_info(id: &str) -> PresetInfo {
    let base = preset_base(id);
    read(&format!("{base}/preset.json"))
        .and_then(|j| serde_json::from_str(&j).ok())
        .unwrap_or(PresetInfo {
            id: id.to_string(),
            name_ru: id.to_string(),
            desc_ru: String::new(),
            author: String::new(),
            version: 0,
        })
}

/// Все доступные пресеты: встроенные (res://presets) + пользовательские (user://presets).
pub fn discover_presets() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for root in ["res://presets", "user://presets"] {
        if let Some(mut dir) = DirAccess::open(root) {
            let dirs = dir.get_directories();
            for i in 0..dirs.len() {
                let name = dirs.get(i).map(|s| s.to_string()).unwrap_or_default();
                if name.is_empty() || name.starts_with('.') { continue; }
                if !out.contains(&name) {
                    out.push(name);
                }
            }
        }
    }
    if out.is_empty() {
        out.push("core".to_string());
    }
    // core всегда первым
    out.sort_by_key(|p| (p != "core", p.clone()));
    out
}
