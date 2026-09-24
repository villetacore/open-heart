use super::*;
use godot::classes::Button;
use std::sync::atomic::{AtomicBool, Ordering};

static REQUESTED: AtomicBool = AtomicBool::new(false);
pub(crate) fn request() { REQUESTED.store(true, Ordering::SeqCst); }
pub(super) fn take_request() -> bool {
    REQUESTED.swap(false, Ordering::SeqCst)
        || godot::classes::Os::singleton().get_cmdline_user_args().as_slice().iter().any(|s| *s == "--creative")
}

impl Game3D {
    pub(super) fn creative_refill(&mut self) {
        if !self.creative { return; }
        if let Some(state) = self.state.as_mut() {
            state.gold = 999_999;
            state.perk_points = 999;
            state.weapon_mod_cores = 999;
            state.abilities.cooldowns = [0.0; 2];
            if let Some(cfg) = self.cfg.as_ref() {
                for item in &cfg.items {
                    if let Some(owned) = state.inventory.items.iter_mut().find(|owned| owned.id == item.id) {
                        owned.qty = owned.qty.max(99);
                    } else {
                        state.inventory.add(crate::item::Item::new(&item.id, item.name(&self.settings.lang), "", 99));
                    }
                }
            }
        }
        for slot in 0..8 {
            let id = WeaponId::from_slot(slot);
            self.arsenal.give_weapon(id);
            self.arsenal.clips[slot] = weapon_def(id).magazine;
        }
        self.arsenal.ammo = [9999; 4];
        self.player_statuses = crate::status::StatusSet::new();
        if let Some(mut p) = self.player().and_then(|p| p.try_cast::<Player>().ok()) {
            let hp = p.bind().max_hp;
            p.bind_mut().hp = hp;
        }
    }

    pub(super) fn build_creative_panel(&mut self) {
        let en = self.settings.lang == "en";
        let mut layer = CanvasLayer::new_alloc();
        layer.set_layer(7);
        let mut open = Button::new_alloc();
        open.set_text(if en {"CREATIVE [F2] · unlimited supplies · no campaign saves"} else {"КРЕАТИВ [F2] · всё доступно · без записи кампании"});
        open.set_position(Vector2::new(520.0, 75.0));
        open.set_size(Vector2::new(880.0, 42.0));
        open.connect("pressed", &self.to_gd().callable("ui_creative_action").bind(&[(-2i64).to_variant()]));
        layer.add_child(&open);
        let mut panel = Panel::new_alloc();
        place(&panel, 0.5, 0.5, 460.0, 160.0, 1000.0, 740.0);
        panel.add_theme_stylebox_override("panel", &make_style(C_UI_BG, C_CYAN, 2));
        let mut title = Label::new_alloc();
        title.set_text(if en {"CREATIVE LAB · weapons 1–8 · abilities F/G"} else {"ТВОРЧЕСКАЯ ЛАБОРАТОРИЯ · оружие 1–8 · умения F/G"});
        title.set_position(Vector2::new(28.0, 24.0));
        title.add_theme_font_size_override("font_size", 25);
        panel.add_child(&title);
        let mut actions: Vec<(i64, String)> = Vec::new();
        for ci in 0..3 { for si in 0..3 {
            actions.push((10 + ci * 3 + si, format!("{} / {}", classes()[ci as usize].name(&self.settings.lang), classes()[ci as usize].specs[si as usize].name(&self.settings.lang))));
        }}
        for depth in 1..=8 { actions.push((100 + depth, if en {format!("Dungeon · depth {depth}")} else {format!("Подземелье · глубина {depth}")})); }
        actions.push((0, if en {"Return to city"} else {"Вернуться в город"}.into()));
        actions.push((1, if en {"Unlock maximum perk ranks"} else {"Открыть все ранги перков"}.into()));
        actions.push((2, if en {"Wardrobe · all accessories"} else {"Гардероб · все аксессуары"}.into()));
        actions.push((-1, if en {"Return to game [Esc]"} else {"Вернуться в игру [Esc]"}.into()));
        for (index, (action, label)) in actions.into_iter().enumerate() {
            let mut button = Button::new_alloc();
            button.set_text(&label);
            button.set_position(Vector2::new(28.0 + (index % 3) as f32 * 316.0, 88.0 + (index / 3) as f32 * 82.0));
            button.set_size(Vector2::new(306.0, 68.0));
            button.add_theme_font_size_override("font_size", 17);
            button.connect("pressed", &self.to_gd().callable("ui_creative_action").bind(&[action.to_variant()]));
            panel.add_child(&button);
        }
        panel.set_visible(false);
        layer.add_child(&panel);
        self.base_mut().add_child(&layer);
        self.creative_panel = Some(panel);
    }

    pub(super) fn creative_action(&mut self, action: i64) {
        if !self.creative { return; }
        if action == -2 {
            if self.mode != Mode::Explore { return; }
            self.mode = Mode::Creative;
            self.freeze_player(true);
            Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
            if let Some(p) = self.creative_panel.as_mut() { p.set_visible(true); }
            return;
        }
        if self.mode != Mode::Creative { return; }
        if let Some(p) = self.creative_panel.as_mut() { p.set_visible(false); }
        self.set_mode_explore();
        match action {
            0 => { if self.loc == Loc::Dungeon { self.exit_dungeon(); } },
            1 => {
                if let Some(state) = self.state.as_mut() { for perk in crate::perk::perks() { state.perks.insert(perk.id.clone(), perk.max_ranks); } }
                let (ci, si) = self.state.as_ref().map(|s| (s.class_idx.unwrap_or(0), s.spec_idx)).unwrap();
                self.apply_loadout(ci, si, false);
            },
            2 => self.open_wardrobe(),
            10..=18 => self.confirm_class(((action - 10) / 3) as usize, ((action - 10) % 3) as usize),
            101..=108 => self.enter_dungeon((action - 100) as u32),
            _ => {},
        }
        self.creative_refill();
    }
}
