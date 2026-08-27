//! Проверки планировщика данжа.
//!
//! Главное, что здесь охраняется: **один сид — один данж**. От этого зависит вся
//! сетевая часть: сервер и клиент строят мир по одному сиду и обязаны получить
//! одинаковую геометрию (docs/MULTIPLAYER.md §5).

use std::path::Path;

use crate::config::GameConfig;
use crate::worldgen::dungeon::plan;
use crate::worldgen::GRID;

fn config() -> GameConfig {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../game/presets/core");
    GameConfig::load_from(&base.to_string_lossy())
}

/// Отпечаток плана: если хоть одна клетка сетки уедет, число изменится.
fn fingerprint(layout: &crate::worldgen::dungeon::DungeonLayout) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut eat = |value: u64| {
        hash ^= value;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    };
    eat(layout.rooms.len() as u64);
    eat(layout.boss_idx as u64);
    for (index, walkable) in layout.floor.iter().enumerate() {
        if *walkable {
            eat(index as u64);
            eat(layout.floor_heights[index].to_bits() as u64);
        }
    }
    for room in &layout.rooms {
        eat(room.x as u64);
        eat(room.z as u64);
        eat(room.w as u64);
        eat(room.h as u64);
        eat(room.floor_y.to_bits() as u64);
    }
    hash
}

#[test]
fn same_seed_gives_same_dungeon() {
    let cfg = config();
    let a = plan(3, 0xC0FF_EE12_3456_789A, &cfg);
    let b = plan(3, 0xC0FF_EE12_3456_789A, &cfg);
    assert_eq!(
        fingerprint(&a),
        fingerprint(&b),
        "один сид дал разные данжи — клиент и сервер разойдутся"
    );
    // И состояние генератора одинаково: клиент продолжает поток с него.
    assert_eq!(a.rng.0, b.rng.0);
}

#[test]
fn different_seeds_give_different_dungeons() {
    let cfg = config();
    let a = plan(3, 1, &cfg);
    let b = plan(3, 2, &cfg);
    assert_ne!(fingerprint(&a), fingerprint(&b));
}

#[test]
fn layout_is_connected_and_sane() {
    let cfg = config();
    for depth in 1..=6u32 {
        let layout = plan(depth, 0x5EED_0000 + depth as u64, &cfg);

        // Точное число плавает от сида (комнаты, которые не влезли, отбрасываются),
        // но пустой или почти пустой данж — это баг генератора.
        assert!(
            layout.rooms.len() >= 4,
            "глубина {depth}: комнат всего {}",
            layout.rooms.len()
        );
        assert!(layout.boss_idx < layout.rooms.len());
        assert_eq!(layout.floor.len(), GRID * GRID);
        assert_eq!(layout.floor_heights.len(), GRID * GRID);
        assert_eq!(layout.ceil_heights.len(), GRID * GRID);

        // Каждая комната достижима из стартовой — иначе забег в тупике.
        for (index, room) in layout.rooms.iter().enumerate() {
            let (cx, cz) = room.center();
            let cell = cz as usize * GRID + cx as usize;
            assert!(
                layout.floor[cell],
                "глубина {depth}: центр комнаты {index} не пол"
            );
            assert!(
                layout.reachable[cell],
                "глубина {depth}: комната {index} недостижима из старта"
            );
        }

        // Пандусы всегда лежат на проходимых клетках.
        for (index, ramp) in layout.is_ramp.iter().enumerate() {
            if *ramp {
                assert!(layout.floor[index], "пандус в непроходимой клетке");
            }
        }
    }
}
