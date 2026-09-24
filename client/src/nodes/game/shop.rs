//! Магазин Торговца: покупка расходников, патронов и ядер модификации за золото.
//!
//! Открывается при взаимодействии с NPC `merchant` (см. `conversation.rs`).
//! Панель пересобирается при каждой покупке — так цены и баланс всегда свежие.
use super::*;
use godot::classes::{Button, GridContainer};
use crate::config::{AMMO_PACK_PRICE, WEAPON_CORE_PRICE};

impl Game3D {
    pub(super) fn open_shop(&mut self) {
        self.mode = Mode::Shop;
        self.freeze_player(true);
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
        self.rebuild_shop();
    }

    pub(super) fn close_shop(&mut self) {
        if let Some(mut layer) = self.shop_layer.take() {
            layer.queue_free();
        }
        self.set_mode_explore();
    }

    pub(super) fn process_shop(&mut self) {
        let input = Input::singleton();
        if input.is_action_just_pressed("escape") || input.is_action_just_pressed("interact") {
            self.close_shop();
        }
    }

    /// Собрать предложения магазина: (id, подпись, цена).
    fn shop_offers(&self) -> Vec<(String, String, i32)> {
        let lang = &self.settings.lang;
        let en = lang == "en";
        let mut offers = Vec::new();
        if let Some(cfg) = self.cfg.as_ref() {
            for item in cfg.items.iter().filter(|i| i.is_for_sale()) {
                offers.push((item.id.clone(), item.name(lang).to_string(), item.shop_price()));
            }
        }
        for t in [AmmoType::Bullets, AmmoType::Shells, AmmoType::Cells, AmmoType::Rockets] {
            offers.push((
                format!("ammo_{}", t.idx()),
                format!("{} ×{}", t.name(lang), t.pack_size()),
                AMMO_PACK_PRICE,
            ));
        }
        offers.push((
            "core".into(),
            if en { "Weapon mod core".into() } else { "Ядро модификации".into() },
            WEAPON_CORE_PRICE,
        ));
        offers
    }

    /// Что игрок может продать: расходники из инвентаря. (`sell_<id>`, подпись, цена).
    fn sell_offers(&self) -> Vec<(String, String, i32)> {
        let lang = &self.settings.lang;
        let (Some(cfg), Some(state)) = (self.cfg.as_ref(), self.state.as_ref()) else {
            return Vec::new();
        };
        cfg.items
            .iter()
            .filter(|i| i.is_for_sale())
            .filter_map(|item| {
                let qty = state.inventory.count(&item.id);
                if qty == 0 {
                    return None;
                }
                Some((
                    format!("sell_{}", item.id),
                    format!(
                        "{} {} ×{}",
                        if lang == "en" { "Sell" } else { "Продать" },
                        item.name(lang),
                        qty
                    ),
                    item.sell_price(),
                ))
            })
            .collect()
    }

    fn rebuild_shop(&mut self) {
        if let Some(mut layer) = self.shop_layer.take() {
            layer.queue_free();
        }
        let en = self.settings.lang == "en";
        let gold = self.state.as_ref().map(|s| s.gold).unwrap_or(0);
        let offers = self.shop_offers();

        let mut layer = CanvasLayer::new_alloc();
        layer.set_layer(6);
        let mut panel = Panel::new_alloc();
        place(&panel, 0.5, 0.5, 400.0, 150.0, 1120.0, 780.0);
        panel.add_theme_stylebox_override("panel", &make_style(C_UI_BG, C_GOLD, 2));

        let mut title = Label::new_alloc();
        title.set_text(&format!(
            "{}  ·  {} {}",
            if en { "MERCHANT" } else { "ТОРГОВЕЦ" },
            gold,
            if en { "gold" } else { "золота" }
        ));
        title.set_position(Vector2::new(28.0, 20.0));
        title.add_theme_font_size_override("font_size", 28);
        title.add_theme_color_override("font_color", C_GOLD);
        panel.add_child(&title);

        let mut scroll = ScrollContainer::new_alloc();
        scroll.set_position(Vector2::new(28.0, 78.0));
        scroll.set_size(Vector2::new(1064.0, 616.0));
        let mut grid = GridContainer::new_alloc();
        grid.set_columns(3);
        grid.add_theme_constant_override("h_separation", 12);
        grid.add_theme_constant_override("v_separation", 12);
        for (id, label_text, price) in &offers {
            let mut b = Button::new_alloc();
            b.set_custom_minimum_size(Vector2::new(338.0, 70.0));
            b.set_text(&format!("{}\n{} {}", label_text, price, if en { "gold" } else { "зол." }));
            b.add_theme_font_size_override("font_size", 20);
            for (state, color) in [("normal", C_BORDER), ("hover", C_CYAN), ("pressed", C_PINK), ("disabled", C_DIM)] {
                b.add_theme_stylebox_override(state, &make_style(C_UI_BG, color, 1));
            }
            b.set_disabled(gold < *price);
            b.connect("pressed", &self.to_gd().callable("ui_shop_buy").bind(&[id.to_variant()]));
            grid.add_child(&b);
        }
        // Продажа расходников: кнопки с бирюзовым контуром, чтобы отличать от покупки.
        for (id, label_text, price) in self.sell_offers() {
            let mut b = Button::new_alloc();
            b.set_custom_minimum_size(Vector2::new(338.0, 70.0));
            b.set_text(&format!("{}\n+{} {}", label_text, price, if en { "gold" } else { "зол." }));
            b.add_theme_font_size_override("font_size", 20);
            for (state, color) in [("normal", C_CYAN), ("hover", C_GOLD), ("pressed", C_PINK), ("disabled", C_DIM)] {
                b.add_theme_stylebox_override(state, &make_style(C_UI_BG, color, 1));
            }
            b.connect("pressed", &self.to_gd().callable("ui_shop_buy").bind(&[id.to_variant()]));
            grid.add_child(&b);
        }
        scroll.add_child(&grid);
        panel.add_child(&scroll);

        let mut close = Button::new_alloc();
        close.set_text(if en { "CLOSE  [Esc]" } else { "ЗАКРЫТЬ  [Esc]" });
        close.set_position(Vector2::new(902.0, 712.0));
        close.set_custom_minimum_size(Vector2::new(190.0, 50.0));
        close.add_theme_stylebox_override("normal", &make_style(C_UI_BG, C_BORDER, 1));
        close.connect("pressed", &self.to_gd().callable("ui_close_shop"));
        panel.add_child(&close);

        layer.add_child(&panel);
        self.base_mut().add_child(&layer);
        self.shop_layer = Some(layer);
    }

    pub(super) fn ui_shop_buy_impl(&mut self, id: GString) {
        let id = id.to_string();
        let en = self.settings.lang == "en";
        // Продажа: убрать один предмет из инвентаря, начислить золото.
        if let Some(item_id) = id.strip_prefix("sell_") {
            let price = self
                .cfg
                .as_ref()
                .and_then(|c| c.items.iter().find(|i| i.id == item_id))
                .map(|i| i.sell_price());
            let sold_name = self
                .cfg
                .as_ref()
                .and_then(|c| c.items.iter().find(|i| i.id == item_id))
                .map(|i| i.name(&self.settings.lang).to_string());
            if let (Some(price), Some(name)) = (price, sold_name) {
                let removed = self
                    .state
                    .as_mut()
                    .map(|s| s.inventory.remove(item_id, 1))
                    .unwrap_or(0);
                if removed > 0 {
                    if let Some(s) = self.state.as_mut() {
                        s.gold += price;
                    }
                    self.show_flash(&if en {
                        format!("Sold: {name} (+{price})")
                    } else {
                        format!("Продано: {name} (+{price})")
                    });
                    self.rebuild_shop();
                    self.refresh_inventory_ui();
                }
            }
            return;
        }
        let bought: Option<String> = if id == "core" {
            if self.state.as_mut().is_some_and(|s| s.spend_gold(WEAPON_CORE_PRICE)) {
                if let Some(s) = self.state.as_mut() { s.weapon_mod_cores += 1; }
                Some(if en { "Weapon mod core".into() } else { "Ядро модификации".into() })
            } else {
                None
            }
        } else if let Some(idx) = id.strip_prefix("ammo_").and_then(|n| n.parse::<usize>().ok()) {
            let t = AmmoType::from_idx(idx);
            if self.state.as_mut().is_some_and(|s| s.spend_gold(AMMO_PACK_PRICE)) {
                self.arsenal.add_ammo(t, t.pack_size(), 1.0);
                Some(t.name(&self.settings.lang).to_string())
            } else {
                None
            }
        } else {
            let item = self
                .cfg
                .as_ref()
                .and_then(|c| c.items.iter().find(|i| i.id == id && i.is_for_sale()))
                .cloned();
            match item {
                Some(item) => {
                    let price = item.shop_price();
                    if self.state.as_mut().is_some_and(|s| s.spend_gold(price)) {
                        let name = item.name(&self.settings.lang).to_string();
                        if let Some(s) = self.state.as_mut() {
                            s.inventory.add(crate::item::Item::new(&item.id, &name, "", 1));
                        }
                        Some(name)
                    } else {
                        None
                    }
                }
                None => None,
            }
        };
        match bought {
            Some(name) => {
                self.show_flash(&if en { format!("Bought: {name}") } else { format!("Куплено: {name}") });
                self.rebuild_shop();
                self.refresh_inventory_ui();
            }
            None => self.show_flash(if en { "Not enough gold" } else { "Недостаточно золота" }),
        }
    }
}
