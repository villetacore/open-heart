//! Генерация и вход в данж.

use super::*;

// ── Данж ──────────────────────────────────────────────────────────────────────

impl Game3D {
    pub(super) fn use_dungeon_event(&mut self, index: usize) {
        let Some(event) = self.dungeon_events.get(index) else {
            return;
        };
        if event.used {
            return;
        }
        let kind = event.kind.clone();
        let group = event.group;
        let step = event.step;
        let position = event.pos;
        self.near_dungeon_event = None;

        if kind == "story_echo" {
            if let Some(event) = self.dungeon_events.get_mut(index) {
                event.used = true;
                if let Some(marker) = event.marker.as_mut() {
                    marker.set_visible(false);
                }
            }
            if let Some(state) = self.state.as_mut() {
                state.add_xp(35 + self.dungeon_depth * 5);
            }
            self.bump_quests("discover_lore", &self.dungeon_depth.to_string());
            let messages_en = [
                "Memory: the first wardens sealed the heart below.",
                "Memory: the archive erased every map of the lower choir.",
                "Memory: the machines learned to imitate a pulse.",
                "Memory: the tyrant is only the lock, not what sleeps behind it.",
            ];
            let messages_ru = [
                "Память: первые смотрители запечатали сердце внизу.",
                "Память: архив стёр все карты нижнего хора.",
                "Память: машины научились подражать пульсу.",
                "Память: тиран — лишь замок, а не то, что спит за ним.",
            ];
            let message_index = self.dungeon_depth.saturating_sub(1) as usize % messages_en.len();
            let message = if self.settings.lang == "en" {
                messages_en[message_index]
            } else {
                messages_ru[message_index]
            };
            self.show_flash(&format!("{message}  +{} XP", 35 + self.dungeon_depth * 5));
            self.spawn_light_fx(
                position,
                Color::from_rgba(0.72, 0.56, 1.0, 1.0),
                1.8,
                8.0,
                0.8,
            );
            self.auto_save();
            return;
        }

        let expected_step = self
            .dungeon_events
            .iter()
            .filter(|candidate| candidate.group == group && candidate.used)
            .count() as u32;
        if step != expected_step {
            for candidate in self
                .dungeon_events
                .iter_mut()
                .filter(|candidate| candidate.group == group)
            {
                candidate.used = false;
                if let Some(marker) = candidate.marker.as_mut() {
                    marker.set_visible(true);
                }
            }
            self.show_flash(if self.settings.lang == "en" {
                "Wrong relay order — sequence reset."
            } else {
                "Неверный порядок реле — последовательность сброшена."
            });
            return;
        }

        if let Some(event) = self.dungeon_events.get_mut(index) {
            event.used = true;
            if let Some(marker) = event.marker.as_mut() {
                marker.set_visible(false);
            }
        }
        self.spawn_light_fx(
            position,
            Color::from_rgba(0.25, 1.0, 0.72, 1.0),
            1.5,
            6.0,
            0.5,
        );
        let total = self
            .dungeon_events
            .iter()
            .filter(|candidate| candidate.group == group)
            .count();
        let solved = self
            .dungeon_events
            .iter()
            .filter(|candidate| candidate.group == group && candidate.used)
            .count();
        if solved == total && total > 0 {
            let reward = 30 + self.dungeon_depth as i32 * 12;
            if let Some(state) = self.state.as_mut() {
                state.gold += reward;
                state.add_xp(45 + self.dungeon_depth * 8);
            }
            self.bump_quests("solve_puzzle", &self.dungeon_depth.to_string());
            self.show_flash(&if self.settings.lang == "en" {
                format!("RELAY SOLVED  +{reward} gold")
            } else {
                format!("РЕЛЕ ЗАПУЩЕНО  +{reward} зол.")
            });
            self.auto_save();
        } else {
            self.show_flash(&if self.settings.lang == "en" {
                format!("Relay {}/{} synchronized", solved, total)
            } else {
                format!("Реле {}/{} синхронизировано", solved, total)
            });
        }
    }

    pub(super) fn enter_dungeon(&mut self, depth: u32) {
        // cfg берём ДО разрушительных шагов: ранний выход не должен оставить
        // игрока в пустоте с уже снесённым данжем
        let Some(cfg) = self.cfg.take() else { return };
        self.clear_dungeon();

        let seed = {
            let st = self.state.as_mut().unwrap();
            st.dungeon_seed = st.dungeon_seed.wrapping_add(1);
            st.dungeon_seed
        };
        // Дроп с убийств тоже зависит от сида — забег полностью воспроизводим
        // (раньше сессионный Rng стартовал с константы).
        self.rng = Rng::new(seed ^ 0x00D1_CED0);

        let plan: DungeonPlan =
            dungeon::generate(depth, seed, &mut self.cache, &cfg, &self.settings.lang);

        let mut root = plan.root.clone();
        root.set_position(DUNGEON_OFFSET);
        self.base_mut().add_child(&root);
        self.dungeon_root = Some(root);
        self.dungeon_depth = depth;
        self.dungeon_name = plan.theme_name.clone();
        self.dungeon_room_roles = plan.room_roles.clone();
        self.dungeon_room_markers = plan.room_markers.clone();
        self.dungeon_dressing_nodes = plan.dressing_nodes;
        self.dungeon_dressing_variants = plan.dressing_variants.clone();
        self.minimap_floor = plan.floor_map.clone();
        self.exit_portal = DUNGEON_OFFSET + plan.exit_portal;
        self.next_portal = DUNGEON_OFFSET + plan.next_portal;
        self.boss_alive = plan.enemies.iter().any(|e| e.is_boss);
        self.dungeon_hazards = plan
            .hazards
            .iter()
            .map(|hazard| crate::dungeon::HazardZone {
                pos: DUNGEON_OFFSET + hazard.pos,
                radius: hazard.radius,
                dps: hazard.dps,
                kind: hazard.kind,
                phase_offset: hazard.phase_offset,
                phase: crate::dungeon::HazardPhase::Dormant,
                player_inside: false,
            })
            .collect();
        self.dungeon_hazard_tick = 0.0;
        self.dungeon_hazard_time = 0.0;
        self.dungeon_hazard_active = false;
        for event in &plan.events {
            let world_position = DUNGEON_OFFSET + event.pos;
            let mut marker = make_billboard(
                &mut self.cache,
                "res://assets/sprites/pickups/soul.png",
                world_position,
                0.011,
            );
            if let Some(sprite) = marker.as_mut() {
                let color = if event.kind == "story_echo" {
                    Color::from_rgba(0.72, 0.56, 1.0, 1.0)
                } else {
                    match event.step {
                        0 => Color::from_rgba(0.2, 0.85, 1.0, 1.0),
                        1 => Color::from_rgba(0.25, 1.0, 0.65, 1.0),
                        _ => Color::from_rgba(1.0, 0.42, 0.78, 1.0),
                    }
                };
                sprite.set_modulate(color);
                self.base_mut().add_child(&*sprite);
            }
            self.dungeon_events.push(DungeonEventNode {
                kind: event.kind.clone(),
                group: event.group,
                step: event.step,
                pos: world_position,
                marker,
                used: false,
            });
        }
        // навигация: сетка проходимости данжа для A* врагов (до спавнов!)
        self.dungeon_nav = Some(NavGrid::new(
            plan.floor_map.clone(),
            plan.floor_heights.clone(),
        ));

        for es in &plan.enemies {
            self.spawn_enemy(
                &cfg,
                &es.kind,
                DUNGEON_OFFSET + es.pos,
                es.mult,
                es.is_boss,
                true,
                &es.affixes,
            );
        }
        for (kind, pos) in &plan.items {
            self.spawn_item(&cfg, kind, DUNGEON_OFFSET + *pos, true);
        }
        self.cfg = Some(cfg);
        for (t, n, pos) in &plan.ammo {
            self.spawn_ammo_pickup(*t, *n, DUNGEON_OFFSET + *pos, true);
        }
        for (w, pos) in &plan.weapons {
            self.spawn_weapon_pickup(*w, DUNGEON_OFFSET + *pos, true);
        }

        if let Some(ref p) = self.player {
            if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                pl.bind_mut()
                    .teleport(DUNGEON_OFFSET + plan.player_spawn + Vector3::new(0.0, 1.0, 0.0));
            }
        }
        self.loc = Loc::Dungeon;
        self.bump_quests("enter_dungeon", &depth.to_string());

        self.show_flash(&if self.settings.lang == "en" {
            format!("\"{}\" — depth {}", plan.theme_name, depth)
        } else {
            format!("«{}» — глубина {}", plan.theme_name, depth)
        });
        self.build_minimap_texture();
        if let Some(ref mut bg) = self.minimap_bg {
            bg.set_visible(true);
        }
        if let Some(ref mut mr) = self.minimap_rect {
            mr.set_visible(true);
        }
        if let Some(ref mut d) = self.minimap_dot {
            d.set_visible(true);
        }
        if let Some(ref mut legend) = self.minimap_legend {
            legend.set_visible(true);
        }
        self.update_loc_label();
    }

    pub(super) fn exit_dungeon(&mut self) {
        self.exit_dungeon_impl(true);
    }

    pub(super) fn exit_dungeon_impl(&mut self, save_after: bool) {
        self.clear_dungeon();
        self.loc = Loc::World;
        if let Some(ref p) = self.player {
            if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                let gate = self.gate_pos;
                pl.bind_mut().teleport(gate + Vector3::new(0.0, 1.0, 3.5));
            }
        }
        if let Some(ref mut bg) = self.minimap_bg {
            bg.set_visible(false);
        }
        if let Some(ref mut mr) = self.minimap_rect {
            mr.set_visible(false);
        }
        if let Some(ref mut d) = self.minimap_dot {
            d.set_visible(false);
        }
        if let Some(ref mut legend) = self.minimap_legend {
            legend.set_visible(false);
        }
        self.minimap_floor.clear();
        self.show_flash(if self.settings.lang == "en" {
            "Wastes of the Neon Heart"
        } else {
            "Пустоши Неонового Сердца"
        });
        self.update_loc_label();
        if save_after {
            self.auto_save();
        }
    }

    pub(super) fn clear_dungeon(&mut self) {
        self.dungeon_nav = None;
        if let Some(root) = self.dungeon_root.take() {
            root.free();
        }
        // убрать врагов и предметы данжа
        let pl = self.player.clone();
        let _ = pl;
        let mut i = 0;
        while i < self.enemies.len() {
            let in_d = self.enemies[i].get_position().x > 250.0;
            if in_d {
                let e = self.enemies.remove(i);
                e.free();
            } else {
                i += 1;
            }
        }
        let mut i = 0;
        while i < self.world_items.len() {
            if self.world_items[i].in_dungeon {
                let it = self.world_items.remove(i);
                it.node.free();
            } else {
                i += 1;
            }
        }
        for p in self.projectiles.drain(..) {
            p.node.free();
        }
        for p in self.enemy_projectiles.drain(..) {
            p.node.free();
        }
        self.boss_alive = false;
        for event in self.dungeon_events.drain(..) {
            if let Some(marker) = event.marker {
                marker.free();
            }
        }
        self.near_dungeon_event = None;
        self.dungeon_room_roles.clear();
        self.dungeon_room_markers.clear();
        self.dungeon_dressing_nodes = 0;
        self.dungeon_dressing_variants.clear();
        self.dungeon_hazards.clear();
        self.dungeon_hazard_tick = 0.0;
        self.dungeon_hazard_time = 0.0;
        self.dungeon_hazard_active = false;
    }

    pub(super) fn tick_dungeon_hazards(&mut self, dt: f32) {
        if self.loc != Loc::Dungeon || self.dungeon_hazards.is_empty() {
            return;
        }
        self.dungeon_hazard_time += dt;
        self.dungeon_hazard_tick -= dt;
        if self.dungeon_hazard_tick > 0.0 {
            return;
        }
        const TICK: f32 = 0.1;
        self.dungeon_hazard_tick = TICK;
        let Some(player_pos) = self
            .player
            .as_ref()
            .map(|player| player.get_global_position())
        else {
            return;
        };
        let mut damage = 0.0;
        let mut statuses = Vec::new();
        let mut pulses = Vec::new();
        let mut warning = None;
        let mut player_in_active_hazard = false;
        for hazard in &mut self.dungeon_hazards {
            let phase = hazard
                .kind
                .phase(self.dungeon_hazard_time + hazard.phase_offset);
            if phase != hazard.phase {
                match phase {
                    crate::dungeon::HazardPhase::Warning => pulses.push((
                        hazard.pos + Vector3::new(0.0, 0.25, 0.0),
                        hazard.kind.color(),
                        0.55,
                        hazard.radius * 1.8,
                        0.35,
                    )),
                    crate::dungeon::HazardPhase::Active => pulses.push((
                        hazard.pos + Vector3::new(0.0, 0.45, 0.0),
                        hazard.kind.color(),
                        1.8,
                        hazard.radius * 2.2,
                        0.28,
                    )),
                    crate::dungeon::HazardPhase::Dormant => {}
                }
            }
            let delta = player_pos - hazard.pos;
            let inside = Vector2::new(delta.x, delta.z).length() <= hazard.radius;
            let active = phase == crate::dungeon::HazardPhase::Active;
            if inside && active {
                player_in_active_hazard = true;
                damage += hazard.dps * hazard.kind.damage_mult() * TICK;
                if !hazard.player_inside || hazard.phase != crate::dungeon::HazardPhase::Active {
                    statuses.push(hazard.kind.status());
                    warning.get_or_insert(hazard.kind);
                }
            }
            hazard.phase = phase;
            hazard.player_inside = inside;
        }
        for (position, color, energy, range, ttl) in pulses {
            self.spawn_light_fx(position, color, energy, range, ttl);
        }
        if damage > 0.0 {
            if let Some(kind) = warning {
                self.show_flash(kind.warning(&self.settings.lang));
            }
            self.damage_player(damage);
        }
        for status in statuses {
            self.apply_status_to_player(status);
        }
        self.dungeon_hazard_active = player_in_active_hazard;
    }
}
