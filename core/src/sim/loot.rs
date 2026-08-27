//! Лут: у каждого игрока свой.
//!
//! Общая куча в кооперативе — это гонка за предметом и обиженный товарищ.
//! Поэтому с убитого врага каждому живому игроку роллится **свой** дроп, и
//! видит его только он (docs/MULTIPLAYER.md §11, пункт 4).
//!
//! Ролл детерминированный: считает сервер своим генератором, клиент только
//! рисует то, что пришло в снапшоте.

use serde::{Deserialize, Serialize};

use crate::config::GameConfig;
use crate::math::Vec3;
use crate::rng::Rng;

/// Сколько предмет лежит на земле, прежде чем исчезнуть.
pub const ITEM_TTL: f32 = 120.0;
/// С какого расстояния предмет поднимается.
pub const PICKUP_RANGE: f32 = 1.6;
/// Куда предмет отлетает от места смерти врага.
const SCATTER: f32 = 1.2;

/// Что именно лежит на земле.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Drop {
    /// Предмет из `items.json`.
    Item(String),
    /// Патроны: индекс типа из `AmmoType`.
    Ammo(u8),
}

/// Предмет в мире, принадлежащий конкретному игроку.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldItem {
    pub id: u16,
    /// Кому виден и кто может поднять.
    pub owner: u16,
    pub drop: Drop,
    pub pos: Vec3,
    /// Сколько ещё пролежит.
    pub ttl: f32,
}

/// Роллить дроп с убитого врага — отдельно для каждого получателя.
///
/// `owners` — живые игроки, которым положен лут. Каждому достаётся свой бросок:
/// одному повезло, другому нет, и это никак не связано.
pub fn roll_kill_drops(
    cfg: &GameConfig,
    rng: &mut Rng,
    at: Vec3,
    owners: &[u16],
    next_id: &mut u16,
) -> Vec<WorldItem> {
    let mut out = Vec::new();
    for (index, owner) in owners.iter().enumerate() {
        for entry in &cfg.loot.kill_drops {
            if !rng.chance(entry.chance) {
                continue;
            }
            let drop = match entry.kind.as_str() {
                "ammo" => Drop::Ammo((rng.below(4)) as u8),
                _ => match entry.id.as_ref() {
                    Some(id) => Drop::Item(id.clone()),
                    None => continue,
                },
            };
            // Раскладываем по кругу, чтобы предметы не лежали друг в друге.
            let angle = (index as f32 + out.len() as f32) * 1.7;
            let pos = Vec3::new(
                at.x + angle.cos() * SCATTER,
                at.y,
                at.z + angle.sin() * SCATTER,
            );
            let id = *next_id;
            *next_id = next_id.wrapping_add(1).max(1);
            out.push(WorldItem { id, owner: *owner, drop, pos, ttl: ITEM_TTL });
        }
    }
    out
}

impl WorldItem {
    /// Индекс вида для снапшота: предметам он не нужен, но поле общее.
    pub fn type_id(&self) -> u16 {
        match &self.drop {
            Drop::Ammo(kind) => *kind as u16 + 1,
            Drop::Item(_) => 0,
        }
    }

    /// Человеческая метка для события: клиент покажет её в ленте.
    pub fn label(&self) -> String {
        match &self.drop {
            Drop::Item(id) => id.clone(),
            Drop::Ammo(kind) => format!("ammo:{kind}"),
        }
    }
}

/// Может ли этот игрок поднять этот предмет.
pub fn can_pick_up(item: &WorldItem, peer: u16, player_pos: Vec3) -> bool {
    item.owner == peer && (item.pos - player_pos).length() <= PICKUP_RANGE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{GameConfig, KillDrop};
    use std::path::Path;

    fn config() -> GameConfig {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../game/presets/core");
        GameConfig::load_from(&base.to_string_lossy())
    }

    /// Конфиг, где дроп гарантирован: так проверяется сама механика, а не удача.
    fn always_dropping() -> GameConfig {
        let mut cfg = config();
        cfg.loot.kill_drops = vec![KillDrop {
            kind: "item".into(),
            id: Some("medkit".into()),
            chance: 1.0,
        }];
        cfg
    }

    #[test]
    fn every_player_gets_own_drop() {
        let cfg = always_dropping();
        let mut rng = Rng::new(1);
        let mut next = 1;

        let items = roll_kill_drops(&cfg, &mut rng, Vec3::ZERO, &[1, 2, 3], &mut next);

        assert_eq!(items.len(), 3, "каждому игроку — свой предмет");
        for peer in [1, 2, 3] {
            assert!(
                items.iter().any(|item| item.owner == peer),
                "игрок {peer} остался без лута"
            );
        }
        let ids: std::collections::HashSet<u16> = items.iter().map(|item| item.id).collect();
        assert_eq!(ids.len(), 3, "id предметов обязаны быть разными");
    }

    #[test]
    fn only_owner_can_pick_up() {
        let item = WorldItem {
            id: 1,
            owner: 7,
            drop: Drop::Item("medkit".into()),
            pos: Vec3::ZERO,
            ttl: ITEM_TTL,
        };
        assert!(can_pick_up(&item, 7, Vec3::new(0.5, 0.0, 0.0)));
        assert!(!can_pick_up(&item, 8, Vec3::new(0.5, 0.0, 0.0)), "чужой лут");
        assert!(
            !can_pick_up(&item, 7, Vec3::new(10.0, 0.0, 0.0)),
            "издалека поднимать нельзя"
        );
    }

    #[test]
    fn rolls_are_deterministic() {
        let cfg = config();
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        let (mut id_a, mut id_b) = (1, 1);

        let first = roll_kill_drops(&cfg, &mut a, Vec3::ZERO, &[1, 2], &mut id_a);
        let second = roll_kill_drops(&cfg, &mut b, Vec3::ZERO, &[1, 2], &mut id_b);

        assert_eq!(first.len(), second.len());
        for (x, y) in first.iter().zip(second.iter()) {
            assert_eq!(x.drop, y.drop);
            assert_eq!(x.owner, y.owner);
        }
    }

    #[test]
    fn dead_players_get_nothing() {
        let cfg = always_dropping();
        let mut rng = Rng::new(5);
        let mut next = 1;
        let items = roll_kill_drops(&cfg, &mut rng, Vec3::ZERO, &[], &mut next);
        assert!(items.is_empty(), "без получателей лут не роллится");
    }
}
