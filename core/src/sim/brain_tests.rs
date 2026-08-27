//! Проверки того, что решение сети действительно управляет врагом.
//!
//! Мозгов в пресете пока нет — поэтому здесь они собираются вручную: сеть
//! с одной строкой весов, всегда выбирающая нужное намерение.

use crate::ai::net::{Activation, Layer, Mlp};
use crate::ai::policy::{Brain, Intent, Situation, FEATURES, INTENTS};
use crate::math::Vec3;
use crate::sim::event;
use crate::sim::tests_support::delve_state;

/// Сеть, которая при любом входе выбирает одно намерение.
fn constant_brain(intent: Intent) -> Brain {
    let index = INTENTS.iter().position(|i| *i == intent).unwrap();
    let mut bias = vec![0.0; INTENTS.len()];
    bias[index] = 1.0;
    Brain {
        id: "test".into(),
        net: Mlp {
            layers: vec![Layer {
                w: vec![vec![0.0; FEATURES]; INTENTS.len()],
                b: bias,
                act: Activation::Linear,
            }],
        },
    }
}

fn situation_near() -> Situation {
    Situation {
        distance: 1.0,
        attack_range: 2.0,
        chase_range: 14.0,
        hp_frac: 1.0,
        target_hp_frac: 1.0,
        attack_ready: 1.0,
        allies_near: 0,
        has_path: true,
    }
}

#[test]
fn brain_overrides_the_default_behaviour() {
    // Вплотную и с готовой атакой обычный автомат бил бы; сеть велит отойти.
    let brain = constant_brain(Intent::Retreat);
    assert_eq!(brain.decide(&situation_near()), Intent::Retreat);
    assert_eq!(
        crate::ai::policy::fallback_intent(&situation_near()),
        Intent::Attack,
        "иначе тест ничего не проверяет"
    );
}

#[test]
fn retreating_enemy_does_not_attack() {
    let mut state = delve_state(0xB2A1);
    state.join(1, "vasya".into());

    let index = 0;
    let enemy_pos = state.enemies[index].pos;
    state.players.get_mut(&1).unwrap().pos = enemy_pos;

    let brain = constant_brain(Intent::Retreat);
    let players: Vec<crate::sim::Player> = state.players.values().cloned().collect();
    let refs: Vec<&crate::sim::Player> = players.iter().collect();

    let mut events = Vec::new();
    let world = state.world.clone();
    for _ in 0..40 {
        state.enemies[index].tick(0.05, &world, &refs, Some(&brain), &mut events);
    }

    assert!(
        !events.iter().any(|e| e.kind == event::DAMAGE),
        "враг с намерением «отойти» всё равно бил"
    );
}

#[test]
fn attacking_enemy_hits_when_close() {
    let mut state = delve_state(0xB2A2);
    state.join(1, "vasya".into());

    let index = 0;
    let enemy_pos = state.enemies[index].pos;
    state.players.get_mut(&1).unwrap().pos = enemy_pos;

    let brain = constant_brain(Intent::Attack);
    let players: Vec<crate::sim::Player> = state.players.values().cloned().collect();
    let refs: Vec<&crate::sim::Player> = players.iter().collect();

    let mut events = Vec::new();
    let world = state.world.clone();
    for _ in 0..40 {
        state.enemies[index].tick(0.05, &world, &refs, Some(&brain), &mut events);
    }

    assert!(
        events.iter().any(|e| e.kind == event::DAMAGE),
        "враг с намерением «атаковать» так и не ударил"
    );
}

/// Без мозга поведение обязано остаться прежним — пресеты без `brains.json`
/// не должны заметить появления сети.
#[test]
fn without_brain_enemy_behaves_as_before() {
    let mut state = delve_state(0xB2A3);
    state.join(1, "vasya".into());

    let index = 0;
    let enemy_pos = state.enemies[index].pos;
    state.players.get_mut(&1).unwrap().pos = enemy_pos + Vec3::new(0.5, 0.0, 0.0);

    let players: Vec<crate::sim::Player> = state.players.values().cloned().collect();
    let refs: Vec<&crate::sim::Player> = players.iter().collect();

    let mut events = Vec::new();
    let world = state.world.clone();
    for _ in 0..40 {
        state.enemies[index].tick(0.05, &world, &refs, None, &mut events);
    }

    assert!(
        events.iter().any(|e| e.kind == event::DAMAGE),
        "без сети враг вплотную обязан бить"
    );
}
