//! Проверки даун/ревайва: смерть одного не должна заканчивать забег пати.

use crate::math::Vec3;
use crate::sim::rescue::{BLEEDOUT, REVIVE_HP_FRAC, REVIVE_RANGE, REVIVE_TIME};
use crate::sim::tests_support::delve_state;
use crate::sim::{buttons, event, flags, Input, State};

/// Свалить игрока: урон больше здоровья приходит от врага (actor >= 1000).
fn knock_down(state: &mut State, peer: u16) {
    let hp = state.players[&peer].hp;
    let mut damage = crate::sim::Event::new(event::DAMAGE);
    damage.actor = 1000;
    damage.target = peer;
    damage.amount = hp + 10.0;
    let events = vec![damage];
    let extra = state.apply_enemy_damage_for_test(&events);
    assert!(
        extra.iter().any(|e| e.kind == event::DOWNED),
        "игрок должен был лечь"
    );
}

/// Ввод «держу кнопку использования» рядом с лежащим.
fn hold_use(state: &mut State, peer: u16, pos: Vec3) {
    state.apply_input(
        peer,
        Input { pos, buttons: buttons::USE, ..Default::default() },
        0.05,
    );
}

fn two_player_room(seed: u64) -> State {
    let mut state = delve_state(seed);
    state.join(1, "первый".into());
    state.join(2, "второй".into());
    state
}

#[test]
fn zero_hp_knocks_down_but_does_not_kill() {
    let mut state = two_player_room(0x5A1);
    knock_down(&mut state, 1);

    let player = &state.players[&1];
    assert!(player.downed(), "игрок обязан лечь");
    assert!(!player.out, "он ещё не выбыл — его можно поднять");
    assert_ne!(player.flags & flags::DOWNED, 0, "флаг для снапшота не выставлен");
}

#[test]
fn teammate_revives_after_holding_the_button() {
    let mut state = two_player_room(0x5A2);
    knock_down(&mut state, 1);
    let where_fell = state.players[&1].pos;

    // Товарищ подходит вплотную и держит «использовать».
    let helper_spot = where_fell + Vec3::new(REVIVE_RANGE * 0.5, 0.0, 0.0);
    let mut revived = false;
    for _ in 0..((REVIVE_TIME / 0.05) as usize + 5) {
        hold_use(&mut state, 2, helper_spot);
        let result = state.tick(0.05, &[]);
        if result.events.iter().any(|e| e.kind == event::REVIVED) {
            revived = true;
            break;
        }
    }

    assert!(revived, "товарищ так и не поднял");
    let player = &state.players[&1];
    assert!(!player.downed(), "после подъёма игрок стоит на ногах");
    assert!(
        (player.hp - player.max_hp * REVIVE_HP_FRAC).abs() < 0.01,
        "поднимать нужно с долей здоровья, а не в полную силу"
    );
}

#[test]
fn revive_needs_proximity() {
    let mut state = two_player_room(0x5A3);
    knock_down(&mut state, 1);
    let far = state.players[&1].pos + Vec3::new(REVIVE_RANGE * 4.0, 0.0, 0.0);
    // Ставим спасателя далеко напрямую: обычный ввод с таким прыжком сервер
    // справедливо отклонил бы как телепорт.
    state.players.get_mut(&2).unwrap().pos = far;

    for _ in 0..((REVIVE_TIME / 0.05) as usize + 5) {
        hold_use(&mut state, 2, far);
        let result = state.tick(0.05, &[]);
        assert!(
            !result.events.iter().any(|e| e.kind == event::REVIVED),
            "подъём издалека не должен засчитываться"
        );
    }
    assert!(state.players[&1].downed());
}

#[test]
fn bleedout_takes_the_player_out() {
    let mut state = two_player_room(0x5A4);
    knock_down(&mut state, 1);

    let mut out = false;
    // Второй игрок ничего не делает — первый истекает кровью.
    for _ in 0..((BLEEDOUT / 0.5) as usize + 4) {
        let result = state.tick(0.5, &[]);
        if result.events.iter().any(|e| e.kind == event::PLAYER_OUT) {
            out = true;
            break;
        }
    }

    assert!(out, "истечение кровью не сработало");
    assert!(state.players[&1].out);
}

#[test]
fn run_fails_only_when_everyone_is_down() {
    let mut state = two_player_room(0x5A5);

    knock_down(&mut state, 1);
    let result = state.tick(0.05, &[]);
    assert!(
        !result.events.iter().any(|e| e.kind == event::WIPE),
        "пока кто-то на ногах, забег продолжается"
    );

    knock_down(&mut state, 2);
    let result = state.tick(0.05, &[]);
    assert!(
        result.events.iter().any(|e| e.kind == event::WIPE),
        "легли все — забег обязан провалиться"
    );

    // Событие одноразовое: спамить им каждый тик нельзя.
    let again = state.tick(0.05, &[]);
    assert!(!again.events.iter().any(|e| e.kind == event::WIPE));
}

#[test]
fn downed_player_does_not_move() {
    let mut state = two_player_room(0x5A6);
    knock_down(&mut state, 1);
    let where_fell = state.players[&1].pos;

    state.apply_input(
        1,
        Input { pos: where_fell + Vec3::new(1.0, 0.0, 0.0), ..Default::default() },
        0.05,
    );
    assert_eq!(state.players[&1].pos, where_fell, "лежачий не должен ползти");
}
