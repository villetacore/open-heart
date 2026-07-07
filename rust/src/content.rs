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

// ── Тесты контента ────────────────────────────────────────────────────────────
// `cargo test` без движка: парсит ВСЕ json каждого пресета и проверяет ссылочную
// целостность (quest → npc/enemy/item, спавны карт → enemies/items). CI-страховка:
// битая запись в пресете ловится до запуска игры (DESIGN_PLAN §14).

#[cfg(test)]
mod preset_tests {
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};

    use crate::config::{EnemiesFile, ItemsFile, LevelCfg, NpcCfg, QuestCfg};

    fn presets_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../godot/presets")
    }

    fn read(preset: &Path, file: &str) -> Option<String> {
        std::fs::read_to_string(preset.join(file)).ok()
    }

    fn must_read(preset: &Path, file: &str) -> String {
        read(preset, file).unwrap_or_else(|| panic!("{}: нет файла {file}", preset.display()))
    }

    fn parse<T: serde::de::DeserializeOwned>(preset: &Path, file: &str) -> T {
        let text = must_read(preset, file);
        serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("{}/{file}: {e}", preset.display()))
    }

    fn preset_dirs() -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = std::fs::read_dir(presets_root())
            .expect("нет папки godot/presets")
            .filter_map(|d| d.ok().map(|d| d.path()))
            .filter(|p| p.is_dir())
            .collect();
        out.sort();
        assert!(!out.is_empty(), "в godot/presets не найдено ни одного пресета");
        out
    }

    #[test]
    fn all_presets_parse_and_link() {
        for preset in preset_dirs() {
            let name = preset.file_name().unwrap().to_string_lossy().to_string();

            // форматы: любой битый файл валит тест с именем пресета и ошибкой serde
            let _info: super::PresetInfo = parse(&preset, "preset.json");
            crate::weapon::parse(&must_read(&preset, "weapons.json"))
                .unwrap_or_else(|e| panic!("{name}/weapons.json: {e}"));
            crate::classes::parse(&must_read(&preset, "classes.json"))
                .unwrap_or_else(|e| panic!("{name}/classes.json: {e}"));
            crate::perk::parse_perks(&must_read(&preset, "perks.json"))
                .unwrap_or_else(|e| panic!("{name}/perks.json: {e}"));
            crate::perk::parse_syn(&must_read(&preset, "synergies.json"))
                .unwrap_or_else(|e| panic!("{name}/synergies.json: {e}"));

            let enemies: EnemiesFile = parse(&preset, "enemies.json");
            let items:   ItemsFile   = parse(&preset, "items.json");
            let level:   Option<LevelCfg> = read(&preset, "level.json")
                .map(|t| serde_json::from_str(&t)
                    .unwrap_or_else(|e| panic!("{name}/level.json: {e}")));
            let npcs: Vec<NpcCfg> = read(&preset, "npcs.json")
                .map(|t| serde_json::from_str(&t)
                    .unwrap_or_else(|e| panic!("{name}/npcs.json: {e}")))
                .unwrap_or_default();
            let quests: Vec<QuestCfg> = read(&preset, "quests.json")
                .map(|t| serde_json::from_str(&t)
                    .unwrap_or_else(|e| panic!("{name}/quests.json: {e}")))
                .unwrap_or_default();

            // карты
            let mut map_spawns: Vec<(String, String)> = Vec::new(); // (kind_type, id)
            if let Ok(maps) = std::fs::read_dir(preset.join("maps")) {
                for m in maps.filter_map(|m| m.ok()) {
                    let p = m.path();
                    if p.extension().map(|e| e != "json").unwrap_or(true) { continue; }
                    let text = std::fs::read_to_string(&p).unwrap();
                    let def: crate::map::MapDef = serde_json::from_str(&text)
                        .unwrap_or_else(|e| panic!("{name}/maps/{}: {e}",
                            p.file_name().unwrap().to_string_lossy()));
                    for s in &def.spawns.spawn_enemies {
                        map_spawns.push(("enemy".into(), s.kind.clone()));
                    }
                    for s in &def.spawns.spawn_items {
                        map_spawns.push(("item".into(), s.kind.clone()));
                    }
                }
            }

            // ссылочная целостность
            let enemy_ids: HashSet<&str> = enemies.enemies.iter().map(|e| e.id.as_str()).collect();
            let item_ids:  HashSet<&str> = items.items.iter().map(|i| i.id.as_str()).collect();
            let npc_ids:   HashSet<&str> = npcs.iter().map(|n| n.id.as_str()).collect();
            // спец-предметы, спавнящиеся мимо items.json (см. game.rs::spawn_item)
            let special_items: HashSet<&str> = ["heart_1up"].into();

            for q in &quests {
                assert!(npc_ids.contains(q.giver.as_str()),
                    "{name}/quests.json: у квеста '{}' гивер '{}' не найден в npcs.json", q.id, q.giver);
                match q.kind.as_str() {
                    "kill" => assert!(enemy_ids.contains(q.target.as_str()),
                        "{name}/quests.json: цель kill-квеста '{}' ('{}') нет в enemies.json", q.id, q.target),
                    "collect" => assert!(
                        item_ids.contains(q.target.as_str()) || special_items.contains(q.target.as_str()),
                        "{name}/quests.json: цель collect-квеста '{}' ('{}') нет в items.json", q.id, q.target),
                    "clear_dungeon" => {}
                    other => panic!("{name}/quests.json: квест '{}' неизвестного вида '{other}'", q.id),
                }
            }
            for n in &npcs {
                if let Some(q) = &n.quest {
                    assert!(quests.iter().any(|x| &x.id == q),
                        "{name}/npcs.json: NPC '{}' ссылается на квест '{}' — нет в quests.json", n.id, q);
                }
            }

            // диалоги: парс + конвертация + разрешимость ссылок next / npc.scene
            let dialogue_ids: HashSet<String> = match read(&preset, "dialogues.json") {
                None => HashSet::new(),
                Some(t) => {
                    let (scenes, errors) = crate::dialogue::parse_scenes(&t)
                        .unwrap_or_else(|e| panic!("{name}/dialogues.json: {e}"));
                    assert!(errors.is_empty(),
                        "{name}/dialogues.json: битые сцены: {}", errors.join("; "));
                    let ids: HashSet<String> = scenes.iter().map(|s| s.id.clone()).collect();
                    let probe = crate::game_state::GameState::new("test");
                    for s in &scenes {
                        for c in &s.choices {
                            if let Some(next) = &c.next {
                                assert!(ids.contains(next)
                                        || crate::story::get_scene(next, &probe).is_some(),
                                    "{name}/dialogues.json: сцена '{}' ссылается на '{}' — нет ни в JSON, ни в story.rs",
                                    s.id, next);
                            }
                        }
                    }
                    ids
                }
            };
            let probe = crate::game_state::GameState::new("test");
            for n in &npcs {
                if let Some(scene) = &n.scene {
                    if scene.is_empty() || scene == "story" { continue; }
                    assert!(dialogue_ids.contains(scene)
                            || crate::story::get_scene(scene, &probe).is_some(),
                        "{name}/npcs.json: NPC '{}' ссылается на сцену '{}' — нет ни в dialogues.json, ни в story.rs",
                        n.id, scene);
                }
            }
            let check_spawn = |kind_type: &str, id: &str| match kind_type {
                "enemy" => assert!(enemy_ids.contains(id),
                    "{name}: спавн врага '{id}' — нет в enemies.json"),
                _ => assert!(item_ids.contains(id) || special_items.contains(id),
                    "{name}: спавн предмета '{id}' — нет в items.json"),
            };
            for (t, id) in &map_spawns { check_spawn(t, id); }
            if let Some(level) = &level {
                for s in &level.spawn_enemies { check_spawn("enemy", &s.kind); }
                for s in &level.spawn_items   { check_spawn("item",  &s.kind); }
            }
        }
    }

    /// Встроенные копии core обязаны парситься — они последний рубеж фолбэков.
    #[test]
    fn embedded_fallbacks_parse() {
        crate::config::embedded_configs_parse_for_test();
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
