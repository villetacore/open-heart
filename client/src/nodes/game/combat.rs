//! Боёвка, эффекты, урон от врагов и статусы игрока.

use super::*;
use crate::weapon::WeaponModDef;

fn load_sfx_stream(path: &str) -> Option<Gd<AudioStream>> {
    load_pcm_wav(path).or_else(|| godot::tools::try_load::<AudioStream>(path).ok())
}

#[cfg(debug_assertions)]
pub(super) fn weapon_sfx_loadable(path: &str) -> bool {
    load_sfx_stream(path).is_some()
}

fn load_pcm_wav(path: &str) -> Option<Gd<AudioStream>> {
    let mut file = FileAccess::open(path, ModeFlags::READ)?;
    let length = i64::try_from(file.get_length()).ok()?;
    let bytes = file.get_buffer(length);
    let bytes = bytes.as_slice();
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }

    let mut cursor = 12usize;
    let mut audio_format = None;
    let mut channels = None;
    let mut mix_rate = None;
    let mut bits_per_sample = None;
    let mut sample_data = None;

    while cursor.checked_add(8)? <= bytes.len() {
        let chunk_id = &bytes[cursor..cursor + 4];
        let chunk_size =
            u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().ok()?) as usize;
        let data_start = cursor + 8;
        let data_end = data_start.checked_add(chunk_size)?;
        if data_end > bytes.len() {
            return None;
        }

        match chunk_id {
            b"fmt " if chunk_size >= 16 => {
                audio_format = Some(u16::from_le_bytes(
                    bytes[data_start..data_start + 2].try_into().ok()?,
                ));
                channels = Some(u16::from_le_bytes(
                    bytes[data_start + 2..data_start + 4].try_into().ok()?,
                ));
                mix_rate = Some(u32::from_le_bytes(
                    bytes[data_start + 4..data_start + 8].try_into().ok()?,
                ));
                bits_per_sample = Some(u16::from_le_bytes(
                    bytes[data_start + 14..data_start + 16].try_into().ok()?,
                ));
            }
            b"data" => sample_data = Some(&bytes[data_start..data_end]),
            _ => {}
        }

        cursor = data_end.checked_add(chunk_size & 1)?;
    }

    if audio_format? != 1 {
        return None;
    }
    let stereo = match channels? {
        1 => false,
        2 => true,
        _ => return None,
    };
    let format = match bits_per_sample? {
        8 => WavFormat::FORMAT_8_BITS,
        16 => WavFormat::FORMAT_16_BITS,
        _ => return None,
    };
    let mix_rate = i32::try_from(mix_rate?).ok()?;
    let data = PackedByteArray::from(sample_data?);

    let mut stream = AudioStreamWav::new_gd();
    stream.set_data(&data);
    stream.set_format(format);
    stream.set_mix_rate(mix_rate);
    stream.set_stereo(stereo);
    Some(stream.upcast::<AudioStream>())
}

impl Game3D {
    fn equipped_weapon_mod(&self, weapon: WeaponId) -> Option<&'static WeaponModDef> {
        self.state
            .as_ref()
            .and_then(|state| weapon_mod_for(weapon, state.weapon_mods[weapon.slot()]))
    }

    fn blend_feedback_color(base: [f32; 3], tint: [f32; 3], amount: f32) -> [f32; 3] {
        [
            base[0] + (tint[0] - base[0]) * amount,
            base[1] + (tint[1] - base[1]) * amount,
            base[2] + (tint[2] - base[2]) * amount,
        ]
    }

    pub(super) fn effective_weapon_feedback(&self, weapon: WeaponId) -> WeaponFeedback {
        let mut feedback = weapon_def(weapon).feedback;
        let Some(weapon_mod) = self.equipped_weapon_mod(weapon) else {
            return feedback;
        };
        let profile = weapon_mod.feedback;
        feedback.muzzle_color =
            Self::blend_feedback_color(feedback.muzzle_color, profile.tint, profile.color_mix);
        feedback.impact_color =
            Self::blend_feedback_color(feedback.impact_color, profile.tint, profile.color_mix);
        feedback.tracer_color =
            Self::blend_feedback_color(feedback.tracer_color, profile.tint, profile.color_mix);
        feedback.muzzle_energy *= profile.muzzle_mult;
        feedback.muzzle_range *= profile.muzzle_mult.sqrt();
        feedback.impact_scale *= profile.impact_mult;
        feedback.impact_energy *= profile.impact_mult;
        feedback.tracer_scale *= profile.tracer_mult;
        feedback
    }

    pub(super) fn effective_weapon_pitch(&self, weapon: WeaponId) -> f32 {
        self.equipped_weapon_mod(weapon)
            .map(|weapon_mod| weapon_mod.feedback.pitch_mult)
            .unwrap_or(1.0)
    }

    pub(super) fn effective_weapon_bloom(&self) -> f32 {
        let accuracy = weapon_def(self.arsenal.current).accuracy;
        let movement = self.player()
            .map(|player| {
                let velocity = player.get_velocity();
                Vector2::new(velocity.x, velocity.z).length() / 8.0
            })
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        (self.weapon_bloom + movement * accuracy.move_penalty).min(accuracy.max_bloom)
    }

    pub(super) fn add_weapon_bloom(&mut self, amount: f32) {
        let max_bloom = weapon_def(self.arsenal.current).accuracy.max_bloom;
        self.weapon_bloom = (self.weapon_bloom + amount).min(max_bloom);
    }

    pub(super) fn register_weapon_hit(&mut self, killed: bool, critical: bool) {
        self.hit_confirm_timer = self.hit_confirm_timer.max(0.14);
        if critical {
            self.critical_confirm_timer = self.critical_confirm_timer.max(0.2);
        }
        if killed {
            self.kill_confirm_timer = self.kill_confirm_timer.max(0.24);
        }
    }

    pub(super) fn tick_weapon_accuracy(&mut self, dt: f32) {
        let recovery = weapon_def(self.arsenal.current).accuracy.recovery;
        self.weapon_bloom = (self.weapon_bloom - recovery * dt).max(0.0);
        self.hit_confirm_timer = (self.hit_confirm_timer - dt).max(0.0);
        self.critical_confirm_timer = (self.critical_confirm_timer - dt).max(0.0);
        self.kill_confirm_timer = (self.kill_confirm_timer - dt).max(0.0);
        self.update_crosshair();
    }

    pub(super) fn try_reload(&mut self) {
        if !matches!(self.weapon_anim, WeaponAnim::Idle) {
            return;
        }
        let id = self.arsenal.current;
        let def = weapon_def(id);
        if def.magazine == 0 || self.arsenal.clips[id.slot()] >= def.magazine {
            return;
        }
        if self.arsenal.reload(id) == 0 {
            return;
        }
        self.weapon_anim = WeaponAnim::Reload(def.reload_time);
        self.anim_timer = 0.0;
        let pitch_mult = self.effective_weapon_pitch(id);
        self.play_weapon_sfx(
            &def.audio.reload_sfx,
            [
                def.audio.reload_pitch[0] * pitch_mult,
                def.audio.reload_pitch[1] * pitch_mult,
            ],
            def.audio.reload_volume_db,
        );
        if let Some(frame) = def.reload_frames.first() {
            self.set_weapon_frame(*frame);
        }
    }

    // ── Боёвка ───────────────────────────────────────────────────────────────

    pub(super) fn player_aim(&self) -> Option<(Vector3, Vector3)> {
        let p = self.player()?;
        let pl = p.clone().try_cast::<Player>().ok()?;
        let b = pl.bind();
        Some((b.eye_pos(), b.aim_dir()))
    }

    pub(super) fn try_fire(&mut self) {
        self.try_fire_mode(false);
    }

    pub(super) fn try_alt_fire(&mut self) {
        self.try_fire_mode(true);
    }

    fn try_fire_mode(&mut self, secondary: bool) {
        let cur = self.arsenal.current;
        let def = weapon_def(cur);
        let alt = if secondary {
            let Some(alt) = def.alt_fire.as_ref() else {
                return;
            };
            Some(alt)
        } else {
            None
        };
        let fire_kind = alt.map(|mode| mode.kind).unwrap_or(def.kind);
        let damage_mult = alt.map(|mode| mode.damage_mult).unwrap_or(1.0);
        let cooldown_mult = alt.map(|mode| mode.cooldown_mult).unwrap_or(1.0);
        let range_mult = alt.map(|mode| mode.range_mult).unwrap_or(1.0);
        let recoil_mult = alt.map(|mode| mode.recoil_mult).unwrap_or(1.0);
        let bloom_mult = alt.map(|mode| mode.bloom_mult).unwrap_or(1.0);
        let equipped_mod = self
            .state
            .as_ref()
            .and_then(|state| weapon_mod_for(cur, state.weapon_mods[cur.slot()]));
        let mod_damage = equipped_mod.map(|entry| entry.damage_mult).unwrap_or(1.0);
        let mod_cooldown = equipped_mod.map(|entry| entry.cooldown_mult).unwrap_or(1.0);
        let mod_range = equipped_mod.map(|entry| entry.range_mult).unwrap_or(1.0);
        let mod_recoil = equipped_mod.map(|entry| entry.recoil_mult).unwrap_or(1.0);
        let mod_bloom = equipped_mod.map(|entry| entry.bloom_mult).unwrap_or(1.0);

        if !self.arsenal.can_fire(cur) {
            if def.magazine > 0
                && def
                    .ammo
                    .map(|(ammo_type, _)| self.arsenal.ammo_of(ammo_type) > 0)
                    .unwrap_or(false)
            {
                self.try_reload();
                return;
            }
            self.shoot_cd = 0.35;
            self.show_flash(&format!(
                "{}: {}",
                if self.settings.lang == "en" {
                    "No ammunition"
                } else {
                    "Нет боеприпасов"
                },
                def.ammo
                    .map(|(t, _)| t.name(&self.settings.lang))
                    .unwrap_or("—")
            ));
            return;
        }

        if !matches!(self.weapon_anim, WeaponAnim::Idle | WeaponAnim::Fire { .. }) {
            return;
        }
        self.shoot_cd = def.cooldown * cooldown_mult * mod_cooldown * self.loadout.cd_mult;
        self.arsenal.consume(cur);
        let fire_frame = 0;
        self.weapon_anim = WeaponAnim::Fire {
            frame: fire_frame,
            secondary,
        };
        self.anim_timer = 0.0;
        self.set_weapon_frame(def.attack_frames(secondary)[fire_frame]);
        self.flash_muzzle(self.effective_weapon_feedback(cur));
        self.weapon_impact_sfx_pending = true;
        if let Some(player) = self.player() {
            if let Ok(mut player) = player.clone().try_cast::<Player>() {
                player
                    .bind_mut()
                    .apply_weapon_recoil(def.recoil * recoil_mult * mod_recoil);
            }
        }

        // звук выстрела/удара
        let pitch_mult = self.effective_weapon_pitch(cur);
        self.play_weapon_sfx(
            &def.audio.fire_sfx,
            [
                def.audio.fire_pitch[0] * pitch_mult,
                def.audio.fire_pitch[1] * pitch_mult,
            ],
            def.audio.fire_volume_db,
        );

        // шум выстрела будит врагов в округе (мили — тихо, вдвое меньший радиус)
        if let Some(p) = self.player() {
            let pos = p.get_global_position();
            let noise = if matches!(fire_kind, FireKind::Melee) {
                8.0
            } else {
                18.0
            };
            self.alert_enemies(pos, noise);
        }

        let Some((eye, dir)) = self.player_aim() else {
            return;
        };
        // Остальным клиентам нужен наш выстрел — для звука и вспышки оружия.
        if self.net_ready() {
            let weapon = def.id.id().to_string();
            self.net_report_fire(&weapon, eye, dir);
        }
        let dmg = def.damage * damage_mult * mod_damage * self.loadout.dmg_mult;
        let dtype = def.dmg_type;
        let range = def.range * range_mult * mod_range;
        let spread_multiplier = 1.0 + self.effective_weapon_bloom();

        match fire_kind {
            FireKind::Melee => {
                let frames = def.attack_frames(secondary);
                let hit_frame = ((frames.len().saturating_sub(1) as f32
                    * def.attack_hit_ratio(secondary))
                .round() as usize)
                    .min(frames.len() - 1);
                self.melee_impact_pending = Some((dmg, range, dtype, hit_frame));
            }
            FireKind::Hitscan { pellets, spread } => {
                for _ in 0..pellets {
                    let sx = (self.rng.f32() - 0.5) * 2.0 * spread * spread_multiplier;
                    let sy = (self.rng.f32() - 0.5) * 2.0 * spread * spread_multiplier;
                    // разброс в плоскости, перпендикулярной взгляду
                    let right = dir.cross(Vector3::UP).normalized();
                    let up = right.cross(dir).normalized();
                    let d = (dir + right * sx + up * sy).normalized();
                    self.fire_ray(eye, d, range, dmg, dtype);
                }
            }
            FireKind::Projectile { speed, splash } => {
                self.spawn_projectile(
                    eye + dir * 0.6,
                    dir * speed,
                    dmg,
                    dtype,
                    splash,
                    range / speed,
                    cur,
                );
            }
        }
        self.add_weapon_bloom(def.accuracy.bloom_per_shot * bloom_mult * mod_bloom);
        if !matches!(fire_kind, FireKind::Melee) {
            self.process_kills();
        }
    }

    pub(super) fn fire_melee(&mut self, dmg: f32, range: f32, dtype: DmgType) {
        let Some((eye, dir)) = self.player_aim() else {
            return;
        };
        let flat_dir = Vector3::new(dir.x, 0.0, dir.z).normalized();
        let mut best: Option<(usize, f32)> = None;
        for (i, e) in self.enemies.iter().enumerate() {
            let eb = e.bind();
            if !eb.alive {
                continue;
            }
            let epos = e.get_global_position();
            let to = Vector3::new(epos.x - eye.x, 0.0, epos.z - eye.z);
            let d = to.length();
            if d > range {
                continue;
            }
            let dot = flat_dir.dot(to.normalized());
            if dot > 0.45 {
                let score = dot / (d + 0.1);
                if best.map(|(_, s)| score > s).unwrap_or(true) {
                    best = Some((i, score));
                }
            }
        }
        if let Some((idx, _)) = best {
            let epos = self.enemies[idx].get_global_position();
            let dealt = self.enemies[idx].bind_mut().take_damage(dmg, dtype);
            self.apply_weapon_status_idx(idx);
            let killed = !self.enemies[idx].bind().alive;
            self.spawn_weapon_impact(
                self.effective_weapon_feedback(self.arsenal.current),
                epos + Vector3::new(0.0, 1.1, 0.0),
                true,
            );
            self.play_pending_weapon_impact(epos + Vector3::new(0.0, 1.1, 0.0));
            self.register_weapon_hit(killed, false);
            // вампиризм — от фактически нанесённого урона
            if self.loadout.lifesteal > 0.0 {
                let heal = dealt * self.loadout.lifesteal;
                if let Some(p) = self.player() {
                    if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                        pl.bind_mut().heal(heal);
                    }
                }
            }
        }
    }

    /// Наложить статус текущего оружия на врага по индексу (после take_damage).
    pub(super) fn apply_weapon_status_idx(&mut self, idx: usize) {
        let wstatus = weapon_def(self.arsenal.current).status.clone();
        if let Some(sc) = self.rolled_enemy_status(&wstatus) {
            if let Some(e) = self.enemies.get(idx) {
                e.clone().bind_mut().apply_status(&sc);
            }
        }
    }

    /// Урон врагу от игрока.
    ///
    /// В одиночной игре считается здесь же; в сетевой — уходит серверу заявкой,
    /// а урон вернётся снапшотом (docs/MULTIPLAYER.md §5).
    pub(super) fn hit_enemy(
        &mut self,
        enemy: &mut Gd<Enemy>,
        hit_position: Vector3,
        damage: f32,
        damage_type: DmgType,
    ) -> (f32, bool) {
        if self.net_ready() {
            let weapon = weapon_def(self.arsenal.current).id.id().to_string();
            self.net_report_hit(enemy, &weapon, hit_position);
            let critical = enemy.bind().weak_point_multiplier_at(hit_position) > 1.0;
            return (0.0, critical);
        }
        // Крит оружия (crit_chance/crit_mult) — тот же расчёт, что в серверном
        // sim::damage; иначе data-driven крит не работал бы в одиночной игре.
        // Стакается с уроном по слабой точке внутри apply_precise_damage.
        let def = weapon_def(self.arsenal.current);
        let weapon_crit = def.crit_chance > 0.0 && self.rng.chance(def.crit_chance);
        let crit_mult = if weapon_crit { def.crit_mult } else { 1.0 };
        let (dealt, weak_crit) =
            Self::apply_precise_damage(enemy, hit_position, damage * crit_mult, damage_type);
        (dealt, weak_crit || weapon_crit)
    }

    pub(super) fn apply_precise_damage(
        enemy: &mut Gd<Enemy>,
        hit_position: Vector3,
        damage: f32,
        damage_type: DmgType,
    ) -> (f32, bool) {
        let multiplier = enemy.bind().weak_point_multiplier_at(hit_position);
        let critical = multiplier > 1.0;
        let dealt = enemy
            .bind_mut()
            .take_damage(damage * multiplier, damage_type);
        (dealt, critical)
    }

    pub(super) fn fire_ray(
        &mut self,
        from: Vector3,
        dir: Vector3,
        range: f32,
        dmg: f32,
        dtype: DmgType,
    ) {
        let to = from + dir * range;
        let hit = self.raycast(from, to);
        let feedback = self.effective_weapon_feedback(self.arsenal.current);
        let tracer_end = hit.as_ref().map(|(pos, _)| *pos).unwrap_or(to);
        self.spawn_weapon_tracer(feedback, from + dir * 0.8, tracer_end);
        if let Some((pos, _)) = hit.as_ref() {
            self.play_pending_weapon_impact(*pos);
        }
        match hit {
            Some((pos, Some(mut enemy))) => {
                let (_, critical) = self.hit_enemy(&mut enemy, pos, dmg, dtype);
                let wstatus = weapon_def(self.arsenal.current).status.clone();
                if let Some(sc) = self.rolled_enemy_status(&wstatus) {
                    enemy.bind_mut().apply_status(&sc);
                }
                let killed = !enemy.bind().alive;
                self.spawn_weapon_impact(feedback, pos, true);
                self.register_weapon_hit(killed, critical);
            }
            Some((pos, None)) => {
                self.spawn_weapon_impact(feedback, pos - dir * 0.1, false);
            }
            None => {}
        }
    }

    /// Луч: возвращает точку и врага (если попали в него).
    pub(super) fn raycast(
        &mut self,
        from: Vector3,
        to: Vector3,
    ) -> Option<(Vector3, Option<Gd<Enemy>>)> {
        let world = self.base().get_world_3d()?;
        let mut space = world.clone().get_direct_space_state()?;
        let mut query = PhysicsRayQueryParameters3D::create(from, to)?;
        if let Some(p) = self.player() {
            let mut excl: godot::builtin::Array<Rid> = godot::builtin::Array::new();
            excl.push(p.get_rid());
            query.set_exclude(&excl);
        }
        let hit = space.intersect_ray(&query);
        if hit.is_empty() {
            return None;
        }
        let pos = hit.get("position")?.try_to::<Vector3>().ok()?;
        let enemy = hit
            .get("collider")
            .and_then(|cv| cv.try_to::<Gd<godot::classes::Node>>().ok())
            .and_then(|n| n.try_cast::<Enemy>().ok());
        Some((pos, enemy))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn spawn_projectile(
        &mut self,
        pos: Vector3,
        vel: Vector3,
        dmg: f32,
        dmg_type: DmgType,
        splash: f32,
        ttl: f32,
        weapon: WeaponId,
    ) {
        let mut node = Node3D::new_alloc();
        node.set_position(pos);
        let feedback = self.effective_weapon_feedback(weapon);
        let projectile_color = Self::feedback_color(feedback.tracer_color);
        let (tex, px, color) = match weapon {
            WeaponId::Rocket => (
                "res://assets/sprites/projectiles/rocket.png",
                0.010,
                Color::WHITE,
            ),
            _ => (
                "res://assets/effects/effect_energy.png",
                0.006 * feedback.tracer_scale.max(0.25),
                projectile_color,
            ),
        };
        if let Some(mut sp) = make_billboard(&mut self.cache, tex, Vector3::ZERO, px) {
            sp.set_modulate(color);
            node.add_child(&sp);
        }
        let l = make_light(
            Vector3::ZERO,
            projectile_color,
            feedback.muzzle_energy.max(0.35) * 0.55,
            feedback.muzzle_range.max(2.0) * 0.75,
        );
        node.add_child(&l);
        self.base_mut().add_child(&node);
        // статус оружия фиксируется на снаряде в момент выстрела
        let status = weapon_def(weapon).status.clone();
        self.projectiles.push(Projectile {
            node,
            pos,
            vel,
            dmg,
            dmg_type,
            splash,
            ttl,
            feedback,
            weapon,
            status,
        });
    }

    pub(super) fn tick_projectiles(&mut self, dt: f32) {
        let mut exploded: Vec<(Vector3, f32, DmgType, f32, WeaponFeedback, WeaponId)> = Vec::new();
        let mut direct_hits: Vec<DirectHit> = Vec::new();

        let mut i = 0;
        while i < self.projectiles.len() {
            let new_pos = self.projectiles[i].pos + self.projectiles[i].vel * dt;
            let from = self.projectiles[i].pos;
            let hit = self.raycast(from, new_pos);
            let mut remove = false;

            match hit {
                Some((pos, Some(enemy))) => {
                    let pr = &self.projectiles[i];
                    if pr.splash > 0.0 {
                        exploded.push((
                            pos,
                            pr.dmg,
                            pr.dmg_type,
                            pr.splash,
                            pr.feedback,
                            pr.weapon,
                        ));
                    } else {
                        direct_hits.push((
                            enemy,
                            pr.dmg,
                            pr.dmg_type,
                            pos,
                            pr.status.clone(),
                            pr.feedback,
                            pr.weapon,
                        ));
                    }
                    remove = true;
                }
                Some((pos, None)) => {
                    let pr = &self.projectiles[i];
                    if pr.splash > 0.0 {
                        exploded.push((
                            pos,
                            pr.dmg,
                            pr.dmg_type,
                            pr.splash,
                            pr.feedback,
                            pr.weapon,
                        ));
                    } else {
                        let feedback = pr.feedback;
                        let weapon = pr.weapon;
                        self.spawn_weapon_impact(feedback, pos, false);
                        self.play_weapon_impact_sfx(weapon, pos);
                    }
                    remove = true;
                }
                None => {
                    self.projectiles[i].pos = new_pos;
                    self.projectiles[i].node.set_position(new_pos);
                    self.projectiles[i].ttl -= dt;
                    if self.projectiles[i].ttl <= 0.0 {
                        remove = true;
                    }
                }
            }

            if remove {
                let p = self.projectiles.remove(i);
                p.node.free();
            } else {
                i += 1;
            }
        }

        for (enemy, dmg, dtype, pos, status, feedback, weapon) in direct_hits {
            let mut e = enemy;
            let (_, critical) = self.hit_enemy(&mut e, pos, dmg, dtype);
            if let Some(sc) = self.rolled_enemy_status(&status) {
                e.bind_mut().apply_status(&sc);
            }
            let killed = !e.bind().alive;
            self.spawn_weapon_impact(feedback, pos, true);
            self.play_weapon_impact_sfx(weapon, pos);
            self.register_weapon_hit(killed, critical);
        }
        for (pos, dmg, dtype, splash, feedback, weapon) in exploded {
            self.play_weapon_impact_sfx(weapon, pos);
            self.explode(pos, dmg, dtype, splash, feedback);
        }
        self.process_kills();
    }

    pub(super) fn explode(
        &mut self,
        pos: Vector3,
        dmg: f32,
        dtype: DmgType,
        radius: f32,
        feedback: WeaponFeedback,
    ) {
        self.spawn_tinted_fx(
            "res://assets/effects/effect_explosion.png",
            pos,
            0.022 * feedback.impact_scale,
            feedback.impact_duration.max(0.28),
            Self::feedback_color(feedback.impact_color),
        );
        self.spawn_light_fx(
            pos,
            Self::feedback_color(feedback.impact_color),
            feedback.impact_energy.max(1.0) * 2.2,
            radius * 2.2,
            feedback.impact_duration.max(0.25),
        );
        self.alert_enemies(pos, 22.0); // взрыв слышно издалека

        let mut hit_enemy = false;
        let mut killed_enemy = false;
        for e in self.enemies.iter_mut() {
            let alive = e.bind().alive;
            if !alive {
                continue;
            }
            let d = (e.get_global_position() - pos).length();
            if d < radius {
                let fall = 1.0 - (d / radius) * 0.55;
                e.bind_mut().take_damage(dmg * fall, dtype);
                hit_enemy = true;
                killed_enemy |= !e.bind().alive;
            }
        }
        if hit_enemy {
            self.register_weapon_hit(killed_enemy, false);
        }
        // самоурон
        if let Some(p) = self.player() {
            let d = (p.get_global_position() - pos).length();
            if d < radius * 0.8 {
                if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                    let fall = 1.0 - d / (radius * 0.8);
                    pl.bind_mut().take_damage(dmg * 0.35 * fall);
                    self.damage_flash_timer = 0.3;
                    self.punch_hit();
                }
            }
        }
    }

    /// Обработка убитых врагов: XP, дроп, эффекты, квест босса.
    pub(super) fn process_kills(&mut self) {
        let mut kills: Vec<KillInfo> = Vec::new();
        let mut i = 0;
        while i < self.enemies.len() {
            let alive = self.enemies[i].bind().alive;
            let death_finished = self.enemies[i].bind().death_timer <= 0.0;
            if !alive && death_finished {
                let pos = self.enemies[i].get_global_position();
                let xp = self.enemies[i].bind().xp_value;
                let is_boss = self.enemies[i].bind().is_boss;
                let kind = self.enemies[i].bind().cfg_id.to_string();
                let blast = self.enemies[i].bind().death_blast;
                kills.push((pos, xp, is_boss, kind, blast));
                let e = self.enemies.remove(i);
                e.free();
            } else {
                i += 1;
            }
        }

        for (pos, xp, is_boss, kind, blast) in kills {
            // «Взрывной» аффикс: посмертный взрыв — игроку полный урон, врагам половину
            if let Some((dmg, radius)) = blast {
                self.enemy_death_blast(pos, dmg, radius);
            }
            self.spawn_fx(
                "res://assets/effects/effect_blood.png",
                pos + Vector3::new(0.0, 0.9, 0.0),
                0.014,
                0.4,
            );
            self.play_sfx_at(&SFX_DEATH, pos + Vector3::new(0.0, 1.0, 0.0));
            self.punch_kill();
            // прогресс kill-квестов
            self.bump_quests("kill", &kind);
            if is_boss {
                self.bump_quests("boss", &kind);
            }

            // XP и уровни
            let levels = {
                let st = self.state.as_mut().unwrap();
                st.add_xp(xp as u32)
            };
            if levels > 0 {
                let (ci, si, lvl) = {
                    let st = self.state.as_ref().unwrap();
                    (st.class_idx.unwrap_or(0), st.spec_idx, st.level)
                };
                self.apply_loadout(ci, si, false);
                // подлечить при апе
                if let Some(p) = self.player() {
                    if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                        let heal = pl.bind().max_hp * 0.35;
                        pl.bind_mut().heal(heal);
                    }
                }
                self.show_flash(&format!(
                    "{} {}!",
                    if self.settings.lang == "en" {
                        "LEVEL"
                    } else {
                        "УРОВЕНЬ"
                    },
                    lvl
                ));
            }

            // дроп
            let in_dungeon = self.loc == Loc::Dungeon;
            // Дроп по таблице kill_drops (loot.json): один бросок, записи
            // кумулятивны по chance, остаток вероятности — «ничего».
            let drops = self
                .cfg
                .as_ref()
                .map(|c| c.loot.kill_drops.clone())
                .unwrap_or_default();
            let roll = self.rng.f32();
            let mut acc = 0.0f32;
            for d in &drops {
                acc += d.chance;
                if roll >= acc {
                    continue;
                }
                match d.kind.as_str() {
                    "ammo" => {
                        let t = AmmoType::from_idx(self.rng.below(4) as usize);
                        self.spawn_ammo_pickup(t, t.pack_size() / 2 + 1, pos, in_dungeon);
                    }
                    _ => {
                        let id = d.id.as_deref().unwrap_or("");
                        if id == "heart_1up" {
                            self.spawn_item_heart(pos, in_dungeon);
                        } else if !id.is_empty() {
                            let cfg = self.cfg.take();
                            if let Some(ref cfg) = cfg {
                                self.spawn_item(cfg, id, pos, in_dungeon);
                            }
                            self.cfg = cfg;
                        }
                    }
                }
                break;
            }

            if is_boss {
                self.boss_alive = false;
                let is_en = self.settings.lang == "en";
                let (trophy_id, trophy_name, trophy_description) = match kind.as_str() {
                    "crypt_warden" => (
                        "trophy_warden_seal",
                        if is_en {
                            "Warden's Seal"
                        } else {
                            "Печать Хранителя"
                        },
                        if is_en {
                            "A cold seal taken from the Crypt Warden."
                        } else {
                            "Холодная печать, снятая с Хранителя крипты."
                        },
                    ),
                    "blood_oracle" => (
                        "trophy_oracle_chalice",
                        if is_en {
                            "Oracle's Chalice"
                        } else {
                            "Чаша Оракула"
                        },
                        if is_en {
                            "The Blood Oracle's chalice no longer whispers."
                        } else {
                            "Чаша Кровавого оракула больше не шепчет."
                        },
                    ),
                    "archive_sentinel" => (
                        "trophy_archive_lens",
                        if is_en {
                            "Archive Lens"
                        } else {
                            "Линза Архива"
                        },
                        if is_en {
                            "A focusing lens from the Archive Sentinel."
                        } else {
                            "Фокусирующая линза Стража архива."
                        },
                    ),
                    _ => (
                        "trophy_tyrant_heart",
                        if is_en {
                            "Tyrant's Heart"
                        } else {
                            "Сердце Тирана"
                        },
                        if is_en {
                            "The last ember of the Heart Tyrant."
                        } else {
                            "Последний уголь Тирана Сердца."
                        },
                    ),
                };
                let first_clear = {
                    let st = self.state.as_mut().unwrap();
                    st.dungeons_cleared = st.dungeons_cleared.max(self.dungeon_depth);
                    st.gold += 50;
                    st.weapon_mod_cores += 1;
                    let first_clear = st.flags.insert(format!("boss_defeated_{kind}"));
                    if first_clear {
                        st.inventory.add(crate::item::Item::new(
                            trophy_id,
                            trophy_name,
                            trophy_description,
                            1,
                        ));
                    }
                    let done = st.quests.quests.iter().any(|q| {
                        q.id == "dungeon_heart" && q.state == crate::quest::QuestState::Active
                    });
                    if done {
                        st.quests.complete("dungeon_heart");
                    }
                    st.add_xp(120);
                    first_clear
                };
                let message = if is_en && first_clear {
                    format!("GUARDIAN DEFEATED! +50 gold, +1 weapon core, trophy: {trophy_name}.")
                } else if is_en {
                    "GUARDIAN DEFEATED! +50 gold, +1 weapon core.".to_string()
                } else if first_clear {
                    format!("СТРАЖ ПОВЕРЖЕН! +50 зол., +1 ядро оружия, трофей: {trophy_name}.")
                } else {
                    "СТРАЖ ПОВЕРЖЕН! +50 зол., +1 ядро оружия.".to_string()
                };
                self.show_flash(&message);
                self.build_boss_aftermath();
                self.auto_save();
            }
        }
    }

    /// Посмертный взрыв «взрывной» элиты: полный урон игроку в радиусе (с
    /// затуханием), врагам — половина; будит округу.
    pub(super) fn enemy_death_blast(&mut self, pos: Vector3, dmg: f32, radius: f32) {
        self.spawn_fx(
            "res://assets/effects/effect_explosion.png",
            pos + Vector3::new(0.0, 0.8, 0.0),
            0.024,
            0.45,
        );
        self.spawn_light_fx(
            pos,
            Color::from_rgba(1.0, 0.55, 0.2, 1.0),
            2.8,
            radius * 2.0,
            0.4,
        );
        self.alert_enemies(pos, 20.0);

        if let Some(p) = self.player() {
            let d = (p.get_global_position() - pos).length();
            if d < radius {
                if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                    let fall = 1.0 - (d / radius) * 0.6;
                    pl.bind_mut().take_damage(dmg * fall);
                    self.damage_flash_timer = 0.35;
                    self.punch_hit();
                }
            }
        }
        for e in self.enemies.iter_mut() {
            if !e.bind().alive {
                continue;
            }
            let d = (e.get_global_position() - pos).length();
            if d < radius {
                let fall = 1.0 - (d / radius) * 0.6;
                e.bind_mut().take_damage(dmg * 0.5 * fall, DmgType::Fire);
            }
        }
    }

    pub(super) fn spawn_item_heart(&mut self, pos: Vector3, in_dungeon: bool) {
        let node = self.make_pickup_node("res://assets/sprites/pickups/heart_1up.png", pos, 0.010);
        self.world_items.push(WorldItemNode {
            node,
            item_id: "heart_1up".into(),
            name: if self.settings.lang == "en" {
                "Heart of Life"
            } else {
                "Сердце жизни"
            }
            .into(),
            payload: Payload::Heart,
            in_dungeon,
            ground_offset: 0.55,
        });
    }

    // ── Эффекты ──────────────────────────────────────────────────────────────

    pub(super) fn spawn_fx(&mut self, tex: &str, pos: Vector3, px: f32, ttl: f32) {
        self.spawn_tinted_fx(tex, pos, px, ttl, Color::WHITE);
    }

    pub(super) fn spawn_tinted_fx(
        &mut self,
        tex: &str,
        pos: Vector3,
        px: f32,
        ttl: f32,
        color: Color,
    ) {
        if let Some(mut sp) = make_billboard(&mut self.cache, tex, pos, px) {
            sp.set_modulate(color);
            self.base_mut().add_child(&sp);
            self.sprite_fx.push(SpriteFx {
                node: sp,
                ttl,
                total: ttl,
                color,
            });
        }
    }

    pub(super) fn feedback_color(rgb: [f32; 3]) -> Color {
        Color::from_rgba(rgb[0], rgb[1], rgb[2], 1.0)
    }

    pub(super) fn spawn_weapon_tracer(
        &mut self,
        feedback: WeaponFeedback,
        from: Vector3,
        to: Vector3,
    ) {
        if feedback.tracer_scale <= 0.0 || feedback.tracer_duration <= 0.0 {
            return;
        }
        let delta = to - from;
        let distance = delta.length();
        if distance < 1.0 {
            return;
        }
        let steps = if distance > 18.0 { 3 } else { 2 };
        let color = Self::feedback_color(feedback.tracer_color);
        for step in 1..=steps {
            let t = step as f32 / (steps + 1) as f32;
            self.spawn_tinted_fx(
                "res://assets/effects/effect_energy.png",
                from + delta * t,
                0.0018 * feedback.tracer_scale,
                feedback.tracer_duration,
                color,
            );
        }
    }

    pub(super) fn spawn_weapon_impact(
        &mut self,
        feedback: WeaponFeedback,
        pos: Vector3,
        flesh: bool,
    ) {
        if feedback.impact_scale <= 0.0 || feedback.impact_duration <= 0.0 {
            return;
        }
        let color = Self::feedback_color(feedback.impact_color);
        let texture = if flesh {
            "res://assets/effects/effect_blood.png"
        } else {
            "res://assets/effects/effect_bullet.png"
        };
        self.spawn_tinted_fx(
            texture,
            pos,
            0.008 * feedback.impact_scale,
            feedback.impact_duration,
            color,
        );
        if feedback.impact_energy > 0.0 {
            self.spawn_light_fx(
                pos,
                color,
                feedback.impact_energy,
                2.5 + feedback.impact_scale * 2.0,
                feedback.impact_duration.min(0.3),
            );
        }
    }

    /// Случайный путь из набора вариантов звука.
    pub(super) fn pick_sfx<'a>(&mut self, paths: &[&'a str]) -> &'a str {
        if paths.len() <= 1 {
            return paths[0];
        }
        let i = ((self.rng.f32() * paths.len() as f32) as usize).min(paths.len() - 1);
        paths[i]
    }

    fn pick_weapon_sfx(&mut self, paths: &[String]) -> Option<String> {
        if paths.is_empty() {
            return None;
        }
        let index = if paths.len() == 1 {
            0
        } else {
            ((self.rng.f32() * paths.len() as f32) as usize).min(paths.len() - 1)
        };
        Some(paths[index].clone())
    }

    fn weapon_pitch(&mut self, range: [f32; 2]) -> f32 {
        range[0] + (range[1] - range[0]) * self.rng.f32()
    }

    pub(super) fn play_weapon_sfx(
        &mut self,
        paths: &[String],
        pitch_range: [f32; 2],
        volume_db: f32,
    ) {
        let Some(path) = self.pick_weapon_sfx(paths) else {
            return;
        };
        let Some(stream) = load_sfx_stream(&path) else {
            return;
        };
        let mut player = AudioStreamPlayer::new_alloc();
        player.set_stream(&stream);
        player.set_pitch_scale(self.weapon_pitch(pitch_range));
        player.set_volume_db(volume_db);
        player.set_bus("SFX");
        self.base_mut().add_child(&player);
        player.play();
        self.sfx_2d.push(player);
    }

    pub(super) fn play_weapon_impact_sfx(&mut self, weapon: WeaponId, pos: Vector3) {
        let audio = &weapon_def(weapon).audio;
        let Some(path) = self.pick_weapon_sfx(&audio.impact_sfx) else {
            return;
        };
        let Some(stream) = load_sfx_stream(&path) else {
            return;
        };
        let mut player = AudioStreamPlayer3D::new_alloc();
        player.set_stream(&stream);
        player.set_position(pos);
        let pitch_mult = self.effective_weapon_pitch(weapon);
        player.set_pitch_scale(self.weapon_pitch([
            audio.impact_pitch[0] * pitch_mult,
            audio.impact_pitch[1] * pitch_mult,
        ]));
        player.set_volume_db(audio.impact_volume_db);
        player.set_max_distance(38.0);
        player.set_bus("SFX");
        self.base_mut().add_child(&player);
        player.play();
        self.sfx_3d.push(player);
    }

    fn play_pending_weapon_impact(&mut self, pos: Vector3) {
        if !self.weapon_impact_sfx_pending {
            return;
        }
        self.weapon_impact_sfx_pending = false;
        self.play_weapon_impact_sfx(self.arsenal.current, pos);
    }

    /// Проиграть 3D-звук в точке мира (звуки врагов — смерть, пробуждение).
    pub(super) fn play_sfx_at(&mut self, paths: &[&str], pos: Vector3) {
        let path = self.pick_sfx(paths);
        let Some(stream) = load_sfx_stream(path) else {
            return;
        };
        let mut p = AudioStreamPlayer3D::new_alloc();
        p.set_stream(&stream);
        p.set_position(pos);
        p.set_max_distance(45.0);
        p.set_bus("SFX");
        self.base_mut().add_child(&p);
        p.play();
        self.sfx_3d.push(p);
    }

    /// Освободить закончившиеся аудио-плееры (вызывается каждый кадр).
    pub(super) fn tick_sfx(&mut self) {
        let mut i = 0;
        while i < self.sfx_2d.len() {
            if self.sfx_2d[i].is_playing() {
                i += 1;
            } else {
                self.sfx_2d.remove(i).free();
            }
        }
        let mut i = 0;
        while i < self.sfx_3d.len() {
            if self.sfx_3d[i].is_playing() {
                i += 1;
            } else {
                self.sfx_3d.remove(i).free();
            }
        }
    }

    pub(super) fn spawn_light_fx(
        &mut self,
        pos: Vector3,
        color: Color,
        energy: f32,
        range: f32,
        ttl: f32,
    ) {
        let l = make_light(pos, color, energy, range);
        self.base_mut().add_child(&l);
        self.light_fx.push(LightFx {
            node: l,
            ttl,
            total: ttl,
            energy,
        });
    }

    pub(super) fn tick_fx(&mut self, dt: f32) {
        let mut i = 0;
        while i < self.sprite_fx.len() {
            self.sprite_fx[i].ttl -= dt;
            if self.sprite_fx[i].ttl <= 0.0 {
                let fx = self.sprite_fx.remove(i);
                fx.node.free();
            } else {
                let a = (self.sprite_fx[i].ttl / self.sprite_fx[i].total).clamp(0.0, 1.0);
                let n = self.sprite_fx[i].node.clone();
                let mut n = n;
                let color = self.sprite_fx[i].color;
                n.set_modulate(Color::from_rgba(color.r, color.g, color.b, color.a * a));
                i += 1;
            }
        }
        let mut i = 0;
        while i < self.light_fx.len() {
            self.light_fx[i].ttl -= dt;
            if self.light_fx[i].ttl <= 0.0 {
                let fx = self.light_fx.remove(i);
                fx.node.free();
            } else {
                use godot::classes::light_3d::Param;
                let a = (self.light_fx[i].ttl / self.light_fx[i].total).clamp(0.0, 1.0);
                let e = self.light_fx[i].energy * a;
                let n = self.light_fx[i].node.clone();
                let mut n = n;
                n.set_param(Param::ENERGY, e);
                i += 1;
            }
        }
    }

    pub(super) fn flash_muzzle(&mut self, feedback: WeaponFeedback) {
        self.muzzle_timer = feedback.muzzle_duration;
        if self.muzzle_light.is_none() {
            if let Some(p) = self.player() {
                let l = make_light(
                    Vector3::new(0.0, 0.6, -0.8),
                    Self::feedback_color(feedback.muzzle_color),
                    0.0,
                    feedback.muzzle_range,
                );
                let mut p2 = p.clone();
                p2.add_child(&l);
                self.muzzle_light = Some(l);
            }
        }
        if let Some(ref mut l) = self.muzzle_light {
            use godot::classes::light_3d::Param;
            l.set_color(Self::feedback_color(feedback.muzzle_color));
            l.set_param(Param::RANGE, feedback.muzzle_range);
            l.set_param(Param::ENERGY, feedback.muzzle_energy);
        }
    }

    pub(super) fn tick_muzzle(&mut self, dt: f32) {
        if self.muzzle_timer > 0.0 {
            self.muzzle_timer -= dt;
            if self.muzzle_timer <= 0.0 {
                if let Some(ref mut l) = self.muzzle_light {
                    use godot::classes::light_3d::Param;
                    l.set_param(Param::ENERGY, 0.0);
                }
            }
        }
    }

    // ── Урон от врагов ───────────────────────────────────────────────────────

    pub(super) fn collect_enemy_damage(&mut self, _dt: f32) {
        let mut total_dmg = 0.0f32;
        let mut applied_status: Vec<String> = Vec::new();
        for e in self.enemies.iter_mut() {
            let (dmg, status) = {
                let mut b = e.bind_mut();
                let d = b.pending_dmg;
                b.pending_dmg = 0.0;
                (d, b.take_pending_status())
            };
            if dmg > 0.0 {
                total_dmg += dmg;
            }
            if let Some(s) = status {
                applied_status.push(s);
            }
        }
        if total_dmg > 0.0 {
            self.damage_player(total_dmg);
        }
        for id in applied_status {
            self.apply_status_to_player(&id);
        }
    }

    /// Урон игроку с учётом уязвимости (weakened) + красный флэш.
    pub(super) fn damage_player(&mut self, amount: f32) {
        if self.creative { return; }
        let guard = self.state.as_ref().map(|s| s.abilities.damage_scale()).unwrap_or(1.0);
        let amount = amount * self.player_statuses.vuln_mult() * guard;
        if let Some(p_gd) = self.player() {
            if let Ok(mut player) = p_gd.try_cast::<Player>() {
                player.bind_mut().take_damage(amount);
            }
        }
        self.damage_flash_timer = 0.35;
        self.punch_hit();
    }

    /// Наложить статус на игрока по id (из statuses.json пресета).
    pub(super) fn apply_status_to_player(&mut self, id: &str) {
        let cfg = self
            .cfg
            .as_ref()
            .and_then(|c| c.statuses.iter().find(|s| s.id == id).cloned());
        if let Some(sc) = cfg {
            self.player_statuses.apply(&sc);
        }
    }

    /// StatusCfg по id для наложения на врага (если ролл шанса прошёл).
    pub(super) fn rolled_enemy_status(
        &mut self,
        status: &Option<(String, f32)>,
    ) -> Option<crate::config::StatusCfg> {
        let (id, chance) = status.as_ref()?;
        if self.rng.f32() >= *chance {
            return None;
        }
        self.cfg
            .as_ref()?
            .statuses
            .iter()
            .find(|s| &s.id == id)
            .cloned()
    }

    // ── Статусы игрока ─────────────────────────────────────────────────────────

    pub(super) fn tick_player_statuses(&mut self, dt: f32) {
        // DoT: у игрока нет резистов — суммируем урон тиков
        let dots = self.player_statuses.tick(dt);
        let dot: f32 = dots.iter().map(|(d, _)| *d).sum();
        if dot > 0.0 {
            self.damage_player(dot);
        }
        // замедление → множитель скорости; оглушение → флаг стана
        let sm = self.player_statuses.slow_mult()
            * self.state.as_ref().map(|s| s.abilities.speed_scale()).unwrap_or(1.0);
        let stunned = self.player_statuses.stunned();
        if let Some(p_gd) = self.player() {
            if let Ok(mut player) = p_gd.try_cast::<Player>() {
                let mut b = player.bind_mut();
                b.speed_mult = sm;
                b.stunned = stunned;
            }
        }
    }

    /// Оглушён ли игрок статусом (стрельба заблокирована).
    pub(super) fn player_stunned(&self) -> bool {
        self.player()
            .and_then(|p| p.clone().try_cast::<Player>().ok())
            .map(|pl| pl.bind().stunned)
            .unwrap_or(false)
    }
}
