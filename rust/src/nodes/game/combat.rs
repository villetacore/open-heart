//! Боёвка, эффекты, урон от врагов и статусы игрока.

use super::*;

impl Game3D {
    // ── Боёвка ───────────────────────────────────────────────────────────────

    pub(super) fn player_aim(&self) -> Option<(Vector3, Vector3)> {
        let p = self.player.as_ref()?;
        let pl = p.clone().try_cast::<Player>().ok()?;
        let b = pl.bind();
        Some((b.eye_pos(), b.aim_dir()))
    }

    pub(super) fn try_fire(&mut self) {
        let cur = self.arsenal.current;
        let def = weapon_def(cur);

        if !self.arsenal.can_fire(cur) {
            self.shoot_cd = 0.35;
            self.show_flash(&format!("Нет боеприпасов: {}",
                def.ammo.map(|(t, _)| t.name_ru()).unwrap_or("—")));
            return;
        }

        self.shoot_cd = def.cooldown * self.loadout.cd_mult;
        self.arsenal.consume(cur);
        self.weapon_anim = WeaponAnim::Fire(0);
        self.anim_timer = 0.0;
        self.set_weapon_frame(def.fire_frames[0]);
        self.flash_muzzle();

        // звук выстрела/удара
        if matches!(def.kind, FireKind::Melee) {
            self.play_sfx(&SFX_MELEE);
        } else {
            self.play_sfx(&SFX_FIRE);
        }

        // шум выстрела будит врагов в округе (мили — тихо, вдвое меньший радиус)
        if let Some(ref p) = self.player {
            let pos = p.get_global_position();
            let noise = if matches!(def.kind, FireKind::Melee) { 8.0 } else { 18.0 };
            self.alert_enemies(pos, noise);
        }

        let Some((eye, dir)) = self.player_aim() else { return };
        let dmg = def.damage * self.loadout.dmg_mult;
        let dtype = def.dmg_type;

        match def.kind {
            FireKind::Melee => self.fire_melee(dmg, def.range, dtype),
            FireKind::Hitscan { pellets, spread } => {
                for _ in 0..pellets {
                    let sx = (self.rng.f32() - 0.5) * 2.0 * spread;
                    let sy = (self.rng.f32() - 0.5) * 2.0 * spread;
                    // разброс в плоскости, перпендикулярной взгляду
                    let right = dir.cross(Vector3::UP).normalized();
                    let up = right.cross(dir).normalized();
                    let d = (dir + right * sx + up * sy).normalized();
                    self.fire_ray(eye, d, def.range, dmg, dtype);
                }
            }
            FireKind::Projectile { speed, splash } => {
                self.spawn_projectile(eye + dir * 0.6, dir * speed, dmg, dtype, splash,
                                      def.range / speed, cur);
            }
        }
        self.process_kills();
    }

    pub(super) fn fire_melee(&mut self, dmg: f32, range: f32, dtype: DmgType) {
        let Some((eye, dir)) = self.player_aim() else { return };
        let flat_dir = Vector3::new(dir.x, 0.0, dir.z).normalized();
        let mut best: Option<(usize, f32)> = None;
        for (i, e) in self.enemies.iter().enumerate() {
            let eb = e.bind();
            if !eb.alive { continue; }
            let epos = e.get_global_position();
            let to = Vector3::new(epos.x - eye.x, 0.0, epos.z - eye.z);
            let d = to.length();
            if d > range { continue; }
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
            self.spawn_fx("res://assets/effects/effect_blood.png",
                          epos + Vector3::new(0.0, 1.1, 0.0), 0.010, 0.28);
            // вампиризм — от фактически нанесённого урона
            if self.loadout.lifesteal > 0.0 {
                let heal = dealt * self.loadout.lifesteal;
                if let Some(ref p) = self.player {
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

    pub(super) fn fire_ray(&mut self, from: Vector3, dir: Vector3, range: f32, dmg: f32, dtype: DmgType) {
        let to = from + dir * range;
        let hit = self.raycast(from, to);
        match hit {
            Some((pos, Some(mut enemy))) => {
                enemy.bind_mut().take_damage(dmg, dtype);
                let wstatus = weapon_def(self.arsenal.current).status.clone();
                if let Some(sc) = self.rolled_enemy_status(&wstatus) {
                    enemy.bind_mut().apply_status(&sc);
                }
                self.spawn_fx("res://assets/effects/effect_blood.png",
                              pos, 0.008, 0.25);
            }
            Some((pos, None)) => {
                self.spawn_fx("res://assets/effects/effect_bullet.png",
                              pos - dir * 0.1, 0.004, 0.15);
            }
            None => {}
        }
    }

    /// Луч: возвращает точку и врага (если попали в него).
    pub(super) fn raycast(&mut self, from: Vector3, to: Vector3) -> Option<(Vector3, Option<Gd<Enemy>>)> {
        let world = self.base().get_world_3d()?;
        let mut space = world.clone().get_direct_space_state()?;
        let mut query = PhysicsRayQueryParameters3D::create(from, to)?;
        if let Some(ref p) = self.player {
            let mut excl: godot::builtin::Array<Rid> = godot::builtin::Array::new();
            excl.push(p.get_rid());
            query.set_exclude(&excl);
        }
        let hit = space.intersect_ray(&query);
        if hit.is_empty() { return None; }
        let pos = hit.get("position")?.try_to::<Vector3>().ok()?;
        let enemy = hit.get("collider")
            .and_then(|cv| cv.try_to::<Gd<godot::classes::Node>>().ok())
            .and_then(|n| n.try_cast::<Enemy>().ok());
        Some((pos, enemy))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn spawn_projectile(&mut self, pos: Vector3, vel: Vector3, dmg: f32, dmg_type: DmgType,
                        splash: f32, ttl: f32, weapon: WeaponId) {
        let mut node = Node3D::new_alloc();
        node.set_position(pos);
        let (tex, px, color) = match weapon {
            WeaponId::Rocket => ("res://assets/sprites/projectiles/rocket.png", 0.010, Color::WHITE),
            _ => ("res://assets/effects/effect_energy.png", 0.006, C_PINK),
        };
        if let Some(mut sp) = make_billboard(&mut self.cache, tex, Vector3::ZERO, px) {
            sp.set_modulate(color);
            node.add_child(&sp);
        }
        let l = make_light(Vector3::ZERO, C_PINK, 0.8, 5.0);
        node.add_child(&l);
        self.base_mut().add_child(&node);
        // статус оружия фиксируется на снаряде в момент выстрела
        let status = weapon_def(weapon).status.clone();
        self.projectiles.push(Projectile { node, pos, vel, dmg, dmg_type, splash, ttl, status });
    }

    pub(super) fn tick_projectiles(&mut self, dt: f32) {
        let mut exploded: Vec<(Vector3, f32, DmgType, f32)> = Vec::new(); // pos, dmg, type, splash
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
                        exploded.push((pos, pr.dmg, pr.dmg_type, pr.splash));
                    } else {
                        direct_hits.push((enemy, pr.dmg, pr.dmg_type, pos, pr.status.clone()));
                    }
                    remove = true;
                }
                Some((pos, None)) => {
                    let pr = &self.projectiles[i];
                    if pr.splash > 0.0 {
                        exploded.push((pos, pr.dmg, pr.dmg_type, pr.splash));
                    } else {
                        self.spawn_fx("res://assets/effects/effect_bullet.png", pos, 0.004, 0.15);
                    }
                    remove = true;
                }
                None => {
                    self.projectiles[i].pos = new_pos;
                    self.projectiles[i].node.set_position(new_pos);
                    self.projectiles[i].ttl -= dt;
                    if self.projectiles[i].ttl <= 0.0 { remove = true; }
                }
            }

            if remove {
                let p = self.projectiles.remove(i);
                p.node.free();
            } else {
                i += 1;
            }
        }

        for (enemy, dmg, dtype, pos, status) in direct_hits {
            let mut e = enemy;
            e.bind_mut().take_damage(dmg, dtype);
            if let Some(sc) = self.rolled_enemy_status(&status) {
                e.bind_mut().apply_status(&sc);
            }
            self.spawn_fx("res://assets/effects/effect_blood.png", pos, 0.008, 0.25);
        }
        for (pos, dmg, dtype, splash) in exploded {
            self.explode(pos, dmg, dtype, splash);
        }
        self.process_kills();
    }

    pub(super) fn explode(&mut self, pos: Vector3, dmg: f32, dtype: DmgType, radius: f32) {
        self.spawn_fx("res://assets/effects/effect_explosion.png", pos, 0.022, 0.4);
        self.spawn_light_fx(pos, Color::from_rgba(1.0, 0.5, 0.6, 1.0), 2.6, radius * 2.2, 0.35);
        self.alert_enemies(pos, 22.0);   // взрыв слышно издалека

        for e in self.enemies.iter_mut() {
            let alive = e.bind().alive;
            if !alive { continue; }
            let d = (e.get_global_position() - pos).length();
            if d < radius {
                let fall = 1.0 - (d / radius) * 0.55;
                e.bind_mut().take_damage(dmg * fall, dtype);
            }
        }
        // самоурон
        if let Some(ref p) = self.player {
            let d = (p.get_global_position() - pos).length();
            if d < radius * 0.8 {
                if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                    let fall = 1.0 - d / (radius * 0.8);
                    pl.bind_mut().take_damage(dmg * 0.35 * fall);
                    self.damage_flash_timer = 0.3;
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
            if !alive {
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
            self.spawn_fx("res://assets/effects/effect_blood.png",
                          pos + Vector3::new(0.0, 0.9, 0.0), 0.014, 0.4);
            self.play_sfx_at(&SFX_DEATH, pos + Vector3::new(0.0, 1.0, 0.0));
            // прогресс kill-квестов
            self.bump_quests("kill", &kind);

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
                if let Some(ref p) = self.player {
                    if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                        let heal = pl.bind().max_hp * 0.35;
                        pl.bind_mut().heal(heal);
                    }
                }
                self.show_flash(&format!("УРОВЕНЬ {}!", lvl));
            }

            // дроп
            let in_dungeon = self.loc == Loc::Dungeon;
            // Дроп по таблице kill_drops (loot.json): один бросок, записи
            // кумулятивны по chance, остаток вероятности — «ничего».
            let drops = self.cfg.as_ref().map(|c| c.loot.kill_drops.clone()).unwrap_or_default();
            let roll = self.rng.f32();
            let mut acc = 0.0f32;
            for d in &drops {
                acc += d.chance;
                if roll >= acc { continue; }
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
                let msgs = {
                    let st = self.state.as_mut().unwrap();
                    st.dungeons_cleared = st.dungeons_cleared.max(self.dungeon_depth);
                    st.gold += 50;
                    let done = st.quests.quests.iter()
                        .any(|q| q.id == "dungeon_heart"
                             && q.state == crate::quest::QuestState::Active);
                    if done { st.quests.complete("dungeon_heart"); }
                    st.add_xp(120)
                };
                let _ = msgs;
                self.show_flash("СТРАЖ ПОВЕРЖЕН! Портал вглубь открыт (+50 зол.)");
                self.auto_save();
            }
        }
    }

    /// Посмертный взрыв «взрывной» элиты: полный урон игроку в радиусе (с
    /// затуханием), врагам — половина; будит округу.
    pub(super) fn enemy_death_blast(&mut self, pos: Vector3, dmg: f32, radius: f32) {
        self.spawn_fx("res://assets/effects/effect_explosion.png", pos + Vector3::new(0.0, 0.8, 0.0), 0.024, 0.45);
        self.spawn_light_fx(pos, Color::from_rgba(1.0, 0.55, 0.2, 1.0), 2.8, radius * 2.0, 0.4);
        self.alert_enemies(pos, 20.0);

        if let Some(ref p) = self.player {
            let d = (p.get_global_position() - pos).length();
            if d < radius {
                if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                    let fall = 1.0 - (d / radius) * 0.6;
                    pl.bind_mut().take_damage(dmg * fall);
                    self.damage_flash_timer = 0.35;
                }
            }
        }
        for e in self.enemies.iter_mut() {
            if !e.bind().alive { continue; }
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
            node, item_id: "heart_1up".into(), name: "Сердце жизни".into(),
            payload: Payload::Heart, in_dungeon,
        });
    }

    // ── Эффекты ──────────────────────────────────────────────────────────────

    pub(super) fn spawn_fx(&mut self, tex: &str, pos: Vector3, px: f32, ttl: f32) {
        if let Some(sp) = make_billboard(&mut self.cache, tex, pos, px) {
            self.base_mut().add_child(&sp);
            self.sprite_fx.push(SpriteFx { node: sp, ttl, total: ttl });
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

    /// Проиграть звук без позиции (звуки игрока — стрельба, удар).
    pub(super) fn play_sfx(&mut self, paths: &[&str]) {
        let path = self.pick_sfx(paths);
        let Ok(stream) = godot::tools::try_load::<AudioStream>(path) else { return };
        let mut p = AudioStreamPlayer::new_alloc();
        p.set_stream(&stream);
        self.base_mut().add_child(&p);
        p.play();
        self.sfx_2d.push(p);
    }

    /// Проиграть 3D-звук в точке мира (звуки врагов — смерть, пробуждение).
    pub(super) fn play_sfx_at(&mut self, paths: &[&str], pos: Vector3) {
        let path = self.pick_sfx(paths);
        let Ok(stream) = godot::tools::try_load::<AudioStream>(path) else { return };
        let mut p = AudioStreamPlayer3D::new_alloc();
        p.set_stream(&stream);
        p.set_position(pos);
        p.set_max_distance(45.0);
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

    pub(super) fn spawn_light_fx(&mut self, pos: Vector3, color: Color, energy: f32, range: f32, ttl: f32) {
        let l = make_light(pos, color, energy, range);
        self.base_mut().add_child(&l);
        self.light_fx.push(LightFx { node: l, ttl, total: ttl, energy });
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
                n.set_modulate(Color::from_rgba(1.0, 1.0, 1.0, a));
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

    pub(super) fn flash_muzzle(&mut self) {
        self.muzzle_timer = 0.09;
        if self.muzzle_light.is_none() {
            if let Some(ref p) = self.player {
                let l = make_light(Vector3::new(0.0, 0.6, -0.8),
                                   Color::from_rgba(1.0, 0.6, 0.8, 1.0), 0.0, 7.0);
                let mut p2 = p.clone();
                p2.add_child(&l);
                self.muzzle_light = Some(l);
            }
        }
        if let Some(ref mut l) = self.muzzle_light {
            use godot::classes::light_3d::Param;
            l.set_param(Param::ENERGY, 1.6);
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
            if dmg > 0.0 { total_dmg += dmg; }
            if let Some(s) = status { applied_status.push(s); }
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
        let amount = amount * self.player_statuses.vuln_mult();
        if let Some(p_gd) = self.player.clone() {
            if let Ok(mut player) = p_gd.try_cast::<Player>() {
                player.bind_mut().take_damage(amount);
            }
        }
        self.damage_flash_timer = 0.35;
    }

    /// Наложить статус на игрока по id (из statuses.json пресета).
    pub(super) fn apply_status_to_player(&mut self, id: &str) {
        let cfg = self.cfg.as_ref().and_then(|c| c.statuses.iter().find(|s| s.id == id).cloned());
        if let Some(sc) = cfg {
            self.player_statuses.apply(&sc);
        }
    }

    /// StatusCfg по id для наложения на врага (если ролл шанса прошёл).
    pub(super) fn rolled_enemy_status(&mut self, status: &Option<(String, f32)>) -> Option<crate::config::StatusCfg> {
        let (id, chance) = status.as_ref()?;
        if self.rng.f32() >= *chance { return None; }
        self.cfg.as_ref()?.statuses.iter().find(|s| &s.id == id).cloned()
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
        let sm = self.player_statuses.slow_mult();
        let stunned = self.player_statuses.stunned();
        if let Some(p_gd) = self.player.clone() {
            if let Ok(mut player) = p_gd.try_cast::<Player>() {
                let mut b = player.bind_mut();
                b.speed_mult = sm;
                b.stunned = stunned;
            }
        }
    }

    /// Оглушён ли игрок статусом (стрельба заблокирована).
    pub(super) fn player_stunned(&self) -> bool {
        self.player.as_ref()
            .and_then(|p| p.clone().try_cast::<Player>().ok())
            .map(|pl| pl.bind().stunned)
            .unwrap_or(false)
    }

}
