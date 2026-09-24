//! Проверки целостности пресетов: всё ли парсится и ссылается друг на друга.
//!
//! Живут в ядре, потому что здесь же лежат парсеры — тестам не нужен ни движок,
//! ни собранная DLL: `cargo test -p openheart-core`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::config::{EnemiesFile, ItemsFile, LevelCfg, NpcCfg, QuestCfg};

fn presets_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../game/presets")
}

fn read(preset: &Path, file: &str) -> Option<String> {
    std::fs::read_to_string(preset.join(file)).ok()
}

fn must_read(preset: &Path, file: &str) -> String {
    read(preset, file).unwrap_or_else(|| panic!("{}: нет файла {file}", preset.display()))
}

fn parse<T: serde::de::DeserializeOwned>(preset: &Path, file: &str) -> T {
    let text = must_read(preset, file);
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}/{file}: {e}", preset.display()))
}

fn preset_dirs() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(presets_root())
        .expect("нет папки godot/presets")
        .filter_map(|d| d.ok().map(|d| d.path()))
        .filter(|p| p.is_dir())
        .collect();
    out.sort();
    assert!(
        !out.is_empty(),
        "в godot/presets не найдено ни одного пресета"
    );
    out
}

fn assert_quest_graph(preset_name: &str, quests: &[QuestCfg]) {
    let mut ids = HashSet::new();
    let mut world_change_ids = HashSet::new();
    let mut world_change_patterns = HashSet::new();
    for quest in quests {
        assert!(
            ids.insert(quest.id.as_str()),
            "{preset_name}/quests.json: повторяющийся id квеста '{}'",
            quest.id
        );
        assert!(
            quest.count > 0,
            "{preset_name}/quests.json: у квеста '{}' цель должна быть положительной",
            quest.id
        );
        assert!(
            quest.reward_xp > 0 || quest.reward_gold > 0 || !quest.reward_items.is_empty(),
            "{preset_name}/quests.json: у квеста '{}' отсутствует награда",
            quest.id
        );
        for reward in &quest.reward_items {
            assert!(
                reward.qty > 0,
                "{preset_name}/quests.json: quest '{}' has a zero item reward",
                quest.id
            );
        }
        let mut requirements = HashSet::new();
        for required in &quest.requires {
            assert!(
                requirements.insert(required.as_str()),
                "{preset_name}/quests.json: квест '{}' дважды требует '{}'",
                quest.id,
                required
            );
        }
        if let Some(change) = &quest.world_change {
            assert!(
                world_change_ids.insert(change.id.as_str()),
                "{preset_name}/quests.json: duplicate world_change id '{}'",
                change.id
            );
            assert!(
                matches!(
                    change.pattern.as_str(),
                    "beacon" | "garden" | "gallery" | "archive" | "shrine"
                ),
                "{preset_name}/quests.json: quest '{}' has unknown world_change pattern '{}'",
                quest.id,
                change.pattern
            );
            world_change_patterns.insert(change.pattern.as_str());
            assert!(
                change.scale >= 0.5 && change.scale <= 3.0,
                "{preset_name}/quests.json: quest '{}' world_change scale is outside 0.5..=3.0",
                quest.id
            );
            assert!(
                change
                    .color
                    .iter()
                    .all(|channel| (0.0..=1.0).contains(channel)),
                "{preset_name}/quests.json: quest '{}' world_change color is invalid",
                quest.id
            );
            assert!(
                change.sprite.is_empty() || change.sprite.starts_with("res://assets/"),
                "{preset_name}/quests.json: quest '{}' world_change sprite must be an asset path",
                quest.id
            );
            assert!(
                matches!(
                    change.activity.as_str(),
                    "patrol" | "support" | "crowd" | "echo" | "guardian"
                ),
                "{preset_name}/quests.json: quest '{}' has unknown world_change activity '{}'",
                quest.id,
                change.activity
            );
            assert!(
                (2..=8).contains(&change.activity_count),
                "{preset_name}/quests.json: quest '{}' world_change activity_count must be 2..=8",
                quest.id
            );
            assert!(
                (1.0..=8.0).contains(&change.activity_radius)
                    && (0.1..=4.0).contains(&change.activity_speed),
                "{preset_name}/quests.json: quest '{}' world_change activity motion is invalid",
                quest.id
            );
            assert!(
                !quests.iter().any(|candidate| {
                    candidate.chain_en == quest.chain_en
                        && candidate.requires.iter().any(|required| required == &quest.id)
                }),
                "{preset_name}/quests.json: quest '{}' changes the world before its chain finale",
                quest.id
            );
        }
    }

    if preset_name == "core" {
        assert_eq!(
            world_change_ids.len(),
            5,
            "core/quests.json: expected one world change for each of five quest chains"
        );
        assert_eq!(
            world_change_patterns.len(),
            5,
            "core/quests.json: quest chain world changes must use five distinct patterns"
        );
    }

    let mut reachable = HashSet::new();
    loop {
        let before = reachable.len();
        for quest in quests {
            if quest
                .requires
                .iter()
                .all(|required| reachable.contains(required.as_str()))
            {
                reachable.insert(quest.id.as_str());
            }
        }
        if reachable.len() == quests.len() {
            break;
        }
        assert_ne!(
            reachable.len(),
            before,
            "{preset_name}/quests.json: цикл зависимостей блокирует квесты: {}",
            quests
                .iter()
                .filter(|quest| !reachable.contains(quest.id.as_str()))
                .map(|quest| quest.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
}

#[test]
fn json_and_ron_sources_do_not_drift() {
    for preset in preset_dirs() {
        for entry in std::fs::read_dir(&preset)
            .unwrap_or_else(|error| panic!("{}: {error}", preset.display()))
            .filter_map(Result::ok)
        {
            let json_path = entry.path();
            if json_path
                .extension()
                .and_then(|extension| extension.to_str())
                != Some("json")
            {
                continue;
            }
            let ron_path = json_path.with_extension("ron");
            if !ron_path.exists() {
                continue;
            }
            let json_text = std::fs::read_to_string(&json_path).unwrap();
            let ron_text = std::fs::read_to_string(&ron_path).unwrap();
            let json_value: serde_json::Value = serde_json::from_str(&json_text)
                .unwrap_or_else(|error| panic!("{}: {error}", json_path.display()));
            let ron_value: serde_json::Value = ron::from_str(&ron_text)
                .unwrap_or_else(|error| panic!("{}: {error}", ron_path.display()));
            assert_eq!(
                ron_value,
                json_value,
                "{} and {} contain different effective content",
                ron_path.display(),
                json_path.display()
            );
        }
    }
}

#[test]
fn all_presets_parse_and_link() {
    for preset in preset_dirs() {
        let name = preset.file_name().unwrap().to_string_lossy().to_string();

        // форматы: любой битый файл валит тест с именем пресета и ошибкой serde
        let info: crate::data::preset::PresetInfo = parse(&preset, "preset.json");
        let weapons_parsed = crate::weapon::parse(&must_read(&preset, "weapons.json"))
            .unwrap_or_else(|e| panic!("{name}/weapons.json: {e}"));
        let weapon_mods_parsed = read(&preset, "weapon_mods.json")
            .map(|text| {
                crate::weapon::parse_mods(&text)
                    .unwrap_or_else(|error| panic!("{name}/weapon_mods.json: {error}"))
            })
            .unwrap_or_default();
        crate::classes::parse(&must_read(&preset, "classes.json"))
            .unwrap_or_else(|e| panic!("{name}/classes.json: {e}"));
        crate::perk::parse_perks(&must_read(&preset, "perks.json"))
            .unwrap_or_else(|e| panic!("{name}/perks.json: {e}"));
        crate::perk::parse_syn(&must_read(&preset, "synergies.json"))
            .unwrap_or_else(|e| panic!("{name}/synergies.json: {e}"));

        let enemies: EnemiesFile = parse(&preset, "enemies.json");
        let items: ItemsFile = parse(&preset, "items.json");
        let level: Option<LevelCfg> = read(&preset, "level.json").map(|t| {
            serde_json::from_str(&t).unwrap_or_else(|e| panic!("{name}/level.json: {e}"))
        });
        let npcs: Vec<NpcCfg> = read(&preset, "npcs.json")
            .map(|t| {
                serde_json::from_str(&t).unwrap_or_else(|e| panic!("{name}/npcs.json: {e}"))
            })
            .unwrap_or_default();
        let quests: Vec<QuestCfg> = read(&preset, "quests.json")
            .map(|t| {
                serde_json::from_str(&t).unwrap_or_else(|e| panic!("{name}/quests.json: {e}"))
            })
            .unwrap_or_default();
        assert_quest_graph(&name, &quests);
        let dialogues: Vec<crate::dialogue::Scene> = read(&preset, "dialogues.json")
            .map(|text| {
                let (scenes, warnings) = crate::dialogue::parse_scenes(&text)
                    .unwrap_or_else(|error| panic!("{name}/dialogues.json: {error}"));
                assert!(
                    warnings.is_empty(),
                    "{name}/dialogues.json: {}",
                    warnings.join("; ")
                );
                scenes
            })
            .unwrap_or_default();

        if info.enabled {
            assert_eq!(
                weapon_mods_parsed.len(),
                16,
                "{name}/weapon_mods.json: expected two branches for eight weapons"
            );
            assert!(
                !info.name_en.is_empty() && !info.desc_en.is_empty(),
                "{name}/preset.json: включённый пресет должен иметь name_en и desc_en"
            );
            let mut weapon_feedback_profiles = HashSet::new();
            let mut weapon_audio_profiles = HashSet::new();
            let mut weapon_accuracy_profiles = HashSet::new();
            let mut weapon_alt_names = HashSet::new();
            for weapon in &weapons_parsed {
                assert!(
                    weapon.switch_frames.len() >= 2,
                    "{name}/weapons.json: оружие '{}' должно иметь отдельную switch-анимацию",
                    weapon.name_ru
                );
                assert_ne!(
                    weapon.switch_frames, weapon.idle_frames,
                    "{name}/weapons.json: switch-анимация оружия '{}' совпадает с idle",
                    weapon.name_ru
                );
                assert_ne!(
                    weapon.switch_frames, weapon.fire_frames,
                    "{name}/weapons.json: switch-анимация оружия '{}' совпадает с fire",
                    weapon.name_ru
                );
                let feedback = weapon.feedback;
                assert!(
                    feedback.muzzle_energy > 0.0
                        && feedback.muzzle_duration > 0.0
                        && feedback.impact_scale > 0.0
                        && feedback.impact_energy > 0.0
                        && feedback.impact_duration > 0.0
                        && feedback.tracer_scale > 0.0
                        && feedback.tracer_duration > 0.0,
                    "{name}/weapons.json: weapon '{}' has incomplete feedback",
                    weapon.name_en
                );
                weapon_feedback_profiles.insert(format!(
                    "{:?}:{:.2}:{:?}:{:.2}:{:?}:{:.2}",
                    feedback.muzzle_color,
                    feedback.muzzle_energy,
                    feedback.impact_color,
                    feedback.impact_scale,
                    feedback.tracer_color,
                    feedback.tracer_scale,
                ));
                let audio = &weapon.audio;
                assert!(
                    !audio.fire_sfx.is_empty()
                        && !audio.impact_sfx.is_empty()
                        && (weapon.magazine == 0 || !audio.reload_sfx.is_empty()),
                    "{name}/weapons.json: weapon '{}' has incomplete audio",
                    weapon.name_en
                );
                weapon_audio_profiles.insert(format!(
                    "{:?}:{:?}:{:?}:{:?}:{:?}:{:.1}:{:.1}:{:.1}",
                    audio.fire_sfx,
                    audio.impact_sfx,
                    audio.fire_pitch,
                    audio.impact_pitch,
                    audio.reload_pitch,
                    audio.fire_volume_db,
                    audio.impact_volume_db,
                    audio.reload_volume_db,
                ));
                let accuracy = weapon.accuracy;
                weapon_accuracy_profiles.insert(format!(
                    "{:.2}:{:.2}:{:.2}:{:.2}:{:.2}",
                    accuracy.bloom_per_shot,
                    accuracy.max_bloom,
                    accuracy.recovery,
                    accuracy.move_penalty,
                    accuracy.crosshair_scale,
                ));
                let alt = weapon.alt_fire.as_ref().unwrap_or_else(|| {
                    panic!(
                        "{name}/weapons.json: weapon '{}' needs alt_fire",
                        weapon.name_en
                    )
                });
                assert!(
                    weapon_alt_names.insert(alt.name_en.clone()) && !alt.name_ru.is_empty(),
                    "{name}/weapons.json: alt-fire names must be unique and localized"
                );
            }
            assert_eq!(
                weapon_feedback_profiles.len(),
                weapons_parsed.len(),
                "{name}/weapons.json: every weapon needs a unique feedback profile"
            );
            assert_eq!(
                weapon_audio_profiles.len(),
                weapons_parsed.len(),
                "{name}/weapons.json: every weapon needs a unique audio profile"
            );
            assert_eq!(
                weapon_accuracy_profiles.len(),
                weapons_parsed.len(),
                "{name}/weapons.json: every weapon needs a unique accuracy profile"
            );
            assert_eq!(
                weapon_alt_names.len(),
                weapons_parsed.len(),
                "{name}/weapons.json: every weapon needs a unique alt-fire mode"
            );
            for enemy in &enemies.enemies {
                assert!(
                    !enemy.name_en.is_empty(),
                    "{name}/enemies.json: у врага '{}' отсутствует name_en",
                    enemy.id
                );
                let animation = &enemy.animation;
                for (state, frames) in [
                    ("idle", animation.idle_frames),
                    ("move", animation.move_frames),
                    ("attack", animation.attack_frames),
                    ("pain", animation.pain_frames),
                    ("alert", animation.alert_frames),
                    ("cast", animation.cast_frames),
                    ("death", animation.death_frames),
                ] {
                    assert!(
                        (1..=8).contains(&frames),
                        "{name}/enemies.json: '{}' has invalid {state}_frames={frames}",
                        enemy.id
                    );
                }
                for (state, row) in [
                    ("idle", animation.idle_row),
                    ("move", animation.move_row),
                    ("move_left", animation.move_left_row),
                    ("move_right", animation.move_right_row),
                    ("attack", animation.attack_row),
                    ("pain", animation.pain_row),
                    ("alert", animation.alert_row),
                    ("cast", animation.cast_row),
                    ("death", animation.death_row),
                ] {
                    assert!(
                        row < 8,
                        "{name}/enemies.json: '{}' has invalid {state}_row={row}",
                        enemy.id
                    );
                }
                assert!(
                    animation.idle_fps > 0.0
                        && animation.action_fps > 0.0
                        && animation.attack_fps() > 0.0
                        && animation.pain_fps() > 0.0
                        && animation.cast_fps() > 0.0
                        && animation.death_fps() > 0.0
                        && (0.1..=0.9).contains(&animation.attack_hit_ratio),
                    "{name}/enemies.json: '{}' has invalid animation timing",
                    enemy.id
                );
                assert_ne!(
                    animation.idle_row, animation.attack_row,
                    "{name}/enemies.json: '{}' attack animation matches idle row",
                    enemy.id
                );
                assert_ne!(
                    animation.idle_row, animation.pain_row,
                    "{name}/enemies.json: '{}' pain animation matches idle row",
                    enemy.id
                );
                assert_ne!(
                    animation.idle_row, animation.death_row,
                    "{name}/enemies.json: '{}' death animation matches idle row",
                    enemy.id
                );
            }
            let tactical_roles: HashSet<&str> = enemies
                .enemies
                .iter()
                .map(|enemy| enemy.role.as_str())
                .collect();
            for required_role in [
                "pursuer",
                "tank",
                "artillery",
                "support",
                "summoner",
                "controller",
                "commander",
            ] {
                assert!(
                    tactical_roles.contains(required_role),
                    "{name}/enemies.json: production-ростер не покрывает роль '{required_role}'"
                );
            }
            for item in &items.items {
                assert!(
                    !item.name_en.is_empty() && !item.desc_en.is_empty(),
                    "{name}/items.json: у предмета '{}' отсутствует английский перевод",
                    item.id
                );
            }
            for npc in &npcs {
                assert!(
                    !npc.name_en.is_empty(),
                    "{name}/npcs.json: у NPC '{}' отсутствует name_en",
                    npc.id
                );
            }
            for quest in &quests {
                assert!(
                    !quest.title_en.is_empty() && !quest.desc_en.is_empty(),
                    "{name}/quests.json: у квеста '{}' отсутствует английский перевод",
                    quest.id
                );
            }
        }

        // карты
        let mut map_spawns: Vec<(String, String)> = Vec::new(); // (kind_type, id)
        if let Ok(maps) = std::fs::read_dir(preset.join("maps")) {
            for m in maps.filter_map(|m| m.ok()) {
                let p = m.path();
                if p.extension().map(|e| e != "json").unwrap_or(true) {
                    continue;
                }
                let text = std::fs::read_to_string(&p).unwrap();
                let def: crate::map::MapDef = serde_json::from_str(&text).unwrap_or_else(|e| {
                    panic!(
                        "{name}/maps/{}: {e}",
                        p.file_name().unwrap().to_string_lossy()
                    )
                });
                let mut district_ids = HashSet::new();
                for district in &def.districts {
                    assert!(
                        district_ids.insert(district.id.as_str()),
                        "{name}/maps/{}: повторяющийся id района '{}'",
                        p.file_name().unwrap().to_string_lossy(),
                        district.id
                    );
                    assert!(
                        district.radius >= 8.0,
                        "{name}/maps/{}: радиус района '{}' слишком мал",
                        p.file_name().unwrap().to_string_lossy(),
                        district.id
                    );
                    assert!(
                        !district.name_ru.is_empty() && !district.name_en.is_empty(),
                        "{name}/maps/{}: район '{}' должен иметь RU/EN название",
                        p.file_name().unwrap().to_string_lossy(),
                        district.id
                    );
                    assert!(
                        district.landmark.is_some(),
                        "{name}/maps/{}: район '{}' не имеет landmark",
                        p.file_name().unwrap().to_string_lossy(),
                        district.id
                    );
                    assert!(
                        district
                            .color
                            .iter()
                            .all(|channel| (0.0..=1.0).contains(channel)),
                        "{name}/maps/{}: цвет района '{}' вне диапазона 0..1",
                        p.file_name().unwrap().to_string_lossy(),
                        district.id
                    );
                }
                let mut route_ids = HashSet::new();
                for route in &def.route_layers {
                    assert!(
                        route_ids.insert(route.id.as_str())
                            && route.points.len() >= 2
                            && (0.25..=4.0).contains(&route.width)
                            && route
                                .color
                                .iter()
                                .all(|channel| (0.0..=1.0).contains(channel)),
                        "{name}/maps/{}: invalid route layer '{}'",
                        p.file_name().unwrap().to_string_lossy(),
                        route.id
                    );
                }
                let mut beacon_ids = HashSet::new();
                for beacon in &def.skyline_beacons {
                    assert!(
                        beacon_ids.insert(beacon.id.as_str())
                            && (3.0..=30.0).contains(&beacon.height)
                            && beacon
                                .color
                                .iter()
                                .all(|channel| (0.0..=1.0).contains(channel)),
                        "{name}/maps/{}: invalid skyline beacon '{}'",
                        p.file_name().unwrap().to_string_lossy(),
                        beacon.id
                    );
                }
                if info.enabled && def.id == "hub" {
                    assert!(
                        def.districts.len() >= 6,
                        "{name}/maps/hub.json: стартовый мир должен иметь минимум шесть районов"
                    );
                    assert!(
                        def.route_layers.len() >= 8,
                        "{name}/maps/hub.json: hub needs at least eight readable route layers"
                    );
                    assert!(
                        def.skyline_beacons.len() >= 10,
                        "{name}/maps/hub.json: hub needs at least ten skyline beacons"
                    );
                }
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
        let item_ids: HashSet<&str> = items.items.iter().map(|i| i.id.as_str()).collect();
        let npc_ids: HashSet<&str> = npcs.iter().map(|n| n.id.as_str()).collect();
        let dialogue_ids: HashSet<&str> =
            dialogues.iter().map(|scene| scene.id.as_str()).collect();
        for npc in npcs
            .iter()
            .filter(|npc| npc.scene.as_deref() == Some("story"))
        {
            for suffix in ["intro", "repeat"] {
                let scene_id = format!("hub_{}_{}", npc.id, suffix);
                assert!(
                    dialogue_ids.contains(scene_id.as_str()),
                    "{name}/dialogues.json: для story-NPC '{}' отсутствует сцена '{}'",
                    npc.id,
                    scene_id
                );
            }
        }
        for quest in &quests {
            if let Some(giver) = npcs.iter().find(|npc| npc.id == quest.giver) {
                assert_ne!(
                    giver.scene.as_deref(),
                    Some("story"),
                    "{name}/npcs.json: story-сцена NPC '{}' блокирует квест '{}'",
                    giver.id,
                    quest.id
                );
            }
        }
        // спец-предметы, спавнящиеся мимо items.json (см. game.rs::spawn_item)
        let special_items: HashSet<&str> = ["heart_1up"].into();

        for enemy in &enemies.enemies {
            assert!(
                matches!(
                    enemy.role.as_str(),
                    "pursuer"
                        | "tank"
                        | "artillery"
                        | "support"
                        | "summoner"
                        | "controller"
                        | "commander"
                ),
                "{name}/enemies.json: у врага '{}' неизвестная тактическая роль '{}'",
                enemy.id,
                enemy.role
            );
            let animation = &enemy.animation;
            assert!(
                animation.idle_fps > 0.0 && animation.action_fps > 0.0,
                "{name}/enemies.json: у врага '{}' FPS анимации должен быть положительным",
                enemy.id
            );
            for (state, frames) in [
                ("idle", animation.idle_frames),
                ("move", animation.move_frames),
                ("attack", animation.attack_frames),
                ("pain", animation.pain_frames),
                ("alert", animation.alert_frames),
                ("death", animation.death_frames),
            ] {
                assert!(
                    (1..=8).contains(&frames),
                    "{name}/enemies.json: у врага '{}' состояние {state} должно иметь 1–8 кадров",
                    enemy.id
                );
            }
            for row in [
                animation.idle_row,
                animation.move_row,
                animation.move_left_row,
                animation.move_right_row,
                animation.attack_row,
                animation.pain_row,
                animation.alert_row,
                animation.death_row,
            ] {
                assert!(
                    row < 8,
                    "{name}/enemies.json: у врага '{}' строка анимации должна быть 0–7",
                    enemy.id
                );
            }
        }

        for q in &quests {
            assert!(
                npc_ids.contains(q.giver.as_str()),
                "{name}/quests.json: у квеста '{}' гивер '{}' не найден в npcs.json",
                q.id,
                q.giver
            );
            match q.kind.as_str() {
                "kill" => assert!(
                    enemy_ids.contains(q.target.as_str()),
                    "{name}/quests.json: цель kill-квеста '{}' ('{}') нет в enemies.json",
                    q.id,
                    q.target
                ),
                "collect" => assert!(
                    item_ids.contains(q.target.as_str())
                        || special_items.contains(q.target.as_str()),
                    "{name}/quests.json: цель collect-квеста '{}' ('{}') нет в items.json",
                    q.id,
                    q.target
                ),
                "kill_any" => {}
                "collect_category" => assert!(
                    items.items.iter().any(|item| item.category == q.target),
                    "{name}/quests.json: категория collect-квеста '{}' ('{}') не найдена",
                    q.id,
                    q.target
                ),
                "interact" => assert!(
                    npc_ids.contains(q.target.as_str()),
                    "{name}/quests.json: interact target '{}' for quest '{}' is missing",
                    q.target,
                    q.id
                ),
                "boss" => assert!(
                    enemy_ids.contains(q.target.as_str()),
                    "{name}/quests.json: boss target '{}' for quest '{}' is missing",
                    q.target,
                    q.id
                ),
                "discover_weapon" => assert!(
                    q.target == "*" || crate::weapon::WeaponId::from_id(&q.target).is_some(),
                    "{name}/quests.json: weapon target '{}' for quest '{}' is unknown",
                    q.target,
                    q.id
                ),
                "clear_dungeon" | "enter_dungeon" => {}
                "solve_puzzle" | "discover_lore" => assert!(
                    q.target == "*" || q.target.parse::<u32>().is_ok(),
                    "{name}/quests.json: event target '{}' for quest '{}' must be '*' or a depth",
                    q.target,
                    q.id
                ),
                other => panic!(
                    "{name}/quests.json: квест '{}' неизвестного вида '{other}'",
                    q.id
                ),
            }
            assert!(
                !q.chain_ru.is_empty() && !q.chain_en.is_empty() && q.stage > 0,
                "{name}/quests.json: quest '{}' lacks localized chain metadata",
                q.id
            );
            for reward in &q.reward_items {
                assert!(
                    item_ids.contains(reward.id.as_str()),
                    "{name}/quests.json: item reward '{}' for quest '{}' is missing",
                    reward.id,
                    q.id
                );
            }
            for required in &q.requires {
                assert!(
                    quests.iter().any(|candidate| &candidate.id == required),
                    "{name}/quests.json: квест '{}' требует неизвестный квест '{}'",
                    q.id,
                    required
                );
                assert_ne!(
                    &q.id, required,
                    "{name}/quests.json: квест '{}' не может требовать сам себя",
                    q.id
                );
            }
            for (stage, scene_id) in [
                ("offer", q.offer_scene.as_deref()),
                ("progress", q.progress_scene.as_deref()),
                ("complete", q.complete_scene.as_deref()),
            ] {
                if let Some(scene_id) = scene_id {
                    let scene = dialogues.iter().find(|scene| scene.id == scene_id);
                    assert!(
                        scene.is_some(),
                        "{name}/quests.json: у квеста '{}' {stage}_scene '{}' отсутствует в dialogues.json",
                        q.id,
                        scene_id
                    );
                    let scene = scene.unwrap();
                    match stage {
                        "offer" => assert!(
                            scene.choices.iter().any(|choice| {
                                choice.effects.iter().any(|effect| {
                                    matches!(
                                        effect,
                                        crate::dialogue::Effect::Quest { id, .. } if id == &q.id
                                    )
                                })
                            }),
                            "{name}/dialogues.json: offer-сцена '{}' не выдаёт квест '{}'",
                            scene_id,
                            q.id
                        ),
                        "progress" => assert!(
                            scene.choices.iter().all(|choice| {
                                choice.effects.iter().all(|effect| {
                                    !matches!(
                                        effect,
                                        crate::dialogue::Effect::Quest { .. }
                                            | crate::dialogue::Effect::QuestDone(_)
                                            | crate::dialogue::Effect::Xp(_)
                                            | crate::dialogue::Effect::Gold(_)
                                            | crate::dialogue::Effect::Item { .. }
                                    )
                                })
                            }),
                            "{name}/dialogues.json: progress-сцена '{}' преждевременно меняет квест или награду",
                            scene_id
                        ),
                        "complete" => assert!(
                            scene.choices.iter().any(|choice| {
                                let done = choice.effects.iter().any(|effect| {
                                    matches!(
                                        effect,
                                        crate::dialogue::Effect::QuestDone(id) if id == &q.id
                                    )
                                });
                                let xp: u32 = choice
                                    .effects
                                    .iter()
                                    .filter_map(|effect| match effect {
                                        crate::dialogue::Effect::Xp(value) => Some(*value),
                                        _ => None,
                                    })
                                    .sum();
                                let gold: i32 = choice
                                    .effects
                                    .iter()
                                    .filter_map(|effect| match effect {
                                        crate::dialogue::Effect::Gold(value) => Some(*value),
                                        _ => None,
                                    })
                                    .sum();
                                done && xp == q.reward_xp && gold == q.reward_gold
                            }),
                            "{name}/dialogues.json: complete-сцена '{}' не закрывает квест '{}' с наградой {} XP / {} зол.",
                            scene_id,
                            q.id,
                            q.reward_xp,
                            q.reward_gold
                        ),
                        _ => unreachable!(),
                    }
                }
            }
        }
        for n in &npcs {
            if let Some(q) = &n.quest {
                assert!(
                    quests.iter().any(|x| &x.id == q),
                    "{name}/npcs.json: NPC '{}' ссылается на квест '{}' — нет в quests.json",
                    n.id,
                    q
                );
            }
        }

        for e in &enemies.enemies {
            if let Some(b) = &e.behavior {
                assert!(
                    b == "melee" || b == "ranged",
                    "{name}/enemies.json: враг '{}' — неизвестное behavior '{b}'",
                    e.id
                );
            }
            assert!(
                (0.5..=0.95).contains(&e.weak_point.height),
                "{name}/enemies.json: enemy '{}' weak-point height must be within 0.5..0.95",
                e.id
            );
            assert!(
                (1.0..=3.0).contains(&e.weak_point.multiplier),
                "{name}/enemies.json: enemy '{}' weak-point multiplier must be within 1.0..3.0",
                e.id
            );
        }

        // статусы: парс + виды (нет файла — рантайм берёт core)
        let status_src = read(&preset, "statuses.json").or_else(|| {
            std::fs::read_to_string(presets_root().join("core/statuses.json")).ok()
        });
        let mut status_ids: HashSet<String> = HashSet::new();
        if let Some(t) = status_src {
            let ss: Vec<crate::config::StatusCfg> = serde_json::from_str(&t)
                .unwrap_or_else(|e| panic!("{name}/statuses.json: {e}"));
            for s in &ss {
                assert!(
                    matches!(s.kind.as_str(), "dot" | "slow" | "stun" | "vulnerable"),
                    "{name}/statuses.json: '{}' — неизвестный kind '{}'",
                    s.id,
                    s.kind
                );
                status_ids.insert(s.id.clone());
            }
        }
        // ссылки на статусы: enemies.attack_status + weapons.status
        for e in &enemies.enemies {
            if let Some(s) = &e.attack_status {
                assert!(
                    status_ids.contains(&s.id),
                    "{name}/enemies.json: враг '{}' — статус '{}' не найден",
                    e.id,
                    s.id
                );
            }
        }
        for w in &weapons_parsed {
            if let Some((id, _)) = &w.status {
                assert!(
                    status_ids.contains(id),
                    "{name}/weapons.json: оружие '{}' — статус '{id}' не найден",
                    w.name_ru
                );
            }
        }

        // способности: парс + виды + ссылки (нет файла — рантайм берёт core)
        let ability_src = read(&preset, "abilities.json").or_else(|| {
            std::fs::read_to_string(presets_root().join("core/abilities.json")).ok()
        });
        let mut ability_ids: HashSet<String> = HashSet::new();
        if let Some(t) = ability_src {
            let abs: Vec<crate::config::AbilityCfg> = serde_json::from_str(&t)
                .unwrap_or_else(|e| panic!("{name}/abilities.json: {e}"));
            for a in &abs {
                assert!(
                    matches!(
                        a.kind.as_str(),
                        "projectile_burst" | "charge" | "summon" | "heal_pulse"
                    ),
                    "{name}/abilities.json: '{}' — неизвестный kind '{}'",
                    a.id,
                    a.kind
                );
                if a.kind == "summon" {
                    let m = a.minion.as_deref().unwrap_or("");
                    assert!(
                        enemy_ids.contains(m),
                        "{name}/abilities.json: '{}' — миньон '{m}' не найден во врагах",
                        a.id
                    );
                }
                if let Some(s) = &a.status {
                    assert!(
                        status_ids.contains(&s.id),
                        "{name}/abilities.json: '{}' — статус '{}' не найден",
                        a.id,
                        s.id
                    );
                }
                ability_ids.insert(a.id.clone());
            }
        }
        for e in &enemies.enemies {
            for ab in &e.abilities {
                assert!(
                    ability_ids.contains(ab),
                    "{name}/enemies.json: враг '{}' — способность '{ab}' не найдена",
                    e.id
                );
            }
            for phase in &e.phases {
                assert!(
                    (0.0..=1.0).contains(&phase.threshold),
                    "{name}/enemies.json: враг '{}' — порог фазы вне диапазона 0..1",
                    e.id
                );
                assert!(
                    phase.damage_mult > 0.0 && phase.speed_mult > 0.0,
                    "{name}/enemies.json: враг '{}' — множитель фазы должен быть положительным",
                    e.id
                );
                for ab in &phase.abilities {
                    assert!(
                        ability_ids.contains(ab),
                        "{name}/enemies.json: враг '{}' — способность фазы '{ab}' не найдена",
                        e.id
                    );
                }
            }
        }

        // аффиксы: парс (нет файла — рантайм берёт core)
        if let Some(t) = read(&preset, "affixes.json") {
            let afs: Vec<crate::config::AffixCfg> =
                serde_json::from_str(&t).unwrap_or_else(|e| panic!("{name}/affixes.json: {e}"));
            for a in &afs {
                assert!(
                    a.hp_mult > 0.0 && a.speed_mult > 0.0,
                    "{name}/affixes.json: '{}' — неположительный множитель",
                    a.id
                );
            }
        }

        // генерация данжей: парс + ссылки на врагов/предметы/оружие + текстуры тем
        if let Some(t) = read(&preset, "dungeon.json") {
            let d: crate::config::DungeonCfg =
                serde_json::from_str(&t).unwrap_or_else(|e| panic!("{name}/dungeon.json: {e}"));
            for room in &d.room_archetypes {
                assert!(
                    matches!(room.shape.as_str(), "rect" | "round" | "octagon" | "cross"),
                    "{name}/dungeon.json: неизвестная форма комнаты '{}'",
                    room.shape
                );
                assert!(
                    matches!(
                        room.role.as_str(),
                        "" | "safe"
                            | "arena"
                            | "gallery"
                            | "ritual"
                            | "treasure"
                            | "ambush"
                            | "traversal"
                            | "puzzle"
                            | "story"
                            | "antechamber"
                    ),
                    "{name}/dungeon.json: неизвестная роль комнаты '{}'",
                    room.role
                );
                assert!(
                    matches!(
                        room.decor.as_str(),
                        "" | "pipes"
                            | "ossuary"
                            | "crystals"
                            | "machinery"
                            | "archive"
                            | "sanctuary"
                    ),
                    "{name}/dungeon.json: неизвестный декор комнаты '{}'",
                    room.decor
                );
                assert!(
                    room.weight > 0 && room.min_size >= 4 && room.max_size >= room.min_size,
                    "{name}/dungeon.json: некорректный архетип комнаты '{}'",
                    room.id
                );
                assert!(
                    (0.0..=1.0).contains(&room.hazard_chance)
                        && room.hazard_dps >= 0.0
                        && room.hazard_radius >= 0.0,
                    "{name}/dungeon.json: некорректная ловушка комнаты '{}'",
                    room.id
                );
                assert!(
                    matches!(
                        room.hazard_kind.as_str(),
                        "" | "blood" | "void" | "electric" | "embers"
                    ),
                    "{name}/dungeon.json: неизвестный тип ловушки '{}' в комнате '{}'",
                    room.hazard_kind,
                    room.id
                );
                assert!(
                    (0.0..=1.0).contains(&room.event_chance),
                    "{name}/dungeon.json: шанс события комнаты '{}' вне диапазона",
                    room.id
                );
                assert!(
                    matches!(
                        room.focal_pattern.as_str(),
                        "spire"
                            | "ring"
                            | "cross"
                            | "aisle"
                            | "altar"
                            | "well"
                            | "archive"
                            | "gate"
                    ) && room
                        .focal_color
                        .iter()
                        .all(|channel| (0.0..=1.0).contains(channel))
                        && (0.55..=1.8).contains(&room.focal_scale)
                        && (0.5..=4.0).contains(&room.focal_height),
                    "{name}/dungeon.json: invalid focal setpiece for room '{}'",
                    room.id
                );
            }
            if info.enabled {
                assert!(
                    d.room_archetypes.len() >= 16,
                    "{name}/dungeon.json: основной пресет должен иметь не менее 16 архетипов комнат"
                );
                let focal_patterns: HashSet<_> = d
                    .room_archetypes
                    .iter()
                    .map(|room| room.focal_pattern.as_str())
                    .collect();
                assert_eq!(
                    focal_patterns.len(),
                    8,
                    "{name}/dungeon.json: core rooms must cover all eight focal patterns"
                );
                let focal_profiles: HashSet<_> = d
                    .room_archetypes
                    .iter()
                    .map(|room| {
                        format!(
                            "{}:{:?}:{:.2}:{:.2}",
                            room.focal_pattern,
                            room.focal_color,
                            room.focal_scale,
                            room.focal_height
                        )
                    })
                    .collect();
                assert_eq!(
                    focal_profiles.len(),
                    d.room_archetypes.len(),
                    "{name}/dungeon.json: every room archetype needs a unique focal profile"
                );
                for required in [
                    "safe",
                    "arena",
                    "gallery",
                    "ritual",
                    "treasure",
                    "ambush",
                    "traversal",
                    "puzzle",
                    "story",
                    "antechamber",
                ] {
                    assert!(
                        d.room_archetypes.iter().any(|room| room.role == required),
                        "{name}/dungeon.json: отсутствует обязательная роль комнаты '{required}'"
                    );
                }
            }
            let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../game");
            for th in &d.themes {
                for tex in [&th.wall, &th.accent, &th.floor, &th.ceil, &th.lava] {
                    let res = if tex.starts_with("res://") {
                        tex.clone()
                    } else {
                        crate::map::tex_path(tex)
                    };
                    let fs = assets.join(res.trim_start_matches("res://"));
                    assert!(
                        fs.exists(),
                        "{name}/dungeon.json: тема '{}' — текстуры {} нет ({})",
                        th.name_ru,
                        tex,
                        fs.display()
                    );
                }
            }
            for p in &d.pools {
                for e in &p.enemies {
                    assert!(
                        enemy_ids.contains(e.as_str()),
                        "{name}/dungeon.json: пул min_depth={} — враг '{e}' не найден",
                        p.min_depth
                    );
                }
            }
            assert!(
                enemy_ids.contains(d.settings.boss.as_str()),
                "{name}/dungeon.json: босс '{}' не найден во врагах",
                d.settings.boss
            );
            for g in &d.settings.boss_guards {
                assert!(
                    enemy_ids.contains(g.as_str()),
                    "{name}/dungeon.json: страж босса '{g}' не найден"
                );
            }
            let mut boss_depths = HashSet::new();
            for tier in &d.settings.boss_roster {
                assert!(
                    tier.min_depth > 0 && boss_depths.insert(tier.min_depth),
                    "{name}/dungeon.json: повторяющийся или нулевой min_depth {} в boss_roster",
                    tier.min_depth
                );
                assert!(
                    tier.mult > 0.0,
                    "{name}/dungeon.json: у босса '{}' неположительный множитель",
                    tier.boss
                );
                let boss = enemies
                    .enemies
                    .iter()
                    .find(|enemy| enemy.id == tier.boss)
                    .unwrap_or_else(|| {
                        panic!(
                            "{name}/dungeon.json: босс ростера '{}' не найден",
                            tier.boss
                        )
                    });
                if info.enabled {
                    assert!(
                        boss.phases.len() >= 2,
                        "{name}/enemies.json: боссу ростера '{}' нужны минимум две фазы",
                        boss.id
                    );
                }
                for guard in &tier.guards {
                    assert!(
                        enemy_ids.contains(guard.as_str()),
                        "{name}/dungeon.json: страж ростера '{guard}' не найден"
                    );
                }
                for item in &tier.items {
                    assert!(
                        item_ids.contains(item.as_str())
                            || special_items.contains(item.as_str()),
                        "{name}/dungeon.json: награда ростера '{item}' не найдена"
                    );
                }
            }
            if info.enabled {
                assert!(
                    d.settings.boss_roster.len() >= 4
                        && d.settings
                            .boss_roster
                            .iter()
                            .any(|tier| tier.min_depth == 1),
                    "{name}/dungeon.json: нужны четыре уровня боссов начиная с глубины 1"
                );
            }
            for it in &d.settings.boss_items {
                assert!(
                    item_ids.contains(it.as_str()) || special_items.contains(it.as_str()),
                    "{name}/dungeon.json: награда босса '{it}' не найдена в предметах"
                );
            }
            for w in &d.settings.weapon_cache {
                assert!(
                    crate::weapon::WeaponId::from_id(w).is_some(),
                    "{name}/dungeon.json: неизвестное оружие '{w}' в weapon_cache"
                );
            }
        }

        // лут: парс + ссылки + сумма шансов дропа
        if let Some(t) = read(&preset, "loot.json") {
            let l: crate::config::LootCfg =
                serde_json::from_str(&t).unwrap_or_else(|e| panic!("{name}/loot.json: {e}"));
            for e in &l.room_items {
                assert!(
                    item_ids.contains(e.id.as_str()) || special_items.contains(e.id.as_str()),
                    "{name}/loot.json: room_items '{}' не найден в предметах",
                    e.id
                );
            }
            let mut sum = 0.0f32;
            for d in &l.kill_drops {
                sum += d.chance;
                match d.kind.as_str() {
                    "ammo" => {}
                    "item" => {
                        let id = d.id.as_deref().unwrap_or("");
                        assert!(
                            item_ids.contains(id) || special_items.contains(id),
                            "{name}/loot.json: kill_drops item '{id}' не найден"
                        );
                    }
                    other => panic!("{name}/loot.json: kill_drops неизвестный kind '{other}'"),
                }
            }
            assert!(
                sum <= 1.0 + f32::EPSILON,
                "{name}/loot.json: сумма chance у kill_drops {sum} > 1.0"
            );
        }

        // диалоги: парс + конвертация + разрешимость ссылок next / npc.scene
        let dialogue_ids: HashSet<String> = match read(&preset, "dialogues.json") {
            None => HashSet::new(),
            Some(t) => {
                let (scenes, errors) = crate::dialogue::parse_scenes(&t)
                    .unwrap_or_else(|e| panic!("{name}/dialogues.json: {e}"));
                assert!(
                    errors.is_empty(),
                    "{name}/dialogues.json: битые сцены: {}",
                    errors.join("; ")
                );
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
                if scene.is_empty() || scene == "story" {
                    continue;
                }
                assert!(dialogue_ids.contains(scene)
                        || crate::story::get_scene(scene, &probe).is_some(),
                    "{name}/npcs.json: NPC '{}' ссылается на сцену '{}' — нет ни в dialogues.json, ни в story.rs",
                    n.id, scene);
            }
        }
        let check_spawn = |kind_type: &str, id: &str| match kind_type {
            "enemy" => assert!(
                enemy_ids.contains(id),
                "{name}: спавн врага '{id}' — нет в enemies.json"
            ),
            _ => assert!(
                item_ids.contains(id) || special_items.contains(id),
                "{name}: спавн предмета '{id}' — нет в items.json"
            ),
        };
        for (t, id) in &map_spawns {
            check_spawn(t, id);
        }
        if let Some(level) = &level {
            for s in &level.spawn_enemies {
                check_spawn("enemy", &s.kind);
            }
            for s in &level.spawn_items {
                check_spawn("item", &s.kind);
            }
        }
    }
}

/// Встроенные копии core обязаны парситься — они последний рубеж фолбэков.
#[test]
fn embedded_fallbacks_parse() {
    crate::config::embedded_configs_parse_for_test();
}

/// Умения игрока каждого пресета (если файл есть) обязаны быть полными и
/// локализованными: `ability::parse` требует непустые RU/EN имя и описание,
/// корректные числа, валидный тип урона, иконку в границах атласа и покрытие
/// двух слотов на каждый из трёх классов.
#[test]
fn preset_player_abilities_are_valid() {
    for preset in preset_dirs() {
        let name = preset.file_name().unwrap().to_string_lossy().to_string();
        let Some(text) = read(&preset, "player_abilities.json") else {
            continue;
        };
        let defs = crate::combat::ability::parse(&text)
            .unwrap_or_else(|e| panic!("{name}/player_abilities.json: {e}"));
        assert_eq!(
            defs.len(),
            6,
            "{name}/player_abilities.json: ожидаются 2 умения на каждый из 3 классов"
        );
    }
}

/// Магазин Торговца: у core должны быть товары на продажу, и у каждого товара —
/// положительная цена (у расходников `value` нулевой, цена идёт от лечения).
#[test]
fn merchant_shop_prices_every_consumable() {
    let core = presets_root().join("core");
    let items: ItemsFile = parse(&core, "items.json");
    let for_sale: Vec<_> = items.items.iter().filter(|i| i.is_for_sale()).collect();
    assert!(!for_sale.is_empty(), "core/items.json: Торговцу нечего продавать");
    for item in for_sale {
        assert!(
            item.shop_price() > 0,
            "core/items.json: у товара '{}' неположительная цена",
            item.id
        );
        assert!(
            item.sell_price() > 0 && item.sell_price() <= item.shop_price(),
            "core/items.json: цена продажи '{}' должна быть в (0, закупочной]",
            item.id
        );
    }
}

/// Паспорт баланса оружия (docs/WEAPON_BALANCE.md): ростер core обязан держать
/// восемь канонических стволов, покрывать три типа урона и соблюдать
/// экономику — меле без боезапаса и на короткой дистанции, дальнобойное с
/// боезапасом и увеличенной дистанцией. Уникальность профилей/альт-огня
/// проверяет `all_presets_parse_and_link`.
#[test]
fn weapon_roster_matches_balance_passport() {
    use crate::weapon::{DmgType, WeaponId};
    let core = presets_root().join("core");
    let weapons = crate::weapon::parse(&must_read(&core, "weapons.json"))
        .unwrap_or_else(|e| panic!("core/weapons.json: {e}"));

    let ids: HashSet<WeaponId> = weapons.iter().map(|w| w.id).collect();
    assert_eq!(ids.len(), weapons.len(), "core/weapons.json: дублирующиеся id оружия");
    for id in WeaponId::ALL {
        assert!(ids.contains(&id), "core/weapons.json: нет канонического оружия '{}'", id.id());
    }

    let types: HashSet<DmgType> = weapons.iter().map(|w| w.dmg_type).collect();
    for ty in [DmgType::Physical, DmgType::Energy, DmgType::Fire] {
        assert!(types.contains(&ty), "core/weapons.json: ни одно оружие не наносит урон типа {ty:?}");
    }

    for w in &weapons {
        assert!(w.damage > 0.0 && w.range > 0.0, "core/weapons.json: '{}' с нулевым уроном/дистанцией", w.name_en);
        let melee = matches!(w.id, WeaponId::Sword | WeaponId::Chainsaw);
        if melee {
            assert!(w.ammo.is_none(), "core/weapons.json: меле '{}' не должно тратить боезапас", w.name_en);
            assert!(w.range <= 5.0, "core/weapons.json: у меле '{}' дистанция как у дальнобойного", w.name_en);
        } else {
            assert!(w.ammo.is_some(), "core/weapons.json: дальнобойное '{}' обязано тратить боезапас", w.name_en);
            assert!(w.range >= 10.0, "core/weapons.json: у дальнобойного '{}' слишком короткая дистанция", w.name_en);
        }
    }
}
