//! Сложность забега зависит от того, сколько людей в нём реально есть.

use crate::sim::tests_support::preset_base;
use crate::sim::{Cmd, Config, State};

fn solo_delve(seed: u64) -> State {
    State::new(Config {
        preset: "core".into(),
        kind: "delve".into(),
        seed,
        depth: 2,
        party_size: 1,
        preset_base: preset_base(),
    })
}

fn party_cmd(size: i32) -> Cmd {
    Cmd {
        kind: "party".into(),
        peer: 0,
        data: Some(serde_json::json!({ "size": size })),
    }
}

#[test]
fn second_player_makes_the_run_harder() {
    let mut state = solo_delve(0x0002_0001);
    let enemies_before = state.enemies.len();
    let hp_before = state.enemies[0].max_hp;

    state.join(1, "первый".into());
    state.command(&party_cmd(2));

    assert!(
        state.enemies[0].max_hp > hp_before,
        "живым врагам должно добавиться здоровья: было {hp_before}, стало {}",
        state.enemies[0].max_hp
    );
    assert!(
        state.enemies.len() > enemies_before,
        "врагов должно стать больше: было {enemies_before}, стало {}",
        state.enemies.len()
    );
}

#[test]
fn repeating_the_same_size_changes_nothing() {
    let mut state = solo_delve(0x0002_0002);
    state.command(&party_cmd(3));

    let count = state.enemies.len();
    let hp = state.enemies[0].max_hp;

    state.command(&party_cmd(3));
    assert_eq!(state.enemies.len(), count, "повтор не должен досыпать врагов");
    assert_eq!(state.enemies[0].max_hp, hp, "и не должен раздувать здоровье");
}

#[test]
fn leaving_does_not_strip_health_mid_fight() {
    let mut state = solo_delve(0x0002_0003);
    state.command(&party_cmd(4));
    let hp = state.enemies[0].max_hp;

    // Товарищи вышли — отбирать здоровье у врага в разгар боя не надо:
    // игрок этого не заметит, а полоска дёрнется.
    state.command(&party_cmd(1));
    assert_eq!(state.enemies[0].max_hp, hp);
}

#[test]
fn reinforcements_do_not_appear_next_to_players() {
    let mut state = solo_delve(0x0002_0004);
    state.join(1, "vasya".into());
    let player = state.players[&1].pos;

    let before: Vec<u16> = state.enemies.iter().map(|enemy| enemy.id).collect();
    state.command(&party_cmd(4));

    for enemy in state.enemies.iter().filter(|e| !before.contains(&e.id)) {
        let distance = (enemy.pos - player).length();
        assert!(
            distance >= 25.0,
            "подкрепление появилось в {distance:.1} м от игрока — слишком близко"
        );
    }
}

#[test]
fn party_command_reports_the_new_size() {
    let mut state = solo_delve(0x0002_0005);
    let events = state.command(&party_cmd(3));
    let notice = events
        .iter()
        .find(|event| event.text == "party")
        .expect("смена состава должна сообщаться событием");
    assert_eq!(notice.amount, 3.0);
}

/// Сервер шлёт команду завёрнутой (`cmd{kind:"party", args:{size}}`) — ядро
/// обязано её разворачивать, иначе состав пати до сложности не доедет.
#[test]
fn wrapped_command_from_host_is_understood() {
    let mut state = solo_delve(0x0002_0006);
    let hp_before = state.enemies[0].max_hp;

    let wrapped = Cmd {
        kind: "cmd".into(),
        peer: 0,
        data: Some(serde_json::json!({ "kind": "party", "args": { "size": 3 } })),
    };
    state.command(&wrapped);

    assert!(
        state.enemies[0].max_hp > hp_before,
        "завёрнутая команда не сработала"
    );
}
