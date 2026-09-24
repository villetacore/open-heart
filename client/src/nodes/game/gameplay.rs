//! Игровой цикл: боёвка, эффекты, диалоги, инвентарь, перки, сохранение.

use super::*;

// ── Игровой процесс ───────────────────────────────────────────────────────────

impl Game3D {
    pub(super) fn process_explore(&mut self) {
        if self.maybe_end_campaign() { return; }
        let lang = self.settings.lang.clone();
        self.update_world_district();
        self.update_nearby();
        self.update_inv_label();
        self.update_quest_label(&lang);
        self.update_quest_navigation(&lang);

        let input = Input::singleton();

        if input.is_action_just_pressed("quest_next") {
            self.tracked_quest_index = self.tracked_quest_index.wrapping_add(1);
            self.update_quest_navigation(&lang);
        }

        // Смерть проверяется ДО обработки ввода: Esc в кадр смерти не должен
        // открыть паузу поверх мёртвого игрока (и дать сохраниться с hp=0).
        let player_dead = self.player()
            .and_then(|p| p.clone().try_cast::<Player>().ok())
            .map(|pl| pl.bind().dead)
            .unwrap_or(false);
        if player_dead {
            self.mode = Mode::Dead;
            if let Some(ref mut dp) = self.dead_panel {
                dp.set_visible(true);
            }
            Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
            // Сейв НЕ стирается: смерть = возврат в хаб со штрафом (см. DESIGN_PLAN §13).
            return;
        }

        if input.is_action_just_pressed("escape") {
            self.open_pause();
            return;
        }

        if input.is_action_just_pressed("interact") {
            if let Some(kind) = self.near_portal {
                self.use_portal(kind);
            } else if let Some(idx) = self.near_dungeon_event {
                self.use_dungeon_event(idx);
            } else if let Some(idx) = self.near_item {
                self.pick_up_item(idx);
            } else if let Some(idx) = self.near_npc {
                self.start_dialogue(idx);
            }
        }

        // смена оружия: клавиши 1-8
        for slot in 0..8usize {
            let act = format!("weapon_{}", slot + 1);
            if input.is_action_just_pressed(&act) {
                let w = WeaponId::from_slot(slot);
                if self.arsenal.has(w) && self.arsenal.current != w {
                    self.arsenal.current = w;
                    self.refresh_weapon_sheet();
                }
            }
        }
        if input.is_action_just_pressed("weapon_next") {
            let w = self.arsenal.cycle(1);
            if w != self.arsenal.current {
                self.arsenal.current = w;
                self.refresh_weapon_sheet();
            }
        }
        if input.is_action_just_pressed("weapon_prev") {
            let w = self.arsenal.cycle(-1);
            if w != self.arsenal.current {
                self.arsenal.current = w;
                self.refresh_weapon_sheet();
            }
        }
        if input.is_action_just_pressed("reload") {
            self.try_reload();
        }

        // стрельба (оглушение блокирует)
        let has_any_weapon = self.arsenal.owned.iter().any(|o| *o);
        if has_any_weapon && !self.player_stunned() {
            let def = weapon_def(self.arsenal.current);
            let want_alt_fire = input.is_action_just_pressed("alt_shoot");
            let want_fire = if def.auto {
                input.is_action_pressed("shoot")
            } else {
                input.is_action_just_pressed("shoot")
            };
            if want_alt_fire && self.shoot_cd <= 0.0 {
                self.try_alt_fire();
            } else if want_fire && self.shoot_cd <= 0.0 {
                self.try_fire();
            }
        }

        // активные умения класса (F / G)
        for (slot, action) in ["ability_primary", "ability_secondary"].iter().enumerate() {
            if input.is_action_just_pressed(*action) { self.try_player_ability(slot); }
        }

        // быстрое лечение
        if input.is_action_just_pressed("use_med") {
            self.use_first_consumable();
        }

        if input.is_action_just_pressed("inventory") {
            self.open_inventory();
        }

        if input.is_action_just_pressed("craft") {
            self.open_craft();
        }

        if input.is_action_just_pressed("journal") {
            self.open_journal();
        }

        if input.is_action_just_pressed("perks") {
            self.open_perks();
        }
    }

    pub(super) fn process_dead(&mut self) {
        let input = Input::singleton();
        if input.is_action_just_pressed("interact") {
            self.respawn_at_hub();
        }
    }

    // ── Пауза ────────────────────────────────────────────────────────────────

    pub(super) fn open_pause(&mut self) {
        self.mode = Mode::Paused;
        self.freeze_player(true);
        if let Some(ref mut p) = self.pause_panel {
            p.set_visible(true);
        }
        if let Some(ref mut lbl) = self.hint_label {
            lbl.set_visible(false);
        }
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
    }

    pub(super) fn close_pause(&mut self) {
        if let Some(ref mut p) = self.pause_panel {
            p.set_visible(false);
        }
        self.set_mode_explore();
    }

    pub(super) fn process_paused(&mut self) {
        let input = Input::singleton();
        if input.is_action_just_pressed("escape") || input.is_action_just_pressed("choice_1") {
            self.close_pause();
            return;
        }
        if input.is_action_just_pressed("choice_2") {
            self.auto_save();
            self.base()
                .get_tree()
                .change_scene_to_file("res://main_menu.tscn");
        }
    }

    /// Смерть → возврат в хаб: −25 % золота, данж сгорает, сейв сохраняется.
    pub(super) fn respawn_at_hub(&mut self) {
        let lost = {
            let st = self.state.as_mut().unwrap();
            let lost = st.gold / 4;
            st.gold -= lost;
            lost
        };
        // возрождение снимает дебаффы (горение/замедление/оглушение)
        self.player_statuses.clear();
        if let Some(p) = self.player() {
            if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                let mut b = pl.bind_mut();
                b.speed_mult = 1.0;
                b.stunned = false;
            }
        }
        if self.loc == Loc::Dungeon {
            self.clear_dungeon();
            self.loc = Loc::World;
        }
        if let Some(p) = self.player() {
            if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                let max = pl.bind().max_hp;
                {
                    let mut b = pl.bind_mut();
                    b.dead = false;
                    b.hp = max;
                }
                pl.bind_mut().teleport(Vector3::new(0.0, 1.1, 10.0));
            }
        }
        self.mode = Mode::Explore;
        if let Some(ref mut dp) = self.dead_panel {
            dp.set_visible(false);
        }
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::CAPTURED);
        self.update_loc_label();
        self.show_flash(&if self.settings.lang == "en" {
            format!("You awaken in the hub. Gold lost: {lost}.")
        } else {
            format!("Ты очнулся в хабе. Потеряно золота: {lost}.")
        });
        self.auto_save();
    }

    pub(super) fn process_dialogue(&mut self) {
        if let Some(mut p) = self.player() {
            let mut vel = p.get_velocity();
            vel.x = 0.0;
            vel.z = 0.0;
            p.set_velocity(vel);
        }
        let input = Input::singleton();
        if !self.at_choices {
            if input.is_action_just_pressed("interact") {
                self.advance_dialogue();
            }
        } else {
            if input.is_action_just_pressed("choice_1") {
                self.select_choice(0);
            }
            if input.is_action_just_pressed("choice_2") {
                self.select_choice(1);
            }
            if input.is_action_just_pressed("choice_3") {
                self.select_choice(2);
            }
            if input.is_action_just_pressed("choice_4") {
                self.select_choice(3);
            }
        }
    }

    pub(super) fn process_inventory(&mut self) {
        if Input::singleton().is_action_just_pressed("choice_1") {
            self.select_weapon_mod(1);
            return;
        }
        if Input::singleton().is_action_just_pressed("choice_2") {
            self.select_weapon_mod(2);
            return;
        }
        if Input::singleton().is_action_just_pressed("inventory")
            || Input::singleton().is_action_just_pressed("escape")
        {
            self.close_inventory();
        }
        if Input::singleton().is_action_just_pressed("interact") {
            self.ui_use_item();
        }
    }

    // ── Поблизости ───────────────────────────────────────────────────────────

    pub(super) fn update_nearby(&mut self) {
        let lang = self.settings.lang.clone();
        let player_pos = match self.player() {
            Some(p) => p.get_global_position(),
            None => {
                self.near_npc = None;
                return;
            }
        };

        // порталы
        self.near_portal = None;
        match self.loc {
            Loc::World => {
                if (player_pos - self.gate_pos).length() < PORTAL_R {
                    self.near_portal = Some(PortalKind::EnterDungeon);
                }
            }
            Loc::Dungeon => {
                if (player_pos - self.exit_portal).length() < PORTAL_R {
                    self.near_portal = Some(PortalKind::ExitDungeon);
                } else if (player_pos - self.next_portal).length() < PORTAL_R {
                    self.near_portal = Some(PortalKind::DeeperDungeon);
                }
            }
        }

        let mut near_npc: Option<usize> = None;
        if self.loc == Loc::World {
            let mut best_n = INTERACT_R;
            for (i, sp) in self.npc_sprites.iter().enumerate() {
                let d = (player_pos - sp.get_global_position()).length();
                if d < best_n {
                    best_n = d;
                    near_npc = Some(i);
                }
            }
        }
        self.near_npc = near_npc;

        let mut near_dungeon_event = None;
        if self.loc == Loc::Dungeon {
            let mut best_event = INTERACT_R;
            for (index, event) in self.dungeon_events.iter().enumerate() {
                if event.used {
                    continue;
                }
                let distance = (player_pos - event.pos).length();
                if distance < best_event {
                    best_event = distance;
                    near_dungeon_event = Some(index);
                }
            }
        }
        self.near_dungeon_event = near_dungeon_event;

        let mut near_item: Option<usize> = None;
        let mut best_i = PICKUP_R;
        for (i, wi) in self.world_items.iter_mut().enumerate() {
            let mut position = wi.node.get_position();
            let base_y = if matches!(wi.payload, Payload::Weapon(_)) {
                0.65
            } else {
                0.55
            };
            position.y = base_y + (self.game_time * 2.4 + i as f32 * 0.73).sin() * 0.10;
            wi.node.set_position(position);
            if !matches!(wi.payload, Payload::Weapon(_)) {
                if let Some(child) = wi.node.get_child(0) {
                    if let Ok(mut sprite) = child.try_cast::<Sprite3D>() {
                        let frame = ((self.game_time * 4.0 + i as f32 * 0.31) as usize) % 2;
                        sprite.set_region_rect(Rect2::new(
                            Vector2::new(frame as f32 * 64.0, 0.0),
                            Vector2::new(64.0, 64.0),
                        ));
                    }
                }
            }
            let d = (player_pos - wi.node.get_global_position()).length();
            if d < best_i {
                best_i = d;
                near_item = Some(i);
            }
        }
        self.near_item = near_item;

        // ближайший враг для таргет-инфо
        let mut near_enemy: Option<usize> = None;
        let mut best_e = 24.0;
        for (i, e) in self.enemies.iter().enumerate() {
            if e.bind().alive {
                let d = (player_pos - e.get_global_position()).length();
                if d < best_e {
                    best_e = d;
                    near_enemy = Some(i);
                }
            }
        }
        self.near_enemy = near_enemy;

        let hint_text = if let Some(kind) = self.near_portal {
            match kind {
                PortalKind::EnterDungeon => {
                    let depth = self
                        .state
                        .as_ref()
                        .map(|s| s.dungeons_cleared + 1)
                        .unwrap_or(1);
                    if lang == "en" {
                        format!("[E] Enter dungeon (depth {depth})")
                    } else {
                        format!("[E] Войти в данж (глубина {depth})")
                    }
                }
                PortalKind::ExitDungeon => {
                    if lang == "en" {
                        "[E] Return to the world".to_string()
                    } else {
                        "[E] Вернуться в мир".to_string()
                    }
                }
                PortalKind::DeeperDungeon => {
                    if self.boss_alive {
                        if lang == "en" {
                            "The portal is sealed — defeat the guardian".to_string()
                        } else {
                            "Портал запечатан — убей стража данжа".to_string()
                        }
                    } else if self.dungeon_depth == 4 && self.state.as_ref().is_some_and(|s| !s.has("campaign_completed")) {
                        if lang == "en" { "[E] Return home — campaign finale".to_string() } else { "[E] Вернуться домой — финал истории".to_string() }
                    } else {
                        if lang == "en" {
                            format!("[E] Descend deeper (depth {})", self.dungeon_depth + 1)
                        } else {
                            format!("[E] Спуститься глубже (глубина {})", self.dungeon_depth + 1)
                        }
                    }
                }
            }
        } else if let Some(index) = self.near_dungeon_event {
            let event = &self.dungeon_events[index];
            if event.kind == "story_echo" {
                if lang == "en" {
                    "[E] Listen to the memory echo".to_string()
                } else {
                    "[E] Услышать эхо памяти".to_string()
                }
            } else if lang == "en" {
                format!("[E] Activate relay {}", event.step + 1)
            } else {
                format!("[E] Активировать реле {}", event.step + 1)
            }
        } else if let Some(idx) = self.near_item {
            format!("{}: {}", t("hud_pickup", &lang), self.world_items[idx].name)
        } else if let Some(idx) = self.near_npc {
            format!(
                "{} {}",
                t("hud_interact", &lang),
                self.npcs.get(idx).map(|n| n.name.as_str()).unwrap_or("?")
            )
        } else {
            String::new()
        };

        if let Some(ref mut lbl) = self.hint_label {
            if hint_text.is_empty() {
                lbl.set_visible(false);
            } else {
                lbl.set_text(&hint_text);
                lbl.set_visible(true);
            }
        }
    }

    pub(super) fn use_portal(&mut self, kind: PortalKind) {
        match kind {
            PortalKind::EnterDungeon => {
                let depth = self
                    .state
                    .as_ref()
                    .map(|s| s.dungeons_cleared + 1)
                    .unwrap_or(1);
                self.enter_dungeon(depth);
            }
            PortalKind::ExitDungeon => self.exit_dungeon(),
            PortalKind::DeeperDungeon => {
                if self.boss_alive {
                    self.show_flash(if self.settings.lang == "en" {
                        "The portal is sealed! Defeat the guardian first."
                    } else {
                        "Портал запечатан! Сначала убей стража."
                    });
                } else {
                    if self.dungeon_depth == 4 && self.state.as_ref().is_some_and(|s| !s.has("campaign_completed")) {
                        self.exit_dungeon();
                        return;
                    }
                    let d = self.dungeon_depth + 1;
                    self.enter_dungeon(d);
                }
            }
        }
    }
}
