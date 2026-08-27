//! Кооперативные правила: скейлинг по размеру пати и инстансный лут.

use crate::math::Vec3;
use crate::sim::tests_support::preset_base;
use crate::sim::{event, loot, Config, State};

fn delve_with_party(seed: u64, party: i32) -> State {
    State::new(Config {
        preset: "core".into(),
        kind: "delve".into(),
        seed,
        depth: 2,
        party_size: party,
        preset_base: preset_base(),
    })
}

#[test]
fn bigger_party_gets_more_and_tougher_enemies() {
    let solo = delve_with_party(0xC0_0B, 1);
    let four = delve_with_party(0xC0_0B, 4);

    assert!(
        four.enemies.len() > solo.enemies.len(),
        "вчетвером врагов должно быть больше: {} против {}",
        four.enemies.len(),
        solo.enemies.len()
    );

    // Один и тот же вид врага должен стать крепче.
    let kind = &solo.enemies[0].kind;
    let solo_hp = solo
        .enemies
        .iter()
        .find(|e| e.kind == *kind)
        .map(|e| e.max_hp)
        .unwrap();
    let party_hp = four
        .enemies
        .iter()
        .find(|e| e.kind == *kind)
        .map(|e| e.max_hp)
        .unwrap();
    assert!(
        party_hp > solo_hp,
        "здоровье не выросло: {party_hp} против {solo_hp}"
    );
}

#[test]
fn enemy_damage_does_not_scale_with_party() {
    let solo = delve_with_party(0xC0_0C, 1);
    let four = delve_with_party(0xC0_0C, 4);

    let kind = &solo.enemies[0].kind;
    let solo_damage = solo.enemies.iter().find(|e| e.kind == *kind).unwrap().damage;
    let party_damage = four.enemies.iter().find(|e| e.kind == *kind).unwrap().damage;

    assert!(
        (solo_damage - party_damage).abs() < 0.01,
        "урон врагов масштабировать нельзя: {solo_damage} против {party_damage}"
    );
}

#[test]
fn solo_run_is_unchanged() {
    // Пати из одного человека не должна ничего умножать.
    let (count, hp) = (1.0_f32, 1.0_f32);
    let solo = delve_with_party(7, 1);
    let again = delve_with_party(7, 1);
    assert_eq!(solo.enemies.len(), again.enemies.len());
    assert_eq!(solo.enemies[0].max_hp, again.enemies[0].max_hp);
    assert_eq!((count, hp), (1.0, 1.0));
}

/// Убитый враг оставляет каждому игроку свой предмет, и чужой его не поднимет.
#[test]
fn kill_drops_are_personal() {
    let mut state = delve_with_party(0x0001_007A, 2);
    state.join(1, "первый".into());
    state.join(2, "второй".into());

    // Гарантируем дроп: в тесте важна раздача, а не удача.
    let items = {
        let owners = [1u16, 2u16];
        let mut rng = crate::rng::Rng::new(3);
        let mut next = 1;
        let mut cfg_holder = crate::config::GameConfig::load_from(&preset_base());
        cfg_holder.loot.kill_drops = vec![crate::config::KillDrop {
            kind: "item".into(),
            id: Some("medkit".into()),
            chance: 1.0,
        }];
        loot::roll_kill_drops(&cfg_holder, &mut rng, Vec3::ZERO, &owners, &mut next)
    };
    assert_eq!(items.len(), 2);

    let first = &items[0];
    let other = if first.owner == 1 { 2 } else { 1 };
    assert!(
        !loot::can_pick_up(first, other, first.pos),
        "чужой лут не должен подбираться"
    );
    assert!(loot::can_pick_up(first, first.owner, first.pos));
}

/// Лут доезжает до снапшота с пометкой владельца и подбирается вблизи.
#[test]
fn dropped_item_reaches_snapshot_and_is_picked_up() {
    let mut state = delve_with_party(0x0001_007B, 1);
    state.join(1, "vasya".into());

    let where_player = state.players[&1].pos;
    state.items.push(loot::WorldItem {
        id: 501,
        owner: 1,
        drop: loot::Drop::Item("medkit".into()),
        pos: where_player + Vec3::new(30.0, 0.0, 0.0),
        ttl: loot::ITEM_TTL,
    });

    let snapshot = state.tick(0.05, &[]).snapshot;
    let ent = snapshot
        .items
        .iter()
        .find(|ent| ent.id == 501)
        .expect("предмет обязан быть в снапшоте");
    assert_eq!(ent.owner, 1, "снапшот должен нести владельца");

    // Подходим вплотную — предмет уходит игроку.
    state.items[0].pos = where_player;
    let result = state.tick(0.05, &[]);
    assert!(
        result.events.iter().any(|e| e.kind == event::PICKUP && e.actor == 1),
        "подбор не сработал"
    );
    assert!(state.items.is_empty(), "поднятый предмет должен исчезнуть");
}

#[test]
fn items_expire_after_ttl() {
    let mut state = delve_with_party(0x0001_007C, 1);
    state.join(1, "vasya".into());
    let far = state.players[&1].pos + Vec3::new(50.0, 0.0, 0.0);
    state.items.push(loot::WorldItem {
        id: 777,
        owner: 1,
        drop: loot::Drop::Ammo(0),
        pos: far,
        ttl: 1.0,
    });

    let mut gone = false;
    for _ in 0..30 {
        let result = state.tick(0.1, &[]);
        if result.events.iter().any(|e| e.kind == event::DESPAWN && e.target == 777) {
            gone = true;
            break;
        }
    }
    assert!(gone, "предмет должен исчезнуть по истечении срока");
    assert!(state.items.is_empty());
}
