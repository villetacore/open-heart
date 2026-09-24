//! Проверки авторитетной симуляции: враги, урон, правдоподобие движения.

use crate::math::Vec3;
use crate::sim::tests_support::{delve_state as delve, preset_base};
use crate::sim::{event, Config, HitClaim, Input, State};

#[test]
fn delve_spawns_enemies_from_preset() {
    let state = delve(0xD00D);
    assert!(
        !state.enemies.is_empty(),
        "в делве не появилось ни одного врага"
    );
    assert!(
        state.enemies.iter().any(|e| e.is_boss),
        "нет босса — забег нечем заканчивать"
    );
    for enemy in &state.enemies {
        assert!(enemy.hp > 0.0 && enemy.max_hp > 0.0, "враг без здоровья");
        assert!(enemy.id >= 1000, "id врага пересекается с peer игрока");
        assert!(
            state.world.walkable_at(enemy.pos),
            "враг {} появился вне проходимого пола",
            enemy.kind
        );
    }
}

#[test]
fn same_seed_same_enemies() {
    let a = delve(777);
    let b = delve(777);
    assert_eq!(a.enemies.len(), b.enemies.len());
    for (x, y) in a.enemies.iter().zip(b.enemies.iter()) {
        assert_eq!(x.kind, y.kind);
        assert_eq!(x.pos, y.pos);
    }
}

#[test]
fn enemy_walks_towards_player() {
    let mut state = delve(0xA11CE);
    state.join(1, "vasya".into());

    // Берём врага, который точно должен побежать: радиус погони больше дистанции,
    // а бить с неё он ещё не может.
    let index = state
        .enemies
        .iter()
        .position(|e| e.chase_range >= 8.0 && e.attack_range <= 4.0)
        .expect("в пресете нет врага с погоней");
    let enemy_pos = state.enemies[index].pos;

    // Ставим игрока на проходимую клетку рядом — иначе пути до него не будет.
    let start = [(6.0, 0.0), (-6.0, 0.0), (0.0, 6.0), (0.0, -6.0), (4.0, 4.0)]
        .into_iter()
        .map(|(dx, dz)| Vec3::new(enemy_pos.x + dx, enemy_pos.y, enemy_pos.z + dz))
        .find(|p| state.world.walkable_at(*p))
        .expect("вокруг врага нет проходимых клеток");
    state.players.get_mut(&1).unwrap().pos = start;

    let before = (state.enemies[index].pos - start).length_flat();
    for _ in 0..40 {
        state.tick(0.05, &[]);
        // Тест про движение, а не про выживание: держим игрока на ногах,
        // иначе враги свалят его и потеряют цель.
        let player = state.players.get_mut(&1).unwrap();
        player.hp = player.max_hp;
        player.flags = 0;
        player.pos = start;
    }
    let after = (state.enemies[index].pos - start).length_flat();

    assert_eq!(
        state.enemies[index].target,
        Some(1),
        "враг не взял игрока на прицел"
    );
    assert!(
        after < before,
        "враг не приблизился к игроку: было {before:.2}, стало {after:.2}"
    );
}

#[test]
fn enemy_damages_player_in_melee() {
    let mut state = delve(0xB0B);
    state.join(1, "vasya".into());
    let target = state.enemies[0].pos;
    state.players.get_mut(&1).unwrap().pos = target;

    let mut damaged = false;
    for _ in 0..40 {
        let result = state.tick(0.05, &[]);
        if result.events.iter().any(|e| e.kind == event::DAMAGE) {
            damaged = true;
            break;
        }
    }
    assert!(damaged, "враг вплотную так и не ударил");
    assert!(
        state.players[&1].hp < 100.0,
        "здоровье игрока не уменьшилось"
    );
}

#[test]
fn hit_claim_damages_and_kills() {
    let mut state = delve(0xC0FFEE);
    state.join(1, "shooter".into());

    let enemy_id = state.enemies[0].id;
    let enemy_pos = state.enemies[0].pos;
    state.players.get_mut(&1).unwrap().pos = enemy_pos + Vec3::new(2.0, 0.0, 0.0);

    let claim = HitClaim {
        tick: 1,
        weapon: "pistol".into(),
        target: enemy_id,
        pos: enemy_pos,
        part: 0,
    };

    let hp_before = state.enemies[0].hp;
    let events = crate::sim::damage::apply_hit(&mut state, 1, &claim);
    assert!(events.iter().any(|e| e.kind == event::DAMAGE));
    assert!(state.enemies[0].hp < hp_before, "урон не применился");

    // Добиваем: время двигаем, чтобы кулдаун оружия не резал выстрелы.
    for _ in 0..200 {
        if state.enemies.first().map(|e| !e.alive()).unwrap_or(true) {
            break;
        }
        state.time += 1.0;
        crate::sim::damage::apply_hit(&mut state, 1, &claim);
    }
    let died = {
        state.time += 1.0;
        let events = crate::sim::damage::apply_hit(&mut state, 1, &claim);
        events.iter().any(|e| e.kind == event::ENEMY_DIED)
    } || state.enemies[0].hp <= 0.0;
    assert!(died, "враг не умер после серии попаданий");
}

#[test]
fn crit_multiplies_damage_and_flags_event() {
    // Пистолет в core-пресете имеет crit_chance > 0 — стреляя достаточно много
    // раз по бессмертному манекену, обязаны увидеть и криты, и обычные попадания.
    use crate::weapon::{weapon_def, WeaponId};

    let mut state = delve(0xCA11_AB1E);
    state.join(1, "crit".into());

    let enemy_id = state.enemies[0].id;
    let enemy_pos = state.enemies[0].pos;
    // Делаем цель практически бессмертной, чтобы набрать выборку попаданий.
    state.enemies[0].hp = 1.0e9;
    state.enemies[0].max_hp = 1.0e9;
    state.enemies[0].resist = [0.0; 4];
    state.players.get_mut(&1).unwrap().pos = enemy_pos + Vec3::new(2.0, 0.0, 0.0);

    let def = weapon_def(WeaponId::Pistol);
    assert!(def.crit_chance > 0.0, "пистолет в пресете без крита — тест невалиден");
    let base = def.damage; // resist обнулён выше
    let crit_amount = base * def.crit_mult;

    let claim = HitClaim {
        tick: 1,
        weapon: "pistol".into(),
        target: enemy_id,
        pos: enemy_pos,
        part: 0,
    };

    let mut saw_crit = false;
    let mut saw_normal = false;
    for _ in 0..400 {
        state.time += 1.0; // сброс кулдауна
        let events = crate::sim::damage::apply_hit(&mut state, 1, &claim);
        let dmg = events
            .iter()
            .find(|e| e.kind == event::DAMAGE)
            .expect("попадание не засчитано");
        let flagged = dmg
            .extra
            .as_ref()
            .and_then(|v| v.get("crit"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if flagged {
            saw_crit = true;
            assert!(
                (dmg.amount - crit_amount).abs() < 0.01,
                "крит-урон {} не равен base*mult {}",
                dmg.amount,
                crit_amount
            );
        } else {
            saw_normal = true;
            assert!(
                (dmg.amount - base).abs() < 0.01,
                "обычный урон {} не равен base {}",
                dmg.amount,
                base
            );
        }
    }
    assert!(saw_crit, "за 400 выстрелов ни одного крита");
    assert!(saw_normal, "все выстрелы оказались критами — подозрительно");
}

#[test]
fn crit_rolls_are_deterministic_per_seed() {
    // Один сид — одинаковая последовательность критов (клиент и сервер не расходятся).
    fn crit_sequence(seed: u64) -> Vec<bool> {
        let mut state = delve(seed);
        state.join(1, "det".into());
        let enemy_id = state.enemies[0].id;
        let enemy_pos = state.enemies[0].pos;
        state.enemies[0].hp = 1.0e9;
        state.enemies[0].max_hp = 1.0e9;
        state.enemies[0].resist = [0.0; 4];
        state.players.get_mut(&1).unwrap().pos = enemy_pos + Vec3::new(2.0, 0.0, 0.0);
        let claim = HitClaim {
            tick: 1,
            weapon: "pistol".into(),
            target: enemy_id,
            pos: enemy_pos,
            part: 0,
        };
        let mut seq = Vec::new();
        for _ in 0..64 {
            state.time += 1.0;
            let events = crate::sim::damage::apply_hit(&mut state, 1, &claim);
            let flagged = events
                .iter()
                .find(|e| e.kind == event::DAMAGE)
                .and_then(|e| e.extra.as_ref())
                .and_then(|v| v.get("crit"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            seq.push(flagged);
        }
        seq
    }
    assert_eq!(crit_sequence(0x5EED), crit_sequence(0x5EED));
}

#[test]
fn hit_claim_from_across_the_map_is_rejected() {
    let mut state = delve(42);
    state.join(1, "cheater".into());

    let enemy_id = state.enemies[0].id;
    let enemy_pos = state.enemies[0].pos;
    state.players.get_mut(&1).unwrap().pos = enemy_pos + Vec3::new(500.0, 0.0, 500.0);

    let hp_before = state.enemies[0].hp;
    let events = crate::sim::damage::apply_hit(
        &mut state,
        1,
        &HitClaim {
            tick: 1,
            weapon: "pistol".into(),
            target: enemy_id,
            pos: enemy_pos,
            part: 0,
        },
    );
    assert!(events.is_empty(), "выстрел через полкарты засчитан");
    assert_eq!(state.enemies[0].hp, hp_before);
    assert_eq!(state.players[&1].violations, 1);
}

#[test]
fn snapshot_carries_enemies() {
    let mut state = delve(5);
    state.join(1, "vasya".into());
    let result = state.tick(0.05, &[]);
    assert_eq!(result.snapshot.players.len(), 1);
    assert!(
        !result.snapshot.enemies.is_empty(),
        "снапшот без врагов — клиенту нечего рисовать"
    );
    let enemy = &result.snapshot.enemies[0];
    assert_eq!(enemy.kind, crate::sim::kind::ENEMY);
    assert!(enemy.hp > 0);
}

#[test]
fn player_spawns_at_world_entry() {
    let mut state = delve(9);
    state.join(1, "vasya".into());
    assert_eq!(state.players[&1].pos, state.world.player_spawn);

    // Первый ввод рядом с точкой появления принимается, телепорт — нет.
    let near = state.world.player_spawn + Vec3::new(0.3, 0.0, 0.0);
    state.apply_input(1, Input { pos: near, ..Default::default() }, 0.05);
    assert_eq!(state.players[&1].pos, near);

    let far = near + Vec3::new(300.0, 0.0, 0.0);
    state.apply_input(1, Input { pos: far, ..Default::default() }, 0.05);
    assert_eq!(state.players[&1].pos, near, "телепорт принят");
}

#[test]
fn wire_field_names_match_schema() {
    let json = serde_json::to_string(&crate::sim::TaggedInput {
        peer: 7,
        input: Input { tick: 3, ..Default::default() },
    })
    .unwrap();
    assert!(json.contains("\"peer\":7"), "{json}");
    assert!(json.contains("\"in\":"), "{json}");
    assert!(json.contains("\"move\":[0.0,0.0]"), "{json}");
}

#[test]
fn hub_room_join_and_leave() {
    let mut state = State::new(Config {
        preset: "core".into(),
        kind: "hub".into(),
        seed: 42,
        preset_base: preset_base(),
        ..Default::default()
    });
    state.join(1, "vasya".into());
    let result = state.tick(0.05, &[]);
    assert_eq!(result.snapshot.players.len(), 1);
    assert!(result.snapshot.enemies.is_empty(), "в хабе врагов быть не должно");

    state.leave(1);
    assert!(state.tick(0.05, &[]).snapshot.players.is_empty());
}
