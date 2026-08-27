//! Проверки таблицы угрозы: враг идёт на того, кто его бьёт.

use crate::math::Vec3;
use crate::sim::tests_support::delve_state;
use crate::sim::{HitClaim, Player, State};

/// Поставить двух игроков вокруг врага: ближний и дальний.
///
/// Возвращает (индекс врага, peer ближнего, peer дальнего).
fn setup(seed: u64) -> (State, usize, u16, u16) {
    let mut state = delve_state(seed);
    state.join(1, "ближний".into());
    state.join(2, "дальний".into());

    let index = state
        .enemies
        .iter()
        .position(|enemy| enemy.chase_range >= 10.0)
        .expect("нужен враг с приличным радиусом погони");
    let enemy_pos = state.enemies[index].pos;

    state.players.get_mut(&1).unwrap().pos = enemy_pos + Vec3::new(2.0, 0.0, 0.0);
    state.players.get_mut(&2).unwrap().pos = enemy_pos + Vec3::new(8.0, 0.0, 0.0);
    (state, index, 1, 2)
}

fn tick_once(state: &mut State) {
    let players: Vec<Player> = state.players.values().cloned().collect();
    let refs: Vec<&Player> = players.iter().collect();
    let world = state.world.clone();
    let mut events = Vec::new();
    for enemy in &mut state.enemies {
        enemy.tick(0.05, &world, &refs, None, &mut events);
    }
}

#[test]
fn without_damage_enemy_picks_the_closest() {
    let (mut state, index, near, _far) = setup(0x7A_01);
    tick_once(&mut state);
    assert_eq!(
        state.enemies[index].target,
        Some(near),
        "без урона враг обязан идти на ближнего — прежнее поведение"
    );
}

#[test]
fn damage_pulls_the_enemy_away_from_the_closest() {
    let (mut state, index, near, far) = setup(0x7A_02);
    tick_once(&mut state);
    assert_eq!(state.enemies[index].target, Some(near));

    // Дальний расстреливает врага — внимание должно перейти к нему.
    let enemy_id = state.enemies[index].id;
    let enemy_pos = state.enemies[index].pos;
    let claim = HitClaim {
        tick: 1,
        weapon: "pistol".into(),
        target: enemy_id,
        pos: enemy_pos,
        part: 0,
    };
    for _ in 0..6 {
        state.time += 1.0;
        crate::sim::damage::apply_hit(&mut state, far, &claim);
    }

    tick_once(&mut state);
    assert_eq!(
        state.enemies[index].target,
        Some(far),
        "враг обязан развернуться к тому, кто наносит урон"
    );
    assert!(state.enemies[index].threat_of(far) > state.enemies[index].threat_of(near));
}

#[test]
fn small_threat_does_not_flip_the_target() {
    let (mut state, index, near, far) = setup(0x7A_03);
    tick_once(&mut state);
    assert_eq!(state.enemies[index].target, Some(near));

    // Царапина от дальнего не должна перетягивать врага: иначе он будет
    // разворачиваться на каждый случайный выстрел.
    state.enemies[index].add_threat(far, 1.0);
    tick_once(&mut state);
    assert_eq!(
        state.enemies[index].target,
        Some(near),
        "мелкая угроза не должна менять цель"
    );
}

#[test]
fn threat_fades_over_time() {
    let (mut state, index, _near, far) = setup(0x7A_04);
    state.enemies[index].add_threat(far, 100.0);
    let before = state.enemies[index].threat_of(far);

    // Минута без урона — внимание должно заметно ослабнуть.
    for _ in 0..120 {
        tick_once(&mut state);
        state.enemies[index].tick(
            0.5,
            &state.world.clone(),
            &[],
            None,
            &mut Vec::new(),
        );
    }

    let after = state.enemies[index].threat_of(far);
    assert!(
        after < before * 0.5,
        "угроза обязана забываться: было {before:.1}, стало {after:.1}"
    );
}

#[test]
fn downed_player_is_not_a_target() {
    let (mut state, index, near, far) = setup(0x7A_05);
    state.enemies[index].add_threat(near, 500.0);
    tick_once(&mut state);
    assert_eq!(state.enemies[index].target, Some(near));

    // Ближний упал — враг обязан переключиться на того, кто ещё стоит.
    let player = state.players.get_mut(&near).unwrap();
    player.hp = 0.0;
    crate::sim::rescue::knock_down(player);

    tick_once(&mut state);
    assert_eq!(
        state.enemies[index].target,
        Some(far),
        "лежачего добивать не нужно — есть кому мешать"
    );
}
