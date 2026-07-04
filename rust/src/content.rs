//! ContentDb — загрузка всего контента из `res://data/*.json` при старте игры.
//!
//! Каждый тип контента при ошибке разбора падает на встроенную (`include_str!`)
//! копию с предупреждением в лог, поэтому игра запускается даже с битым конфигом.
//! Так реализуется принцип «всё в конфигурации» из DESIGN_PLAN §2.

use godot::classes::{FileAccess, file_access::ModeFlags};

fn read(path: &str) -> Option<String> {
    FileAccess::open(path, ModeFlags::READ).map(|f| f.get_as_text().to_string())
}

/// Загрузить весь data-driven контент. Вызывать один раз в начале `Game3D::ready`.
pub fn load_all() {
    crate::weapon::init(read("res://data/weapons.json").as_deref());
    crate::classes::init(read("res://data/classes.json").as_deref());
    crate::perk::init(
        read("res://data/perks.json").as_deref(),
        read("res://data/synergies.json").as_deref(),
    );
}
