//! Игровой цикл: боёвка, эффекты, диалоги, инвентарь, перки, сохранение.

use super::*;

// ── Игровой процесс ───────────────────────────────────────────────────────────

impl Game3D {
    pub(super) fn process_explore(&mut self) {
        let lang = self.settings.lang.clone();
        self.update_nearby();
        self.update_inv_label();
        self.update_quest_label(&lang);

        let input = Input::singleton();

        // Смерть проверяется ДО обработки ввода: Esc в кадр смерти не должен
        // открыть паузу поверх мёртвого игрока (и дать сохраниться с hp=0).
        let player_dead = self.player.as_ref()
            .and_then(|p| p.clone().try_cast::<Player>().ok())
            .map(|pl| pl.bind().dead)
            .unwrap_or(false);
        if player_dead {
            self.mode = Mode::Dead;
            if let Some(ref mut dp) = self.dead_panel { dp.set_visible(true); }
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

        // стрельба (оглушение блокирует)
        let has_any_weapon = self.arsenal.owned.iter().any(|o| *o);
        if has_any_weapon && !self.player_stunned() {
            let def = weapon_def(self.arsenal.current);
            let want_fire = if def.auto {
                input.is_action_pressed("shoot")
            } else {
                input.is_action_just_pressed("shoot")
            };
            if want_fire && self.shoot_cd <= 0.0 {
                self.try_fire();
            }
        }

        // быстрое лечение
        if input.is_action_just_pressed("use_med") {
            self.use_first_consumable();
        }

        if input.is_action_just_pressed("inventory") {
            self.open_inventory();
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
        if let Some(ref mut p) = self.pause_panel { p.set_visible(true); }
        if let Some(ref mut lbl) = self.hint_label { lbl.set_visible(false); }
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
    }

    pub(super) fn close_pause(&mut self) {
        if let Some(ref mut p) = self.pause_panel { p.set_visible(false); }
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
            self.base().get_tree().change_scene_to_file("res://main_menu.tscn");
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
        if let Some(ref p) = self.player {
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
        if let Some(ref p) = self.player {
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
        if let Some(ref mut dp) = self.dead_panel { dp.set_visible(false); }
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::CAPTURED);
        self.update_loc_label();
        self.show_flash(&format!("Ты очнулся в хабе. Потеряно золота: {}.", lost));
        self.auto_save();
    }

    pub(super) fn process_dialogue(&mut self) {
        if let Some(ref mut p) = self.player {
            let mut vel = p.get_velocity();
            vel.x = 0.0;
            vel.z = 0.0;
            p.set_velocity(vel);
        }
        let input = Input::singleton();
        if !self.at_choices {
            if input.is_action_just_pressed("interact") { self.advance_dialogue(); }
        } else {
            if input.is_action_just_pressed("choice_1") { self.select_choice(0); }
            if input.is_action_just_pressed("choice_2") { self.select_choice(1); }
            if input.is_action_just_pressed("choice_3") { self.select_choice(2); }
            if input.is_action_just_pressed("choice_4") { self.select_choice(3); }
        }
    }

    pub(super) fn process_inventory(&mut self) {
        if Input::singleton().is_action_just_pressed("inventory")
            || Input::singleton().is_action_just_pressed("escape") {
            self.close_inventory();
        }
        if Input::singleton().is_action_just_pressed("interact") {
            self.use_first_consumable();
            self.refresh_inventory_ui();
        }
    }

    // ── Поблизости ───────────────────────────────────────────────────────────

    pub(super) fn update_nearby(&mut self) {
        let lang = self.settings.lang.clone();
        let player_pos = match self.player.as_ref() {
            Some(p) => p.get_global_position(),
            None => { self.near_npc = None; return; }
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
                if d < best_n { best_n = d; near_npc = Some(i); }
            }
        }
        self.near_npc = near_npc;

        let mut near_item: Option<usize> = None;
        let mut best_i = PICKUP_R;
        for (i, wi) in self.world_items.iter().enumerate() {
            let d = (player_pos - wi.node.get_global_position()).length();
            if d < best_i { best_i = d; near_item = Some(i); }
        }
        self.near_item = near_item;

        // ближайший враг для таргет-инфо
        let mut near_enemy: Option<usize> = None;
        let mut best_e = 24.0;
        for (i, e) in self.enemies.iter().enumerate() {
            if e.bind().alive {
                let d = (player_pos - e.get_global_position()).length();
                if d < best_e { best_e = d; near_enemy = Some(i); }
            }
        }
        self.near_enemy = near_enemy;

        let hint_text = if let Some(kind) = self.near_portal {
            match kind {
                PortalKind::EnterDungeon => {
                    let depth = self.state.as_ref().map(|s| s.dungeons_cleared + 1).unwrap_or(1);
                    format!("[E] Войти в данж (глубина {})", depth)
                }
                PortalKind::ExitDungeon => "[E] Вернуться в мир".to_string(),
                PortalKind::DeeperDungeon => {
                    if self.boss_alive {
                        "Портал запечатан — убей стража данжа".to_string()
                    } else {
                        format!("[E] Спуститься глубже (глубина {})", self.dungeon_depth + 1)
                    }
                }
            }
        } else if let Some(idx) = self.near_item {
            format!("{}: {}", t("hud_pickup", &lang), self.world_items[idx].name)
        } else if let Some(idx) = self.near_npc {
            format!("{} {}", t("hud_interact", &lang),
                    self.npcs.get(idx).map(|n| n.name.as_str()).unwrap_or("?"))
        } else {
            String::new()
        };

        if let Some(ref mut lbl) = self.hint_label {
            if hint_text.is_empty() { lbl.set_visible(false); }
            else { lbl.set_text(&hint_text); lbl.set_visible(true); }
        }
    }

    pub(super) fn use_portal(&mut self, kind: PortalKind) {
        match kind {
            PortalKind::EnterDungeon => {
                let depth = self.state.as_ref().map(|s| s.dungeons_cleared + 1).unwrap_or(1);
                self.enter_dungeon(depth);
            }
            PortalKind::ExitDungeon => self.exit_dungeon(),
            PortalKind::DeeperDungeon => {
                if self.boss_alive {
                    self.show_flash("Портал запечатан! Сначала убей стража.");
                } else {
                    let d = self.dungeon_depth + 1;
                    self.enter_dungeon(d);
                }
            }
        }
    }

}
