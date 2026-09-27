use crate::{dialogue, game_state::GameState, save::SaveData, weapon::Arsenal};
use std::path::Path;

#[test]
fn evening_content_matches_runtime_ron_and_artwork_exists() {
    let game = Path::new(env!("CARGO_MANIFEST_DIR")).join("../game");
    for stem in ["quests", "dialogues"] {
        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(game.join(format!("presets/core/{stem}.json"))).unwrap(),
        )
        .unwrap();
        let ron: serde_json::Value = ron::from_str(
            &std::fs::read_to_string(game.join(format!("presets/core/{stem}.ron"))).unwrap(),
        )
        .unwrap();
        assert_eq!(json, ron, "runtime RON differs from editable JSON: {stem}");
    }
    let text = std::fs::read_to_string(game.join("presets/core/dialogues.json")).unwrap();
    let (scenes, errors) = dialogue::parse_scenes(&text).unwrap();
    assert!(errors.is_empty(), "{errors:?}");
    let scenes: Vec<_> = scenes
        .iter()
        .filter(|s| s.id.starts_with("evening_"))
        .collect();
    assert_eq!(scenes.len(), 21);
    for scene in scenes {
        for line in &scene.lines {
            assert!(!dialogue::localized(&line.text, "ru").is_empty());
            assert!(!dialogue::localized(&line.text, "en").is_empty());
            if let Some(path) = dialogue::artwork_path(&scene.id, &line.portrait) {
                assert!(
                    game.join(path.strip_prefix("res://").unwrap()).is_file(),
                    "missing {path}"
                );
            }
        }
    }
}

#[test]
fn artwork_resolver_preserves_legacy_and_rejects_invalid_references() {
    assert_eq!(
        dialogue::artwork_path("neighbors_ivo_city", "ivo").unwrap(),
        "res://assets/illustrations/neighbors/ivo.tres"
    );
    assert_eq!(
        dialogue::artwork_path("evening_circuit_offer", "ash").unwrap(),
        "res://assets/portraits/neighbors/ash.tres"
    );
    let explicit = "res://assets/illustrations/neighbors/shared_evening_v1.png";
    assert_eq!(
        dialogue::artwork_path("any_scene", explicit).as_deref(),
        Some(explicit)
    );
    for bad in [
        "",
        "missing_npc",
        "res://assets/../secret.png",
        "res://assets/a//b.png",
        "res://assets/a/./b.png",
        "res://assets/a\\b.png",
        "res://assets/script.gd",
        "https://example.org/a.png",
    ] {
        assert!(
            dialogue::artwork_path("any_scene", bad).is_none(),
            "accepted {bad}"
        );
    }
}

#[test]
fn evening_finale_and_reward_guard_survive_save_reload() {
    let text = include_str!("../../../game/presets/core/dialogues.json");
    let (scenes, errors) = dialogue::parse_scenes(text).unwrap();
    assert!(errors.is_empty());
    let complete = scenes
        .iter()
        .find(|s| s.id == "evening_table_complete")
        .unwrap();
    let mut state = GameState::new("Evening test");
    state.quests.add("evening_table", "table", "invite Ren");
    state.quest_kills.insert("evening_table".into(), 1);
    let before = state.gold;
    state.apply(&complete.choices[0].effects, "en");
    assert_eq!(state.gold, before + 60);
    assert!(state.quests.is_completed("evening_table"));
    assert!(state.has("shared_evening"));
    let json = SaveData::from_game(&state, 83.0, &Arsenal::new())
        .to_json()
        .unwrap();
    let (mut restored, hp, _) = SaveData::from_json(&json).unwrap().into_game();
    assert_eq!(hp, 83.0);
    assert_eq!(restored.quest_kills.get("evening_table"), Some(&1));
    assert!(restored.quests.is_completed("evening_table") && restored.has("shared_evening"));
    let before = SaveData::from_game(&restored, hp, &Arsenal::new());
    assert!(restored
        .apply(&complete.choices[0].effects, "ru")
        .is_empty());
    assert_eq!(restored.gold, before.gold);
    assert_eq!(restored.xp, state.xp);
    assert_eq!(restored.level, state.level);
    let epilogue = scenes.iter().find(|s| s.id == "evening_epilogue").unwrap();
    assert!(epilogue.choices.iter().all(|c| c.effects.is_empty()));
}
