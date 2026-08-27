//! ContentDb — загрузка контента ПРЕСЕТА из `res://presets/<id>/` (или `user://presets/<id>/`).
//!
//! Пресет — самодостаточный набор игровых данных (оружие, классы, перки, враги,
//! предметы, NPC, квесты, карты). Их может быть несколько — в главном меню выбирается
//! активный, и по сути это «разные игры» на одном движке (DESIGN_PLAN, запрос «пресеты»).
//!
//! Каждый тип контента при ошибке разбора падает на встроенную (`include_str!`)
//! копию из core-пресета с предупреждением в лог — игра запускается всегда.

use godot::classes::{file_access::ModeFlags, DirAccess, FileAccess};

/// Корневая папка пресета: сначала ищем в ресурсах игры, затем в пользовательской.
pub fn preset_base(id: &str) -> String {
    let res = format!("res://presets/{}", id);
    if FileAccess::file_exists(&format!("{res}/preset.toml"))
        || FileAccess::file_exists(&format!("{res}/preset.json"))
    {
        return res;
    }
    let user = format!("user://presets/{}", id);
    if FileAccess::file_exists(&format!("{user}/preset.toml"))
        || FileAccess::file_exists(&format!("{user}/preset.json"))
    {
        return user;
    }
    res // фолбэк: даже если манифеста нет, читаем res://-путь (сработают embedded-копии)
}

/// Загрузить весь data-driven контент пресета. Вызывать в начале `Game3D::ready`.
pub fn load_preset(id: &str) {
    let base = preset_base(id);
    let weapons = crate::format::normalized_json(&base, "weapons").and_then(Result::ok);
    let weapon_mods =
        crate::format::normalized_json(&base, "weapon_mods").and_then(Result::ok);
    let classes = crate::format::normalized_json(&base, "classes").and_then(Result::ok);
    let perks = crate::format::normalized_json(&base, "perks").and_then(Result::ok);
    let synergies = crate::format::normalized_json(&base, "synergies").and_then(Result::ok);
    crate::weapon::load(weapons.as_deref(), weapon_mods.as_deref());
    crate::classes::load(classes.as_deref());
    crate::perk::load(perks.as_deref(), synergies.as_deref());
}

// ── Манифест и обнаружение пресетов ──────────────────────────────────────────

pub use openheart_core::data::preset::PresetInfo;

pub fn preset_info(id: &str) -> PresetInfo {
    let base = preset_base(id);
    crate::format::find(&base, "preset")
        .and_then(|file| crate::format::parse(&file).ok())
        .unwrap_or(PresetInfo {
            id: id.to_string(),
            schema_version: 1,
            name_ru: id.to_string(),
            name_en: id.to_string(),
            desc_ru: String::new(),
            desc_en: String::new(),
            author: String::new(),
            version: 0,
            enabled: true,
            base: None,
            entry_map: None,
            tags: Vec::new(),
        })
}

// ── Тесты контента ────────────────────────────────────────────────────────────
// `cargo test` без движка: парсит ВСЕ json каждого пресета и проверяет ссылочную
// целостность (quest → npc/enemy/item, спавны карт → enemies/items). CI-страховка:
// битая запись в пресете ловится до запуска игры (DESIGN_PLAN §14).
/// Отпечаток данных пресета — его сервер сверяет при входе.
///
/// Файлы перечисляются движком (`DirAccess`), а считает хеш ядро — тем же
/// алгоритмом, что и сервер на Go.
pub fn preset_content_hash(id: &str) -> String {
    let base = preset_base(id);
    let mut files = Vec::new();
    collect_content_files(&base, "", &mut files);
    if files.is_empty() {
        return String::new();
    }
    openheart_core::data::hash::preset_hash(&files)
}

/// Рекурсивно собрать файлы данных пресета: (относительный путь, содержимое).
fn collect_content_files(base: &str, rel: &str, out: &mut Vec<(String, String)>) {
    let dir_path = if rel.is_empty() {
        base.to_string()
    } else {
        format!("{base}/{rel}")
    };
    let Some(dir) = DirAccess::open(&dir_path) else {
        return;
    };

    let files = dir.get_files();
    for index in 0..files.len() {
        let name = files.get(index).map(|s| s.to_string()).unwrap_or_default();
        if !openheart_core::data::hash::is_content_file(&name) {
            continue;
        }
        let rel_path = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        if let Some(file) = FileAccess::open(&format!("{base}/{rel_path}"), ModeFlags::READ) {
            out.push((rel_path, file.get_as_text().to_string()));
        }
    }

    let dirs = dir.get_directories();
    for index in 0..dirs.len() {
        let name = dirs.get(index).map(|s| s.to_string()).unwrap_or_default();
        if name.is_empty() || name.starts_with('.') {
            continue;
        }
        let child = if rel.is_empty() {
            name
        } else {
            format!("{rel}/{name}")
        };
        collect_content_files(base, &child, out);
    }
}

/// Все доступные пресеты: встроенные (res://presets) + пользовательские (user://presets).
pub fn discover_presets() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for root in ["res://presets", "user://presets"] {
        if let Some(dir) = DirAccess::open(root) {
            let dirs = dir.get_directories();
            for i in 0..dirs.len() {
                let name = dirs.get(i).map(|s| s.to_string()).unwrap_or_default();
                if name.is_empty() || name.starts_with('.') {
                    continue;
                }
                if !preset_info(&name).enabled {
                    continue;
                }
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
