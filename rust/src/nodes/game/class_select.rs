//! Экран выбора класса и специализации.

use super::*;

// ── Выбор класса ──────────────────────────────────────────────────────────────

impl Game3D {
    pub(super) fn build_select_ui(&mut self) {
        let mut layer = CanvasLayer::new_alloc();
        layer.set_layer(5);
        self.base_mut().add_child(&layer);

        let mut panel = Panel::new_alloc();
        panel.set_position(Vector2::ZERO);
        panel.set_size(Vector2::new(HUD_W, HUD_H));
        panel.add_theme_stylebox_override("panel",
            &make_style(Color::from_rgba(0.02, 0.01, 0.04, 0.97), C_BORDER, 0));
        panel.set_visible(false);

        let mut title = Label::new_alloc();
        title.set_text("ВЫБЕРИ КЛАСС");
        title.set_position(Vector2::new(0.0, 90.0));
        title.set_size(Vector2::new(HUD_W, 70.0));
        title.set_horizontal_alignment(HorizontalAlignment::CENTER);
        title.add_theme_font_size_override("font_size", 52);
        title.add_theme_color_override("font_color", C_PINK);
        panel.add_child(&title);
        self.select_title = Some(title);

        let card_w = 480.0;
        let card_h = 560.0;
        let gap = 60.0;
        let total = card_w * 3.0 + gap * 2.0;
        let x0 = (HUD_W - total) * 0.5;
        let y0 = 240.0;

        for i in 0..3 {
            let mut card = Panel::new_alloc();
            card.set_position(Vector2::new(x0 + i as f32 * (card_w + gap), y0));
            card.set_size(Vector2::new(card_w, card_h));
            card.add_theme_stylebox_override("panel", &make_style(C_UI_BG, C_BORDER, 2));

            let mut key = Label::new_alloc();
            key.set_text(&format!("[ {} ]", i + 1));
            key.set_position(Vector2::new(0.0, 22.0));
            key.set_size(Vector2::new(card_w, 40.0));
            key.set_horizontal_alignment(HorizontalAlignment::CENTER);
            key.add_theme_font_size_override("font_size", 30);
            key.add_theme_color_override("font_color", C_GOLD);
            card.add_child(&key);

            let mut ct = Label::new_alloc();
            ct.set_position(Vector2::new(0.0, 80.0));
            ct.set_size(Vector2::new(card_w, 46.0));
            ct.set_horizontal_alignment(HorizontalAlignment::CENTER);
            ct.add_theme_font_size_override("font_size", 34);
            ct.add_theme_color_override("font_color", C_PINK);
            card.add_child(&ct);
            self.card_titles.push(ct);

            let mut cb = Label::new_alloc();
            cb.set_position(Vector2::new(28.0, 150.0));
            cb.set_size(Vector2::new(card_w - 56.0, card_h - 180.0));
            cb.add_theme_font_size_override("font_size", 19);
            cb.add_theme_color_override("font_color", C_MAIN);
            cb.set_autowrap_mode(godot::classes::text_server::AutowrapMode::WORD);
            card.add_child(&cb);
            self.card_bodies.push(cb);

            panel.add_child(&card);
        }

        let mut hint = Label::new_alloc();
        hint.set_text("Нажми 1, 2 или 3");
        hint.set_position(Vector2::new(0.0, HUD_H - 120.0));
        hint.set_size(Vector2::new(HUD_W, 40.0));
        hint.set_horizontal_alignment(HorizontalAlignment::CENTER);
        hint.add_theme_font_size_override("font_size", 22);
        hint.add_theme_color_override("font_color", C_DIM);
        panel.add_child(&hint);

        layer.add_child(&panel);
        self.select_panel = Some(panel);
    }

    pub(super) fn open_class_select(&mut self) {
        self.mode = Mode::ClassSelect;
        self.freeze_player(true);
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
        if let Some(ref mut t) = self.select_title { t.set_text("ВЫБЕРИ КЛАСС"); }
        for (i, c) in classes().iter().enumerate() {
            self.fill_class_card(i, c);
        }
        if let Some(ref mut p) = self.select_panel { p.set_visible(true); }
    }

    pub(super) fn fill_class_card(&mut self, i: usize, c: &ClassDef) {
        if let Some(t) = self.card_titles.get_mut(i) {
            t.set_text(&c.name_ru);
        }
        if let Some(b) = self.card_bodies.get_mut(i) {
            let weapons: Vec<&str> = c.start_weapons.iter()
                .map(|w| weapon_def(*w).name_ru.as_str()).collect();
            b.set_text(&format!(
                "Роль: {}\n\n{}\n\nHP: {:.0}\nСкорость: {:.1}\nОружие: {}\n\nСпеки:\n• {}\n• {}\n• {}",
                c.role_ru, c.desc_ru, c.base_hp, c.speed, weapons.join(", "),
                c.specs[0].name_ru, c.specs[1].name_ru, c.specs[2].name_ru,
            ));
        }
    }

    pub(super) fn open_spec_select(&mut self, class_idx: usize) {
        self.mode = Mode::SpecSelect;
        self.class_pick = class_idx;
        let c = &classes()[class_idx];
        if let Some(ref mut t) = self.select_title {
            t.set_text(&format!("{} — ВЫБЕРИ СПЕЦИАЛИЗАЦИЮ", c.name_ru));
        }
        for i in 0..3 {
            let s = &c.specs[i];
            if let Some(t) = self.card_titles.get_mut(i) {
                t.set_text(&s.name_ru);
            }
            if let Some(b) = self.card_bodies.get_mut(i) {
                b.set_text(&format!("{}\n\n(Esc — назад к классам)", s.desc_ru));
            }
        }
    }

    pub(super) fn process_class_select(&mut self) {
        let input = Input::singleton();
        for i in 0..3usize {
            let act = ["choice_1", "choice_2", "choice_3"][i];
            if input.is_action_just_pressed(act) {
                self.open_spec_select(i);
                return;
            }
        }
    }

    pub(super) fn process_spec_select(&mut self) {
        let input = Input::singleton();
        if input.is_action_just_pressed("escape") {
            self.open_class_select();
            return;
        }
        for i in 0..3usize {
            let act = ["choice_1", "choice_2", "choice_3"][i];
            if input.is_action_just_pressed(act) {
                self.confirm_class(self.class_pick, i);
                return;
            }
        }
    }

    pub(super) fn confirm_class(&mut self, class_idx: usize, spec_idx: usize) {
        {
            let st = self.state.as_mut().unwrap();
            st.class_idx = Some(class_idx);
            st.spec_idx = spec_idx;
            st.perk_points += 1;   // стартовое очко перка
        }
        self.apply_loadout(class_idx, spec_idx, true);
        if let Some(ref mut p) = self.select_panel { p.set_visible(false); }
        self.set_mode_explore();
        self.refresh_weapon_sheet();
        let c = &classes()[class_idx];
        self.show_flash(&format!("{} / {}. Вперёд!", c.name_ru, c.specs[spec_idx].name_ru));
        self.update_loc_label();
        self.auto_save();
    }

    /// Пересчитать статы из класса/спека/уровня. give_kit — выдать стартовый набор.
    pub(super) fn apply_loadout(&mut self, class_idx: usize, spec_idx: usize, give_kit: bool) {
        let level = self.state.as_ref().map(|s| s.level).unwrap_or(1);
        self.loadout = compute_loadout(class_idx, spec_idx, level);
        let hearts = self.state.as_ref().map(|s| s.stat_hearts()).unwrap_or(0);
        self.loadout.max_hp += hearts as f32 * 15.0;

        // модификаторы перков и активных синергий
        let owned_perks = self.state.as_ref().map(|s| s.perks.clone()).unwrap_or_default();
        let mods = crate::perk::mods_for(&owned_perks);
        self.loadout.max_hp    += mods.max_hp_add;
        self.loadout.speed     *= mods.speed_mult;
        self.loadout.dmg_mult  += mods.dmg_add;
        self.loadout.cd_mult   *= mods.cd_mult;
        self.loadout.lifesteal += mods.lifesteal_add;
        self.loadout.ammo_mult += mods.ammo_add;
        self.loadout.max_hp = self.loadout.max_hp.max(40.0);

        let c = &classes()[class_idx];
        let s = &c.specs[spec_idx];

        if give_kit {
            self.arsenal = Arsenal::new();
            for w in &c.start_weapons {
                self.arsenal.give_weapon(*w);
            }
            if let Some(w) = s.extra_weapon {
                self.arsenal.give_weapon(w);
                if let Some((t, _)) = weapon_def(w).ammo {
                    self.arsenal.add_ammo(t, t.pack_size() * 2, self.loadout.ammo_mult);
                }
            }
            for (t, n) in &c.start_ammo {
                self.arsenal.add_ammo(*t, *n, self.loadout.ammo_mult);
            }
            self.arsenal.current = c.start_weapons[0];
        }

        let (max_hp, speed) = (self.loadout.max_hp, self.loadout.speed);
        if let Some(ref p) = self.player {
            if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                let mut b = pl.bind_mut();
                b.max_hp = max_hp;
                if give_kit { b.hp = max_hp; }
                else { b.hp = b.hp.min(max_hp); }
                b.speed = speed;
            }
        }
    }

    pub(super) fn freeze_player(&mut self, frozen: bool) {
        if let Some(ref p) = self.player {
            if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                pl.bind_mut().frozen = frozen;
            }
        }
    }

    pub(super) fn set_mode_explore(&mut self) {
        self.mode = Mode::Explore;
        self.freeze_player(false);
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::CAPTURED);
    }
}
