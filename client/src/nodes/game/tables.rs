//! Статические таблицы контента и лукапы спрайтов/сцен.
//!
//! Legacy-таблица `NPC_DATA` (фолбэк, когда у пресета нет `npcs.json`),
//! соответствия id → путь спрайта и динамический выбор сцены NPC.
//! Вынесено из `mod.rs`; в `mod.rs` реэкспортируется через `use tables::*`.
use super::*;

#[allow(dead_code)]
pub(super) struct NpcCfg {
    pub(super) id: &'static str,
    pub(super) name: &'static str,
    pub(super) scene_id: &'static str,
    pub(super) pos: Vector3,
    pub(super) color: Color,
}

pub(super) const NPC_DATA: &[NpcCfg] = &[
    NpcCfg {
        id: "vale",
        name: "Ms. Вейл",
        scene_id: "meet_vale",
        pos: Vector3::new(-6.0, 0.0, -8.0),
        color: Color::from_rgba(1.0, 0.75, 0.85, 1.0),
    },
    NpcCfg {
        id: "victor",
        name: "Виктор",
        scene_id: "intro_victor",
        pos: Vector3::new(6.0, 0.0, -8.0),
        color: Color::from_rgba(0.75, 1.0, 0.8, 1.0),
    },
    NpcCfg {
        id: "elena",
        name: "Елена",
        scene_id: "first_elena",
        pos: Vector3::new(-11.0, 0.0, 3.0),
        color: Color::from_rgba(0.75, 0.8, 1.0, 1.0),
    },
    NpcCfg {
        id: "sofia",
        name: "София",
        scene_id: "meet_sofia",
        pos: Vector3::new(11.0, 0.0, 3.0),
        color: Color::from_rgba(1.0, 0.95, 0.7, 1.0),
    },
    NpcCfg {
        id: "guard",
        name: "Охранник",
        scene_id: "meet_guard",
        pos: Vector3::new(-2.5, 0.0, -18.0),
        color: Color::from_rgba(0.85, 0.85, 0.85, 1.0),
    },
    NpcCfg {
        id: "merchant",
        name: "Торговец",
        scene_id: "meet_merchant",
        pos: Vector3::new(17.5, 0.0, -1.0),
        color: Color::from_rgba(1.0, 0.85, 0.6, 1.0),
    },
    NpcCfg {
        id: "scientist",
        name: "Учёный",
        scene_id: "meet_scientist",
        pos: Vector3::new(-18.0, 0.0, 0.0),
        color: Color::from_rgba(0.7, 1.0, 1.0, 1.0),
    },
    NpcCfg {
        id: "stranger",
        name: "Незнакомец",
        scene_id: "meet_stranger",
        pos: Vector3::new(5.0, 0.0, 16.0),
        color: Color::from_rgba(0.8, 0.65, 0.95, 1.0),
    },
];

pub(super) fn npc_sprite_tex(id: &str) -> (&'static str, &'static str) {
    match id {
        "vale" => (
            "res://assets/sprites/characters/npc_vale.png",
            "res://assets/sprites/femboy_pink.png",
        ),
        "victor" => (
            "res://assets/sprites/characters/npc_victor.png",
            "res://assets/sprites/femboy_dark2.png",
        ),
        "elena" => (
            "res://assets/sprites/characters/npc_elena.png",
            "res://assets/sprites/femboy_dark1.png",
        ),
        "sofia" => (
            "res://assets/sprites/characters/npc_sofia.png",
            "res://assets/sprites/femboy_pink.png",
        ),
        "guard" => (
            "res://assets/sprites/characters/npc_guard.png",
            "res://assets/sprites/femboy_dark2.png",
        ),
        "merchant" => (
            "res://assets/sprites/characters/npc_merchant.png",
            "res://assets/sprites/femboy_pink.png",
        ),
        "scientist" => (
            "res://assets/sprites/characters/npc_scientist.png",
            "res://assets/sprites/femboy_dark1.png",
        ),
        "stranger" => (
            "res://assets/sprites/characters/npc_stranger.png",
            "res://assets/sprites/femboy_dark2.png",
        ),
        _ => (
            "res://assets/sprites/femboy_dark1.png",
            "res://assets/sprites/femboy_dark1.png",
        ),
    }
}

pub(super) fn item_sprite_tex(id: &str) -> &'static str {
    match id {
        "medkit" => "res://assets/sprites/items/item_medkit.png",
        "key" => "res://assets/sprites/items/item_key.png",
        "gold_coin" | "gold_stack" => "res://assets/sprites/items/item_gold.png",
        "armor_shard" => "res://assets/sprites/items/item_armor.png",
        "energy_drink" => "res://assets/sprites/items/item_energy_drink.png",
        "potion" => "res://assets/sprites/items/item_potion.png",
        "ancient_ruby" => "res://assets/sprites/items/item_ruby.png",
        "heart_1up" => "res://assets/sprites/pickups/heart_1up.png",
        "soul" => "res://assets/sprites/pickups/soul.png",
        _ => "",
    }
}

/// Динамический выбор сцены для NPC.
pub(super) fn npc_scene_id(npc_id: &str, state: &GameState) -> &'static str {
    match npc_id {
        "vale" => {
            if state.has("boss_defeated_archive_sentinel")
                && !state.has("discussed_archive_sentinel")
            {
                "hub_vale_archive_aftermath"
            } else if state.has("met_vale") {
                "hub_vale_repeat"
            } else {
                "hub_vale_intro"
            }
        }
        "victor" => {
            if state.has("boss_defeated_crypt_warden") && !state.has("discussed_crypt_warden") {
                "hub_victor_warden_aftermath"
            } else if state.has("met_victor") {
                "hub_victor_repeat"
            } else {
                "hub_victor_intro"
            }
        }
        "elena" => {
            if state.has("boss_defeated_blood_oracle") && !state.has("discussed_blood_oracle") {
                "hub_elena_oracle_aftermath"
            } else if state.has("met_elena") {
                "hub_elena_repeat"
            } else {
                "hub_elena_intro"
            }
        }
        "sofia" => {
            if state.has("met_sofia") {
                "hub_sofia_repeat"
            } else {
                "hub_sofia_intro"
            }
        }
        "guard" => {
            if state.has("met_guard") {
                "hub_guard_repeat"
            } else {
                "hub_guard_intro"
            }
        }
        "merchant" => {
            if state.has("met_merchant") {
                "hub_merchant_repeat"
            } else {
                "hub_merchant_intro"
            }
        }
        "stranger" => {
            if state.has("boss_defeated_heart_tyrant") && !state.has("discussed_heart_tyrant") {
                "hub_stranger_tyrant_aftermath"
            } else if state.has("met_stranger") {
                "hub_stranger_repeat"
            } else {
                "hub_stranger_intro"
            }
        }
        _ => "",
    }
}
