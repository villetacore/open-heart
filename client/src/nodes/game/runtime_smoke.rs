//! Отладочные runtime-хуки для смоук-теста `game/tools/runtime_smoke.gd`.
//!
//! Секция вынесена из `mod.rs` без изменения поведения: только в debug-сборке,
//! как вторичный `#[godot_api]`-блок (`#[godot_api(secondary)]`).
use super::*;

#[cfg(debug_assertions)]
#[godot_api(secondary)]
impl Game3D {
    #[cfg(debug_assertions)]
    #[func]
    fn runtime_review_page(&mut self, page: GString) {
        self.autosave_enabled=false;
        for mut panel in [self.inv_panel.clone(),self.perk_panel.clone(),self.journal_panel.clone(),self.select_panel.clone(),self.dlg_panel.clone(),self.creative_panel.clone()].into_iter().flatten() {panel.set_visible(false);}
        if self.state.as_ref().is_none_or(|s| !s.has("review_fixture")) {
            let mut state=GameState::new("Review");
            state.flags.insert("review_fixture".into());
            state.gold=420;
            state.perk_points=5;
            state.weapon_mod_cores=2;
            if let Some(cfg)=self.cfg.as_ref() {
                for item in &cfg.items {state.inventory.add(crate::item::Item::new(&item.id,item.name(&self.settings.lang),"",3));}
            }
            self.state=Some(state);
            self.confirm_class(1,0);
        }
        match page.to_string().as_str() {
            "classes"=>self.open_class_select(),
            "inventory"=>self.open_inventory(),
            "perks"=>self.open_perks(),
            "journal"=>self.open_journal(),
            "atelier"=>{ if let Some(i) = self.npcs.iter().position(|n| n.id == "stylist") { self.start_dialogue(i); } },
            "wardrobe"=>self.open_wardrobe(),
            "creative"=>{ self.set_mode_explore(); self.creative_action(-2); },
            "quarter"=>{ self.set_mode_explore(); if let Some(mut player) = self.player().and_then(|p| p.try_cast::<Player>().ok()) { player.bind_mut().teleport(Vector3::new(-5.0, 0.0, 3.0)); } },
            "dungeon"=>{self.set_mode_explore();self.enter_dungeon_seeded(1,0x5EED_0001);},
            _=>self.set_mode_explore(),
        }
    }

    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_player_skills(&mut self) -> VarDictionary {
        self.autosave_enabled=false;
        self.confirm_class(1,0);
        self.state.as_mut().unwrap().abilities=Default::default();
        self.try_player_ability(1);
        let state=self.state.as_ref().unwrap();
        let activated=state.abilities.guard_time>0.0 && state.abilities.cooldowns[1]>0.0;
        let saved=save::SaveData::from_game(state,100.0,&self.arsenal).to_json().unwrap();
        let (restored,_,_)=save::SaveData::from_json(&saved).unwrap().into_game();
        let persisted=restored.abilities.cooldowns[1]==state.abilities.cooldowns[1];
        self.open_inventory();
        let slots=self.inventory_grid.as_ref().map(|g|g.get_child_count()).unwrap_or(0);
        self.close_inventory();
        self.open_perks();
        let perks=self.perk_grid.as_ref().map(|g|g.get_child_count()).unwrap_or(0);
        self.close_perks();
        let mut result=VarDictionary::new();
        result.set("activated",activated);
        result.set("persisted",persisted);
        result.set("slots",slots);
        result.set("perks",perks);
        result.set("icons",self.ability_icons.len() as i64);
        result
    }
    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_shutdown_audio(&mut self) {
        for mut player in self.sfx_2d.drain(..) {
            player.stop();
            player.free();
        }
        for mut player in self.sfx_3d.drain(..) {
            player.stop();
            player.free();
        }
    }

    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_hazard_profiles(&self) -> VarDictionary {
        let kinds = [
            dungeon::HazardKind::Blood,
            dungeon::HazardKind::Void,
            dungeon::HazardKind::Electric,
            dungeon::HazardKind::Embers,
        ];
        let mut warning_profiles = 0;
        let mut dormant_profiles = 0;
        let mut active_profiles = 0;
        let mut statuses = Vec::new();
        let mut localized = 0;
        for kind in kinds {
            let mut has_warning = false;
            let mut has_dormant = false;
            let mut has_active = false;
            for sample in 0..=120 {
                match kind.phase(sample as f32 * 0.05) {
                    dungeon::HazardPhase::Dormant => has_dormant = true,
                    dungeon::HazardPhase::Warning => has_warning = true,
                    dungeon::HazardPhase::Active => has_active = true,
                }
            }
            warning_profiles += i64::from(has_warning);
            dormant_profiles += i64::from(has_dormant);
            active_profiles += i64::from(has_active);
            if !statuses.contains(&kind.status()) {
                statuses.push(kind.status());
            }
            localized +=
                i64::from(!kind.warning("ru").is_empty() && !kind.warning("en").is_empty());
        }
        let mut snapshot = VarDictionary::new();
        snapshot.set("types", kinds.len() as i64);
        snapshot.set("warning_profiles", warning_profiles);
        snapshot.set("dormant_profiles", dormant_profiles);
        snapshot.set("active_profiles", active_profiles);
        snapshot.set("statuses", statuses.len() as i64);
        snapshot.set("localized", localized);
        snapshot
    }

    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_trigger_hazard(&mut self) -> VarDictionary {
        let Some(hazard) = self.dungeon_hazards.first().copied() else {
            // Все ключи заполняем всегда: смоук-скрипт читает .damage/.status_applied
            // безусловно, и их отсутствие роняло бы прогон (SCRIPT ERROR → зависание).
            let mut snapshot = VarDictionary::new();
            snapshot.set("found", false);
            snapshot.set("damage", 0.0);
            snapshot.set("status_applied", false);
            return snapshot;
        };
        self.dungeon_hazard_time = -hazard.phase_offset;
        self.dungeon_hazard_tick = 0.0;
        if let Some(runtime_hazard) = self.dungeon_hazards.first_mut() {
            runtime_hazard.phase = dungeon::HazardPhase::Dormant;
            runtime_hazard.player_inside = false;
        }
        self.player_statuses.clear();
        let mut hp_before = 0.0;
        if let Some(player) = self.player() {
            if let Ok(mut player) = player.try_cast::<Player>() {
                hp_before = player.bind().hp;
                player
                    .bind_mut()
                    .teleport(hazard.pos + Vector3::new(0.0, 1.0, 0.0));
            }
        }
        self.tick_dungeon_hazards(0.11);
        let hp_after = self.player()
            .and_then(|player| player.clone().try_cast::<Player>().ok())
            .map(|player| player.bind().hp)
            .unwrap_or(hp_before);
        let mut snapshot = VarDictionary::new();
        snapshot.set("found", true);
        snapshot.set("kind", hazard.kind.id());
        snapshot.set("damage", hp_before - hp_after);
        snapshot.set("status", hazard.kind.status());
        snapshot.set(
            "status_applied",
            self.player_statuses.has(hazard.kind.status()),
        );
        snapshot
    }

    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_dungeon_events(&mut self) -> VarDictionary {
        let puzzle_group = self
            .dungeon_events
            .iter()
            .find(|event| event.kind == "puzzle_switch")
            .map(|event| event.group);
        let mut puzzle_indices = puzzle_group
            .map(|group| {
                let mut indices: Vec<_> = self
                    .dungeon_events
                    .iter()
                    .enumerate()
                    .filter_map(|(index, event)| {
                        (event.kind == "puzzle_switch" && event.group == group)
                            .then_some((event.step, index))
                    })
                    .collect();
                indices.sort_by_key(|(step, _)| *step);
                indices
            })
            .unwrap_or_default();
        let gold_before = self.state.as_ref().map(|state| state.gold).unwrap_or(0);
        let xp_before = self.state.as_ref().map(|state| state.xp).unwrap_or(0);
        let level_before = self.state.as_ref().map(|state| state.level).unwrap_or(0);
        let wrong_reset = if puzzle_indices.len() >= 2 {
            self.use_dungeon_event(puzzle_indices[1].1);
            puzzle_group.is_some_and(|group| {
                self.dungeon_events
                    .iter()
                    .filter(|event| event.group == group)
                    .all(|event| !event.used)
            })
        } else {
            false
        };
        for (_, index) in puzzle_indices.drain(..) {
            self.use_dungeon_event(index);
        }
        let puzzle_solved = puzzle_group.is_some_and(|group| {
            self.dungeon_events
                .iter()
                .filter(|event| event.group == group)
                .all(|event| event.used)
        });
        let story_index = self
            .dungeon_events
            .iter()
            .position(|event| event.kind == "story_echo" && !event.used);
        if let Some(index) = story_index {
            self.use_dungeon_event(index);
        }
        let gold_after = self.state.as_ref().map(|state| state.gold).unwrap_or(0);
        let xp_after = self.state.as_ref().map(|state| state.xp).unwrap_or(0);
        let level_after = self.state.as_ref().map(|state| state.level).unwrap_or(0);
        let mut snapshot = VarDictionary::new();
        snapshot.set("puzzle_found", puzzle_group.is_some());
        snapshot.set("wrong_reset", wrong_reset);
        snapshot.set("puzzle_solved", puzzle_solved);
        snapshot.set("story_found", story_index.is_some());
        snapshot.set(
            "story_used",
            story_index.is_some_and(|index| self.dungeon_events[index].used),
        );
        snapshot.set("gold_gained", gold_after > gold_before);
        snapshot.set(
            "xp_gained",
            xp_after != xp_before || level_after > level_before,
        );
        snapshot
    }

    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_enter_dungeon(&mut self, depth: i64) -> VarDictionary {
        self.enter_dungeon(depth.clamp(1, 32) as u32);

        let dungeon_enemies = self
            .enemies
            .iter()
            .filter(|enemy| enemy.get_position().x > 250.0)
            .count();
        let dungeon_items = self
            .world_items
            .iter()
            .filter(|item| item.in_dungeon)
            .count();
        let floor_cells = self.minimap_floor.iter().filter(|cell| **cell).count();
        let portal_reachable = self
            .dungeon_nav
            .as_ref()
            .and_then(|navigation| {
                let player_position = self.player()
                    .map(|player| player.get_global_position() - DUNGEON_OFFSET)?;
                navigation.astar(
                    NavGrid::cell_of(convert::to_core(player_position)),
                    NavGrid::cell_of(convert::to_core(self.next_portal - DUNGEON_OFFSET)),
                )
            })
            .is_some();
        let room_focals = self
            .base()
            .get_tree()
            .get_nodes_in_group("dungeon_room_focals");
        let focal_nodes: i64 = room_focals
            .iter_shared()
            .map(|node| node.get_child_count() as i64)
            .sum();
        let mut focal_profiles = Vec::new();
        for node in room_focals.iter_shared() {
            let name = node.get_name().to_string();
            let profile = name.split('_').nth(1).unwrap_or_default().to_string();
            if !focal_profiles.contains(&profile) {
                focal_profiles.push(profile);
            }
        }

        let mut snapshot = VarDictionary::new();
        snapshot.set("depth", self.dungeon_depth as i64);
        snapshot.set("root", self.dungeon_root.is_some());
        snapshot.set("navigation", self.dungeon_nav.is_some());
        snapshot.set("floor_cells", floor_cells as i64);
        snapshot.set("enemies", dungeon_enemies as i64);
        snapshot.set("items", dungeon_items as i64);
        snapshot.set("hazards", self.dungeon_hazards.len() as i64);
        snapshot.set("room_count", self.dungeon_room_roles.len() as i64);
        snapshot.set("map_rooms", self.dungeon_room_markers.len() as i64);
        snapshot.set("map_hazards", self.dungeon_hazards.len() as i64);
        snapshot.set("dressing_nodes", self.dungeon_dressing_nodes as i64);
        snapshot.set("room_focals", room_focals.len() as i64);
        snapshot.set("focal_nodes", focal_nodes);
        snapshot.set("focal_profiles", focal_profiles.len() as i64);
        let mut dressing_variants = self.dungeon_dressing_variants.clone();
        dressing_variants.sort_unstable();
        dressing_variants.dedup();
        snapshot.set("dressing_variants", dressing_variants.len() as i64);
        snapshot.set("events", self.dungeon_events.len() as i64);
        snapshot.set(
            "puzzle_switches",
            self.dungeon_events
                .iter()
                .filter(|event| event.kind == "puzzle_switch")
                .count() as i64,
        );
        snapshot.set(
            "story_echoes",
            self.dungeon_events
                .iter()
                .filter(|event| event.kind == "story_echo")
                .count() as i64,
        );
        snapshot.set(
            "map_ready",
            self.minimap_rect
                .as_ref()
                .and_then(|rect| rect.get_texture())
                .is_some(),
        );
        snapshot.set(
            "map_legend",
            self.minimap_legend
                .as_ref()
                .is_some_and(|legend| legend.is_visible()),
        );
        for role in [
            "safe",
            "arena",
            "gallery",
            "ritual",
            "treasure",
            "ambush",
            "traversal",
            "puzzle",
            "story",
            "antechamber",
        ] {
            snapshot.set(
                format!("has_{role}"),
                self.dungeon_room_roles
                    .iter()
                    .any(|current| current == role),
            );
        }
        snapshot.set("exit_portal", self.exit_portal != Vector3::ZERO);
        snapshot.set("next_portal", self.next_portal != Vector3::ZERO);
        snapshot.set("portal_reachable", portal_reachable);
        snapshot
    }

    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_kill_enemy(&mut self, boss_only: bool) -> VarDictionary {
        let before = self.enemies.len();
        let cores_before = self
            .state
            .as_ref()
            .map(|state| state.weapon_mod_cores)
            .unwrap_or(0);
        let xp_before = self.state.as_ref().map(|state| state.xp).unwrap_or(0);
        let level_before = self.state.as_ref().map(|state| state.level).unwrap_or(0);
        let target = self.enemies.iter().position(|enemy| {
            enemy.get_position().x > 250.0
                && enemy.bind().alive
                && (!boss_only || enemy.bind().is_boss)
        });
        let target_kind = target.map(|index| self.enemies[index].bind().cfg_id.to_string());
        let target_trophy = target_kind.as_deref().map(|kind| match kind {
            "crypt_warden" => "trophy_warden_seal",
            "blood_oracle" => "trophy_oracle_chalice",
            "archive_sentinel" => "trophy_archive_lens",
            _ => "trophy_tyrant_heart",
        });
        if boss_only {
            if let (Some(state), Some(kind), Some(trophy)) =
                (self.state.as_mut(), target_kind.as_ref(), target_trophy)
            {
                state.flags.remove(&format!("boss_defeated_{kind}"));
                state.inventory.items.retain(|item| item.id != trophy);
            }
        }

        let mut dealt = 0.0;
        if let Some(index) = target {
            let lethal_damage = self.enemies[index].bind().max_hp * 100.0 + 1000.0;
            let mut enemy = self.enemies[index].bind_mut();
            dealt = enemy.take_damage(lethal_damage, DmgType::Physical);
            enemy.death_timer = 0.0;
        }
        self.process_kills();

        let xp_after = self.state.as_ref().map(|state| state.xp).unwrap_or(0);
        let level_after = self.state.as_ref().map(|state| state.level).unwrap_or(0);
        let mut snapshot = VarDictionary::new();
        snapshot.set("target_found", target.is_some());
        snapshot.set("damage", dealt as f64);
        snapshot.set("enemy_removed", self.enemies.len() < before);
        snapshot.set(
            "xp_gained",
            level_after > level_before || (level_after == level_before && xp_after > xp_before),
        );
        snapshot.set(
            "boss_quest_progress",
            self.state
                .as_ref()
                .and_then(|state| state.quest_kills.get("heart_execution"))
                .copied()
                .unwrap_or(0) as i64,
        );
        snapshot.set(
            "boss_core_gained",
            self.state
                .as_ref()
                .map(|state| state.weapon_mod_cores > cores_before)
                .unwrap_or(false),
        );
        snapshot.set(
            "boss_flag_set",
            target_kind
                .as_ref()
                .and_then(|kind| {
                    self.state
                        .as_ref()
                        .map(|state| state.flags.contains(&format!("boss_defeated_{kind}")))
                })
                .unwrap_or(false),
        );
        snapshot.set(
            "boss_trophy_gained",
            self.state
                .as_ref()
                .map(|state| {
                    target_trophy
                        .map(|trophy| state.inventory.items.iter().any(|item| item.id == trophy))
                        .unwrap_or(false)
                })
                .unwrap_or(false),
        );
        snapshot.set(
            "boss_memorial_spawned",
            target_kind
                .as_ref()
                .map(|kind| {
                    self.base()
                        .get_node_or_null(&NodePath::from(format!("BossMemorial_{kind}").as_str()))
                        .is_some()
                })
                .unwrap_or(false),
        );
        snapshot
    }

    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_world_districts(&mut self) -> VarDictionary {
        let districts = self.world_districts.clone();
        let original_position = self.player()
            .map(|player| player.get_global_position());
        let mut recognized = 0usize;
        for (index, district) in districts.iter().enumerate() {
            if let Some(player) = self.player() {
                if let Ok(mut player) = player.clone().try_cast::<Player>() {
                    player
                        .bind_mut()
                        .teleport(district.center + Vector3::new(0.0, 1.1, 0.0));
                }
            }
            self.active_district = None;
            self.update_world_district();
            if self.active_district == Some(index) {
                recognized += 1;
            }
        }
        if let (Some(position), Some(player)) = (original_position, self.player()) {
            if let Ok(mut player) = player.clone().try_cast::<Player>() {
                player.bind_mut().teleport(position);
            }
        }
        self.active_district = None;
        self.update_world_district();

        let decor_clusters = self
            .base()
            .get_tree()
            .get_nodes_in_group("map_decor_clusters");
        let decor_nodes: i64 = decor_clusters
            .iter_shared()
            .map(|node| node.get_child_count() as i64)
            .sum();
        let route_layers = self
            .base()
            .get_tree()
            .get_nodes_in_group("map_route_layers");
        let route_nodes: i64 = route_layers
            .iter_shared()
            .map(|node| node.get_child_count() as i64)
            .sum();
        let skyline_beacons = self
            .base()
            .get_tree()
            .get_nodes_in_group("map_skyline_beacons");
        let beacon_nodes: i64 = skyline_beacons
            .iter_shared()
            .map(|node| node.get_child_count() as i64)
            .sum();
        let ambient_before: Vec<Vector3> = self
            .map_ambient
            .iter()
            .map(|ambient| ambient.node.get_position())
            .collect();
        self.tick_map_ambient(0.37);
        let ambient_moved = self
            .map_ambient
            .iter()
            .zip(&ambient_before)
            .filter(|(ambient, before)| ambient.node.get_position().distance_to(**before) > 0.0001)
            .count();
        let mode_before = self.mode;
        self.mode = Mode::Paused;
        let paused_before: Vec<Vector3> = self
            .map_ambient
            .iter()
            .map(|ambient| ambient.node.get_position())
            .collect();
        self.tick_map_ambient(0.5);
        let paused_static = self
            .map_ambient
            .iter()
            .zip(&paused_before)
            .all(|(ambient, before)| ambient.node.get_position().distance_to(*before) <= 0.0001);
        self.mode = mode_before;
        let mut ambient_profiles = Vec::new();
        for ambient in &self.map_ambient {
            let profile = (
                (ambient.speed * 100.0).round() as i32,
                (ambient.bob * 100.0).round() as i32,
                (ambient.spin * 100.0).round() as i32,
            );
            if !ambient_profiles.contains(&profile) {
                ambient_profiles.push(profile);
            }
        }

        let mut snapshot = VarDictionary::new();
        snapshot.set("total", districts.len() as i64);
        snapshot.set("recognized", recognized as i64);
        snapshot.set("decor_clusters", decor_clusters.len() as i64);
        snapshot.set("decor_nodes", decor_nodes);
        snapshot.set("route_layers", route_layers.len() as i64);
        snapshot.set("route_nodes", route_nodes);
        snapshot.set("skyline_beacons", skyline_beacons.len() as i64);
        snapshot.set("beacon_nodes", beacon_nodes);
        snapshot.set("ambient", self.map_ambient.len() as i64);
        snapshot.set("ambient_moved", ambient_moved as i64);
        snapshot.set("ambient_profiles", ambient_profiles.len() as i64);
        snapshot.set("ambient_paused", paused_static);
        snapshot
    }

    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_enemy_roster(&self) -> VarDictionary {
        let mut roles = Vec::<String>::new();
        let total = self.cfg.as_ref().map_or(0, |config| {
            for enemy in &config.enemies {
                if !roles.contains(&enemy.role) {
                    roles.push(enemy.role.clone());
                }
            }
            config.enemies.len()
        });
        let mut snapshot = VarDictionary::new();
        snapshot.set("total", total as i64);
        snapshot.set("roles", roles.len() as i64);
        snapshot
    }

    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_enemy_animations(&mut self) -> VarDictionary {
        let mut total = 0usize;
        let mut full_state_sets = 0usize;
        let mut cast_animated = 0usize;
        let mut pain_animated = 0usize;
        let mut one_shots = 0usize;
        let mut signatures = Vec::new();
        for enemy in self.enemies.iter_mut().filter(|enemy| enemy.bind().alive) {
            let (states, cast, pain, one_shot, signature) =
                enemy.bind_mut().runtime_smoke_animation_profile();
            total += 1;
            full_state_sets += usize::from(states == 9);
            cast_animated += usize::from(cast);
            pain_animated += usize::from(pain);
            one_shots += usize::from(one_shot);
            if !signatures.contains(&signature) {
                signatures.push(signature);
            }
        }
        let mut snapshot = VarDictionary::new();
        snapshot.set("total", total as i64);
        snapshot.set("full_state_sets", full_state_sets as i64);
        snapshot.set("cast_animated", cast_animated as i64);
        snapshot.set("pain_animated", pain_animated as i64);
        snapshot.set("one_shots", one_shots as i64);
        snapshot.set("timing_profiles", signatures.len() as i64);
        snapshot
    }

    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_weapon_profiles(&mut self) -> VarDictionary {
        self.autosave_enabled = false;
        let mut animated = 0usize;
        let mut feedback_complete = 0usize;
        let mut feedback_signatures = Vec::new();
        let mut feedback_profiles = Vec::new();
        let mut audio_complete = 0usize;
        let mut audio_signatures = Vec::new();
        let mut audio_assets = 0usize;
        let mut audio_assets_loaded = 0usize;
        let mut accuracy_complete = 0usize;
        let mut accuracy_signatures = Vec::new();
        let mut alt_names = Vec::new();
        let mut alt_melee = 0usize;
        let mut alt_hitscan = 0usize;
        let mut alt_projectile = 0usize;
        let mut alt_animation_complete = 0usize;
        let mut alt_animation_distinct = 0usize;
        let mut alt_animation_signatures = Vec::new();
        for slot in 0..8 {
            let weapon = WeaponId::from_slot(slot);
            let def = weapon_def(weapon);
            self.arsenal.owned[slot] = true;
            self.arsenal.current = weapon;
            self.refresh_weapon_sheet();
            self.tick_weapon_anim(def.switch_time + 0.01);
            if matches!(self.weapon_anim, WeaponAnim::Idle) && def.switch_frames.len() >= 2 {
                animated += 1;
            }
            let feedback = def.feedback;
            feedback_complete += usize::from(
                feedback.muzzle_energy > 0.0
                    && feedback.muzzle_duration > 0.0
                    && feedback.impact_scale > 0.0
                    && feedback.impact_duration > 0.0
                    && feedback.tracer_scale > 0.0
                    && feedback.tracer_duration > 0.0,
            );
            let signature = format!(
                "{:?}:{:.2}:{:?}:{:.2}:{:?}:{:.2}",
                feedback.muzzle_color,
                feedback.muzzle_energy,
                feedback.impact_color,
                feedback.impact_scale,
                feedback.tracer_color,
                feedback.tracer_scale,
            );
            if !feedback_signatures.contains(&signature) {
                feedback_signatures.push(signature);
            }
            feedback_profiles.push(feedback);
            let audio = &def.audio;
            audio_complete += usize::from(
                !audio.fire_sfx.is_empty()
                    && !audio.impact_sfx.is_empty()
                    && (def.magazine == 0 || !audio.reload_sfx.is_empty()),
            );
            let audio_signature = format!(
                "{:?}:{:?}:{:?}:{:?}:{:?}:{:.1}:{:.1}:{:.1}",
                audio.fire_sfx,
                audio.impact_sfx,
                audio.fire_pitch,
                audio.impact_pitch,
                audio.reload_pitch,
                audio.fire_volume_db,
                audio.impact_volume_db,
                audio.reload_volume_db,
            );
            if !audio_signatures.contains(&audio_signature) {
                audio_signatures.push(audio_signature);
            }
            for path in audio
                .fire_sfx
                .iter()
                .chain(audio.impact_sfx.iter())
                .chain(audio.reload_sfx.iter())
            {
                audio_assets += 1;
                audio_assets_loaded += usize::from(combat::weapon_sfx_loadable(path));
            }
            let accuracy = def.accuracy;
            accuracy_complete += usize::from(
                accuracy.bloom_per_shot >= 0.0
                    && accuracy.max_bloom > 0.0
                    && accuracy.recovery > 0.0
                    && accuracy.crosshair_scale > 0.0,
            );
            let accuracy_signature = format!(
                "{:.2}:{:.2}:{:.2}:{:.2}:{:.2}",
                accuracy.bloom_per_shot,
                accuracy.max_bloom,
                accuracy.recovery,
                accuracy.move_penalty,
                accuracy.crosshair_scale,
            );
            if !accuracy_signatures.contains(&accuracy_signature) {
                accuracy_signatures.push(accuracy_signature);
            }
            if let Some(alt) = def.alt_fire.as_ref() {
                if !alt_names.contains(&alt.name_en) {
                    alt_names.push(alt.name_en.clone());
                }
                match alt.kind {
                    FireKind::Melee => alt_melee += 1,
                    FireKind::Hitscan { .. } => alt_hitscan += 1,
                    FireKind::Projectile { .. } => alt_projectile += 1,
                }
                alt_animation_complete += usize::from(
                    !alt.animation_frames.is_empty()
                        && alt.animation_fps > 0.0
                        && (0.05..=0.95).contains(&alt.hit_ratio),
                );
                alt_animation_distinct +=
                    usize::from(def.attack_frames(true) != def.attack_frames(false));
                let signature = format!(
                    "{:?}:{:.2}:{:.2}",
                    def.attack_frames(true),
                    def.attack_fps(true),
                    def.attack_hit_ratio(true),
                );
                if !alt_animation_signatures.contains(&signature) {
                    alt_animation_signatures.push(signature);
                }
            }
        }

        let impact_before = self.sprite_fx.len();
        let impact_lights_before = self.light_fx.len();
        for (slot, feedback) in feedback_profiles.iter().copied().enumerate() {
            self.spawn_weapon_impact(
                feedback,
                Vector3::new(slot as f32 * 0.35, 1.0, -3.0),
                slot % 2 == 0,
            );
        }
        let impact_fx = self.sprite_fx.len().saturating_sub(impact_before);
        let impact_lights = self.light_fx.len().saturating_sub(impact_lights_before);

        let tracer_before = self.sprite_fx.len();
        for (slot, feedback) in feedback_profiles.iter().copied().enumerate() {
            let x = slot as f32 * 0.25;
            self.spawn_weapon_tracer(
                feedback,
                Vector3::new(x, 1.1, -1.0),
                Vector3::new(x, 1.1, -12.0),
            );
            self.flash_muzzle(feedback);
        }
        let tracer_fx = self.sprite_fx.len().saturating_sub(tracer_before);

        let impact_audio_before = self.sfx_3d.len();
        for slot in 0..8 {
            self.play_weapon_impact_sfx(
                WeaponId::from_slot(slot),
                Vector3::new(slot as f32 * 0.3, 1.0, -4.0),
            );
        }
        let impact_audio_players = self.sfx_3d.len().saturating_sub(impact_audio_before);

        self.weapon_bloom = 0.0;
        self.hit_confirm_timer = 0.0;
        self.critical_confirm_timer = 0.0;
        self.kill_confirm_timer = 0.0;
        self.update_crosshair();
        let precise_crosshair = self
            .crosshair
            .as_ref()
            .map(|crosshair| crosshair.get_text().to_string())
            .unwrap_or_default();
        let accuracy = weapon_def(self.arsenal.current).accuracy;
        self.add_weapon_bloom(accuracy.max_bloom);
        let bloom_peak = self.weapon_bloom;
        self.update_crosshair();
        let bloom_crosshair = self
            .crosshair
            .as_ref()
            .map(|crosshair| crosshair.get_text().to_string())
            .unwrap_or_default();
        self.tick_weapon_accuracy(accuracy.max_bloom / accuracy.recovery + 0.05);
        let bloom_recovered = self.weapon_bloom < 0.001;
        self.register_weapon_hit(false, false);
        self.update_crosshair();
        let hit_crosshair = self
            .crosshair
            .as_ref()
            .map(|crosshair| crosshair.get_text().to_string())
            .unwrap_or_default();
        self.register_weapon_hit(true, false);
        self.update_crosshair();
        let kill_crosshair = self
            .crosshair
            .as_ref()
            .map(|crosshair| crosshair.get_text().to_string())
            .unwrap_or_default();

        self.weapon_anim = WeaponAnim::Idle;
        self.shoot_cd = 0.0;
        self.weapon_bloom = 0.0;
        let alt_weapon = self.arsenal.current;
        let alt_def = weapon_def(alt_weapon);
        self.arsenal.clips[alt_weapon.slot()] = alt_def.magazine.max(1);
        let alt_projectiles_before = self.projectiles.len();
        self.try_alt_fire();
        let alt_animation_started = matches!(
            self.weapon_anim,
            WeaponAnim::Fire {
                frame: 0,
                secondary: true
            }
        );
        let alt_spawned_projectile = self.projectiles.len() > alt_projectiles_before;
        let alt_extended_cooldown = self.shoot_cd > alt_def.cooldown * self.loadout.cd_mult;
        let alt_added_bloom = self.weapon_bloom > alt_def.accuracy.bloom_per_shot;

        let weapon = self.arsenal.current;
        let def = weapon_def(weapon);
        self.weapon_anim = WeaponAnim::Idle;
        self.shoot_cd = 0.0;
        self.arsenal.clips[weapon.slot()] = def.magazine.max(1);
        let clip_before = self.arsenal.clips[weapon.slot()];
        let aim_before = self.player()
            .and_then(|player| player.clone().try_cast::<Player>().ok())
            .map(|player| player.bind().aim_dir().y)
            .unwrap_or(0.0);
        let fire_audio_before = self.sfx_2d.len();
        self.try_fire();
        let aim_after = self.player()
            .and_then(|player| player.clone().try_cast::<Player>().ok())
            .map(|player| player.bind().aim_dir().y)
            .unwrap_or(0.0);

        let mut snapshot = VarDictionary::new();
        snapshot.set("total", 8i64);
        snapshot.set("animated", animated as i64);
        snapshot.set("feedback_complete", feedback_complete as i64);
        snapshot.set("feedback_profiles", feedback_signatures.len() as i64);
        snapshot.set("audio_complete", audio_complete as i64);
        snapshot.set("audio_profiles", audio_signatures.len() as i64);
        snapshot.set("audio_assets", audio_assets as i64);
        snapshot.set("audio_assets_loaded", audio_assets_loaded as i64);
        snapshot.set("accuracy_complete", accuracy_complete as i64);
        snapshot.set("accuracy_profiles", accuracy_signatures.len() as i64);
        snapshot.set("alt_profiles", alt_names.len() as i64);
        snapshot.set("alt_melee", alt_melee as i64);
        snapshot.set("alt_hitscan", alt_hitscan as i64);
        snapshot.set("alt_projectile", alt_projectile as i64);
        snapshot.set("alt_animation_complete", alt_animation_complete as i64);
        snapshot.set("alt_animation_distinct", alt_animation_distinct as i64);
        snapshot.set(
            "alt_animation_profiles",
            alt_animation_signatures.len() as i64,
        );
        snapshot.set("alt_animation_started", alt_animation_started);
        snapshot.set("alt_spawned_projectile", alt_spawned_projectile);
        snapshot.set("alt_extended_cooldown", alt_extended_cooldown);
        snapshot.set("alt_added_bloom", alt_added_bloom);
        snapshot.set("bloom_peak", bloom_peak as f64);
        snapshot.set("bloom_recovered", bloom_recovered);
        snapshot.set("crosshair_expanded", precise_crosshair != bloom_crosshair);
        snapshot.set("hit_confirmed", hit_crosshair.contains('×'));
        snapshot.set("kill_confirmed", kill_crosshair.contains('X'));
        snapshot.set("impact_audio_players", impact_audio_players as i64);
        snapshot.set("fire_audio_played", self.sfx_2d.len() > fire_audio_before);
        snapshot.set("impact_fx", impact_fx as i64);
        snapshot.set("impact_lights", impact_lights as i64);
        snapshot.set("tracer_fx", tracer_fx as i64);
        snapshot.set(
            "muzzle_active",
            self.muzzle_timer > 0.0 && self.muzzle_light.is_some(),
        );
        snapshot.set(
            "ammo_consumed",
            self.arsenal.clips[weapon.slot()] < clip_before,
        );
        snapshot.set("recoil_applied", aim_after > aim_before);
        snapshot
    }

    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_weapon_mods(&mut self) -> VarDictionary {
        self.autosave_enabled = false;
        let mods = weapon_mods();
        let localized = mods
            .iter()
            .filter(|entry| {
                !entry.name_ru.is_empty()
                    && !entry.name_en.is_empty()
                    && !entry.desc_ru.is_empty()
                    && !entry.desc_en.is_empty()
            })
            .count();
        let visual_complete = mods
            .iter()
            .filter(|entry| {
                entry.feedback.color_mix > 0.0
                    && entry.feedback.muzzle_mult > 0.0
                    && entry.feedback.tracer_mult > 0.0
                    && entry.feedback.impact_mult > 0.0
                    && entry.feedback.pitch_mult > 0.0
            })
            .count();
        let mut visual_signatures = Vec::new();
        for entry in mods {
            let signature = format!(
                "{:?}:{:.2}:{:.2}:{:.2}:{:.2}:{:.2}",
                entry.feedback.tint,
                entry.feedback.color_mix,
                entry.feedback.muzzle_mult,
                entry.feedback.tracer_mult,
                entry.feedback.impact_mult,
                entry.feedback.pitch_mult,
            );
            if !visual_signatures.contains(&signature) {
                visual_signatures.push(signature);
            }
        }
        let mut branch_pairs_distinct = 0usize;
        let mut effective_signatures = Vec::new();
        let impact_before = self.sprite_fx.len();
        let light_before = self.light_fx.len();
        for weapon in WeaponId::ALL {
            let mut pair = Vec::new();
            for branch in 1..=2 {
                if let Some(state) = self.state.as_mut() {
                    state.weapon_mods[weapon.slot()] = branch;
                }
                let feedback = self.effective_weapon_feedback(weapon);
                let signature = format!(
                    "{:?}:{:.2}:{:?}:{:.2}:{:?}:{:.2}:{:.2}",
                    feedback.muzzle_color,
                    feedback.muzzle_energy,
                    feedback.impact_color,
                    feedback.impact_scale,
                    feedback.tracer_color,
                    feedback.tracer_scale,
                    self.effective_weapon_pitch(weapon),
                );
                pair.push(signature.clone());
                if !effective_signatures.contains(&signature) {
                    effective_signatures.push(signature);
                }
                self.spawn_weapon_impact(
                    feedback,
                    Vector3::new(weapon.slot() as f32, 1.0, -(branch as f32)),
                    false,
                );
            }
            branch_pairs_distinct += usize::from(pair[0] != pair[1]);
        }
        let branch_impact_fx = self.sprite_fx.len().saturating_sub(impact_before);
        let branch_impact_lights = self.light_fx.len().saturating_sub(light_before);
        let weapon = WeaponId::Pistol;
        self.arsenal.current = weapon;
        if let Some(state) = self.state.as_mut() {
            state.weapon_mods = [0; 8];
            state.weapon_mod_cores = 2;
        }
        self.select_weapon_mod(1);
        let first_view_color = self.weapon_mod_view_color(weapon);
        let first = self
            .state
            .as_ref()
            .map(|state| (state.weapon_mods[weapon.slot()], state.weapon_mod_cores))
            .unwrap_or_default();
        let first_changes_stats = weapon_mod_for(weapon, 1)
            .map(|entry| {
                entry.damage_mult != 1.0
                    || entry.cooldown_mult != 1.0
                    || entry.range_mult != 1.0
                    || entry.recoil_mult != 1.0
                    || entry.bloom_mult != 1.0
            })
            .unwrap_or(false);
        self.select_weapon_mod(2);
        let second_view_color = self.weapon_mod_view_color(weapon);
        let second = self
            .state
            .as_ref()
            .map(|state| (state.weapon_mods[weapon.slot()], state.weapon_mod_cores))
            .unwrap_or_default();
        let mut snapshot = VarDictionary::new();
        snapshot.set("total", mods.len() as i64);
        snapshot.set("localized", localized as i64);
        snapshot.set("visual_complete", visual_complete as i64);
        snapshot.set("visual_profiles", visual_signatures.len() as i64);
        snapshot.set("effective_profiles", effective_signatures.len() as i64);
        snapshot.set("branch_pairs_distinct", branch_pairs_distinct as i64);
        snapshot.set("branch_impact_fx", branch_impact_fx as i64);
        snapshot.set("branch_impact_lights", branch_impact_lights as i64);
        snapshot.set("first_selected", first.0 as i64);
        snapshot.set("first_cores", first.1 as i64);
        snapshot.set("second_selected", second.0 as i64);
        snapshot.set("second_cores", second.1 as i64);
        snapshot.set("first_changes_stats", first_changes_stats);
        snapshot.set(
            "view_identity_distinct",
            first_view_color != second_view_color,
        );
        snapshot.set(
            "view_visual_applied",
            self.weapon_rect
                .as_ref()
                .map(|weapon_rect| weapon_rect.get_modulate() == second_view_color)
                .unwrap_or(false),
        );
        snapshot
    }

    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_weak_points(&mut self) -> VarDictionary {
        let configured = self
            .cfg
            .as_ref()
            .map(|cfg| {
                cfg.enemies
                    .iter()
                    .filter(|enemy| {
                        (0.5..=0.95).contains(&enemy.weak_point.height)
                            && (1.0..=3.0).contains(&enemy.weak_point.multiplier)
                    })
                    .count()
            })
            .unwrap_or(0);
        let boss_ids = [
            "crypt_warden",
            "blood_oracle",
            "archive_sentinel",
            "heart_tyrant",
        ];
        let boss_profiles = self
            .cfg
            .as_ref()
            .map(|cfg| {
                boss_ids
                    .iter()
                    .filter_map(|id| cfg.enemy(id))
                    .filter(|enemy| {
                        enemy.weak_point.height != 0.7 || enemy.weak_point.multiplier != 1.6
                    })
                    .count()
            })
            .unwrap_or(0);

        let mut normal_damage = 0.0;
        let mut critical_damage = 0.0;
        let mut critical = false;
        if let Some(mut enemy) = self
            .enemies
            .iter()
            .find(|enemy| enemy.bind().alive)
            .cloned()
        {
            let origin = enemy.get_global_position();
            let max_hp = enemy.bind().max_hp;
            enemy.bind_mut().hp = max_hp;
            (normal_damage, _) = Self::apply_precise_damage(
                &mut enemy,
                origin + Vector3::UP * 0.1,
                8.0,
                DmgType::Physical,
            );
            enemy.bind_mut().hp = max_hp;
            (critical_damage, critical) = Self::apply_precise_damage(
                &mut enemy,
                origin + Vector3::UP * 100.0,
                8.0,
                DmgType::Physical,
            );
            enemy.bind_mut().hp = max_hp;
        }

        self.hit_confirm_timer = 0.0;
        self.critical_confirm_timer = 0.0;
        self.kill_confirm_timer = 0.0;
        self.register_weapon_hit(false, true);
        self.update_crosshair();
        let critical_crosshair = self
            .crosshair
            .as_ref()
            .map(|crosshair| crosshair.get_text().to_string().contains("CRIT"))
            .unwrap_or(false);

        let mut snapshot = VarDictionary::new();
        snapshot.set("configured", configured as i64);
        snapshot.set("boss_profiles", boss_profiles as i64);
        snapshot.set("normal_damage", normal_damage as f64);
        snapshot.set("critical_damage", critical_damage as f64);
        snapshot.set("critical", critical);
        snapshot.set("critical_crosshair", critical_crosshair);
        snapshot.set("localized", true);
        snapshot
    }

    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_dialogue_conditions(&mut self) -> VarDictionary {
        let (original_flags, original_stats) = self
            .state
            .as_ref()
            .map(|state| (state.flags.clone(), state.stats.clone()))
            .unwrap_or_else(|| {
                (
                    std::collections::HashSet::new(),
                    crate::character::Stats::new("Smoke"),
                )
            });
        if let Some(state) = self.state.as_mut() {
            for boss in [
                "crypt_warden",
                "blood_oracle",
                "archive_sentinel",
                "heart_tyrant",
            ] {
                state.flags.insert(format!("boss_defeated_{boss}"));
                state.flags.remove(&format!("discussed_{boss}"));
            }
            state.stats.intelligence = 5;
            state.stats.willpower = 5;
            state.flags.remove("heart_answer_self");
        }

        let expected = [
            ("victor", "hub_victor_warden_aftermath"),
            ("elena", "hub_elena_oracle_aftermath"),
            ("vale", "hub_vale_archive_aftermath"),
            ("stranger", "hub_stranger_tyrant_aftermath"),
        ];
        let reactive = self.state.as_ref().map_or(0, |state| {
            expected
                .iter()
                .filter(|(npc, scene)| npc_scene_id(npc, state) == *scene)
                .count()
        });
        let localized = expected
            .iter()
            .filter_map(|(_, scene)| self.resolve_scene(scene))
            .filter(|scene| {
                scene.lines.iter().all(|line| {
                    !crate::dialogue::localized(&line.text, "ru").is_empty()
                        && !crate::dialogue::localized(&line.text, "en").is_empty()
                }) && scene.choices.iter().all(|choice| {
                    !crate::dialogue::localized(&choice.text, "ru").is_empty()
                        && !crate::dialogue::localized(&choice.text, "en").is_empty()
                })
            })
            .count();

        let vale = self.resolve_scene("hub_vale_archive_aftermath");
        let low_stat_choices = vale
            .as_ref()
            .zip(self.state.as_ref())
            .map(|(scene, state)| {
                scene
                    .choices
                    .iter()
                    .filter(|choice| {
                        choice
                            .requires
                            .as_ref()
                            .is_none_or(|requirement| requirement.is_met(state))
                    })
                    .count()
            })
            .unwrap_or(0);
        if let Some(state) = self.state.as_mut() {
            state.stats.intelligence = 10;
        }
        let high_stat_choices = vale
            .as_ref()
            .zip(self.state.as_ref())
            .map(|(scene, state)| {
                scene
                    .choices
                    .iter()
                    .filter(|choice| {
                        choice
                            .requires
                            .as_ref()
                            .is_none_or(|requirement| requirement.is_met(state))
                    })
                    .count()
            })
            .unwrap_or(0);

        let stranger = self.resolve_scene("hub_stranger_tyrant_aftermath");
        let not_flag_before = stranger
            .as_ref()
            .and_then(|scene| scene.choices.get(1))
            .and_then(|choice| choice.requires.as_ref())
            .zip(self.state.as_ref())
            .is_some_and(|(requirement, state)| requirement.is_met(state));
        if let Some(state) = self.state.as_mut() {
            state.flags.insert("heart_answer_self".into());
        }
        let not_flag_after = stranger
            .as_ref()
            .and_then(|scene| scene.choices.get(1))
            .and_then(|choice| choice.requires.as_ref())
            .zip(self.state.as_ref())
            .is_some_and(|(requirement, state)| requirement.is_met(state));
        let quest_conditions = self
            .resolve_scene("hub_elena_oracle_aftermath")
            .map(|scene| {
                scene
                    .choices
                    .iter()
                    .filter(|choice| {
                        choice
                            .requires
                            .as_ref()
                            .is_some_and(|requirement| requirement.quest_done.is_some())
                    })
                    .count()
            })
            .unwrap_or(0);

        if let Some(state) = self.state.as_mut() {
            state.flags = original_flags;
            state.stats = original_stats;
        }
        let mut snapshot = VarDictionary::new();
        snapshot.set("reactive", reactive as i64);
        snapshot.set("localized", localized as i64);
        snapshot.set("low_stat_choices", low_stat_choices as i64);
        snapshot.set("high_stat_choices", high_stat_choices as i64);
        snapshot.set("not_flag_before", not_flag_before);
        snapshot.set("not_flag_after", not_flag_after);
        snapshot.set("quest_conditions", quest_conditions as i64);
        snapshot
    }

    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_complete_quest_chains(&mut self) -> VarDictionary {
        self.autosave_enabled = false;
        if let Some(state) = self.state.as_mut() {
            state.quests = crate::quest::QuestLog::default();
            state.quest_kills.clear();
            state.dungeons_cleared = 0;
            state.inventory = crate::item::Inventory::default();
        }

        let quests = self
            .cfg
            .as_ref()
            .map(|config| config.quests.clone())
            .unwrap_or_default();
        let mut offered = 0usize;
        let mut completed = 0usize;
        let mut stalled = false;

        while completed < quests.len() {
            let next = quests
                .iter()
                .find(|quest| {
                    self.state.as_ref().is_some_and(|state| {
                        !state.quests.has(&quest.id)
                            && quest
                                .requires
                                .iter()
                                .all(|required| state.quests.is_completed(required))
                    })
                })
                .cloned();
            let Some(quest) = next else {
                stalled = true;
                break;
            };

            let offer_effects =
                self.make_quest_scene("Quest Smoke", &quest.id)
                    .and_then(|scene| {
                        scene
                            .choices
                            .into_iter()
                            .find(|choice| {
                            choice.effects.iter().any(
                                |effect| matches!(effect, Effect::Quest { id, .. } if id == &quest.id),
                            )
                            })
                            .map(|choice| choice.effects)
                    });
            let Some(offer_effects) = offer_effects else {
                stalled = true;
                break;
            };
            if let Some(state) = self.state.as_mut() {
                state.apply(&offer_effects, "en");
            }
            if !self
                .state
                .as_ref()
                .is_some_and(|state| state.quests.is_active(&quest.id))
            {
                stalled = true;
                break;
            }
            offered += 1;

            if let Some(state) = self.state.as_mut() {
                if quest.kind == "clear_dungeon" {
                    state.dungeons_cleared = state.dungeons_cleared.max(quest.count);
                } else {
                    state.quest_kills.insert(quest.id.clone(), quest.count);
                }
            }

            let completion_effects =
                self.make_quest_scene("Quest Smoke", &quest.id)
                    .and_then(|scene| {
                        scene
                            .choices
                            .into_iter()
                            .find(|choice| {
                                choice.effects.iter().any(
                                |effect| matches!(effect, Effect::QuestDone(id) if id == &quest.id),
                            )
                            })
                            .map(|choice| choice.effects)
                    });
            let Some(completion_effects) = completion_effects else {
                stalled = true;
                break;
            };
            if let Some(state) = self.state.as_mut() {
                state.apply(&completion_effects, "en");
            }
            if !self
                .state
                .as_ref()
                .is_some_and(|state| state.quests.is_completed(&quest.id))
            {
                stalled = true;
                break;
            }
            completed += 1;
        }

        let mut snapshot = VarDictionary::new();
        let chain_names: std::collections::HashSet<_> =
            quests.iter().map(|quest| quest.chain_en.as_str()).collect();
        let reward_quests = quests
            .iter()
            .filter(|quest| !quest.reward_items.is_empty())
            .count();
        let expected_reward_qty: u32 = quests
            .iter()
            .flat_map(|quest| quest.reward_items.iter())
            .map(|reward| reward.qty)
            .sum();
        let awarded_reward_qty: u32 = self
            .state
            .as_ref()
            .map(|state| state.inventory.items.iter().map(|item| item.qty).sum())
            .unwrap_or(0);
        if let Some(state) = self.state.as_mut() {
            if let Some(first) = state.quests.quests.first_mut() {
                first.state = crate::quest::QuestState::Active;
            }
        }
        self.update_quest_label("ru");
        let journal_ru = self
            .quest_label
            .as_ref()
            .map(|label| label.get_text().to_string())
            .unwrap_or_default();
        self.update_quest_label("en");
        let journal_en = self
            .quest_label
            .as_ref()
            .map(|label| label.get_text().to_string())
            .unwrap_or_default();
        snapshot.set("total", quests.len() as i64);
        snapshot.set("offered", offered as i64);
        snapshot.set("completed", completed as i64);
        snapshot.set("stalled", stalled);
        snapshot.set("chains", chain_names.len() as i64);
        snapshot.set("reward_quests", reward_quests as i64);
        snapshot.set("expected_reward_qty", expected_reward_qty as i64);
        snapshot.set("awarded_reward_qty", awarded_reward_qty as i64);
        snapshot.set(
            "journal_localized",
            journal_ru.contains("Этап") && journal_en.contains("Stage"),
        );
        snapshot.set(
            "exploration_completed",
            self.state.as_ref().map_or(0, |state| {
                ["relay_resonance", "archive_whispers"]
                    .iter()
                    .filter(|quest_id| state.quests.is_completed(quest_id))
                    .count() as i64
            }),
        );
        snapshot
    }

    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_quest_world_changes(&mut self) -> VarDictionary {
        self.refresh_quest_world_changes();
        let changes: Vec<_> = self
            .cfg
            .as_ref()
            .map(|config| {
                config
                    .quests
                    .iter()
                    .filter_map(|quest| {
                        quest
                            .world_change
                            .as_ref()
                            .map(|change| (quest.id.clone(), change.clone()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut built = 0usize;
        let mut nodes = 0i64;
        let mut patterns = std::collections::HashSet::new();
        let mut activity_profiles = std::collections::HashSet::new();
        let expected_activities: u32 = changes
            .iter()
            .map(|(_, change)| change.activity_count)
            .sum();
        let mut activities = 0usize;
        let mut first_activity_path = None;
        for (_, change) in &changes {
            patterns.insert(change.pattern.as_str());
            activity_profiles.insert(change.activity.as_str());
            let path = NodePath::from(format!("QuestEvolution_{}", change.id).as_str());
            if let Some(root) = self.base().get_node_or_null(&path) {
                built += 1;
                nodes += root.get_child_count() as i64;
            }
            for index in 0..change.activity_count {
                let actor_path = format!(
                    "QuestEvolution_{}/QuestActivity_{}_{}",
                    change.id, change.id, index
                );
                if self
                    .base()
                    .get_node_or_null(&NodePath::from(actor_path.as_str()))
                    .is_some()
                {
                    activities += 1;
                    first_activity_path.get_or_insert(actor_path);
                }
            }
        }
        let actor_position = |game: &Game3D, path: &str| {
            game.base()
                .get_node_or_null(&NodePath::from(path))
                .and_then(|node| node.try_cast::<Node3D>().ok())
                .map(|node| node.get_position())
        };
        let (activity_moved, activity_paused) = first_activity_path
            .as_deref()
            .and_then(|path| actor_position(self, path).map(|before| (path, before)))
            .map(|(path, before)| {
                let previous_mode = self.mode;
                self.mode = Mode::Explore;
                self.tick_map_ambient(0.43);
                let after = actor_position(self, path).unwrap_or(before);
                self.mode = Mode::Paused;
                self.tick_map_ambient(0.61);
                let paused = actor_position(self, path).unwrap_or(after);
                self.mode = previous_mode;
                (
                    before.distance_to(after) > 0.001,
                    after.distance_to(paused) < 0.0001,
                )
            })
            .unwrap_or((false, false));
        let persisted = self.state.as_ref().is_some_and(|state| {
            let data = crate::save::SaveData::from_game(state, 100.0, &self.arsenal);
            let (restored, _, _) = data.into_game();
            changes
                .iter()
                .all(|(quest_id, _)| restored.quests.is_completed(quest_id))
        });
        let mut snapshot = VarDictionary::new();
        snapshot.set("configured", changes.len() as i64);
        snapshot.set("built", built as i64);
        snapshot.set("nodes", nodes);
        snapshot.set("patterns", patterns.len() as i64);
        snapshot.set("activity_profiles", activity_profiles.len() as i64);
        snapshot.set("expected_activities", expected_activities as i64);
        snapshot.set("activities", activities as i64);
        snapshot.set("activity_moved", activity_moved);
        snapshot.set("activity_paused", activity_paused);
        snapshot.set("persisted", persisted);
        snapshot
    }

    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_quest_navigation(&mut self) -> VarDictionary {
        let original_states: Vec<_> = self
            .state
            .as_ref()
            .map(|state| {
                state
                    .quests
                    .quests
                    .iter()
                    .map(|quest| quest.state)
                    .collect()
            })
            .unwrap_or_default();
        if let Some(state) = self.state.as_mut() {
            for quest in state.quests.quests.iter_mut().take(2) {
                quest.state = crate::quest::QuestState::Active;
            }
        }
        self.tracked_quest_index = 0;
        self.update_quest_navigation("ru");
        let ru = self
            .quest_objective_label
            .as_ref()
            .map(|label| label.get_text().to_string())
            .unwrap_or_default();
        self.update_quest_navigation("en");
        let first = self
            .quest_objective_label
            .as_ref()
            .map(|label| label.get_text().to_string())
            .unwrap_or_default();
        self.tracked_quest_index = 1;
        self.update_quest_navigation("en");
        let second = self
            .quest_objective_label
            .as_ref()
            .map(|label| label.get_text().to_string())
            .unwrap_or_default();
        let marker_visible = self
            .quest_objective_marker
            .as_ref()
            .is_some_and(|marker| marker.is_visible());
        if let Some(state) = self.state.as_mut() {
            for (quest, original) in state.quests.quests.iter_mut().zip(original_states) {
                quest.state = original;
            }
        }
        let mut snapshot = VarDictionary::new();
        snapshot.set(
            "localized",
            ru.contains("ЦЕЛЬ") && first.contains("TRACKED"),
        );
        snapshot.set("described", first.lines().count() >= 2);
        snapshot.set("distance", first.contains(" m"));
        snapshot.set("target", self.quest_objective_target.is_some());
        snapshot.set("marker_visible", marker_visible);
        snapshot.set("cycled", first != second);
        snapshot
    }

    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_quest_journal(&mut self) -> VarDictionary {
        let original_lang = self.settings.lang.clone();
        let original_states: Vec<_> = self
            .state
            .as_ref()
            .map(|state| {
                state
                    .quests
                    .quests
                    .iter()
                    .map(|quest| quest.state)
                    .collect()
            })
            .unwrap_or_default();
        if let Some(state) = self.state.as_mut() {
            for quest in state.quests.quests.iter_mut().take(2) {
                quest.state = crate::quest::QuestState::Active;
            }
        }
        self.settings.lang = "ru".into();
        self.open_journal();
        let ru = self
            .journal_list
            .as_ref()
            .map(|label| label.get_text().to_string())
            .unwrap_or_default();
        self.settings.lang = "en".into();
        self.refresh_quest_journal_ui();
        let en = self
            .journal_list
            .as_ref()
            .map(|label| label.get_text().to_string())
            .unwrap_or_default();
        self.track_journal_quest(1);
        let tracked = self.tracked_quest_index == 1
            && self
                .journal_list
                .as_ref()
                .is_some_and(|label| label.get_text().to_string().contains("[2] ▶"));
        let panel_visible = self
            .journal_panel
            .as_ref()
            .is_some_and(|panel| panel.is_visible());
        self.close_journal();
        if let Some(state) = self.state.as_mut() {
            for (quest, original) in state.quests.quests.iter_mut().zip(original_states) {
                quest.state = original;
            }
        }
        self.settings.lang = original_lang;
        let mut snapshot = VarDictionary::new();
        snapshot.set(
            "localized",
            ru.contains("АКТИВНЫЕ") && en.contains("ACTIVE:"),
        );
        snapshot.set("chains", en.matches("━━").count() as i64 / 2);
        snapshot.set(
            "states",
            en.contains("◆ ACTIVE") && en.contains("✓ COMPLETED"),
        );
        snapshot.set("rewards", en.contains(" XP") && en.contains("gold"));
        snapshot.set("panel_visible", panel_visible);
        snapshot.set("tracked", tracked);
        snapshot.set("closed", self.mode == Mode::Explore);
        snapshot
    }

    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_dungeon_quest_navigation(&mut self) -> VarDictionary {
        let original_states: Vec<_> = self
            .state
            .as_ref()
            .map(|state| {
                state
                    .quests
                    .quests
                    .iter()
                    .map(|quest| quest.state)
                    .collect()
            })
            .unwrap_or_default();
        if let Some(state) = self.state.as_mut() {
            for quest in &mut state.quests.quests {
                quest.state = if quest.id == "archive_whispers" {
                    crate::quest::QuestState::Active
                } else {
                    crate::quest::QuestState::Completed
                };
            }
        }
        self.tracked_quest_index = 0;
        self.update_quest_navigation("en");
        let target = self.quest_objective_target;
        let points_to_echo = target.is_some_and(|target| {
            self.dungeon_events
                .iter()
                .any(|event| event.kind == "story_echo" && !event.used && event.pos == target)
        });
        let label = self
            .quest_objective_label
            .as_ref()
            .map(|label| label.get_text().to_string())
            .unwrap_or_default();
        if let Some(state) = self.state.as_mut() {
            for (quest, original) in state.quests.quests.iter_mut().zip(original_states) {
                quest.state = original;
            }
        }
        let mut snapshot = VarDictionary::new();
        snapshot.set("target", target.is_some());
        snapshot.set("points_to_echo", points_to_echo);
        snapshot.set("localized", label.contains("TRACKED"));
        snapshot.set(
            "marker_visible",
            self.quest_objective_marker
                .as_ref()
                .is_some_and(|marker| marker.is_visible()),
        );
        snapshot
    }

    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_prepare_quest_events(&mut self) -> VarDictionary {
        self.autosave_enabled = false;
        if let Some(state) = self.state.as_mut() {
            for quest_id in ["echo_contact", "forgotten_cache", "heart_execution"] {
                if !state.quests.has(quest_id) {
                    state.quests.add(quest_id, quest_id, "runtime smoke");
                } else if let Some(quest) = state
                    .quests
                    .quests
                    .iter_mut()
                    .find(|quest| quest.id == quest_id)
                {
                    quest.state = crate::quest::QuestState::Active;
                }
                state.quest_kills.insert(quest_id.to_string(), 0);
            }
        }

        let stranger = self.npcs.iter().position(|npc| npc.id == "stranger");
        if let Some(index) = stranger {
            self.start_dialogue(index);
            self.scene = None;
            self.line_idx = 0;
            self.at_choices = false;
            if let Some(panel) = self.dlg_panel.as_mut() {
                panel.set_visible(false);
            }
            self.set_mode_explore();
        }

        let mut snapshot = VarDictionary::new();
        snapshot.set("stranger_found", stranger.is_some());
        snapshot.set(
            "interact_progress",
            self.state
                .as_ref()
                .and_then(|state| state.quest_kills.get("echo_contact"))
                .copied()
                .unwrap_or(0) as i64,
        );
        snapshot
    }

    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_pick_weapon(&mut self) -> VarDictionary {
        let weapon_index = self
            .world_items
            .iter()
            .position(|item| item.in_dungeon && matches!(&item.payload, Payload::Weapon(_)));
        if let Some(index) = weapon_index {
            self.pick_up_item_impl(index, false);
        }

        let mut snapshot = VarDictionary::new();
        snapshot.set("weapon_found", weapon_index.is_some());
        snapshot.set(
            "discover_progress",
            self.state
                .as_ref()
                .and_then(|state| state.quest_kills.get("forgotten_cache"))
                .copied()
                .unwrap_or(0) as i64,
        );
        snapshot
    }

    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_probe_boss(&mut self) -> VarDictionary {
        let boss_index = self
            .enemies
            .iter()
            .position(|enemy| enemy.get_position().x > 250.0 && enemy.bind().is_boss);

        let mut boss_id = String::new();
        let mut phase_count = 0;
        let mut middle_phase = 0;
        let mut final_phase = 0;
        let mut tactical_role = String::new();
        let mut telegraph_scale = 0.0;
        let mut phase_events = 0usize;
        let mut phase_animation = false;
        let lights_before = self.light_fx.len();
        if let Some(index) = boss_index {
            let mut boss = self.enemies[index].bind_mut();
            boss_id = boss.cfg_id.to_string();
            tactical_role = boss.tactical_role_id().to_string();
            phase_count = boss.boss_phase_count();
            boss.hp = boss.max_hp * 0.5;
            boss.take_damage(0.0, DmgType::Physical);
            middle_phase = boss.boss_phase_index();
            boss.hp = boss.max_hp * 0.2;
            boss.take_damage(0.0, DmgType::Physical);
            final_phase = boss.boss_phase_index();
            telegraph_scale = boss.strongest_telegraph_scale();
            phase_animation = boss.phase_transition_active();
            phase_events = boss.pending_phase_events();
        }
        self.collect_enemy_requests();
        let phase_fx = self.light_fx.len().saturating_sub(lights_before);

        let mut snapshot = VarDictionary::new();
        snapshot.set("boss_found", boss_index.is_some());
        snapshot.set("boss_id", boss_id);
        snapshot.set("phase_count", phase_count as i64);
        snapshot.set("middle_phase", middle_phase as i64);
        snapshot.set("final_phase", final_phase as i64);
        snapshot.set("tactical_role", tactical_role);
        snapshot.set("telegraph_scale", telegraph_scale as f64);
        snapshot.set("phase_events", phase_events as i64);
        snapshot.set("phase_animation", phase_animation);
        snapshot.set("phase_fx", phase_fx as i64);
        snapshot
    }

    #[cfg(debug_assertions)]
    #[func]
    fn runtime_smoke_exit_dungeon(&mut self) -> bool {
        self.exit_dungeon_impl(false);
        self.loc == Loc::World
            && self.dungeon_root.is_none()
            && self.dungeon_nav.is_none()
            && self.dungeon_hazards.is_empty()
    }
}
