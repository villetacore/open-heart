//! Player skills: shared definitions, cooldowns and targeting for offline/online play.
use crate::math::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
pub struct AbilityDef {
    pub id: String,
    pub class: usize,
    pub slot: usize,
    pub name_ru: String,
    pub name_en: String,
    pub desc_ru: String,
    pub desc_en: String,
    pub cooldown: f32,
    pub range: f32,
    pub cone: f32,
    pub damage: f32,
    pub damage_type: String,
    pub max_targets: usize,
    pub heal_per_hit: f32,
    pub duration: f32,
    pub speed: f32,
    pub guard: f32,
    pub slow: f32,
    pub icon: usize,
}

impl AbilityDef {
    pub fn name(&self, lang: &str) -> &str {
        if lang == "en" { &self.name_en } else { &self.name_ru }
    }
    pub fn description(&self, lang: &str) -> &str {
        if lang == "en" { &self.desc_en } else { &self.desc_ru }
    }
    pub fn hits(&self, origin: Vec3, forward: Vec3, target: Vec3) -> bool {
        let delta = target - origin;
        delta.length() <= self.range
            && (delta.length_flat() < 0.01 || delta.flat().normalized().dot(forward.flat().normalized()) >= self.cone)
    }
}

pub fn parse(text: &str) -> Result<Vec<AbilityDef>, String> {
    let defs: Vec<AbilityDef> = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let mut ids = std::collections::HashSet::new();
    let mut slots = std::collections::HashSet::new();
    for d in &defs {
        let localized = [&d.name_ru, &d.name_en, &d.desc_ru, &d.desc_en]
            .iter()
            .all(|s| !s.trim().is_empty());
        if d.class > 2 || d.slot > 1 || !ids.insert(&d.id) || !slots.insert((d.class, d.slot))
            || !d.cooldown.is_finite() || d.cooldown <= 0.0 || !(0.0..=30.0).contains(&d.range)
            || !(-1.0..=1.0).contains(&d.cone) || !(0.0..=200.0).contains(&d.damage)
            || !(0.0..=30.0).contains(&d.duration) || !(1.0..=2.0).contains(&d.speed)
            || !(0.0..=0.8).contains(&d.guard) || !(0.0..=0.8).contains(&d.slow)
            || !(0.0..=10.0).contains(&d.heal_per_hit) || d.max_targets > 8
            || d.icon >= 6 || crate::weapon::DmgType::from_id(&d.damage_type).is_none()
            || !localized
        {
            return Err(format!("invalid player ability: {}", d.id));
        }
    }
    if slots.len() != 6 { return Err("expected two abilities for each class".into()); }
    Ok(defs)
}

pub fn defaults() -> Vec<AbilityDef> {
    parse(include_str!("../../../game/presets/core/player_abilities.json")).expect("built-in abilities")
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AbilityState {
    pub cooldowns: [f32; 2],
    pub guard: f32,
    pub guard_time: f32,
    pub speed: f32,
    pub speed_time: f32,
}

impl AbilityState {
    pub fn tick(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 { return; }
        for c in &mut self.cooldowns { *c = (*c - dt).max(0.0); }
        self.guard_time = (self.guard_time - dt).max(0.0);
        self.speed_time = (self.speed_time - dt).max(0.0);
    }
    /// Активировать умение. `cd_mult` масштабирует только время восстановления
    /// (спек/перки на кулдаун), но не длительность эффекта guard/speed. 1.0 — без
    /// изменений; в сетевом режиме сервер и клиент используют 1.0, чтобы кулдаун
    /// был одинаков у всех (спек-модификаторы умений — пока только offline).
    pub fn activate(&mut self, def: &AbilityDef, cd_mult: f32, alive: bool, stunned: bool) -> bool {
        if !alive || stunned || self.cooldowns[def.slot] > 0.0 { return false; }
        let cd_mult = if cd_mult.is_finite() { cd_mult.clamp(0.1, 3.0) } else { 1.0 };
        self.cooldowns[def.slot] = (def.cooldown * cd_mult).max(0.1);
        if def.guard > 0.0 { self.guard = def.guard; self.guard_time = def.duration; }
        if def.speed > 1.0 { self.speed = def.speed; self.speed_time = def.duration; }
        true
    }
    pub fn damage_scale(&self) -> f32 {
        if self.guard_time > 0.0 { 1.0 - self.guard } else { 1.0 }
    }
    pub fn speed_scale(&self) -> f32 {
        if self.speed_time > 0.0 { self.speed } else { 1.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cooldown_and_death_gate_effects() {
        let d = defaults().remove(3);
        let mut s = AbilityState::default();
        assert!(!s.activate(&d, 1.0, false, false));
        assert!(!s.activate(&d, 1.0, true, true));
        assert!(s.activate(&d, 1.0, true, false));
        assert!(!s.activate(&d, 1.0, true, false));
        assert!((s.damage_scale() - 0.4).abs() < 0.001);
        s.tick(d.duration);
        assert_eq!(s.damage_scale(), 1.0);
        assert!(!s.activate(&d, 1.0, true, false));
        s.tick(d.cooldown);
        assert!(s.activate(&d, 1.0, true, false));
    }

    #[test]
    fn cooldown_scales_with_multiplier_but_effect_duration_does_not() {
        let d = defaults().remove(0); // blood_dash: cooldown 7, speed buff duration 2
        let mut fast = AbilityState::default();
        assert!(fast.activate(&d, 0.5, true, false));
        assert!((fast.cooldowns[d.slot] - d.cooldown * 0.5).abs() < 0.001, "cd_mult must scale cooldown");
        assert!(fast.speed_time > 0.0 && (fast.speed_time - d.duration).abs() < 0.001, "effect duration must ignore cd_mult");
        // faster spec recovers sooner than the base cooldown
        fast.tick(d.cooldown * 0.5);
        assert!(fast.activate(&d, 0.5, true, false));
        // clamp guards against absurd data
        let mut s = AbilityState::default();
        assert!(s.activate(&d, 0.0, true, false));
        assert!(s.cooldowns[d.slot] >= 0.1);
    }
    #[test]
    fn cone_excludes_rear_and_other_floors() {
        let d = defaults().remove(4);
        assert!(d.hits(Vec3::ZERO, Vec3::new(0.0,0.0,-1.0), Vec3::new(0.0,0.0,-5.0)));
        assert!(!d.hits(Vec3::ZERO, Vec3::new(0.0,0.0,-1.0), Vec3::new(0.0,0.0,5.0)));
        assert!(!d.hits(Vec3::ZERO, Vec3::new(0.0,0.0,-1.0), Vec3::new(0.0,20.0,-5.0)));
    }
    #[test]
    fn content_has_unique_complete_class_loadouts() {
        assert_eq!(defaults().len(), 6);
        let bad = include_str!("../../../game/presets/core/player_abilities.json").replace("\"cooldown\":7.0", "\"cooldown\":0.0");
        assert!(parse(&bad).is_err());
    }

    #[test]
    fn rejects_empty_localized_text() {
        let src = include_str!("../../../game/presets/core/player_abilities.json");
        assert!(parse(&src.replace("\"desc_en\":\"Strike ahead and move faster for 2 seconds. F\"", "\"desc_en\":\"\"")).is_err(),
            "empty English description must be rejected");
        assert!(parse(&src.replace("\"name_ru\":\"Кровавый порыв\"", "\"name_ru\":\"   \"")).is_err(),
            "blank Russian name must be rejected");
    }
}
