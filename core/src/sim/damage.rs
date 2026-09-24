//! Урон: проверка заявки клиента и применение с резистами.
//!
//! Клиент стреляет мгновенно (иначе игра ощущается вязкой) и присылает заявку
//! «попал в такого-то». Сервер её не принимает на веру: проверяет, что цель жива,
//! что оружие успело перезарядиться и что до цели действительно можно достать
//! (docs/MULTIPLAYER.md §5).

use crate::sim::{event, Event, HitClaim, State};
use crate::weapon::{weapon_def, WeaponId};

/// Запас к дальности оружия: позиция цели у клиента чуть отличается из-за
/// интерполяции, и отказывать за пару метров — злить игрока на ровном месте.
const RANGE_TOLERANCE: f32 = 1.25;
const RANGE_SLACK: f32 = 2.0;
/// Скидка на кулдаун: сеть даёт джиттер, строгое сравнение резало бы честные выстрелы.
const COOLDOWN_TOLERANCE: f32 = 0.8;

/// Применить заявку о попадании. Возвращает события для рассылки.
pub fn apply_hit(state: &mut State, peer: u16, claim: &HitClaim) -> Vec<Event> {
    let mut events = Vec::new();

    let Some(weapon) = WeaponId::from_id(&claim.weapon) else {
        return events;
    };
    let def = weapon_def(weapon);

    let Some(player) = state.players.get(&peer) else {
        return events;
    };
    let shooter_pos = player.pos;
    let last_fire = player.last_fire;
    if player.downed() {
        return events;
    }
    // Темп стрельбы: клиент не может слать попадания чаще, чем позволяет оружие.
    if state.time - last_fire < def.cooldown * COOLDOWN_TOLERANCE {
        return events;
    }

    let Some(enemy) = state.enemies.iter_mut().find(|e| e.id == claim.target) else {
        return events;
    };
    if !enemy.alive() {
        return events;
    }

    let distance = (enemy.pos - shooter_pos).length();
    if distance > def.range * RANGE_TOLERANCE + RANGE_SLACK {
        // Заявка не проходит по дальности — молча игнорируем: это либо лаг,
        // либо попытка стрелять через полкарты.
        if let Some(player) = state.players.get_mut(&peer) {
            player.violations += 1;
        }
        return events;
    }

    let resist = enemy.resist[def.dmg_type.idx()];
    let base = (def.damage * (1.0 - resist).clamp(0.0, 2.0)).max(0.0);
    // Крит роллится детерминированно (общий rng состояния), чтобы клиент и сервер
    // из одного сида видели одинаковые криты (docs/MULTIPLAYER.md §5).
    let crit = def.crit_chance > 0.0 && state.rng.chance(def.crit_chance);
    let amount = if crit { base * def.crit_mult } else { base };

    let Some(enemy) = state.enemies.iter_mut().find(|e| e.id == claim.target) else {
        return events;
    };
    enemy.hp -= amount;
    // Кто бьёт — тот и получает внимание врага.
    enemy.add_threat(peer, amount);

    let mut damage = Event::new(event::DAMAGE);
    damage.actor = peer;
    damage.target = enemy.id;
    damage.amount = amount;
    damage.pos = Some(enemy.pos);
    if crit {
        // Флаг крита едет в extra — клиент показывает крупнее цифры/эффект.
        damage.extra = Some(serde_json::json!({ "crit": true }));
    }
    events.push(damage);

    if !enemy.alive() {
        let (enemy_id, enemy_pos, xp) = (enemy.id, enemy.pos, enemy.xp);
        // Лут роллится сразу: каждому живому игроку — свой бросок.
        let loot_events = state.drop_loot(enemy_pos);

        let mut died = Event::new(event::ENEMY_DIED);
        died.actor = peer;
        died.target = enemy_id;
        died.pos = Some(enemy_pos);
        events.push(died);

        let mut reward = Event::new(event::XP);
        reward.actor = peer;
        reward.amount = xp;
        events.push(reward);
        events.extend(loot_events);
    }

    if let Some(player) = state.players.get_mut(&peer) {
        player.last_fire = state.time;
    }
    events
}
