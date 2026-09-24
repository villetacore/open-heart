use super::{event, Event, State};
use crate::math::Vec3;
use serde_json::Value;

impl State {
    fn ability_visible(&self, from: Vec3, to: Vec3) -> bool {
        let steps = (from.distance(to) / 0.25).ceil().max(1.0) as usize;
        (0..=steps).all(|i| self.world.walkable_at(from.lerp(to, i as f32 / steps as f32)))
    }

    pub(super) fn cast_ability(&mut self, peer: u16, args: Option<&Value>) -> Vec<Event> {
        let Some(slot) = args.and_then(|v| v.get("slot")).and_then(Value::as_u64).filter(|s| *s < 2) else { return vec![]; };
        let game = self.game();
        let Some(player) = self.players.get_mut(&peer) else { return vec![]; };
        let Some(def) = game.player_abilities.iter().find(|d| Some(d.class) == player.class && d.slot == slot as usize) else { return vec![]; };
        // Сервер спек-нейтрален по умолчанию (спек не в модели sim): кулдаун умений
        // одинаков у всех, спек-модификаторы кулдауна применяются пока только offline.
        if !player.abilities.activate(def, 1.0, player.hp > 0.0 && !player.out, player.flags & super::flags::STUNNED != 0) { return vec![]; }
        let origin = player.pos;
        let forward = Vec3::new(-player.yaw.sin(), 0.0, -player.yaw.cos());
        let mut candidates: Vec<_> = self.enemies.iter().filter(|e| e.alive()
            && def.hits(origin, forward, e.pos) && self.ability_visible(origin, e.pos))
            .map(|e| (e.id, origin.distance(e.pos))).collect();
        candidates.sort_by(|a,b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        candidates.truncate(def.max_targets);
        let mut events = Vec::new();
        let mut deaths = Vec::new();
        let mut hit_count = 0;
        for (id, _) in candidates {
            let enemy = self.enemies.iter_mut().find(|e| e.id == id).unwrap();
            let ty = crate::weapon::DmgType::from_id(&def.damage_type).unwrap();
            let amount = (def.damage * (1.0 - enemy.resist[ty.idx()]).clamp(0.0, 2.0)).min(enemy.hp);
            if amount > 0.0 { hit_count += 1; }
            enemy.hp -= amount;
            enemy.add_threat(peer, amount);
            enemy.slow_time = enemy.slow_time.max(def.duration);
            enemy.slow_amount = if enemy.is_boss { def.slow.min(0.25) } else { def.slow };
            events.push(Event { actor: peer, target: id, amount, pos: Some(enemy.pos), ..Event::new(event::DAMAGE) });
            if !enemy.alive() { deaths.push((id, enemy.pos, enemy.xp)); }
        }
        if let Some(player) = self.players.get_mut(&peer) {
            player.hp = (player.hp + hit_count as f32 * def.heal_per_hit).min(player.max_hp);
        }
        events.push(Event { actor: peer, text: def.id.clone(), pos: Some(origin),
            extra: Some(serde_json::json!({"slot":slot, "cooldown":def.cooldown})), ..Event::new("ability_cast") });
        for (id, pos, xp) in deaths {
            events.push(Event { actor: peer, target: id, pos: Some(pos), ..Event::new(event::ENEMY_DIED) });
            events.push(Event { actor: peer, amount: xp, ..Event::new(event::XP) });
            events.extend(self.drop_loot(pos));
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn server_enforces_class_cooldown_and_death() {
        let mut s = State::new(super::super::Config::default());
        s.join(1, "test".into());
        let args = serde_json::json!({"slot":1});
        assert!(s.cast_ability(1, Some(&args)).is_empty());
        s.players.get_mut(&1).unwrap().class = Some(1);
        assert_eq!(s.cast_ability(1, Some(&args)).len(), 1);
        assert!(s.cast_ability(1, Some(&args)).is_empty());
        s.players.get_mut(&1).unwrap().abilities.tick(20.0);
        s.players.get_mut(&1).unwrap().hp = 0.0;
        assert!(s.cast_ability(1, Some(&args)).is_empty());
    }
}
