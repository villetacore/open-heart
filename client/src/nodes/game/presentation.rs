use super::*;
use godot::classes::{Button, PackedScene, ResourceLoader};

const STYLES: [(&str, &str, &str); 36] = [
    ("heart_choker", "Чокер «Сердце»", "Heart choker"),
    ("rose_stripe_stockings", "Чулки с розовыми полосами", "Rose stripe stockings"),
    ("pleated_battle_skirt", "Боевая юбка", "Battle skirt"),
    ("lace_combat_boots", "Боевые сапоги", "Combat boots"),
    ("rose_hair_bow", "Шёлковый бант", "Silk bow"),
    ("lace_gloves", "Кружевные митенки", "Lace gloves"),
    ("brocade_waistcoat", "Парчовый жилет", "Brocade waistcoat"),
    ("amethyst_ear_chain", "Аметистовая серьга", "Amethyst ear chain"),
    ("heart_satchel", "Сумка-сердце", "Heart satchel"),
    ("rose_eyeliner", "Карандаш для глаз", "Rose eyeliner"),
    ("violet_perfume", "Флакон «Фиалка»", "Violet perfume"),
    ("silver_rose_brooch", "Серебряная роза", "Silver rose brooch"),
    ("moon_brooch", "Лунная брошь", "Moon brooch"),
    ("jasmine_pin", "Заколка «Жасмин»", "Jasmine hairpin"),
    ("silk_cravat", "Шёлковый галстук", "Silk cravat"),
    ("lace_fan", "Кружевной веер", "Lace fan"),
    ("heart_ring", "Кольцо-сердце", "Heart ring"),
    ("voltage_bow", "Бант «Напряжение»", "Voltage bow"),
    ("opera_mask", "Маска «Ноктюрн»", "Nocturne mask"),
    ("rose_beret", "Берет с розой", "Rose beret"),
    ("heart_watch", "Часы на цепочке", "Heart watch"),
    ("moon_collar", "Лунный воротник", "Moon collar"),
    ("pearl_cuff", "Жемчужный кафф", "Pearl ear cuff"),
    ("rose_glove", "Перчатка с вышивкой", "Embroidered glove"),
    ("led_choker", "LED-чокер", "LED choker"),
    ("platform_boots", "Ботинки на платформе", "Platform boots"),
    ("cyber_bow", "Голографический бант", "Holographic bow"),
    ("trans_pin", "Транс-прайд значок", "Trans pride pin"),
    ("punk_cuff", "Панк-браслет", "Punk cuff"),
    ("ar_visor", "AR-визор", "AR visor"),
    ("mesh_sleeves", "Рукава из сетки", "Mesh sleeves"),
    ("tech_skirt", "Техно-юбка", "Tech skirt"),
    ("heart_headphones", "Наушники-сердца", "Heart headphones"),
    ("neon_liner", "Неоновая подводка", "Neon eyeliner"),
    ("cyber_nails", "Хромированные ногти", "Chrome nails"),
    ("punk_bag", "Сумка с патчами", "Patchwork punk bag"),
];

pub(super) fn art_texture(path: &str) -> Option<Gd<Texture2D>> {
    ResourceLoader::singleton().load(path)?.try_cast::<Texture2D>().ok()
}

impl Game3D {
    pub(super) fn build_presentation(&mut self) {
        if self.preset == "core" {
            let mut decor = Node3D::new_alloc();
            decor.set_name("AtelierDecor");
            for (id, pos) in [
                ("atelier_sign", Vector3::new(-5.0, 1.8, -4.0)),
                ("heart_bow_sign", Vector3::new(-9.0, 1.8, -4.0)),
                ("tea_house_sign", Vector3::new(9.0, 1.8, -4.0)),
                ("dressing_mirror", Vector3::new(-7.0, 0.0, -3.0)),
                ("heart_city_banner", Vector3::new(13.0, 1.0, -9.0)),
                ("rose_stained_glass", Vector3::new(-13.0, 1.0, -9.0)),
            ] {
                let path = format!("res://assets/decor/femboy_quarter/{id}.tscn");
                if let Some(scene) = ResourceLoader::singleton().load(&path).and_then(|r| r.try_cast::<PackedScene>().ok()) {
                    if let Some(node) = scene.instantiate().and_then(|r| r.try_cast::<Node3D>().ok()) {
                        let mut node = node;
                        node.set_position(pos);
                        decor.add_child(&node);
                    }
                }
            }
            self.base_mut().add_child(&decor);
            self.build_quarter_props();
        }
        if let Some(mut panel) = self.dlg_panel.clone() {
            place(&panel, 0.5, 0.5, 160.0, 180.0, 1600.0, 760.0);
            if let Some(t) = self.dlg_speaker.as_mut() { t.set_position(Vector2::new(36.0, 30.0)); t.set_size(Vector2::new(1450.0, 44.0)); t.add_theme_font_size_override("font_size", 30); }
            if let Some(t) = self.dlg_text.as_mut() {
                // Authored text gets its own scroll area so long localized
                // lines cannot overlap the illustration below it.
                panel.remove_child(&*t);
                t.set_position(Vector2::ZERO);
                t.set_size(Vector2::new(680.0, 0.0));
                t.set_custom_minimum_size(Vector2::new(680.0, 0.0));
                t.add_theme_font_size_override("font_size", 25);
                let mut text_scroll = ScrollContainer::new_alloc();
                text_scroll.set_position(Vector2::new(36.0, 100.0));
                text_scroll.set_size(Vector2::new(710.0, 230.0));
                text_scroll.add_child(&*t);
                panel.add_child(&text_scroll);
            }
            if let Some(box_) = self.choice_box.as_mut() { box_.set_visible(false); }
            let mut scroll = ScrollContainer::new_alloc();
            scroll.set_position(Vector2::new(790.0, 100.0));
            scroll.set_size(Vector2::new(770.0, 620.0));
            let mut actions = VBoxContainer::new_alloc();
            actions.set_custom_minimum_size(Vector2::new(740.0, 0.0));
            actions.add_theme_constant_override("separation", 12);
            scroll.add_child(&actions);
            panel.add_child(&scroll);
            self.dialogue_actions = Some(actions);
            let mut portrait = TextureRect::new_alloc();
            portrait.set_expand_mode(godot::classes::texture_rect::ExpandMode::IGNORE_SIZE);
            portrait.set_stretch_mode(godot::classes::texture_rect::StretchMode::KEEP_ASPECT_CENTERED);
            portrait.set_position(Vector2::new(36.0, 340.0));
            portrait.set_size(Vector2::new(700.0, 390.0));
            portrait.set_visible(false);
            panel.add_child(&portrait);
            self.dialogue_portrait = Some(portrait);
        }
        let mut layer = CanvasLayer::new_alloc();
        layer.set_layer(2);
        let mut icon = TextureRect::new_alloc();
        icon.set_expand_mode(godot::classes::texture_rect::ExpandMode::IGNORE_SIZE);
        icon.set_stretch_mode(godot::classes::texture_rect::StretchMode::KEEP_ASPECT_CENTERED);
        place(&icon, 0.0, 1.0, 320.0, 982.0, 64.0, 64.0);
        icon.set_visible(false);
        layer.add_child(&icon);
        self.base_mut().add_child(&layer);
        self.fashion_icon = Some(icon);
        let mut layer = CanvasLayer::new_alloc();
        layer.set_layer(2);
        let mut icon = TextureRect::new_alloc();
        icon.set_expand_mode(godot::classes::texture_rect::ExpandMode::IGNORE_SIZE);
        icon.set_stretch_mode(godot::classes::texture_rect::StretchMode::KEEP_ASPECT_CENTERED);
        place(&icon, 1.0, 1.0, 1480.0, 968.0, 90.0, 90.0);
        layer.add_child(&icon);
        self.base_mut().add_child(&layer);
        self.weapon_icon = Some(icon);
    }

    pub(super) fn refresh_fashion(&mut self) {
        if let Some(mut node) = self.base().try_get_node_as::<Node3D>("AtelierDecor") { node.set_visible(self.loc == Loc::World); }
        if let Some(mut node) = self.base().try_get_node_as::<Node3D>("QuarterProps") { node.set_visible(self.loc == Loc::World); }
        if let Some(icon) = self.weapon_icon.as_mut() {
            let id = self.arsenal.current.id();
            if icon.get_tooltip_text() != id {
                if let Some(texture) = art_texture(&format!("res://assets/icons/weapons/{id}.tres")) { icon.set_texture(&texture); }
                icon.set_tooltip_text(id);
            }
        }
        let selected = STYLES.iter().find(|(id, _, _)| self.state.as_ref().is_some_and(|s| s.has(&format!("style_{id}"))));
        if let Some(icon) = self.fashion_icon.as_mut() {
            icon.set_visible(selected.is_some());
            if let Some((id, ru, en)) = selected {
                let name = if self.settings.lang == "en" { en } else { ru };
                // ResourceLoader caches the texture; only change the icon when selection changes.
                if icon.get_tooltip_text() != *name {
                    if let Some(texture) = art_texture(&format!("res://assets/icons/cosmetics/{id}.tres")) { icon.set_texture(&texture); }
                    icon.set_tooltip_text(*name);
                }
            }
        }
    }

    pub(super) fn wardrobe_scene(&self) -> Scene {
        let en = self.settings.lang == "en";
        let unlocked = self.creative || self.state.as_ref().is_some_and(|s| s.quests.is_completed("atelier_supplies"));
        let mut choices = Vec::new();
        for (index, (id, ru, eng)) in STYLES.iter().enumerate() {
            if !unlocked && index > 2 { continue; }
            let mut effects: Vec<_> = STYLES.iter().map(|(other, _, _)| Effect::UnFlag(format!("style_{other}"))).collect();
            effects.push(Effect::Flag(format!("style_{id}")));
            effects.push(Effect::Flash(if en {format!("Personal emblem: {eng}")} else {format!("Деталь образа: {ru}")}));
            let mut choice = Choice::simple(if en { eng } else { ru }, effects);
            choice.next = Some("wardrobe".into());
            choices.push(choice);
        }
        choices.push(Choice::simple(if en {"Done"} else {"Готово"}, vec![]));
        Scene { id: "wardrobe".into(), lines: vec![Line::new(if en {"Silas · atelier"} else {"Сайлас · ателье"}, "stylist", if en {
            "Style belongs to you. Choose an accessory for your character's personal emblem. It appears beside your health bar and is saved with your character; it does not change combat stats or the first-person hands.\n\nThe full collection unlocks after the atelier supply quest. Creative mode unlocks it immediately."
        } else {
            "Твой образ принадлежит тебе. Выбери деталь для личного знака героя: она появится рядом со здоровьем и сохранится с персонажем. Это оформление, без прибавок к боевым характеристикам и без смены рук от первого лица.\n\nВся коллекция открывается после задания «Материал для мечты». В креативе она доступна сразу."
        })], choices }
    }

    pub(super) fn open_wardrobe(&mut self) {
        self.scene = Some(self.wardrobe_scene());
        self.line_idx = 0;
        self.at_choices = true;
        self.mode = Mode::Dialogue;
        self.freeze_player(true);
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
        if let Some(panel) = self.dlg_panel.as_mut() { panel.set_visible(true); }
        self.refresh_dlg_ui();
    }

    pub(super) fn refresh_dialogue_actions(&mut self) {
        let Some(mut list) = self.dialogue_actions.clone() else { return; };
        for mut child in list.get_children().iter_shared() { list.remove_child(&child); child.queue_free(); }
        let Some(scene) = self.scene.as_ref() else { return; };
        let Some(state) = self.state.as_ref() else { return; };
        let en = self.settings.lang == "en";
        let mut options = Vec::new();
        if let Some(portrait) = self.dialogue_portrait.as_mut() {
            let id = scene.lines.get(self.line_idx).map(|l| l.portrait.as_str()).unwrap_or("");
            let index = match id { "stylist" => Some(0), "ren" => Some(1), "mika" => Some(2), _ => None };
            // Always clear the previous line's artwork, including failed loads.
            portrait.set_visible(false);
            portrait.set_texture(Gd::<Texture2D>::null_arg());
            if let Some(path) = crate::dialogue::artwork_path(&scene.id, id) {
                if let Some(texture) = art_texture(&path) {
                    portrait.set_texture(&texture);
                    portrait.set_visible(true);
                }
            } else if let Some(index) = index {
                if let Some(texture) = art_texture("res://assets/sprites/characters/citizens_atlas.png") {
                    let mut atlas = AtlasTexture::new_gd();
                    atlas.set_atlas(&texture);
                    atlas.set_region(Rect2::new(Vector2::new(index as f32 * 512.0, 0.0), Vector2::new(512.0, 500.0)));
                    portrait.set_texture(&atlas);
                    portrait.set_visible(true);
                }
            }
        }
        if self.at_choices {
            for (index, choice) in scene.choices.iter().filter(|c| c.requires.as_ref().is_none_or(|r| r.is_met(state))).enumerate() {
                let icon = choice.effects.iter().find_map(|e| if let Effect::Flag(flag) = e {flag.strip_prefix("style_").map(str::to_string)} else {None});
                options.push((index as i64, crate::dialogue::localized(&choice.text, &self.settings.lang), icon));
            }
        } else { options.push((-1, if en {"Continue [E]"} else {"Далее [E]"}.into(), None)); }
        options.push((-2, if en {"Leave conversation [Esc]"} else {"Закончить разговор [Esc]"}.into(), None));
        for (action, text, icon) in options {
            let mut button = Button::new_alloc();
            button.set_text(&text);
            button.set_custom_minimum_size(Vector2::new(730.0, 74.0));
            button.add_theme_font_size_override("font_size", 22);
            button.set_autowrap_mode(godot::classes::text_server::AutowrapMode::WORD_SMART);
            if let Some(id) = icon {
                if let Some(texture) = art_texture(&format!("res://assets/icons/cosmetics/{id}.tres")) { button.set_button_icon(&texture); button.set_expand_icon(true); button.add_theme_constant_override("icon_max_width", 64); }
            }
            button.connect("pressed", &self.to_gd().callable("ui_dialogue_action").bind(&[action.to_variant()]));
            list.add_child(&button);
        }
        if let Some(box_) = self.choice_box.as_mut() { box_.set_visible(false); }
    }

    fn build_quarter_props(&mut self) {
        let Some(texture) = self.cache.get("res://assets/decor/femboy_quarter/props_atlas.png") else { return; };
        let mut root = Node3D::new_alloc();
        root.set_name("QuarterProps");
        for (index, x, z, height) in [
            (0, -5.0, -4.4, 1.35), (1, 9.0, -4.4, 3.1),
            (2, -10.0, 5.0, 1.3), (2, 10.0, 5.0, 1.3),
            (3, -9.0, -2.0, 2.0), (4, -12.0, 9.0, 3.0),
            (4, 12.0, 9.0, 3.0), (5, 5.0, 17.0, 1.4),
            (5, -14.0, 28.0, 1.25), (2, 15.0, 22.0, 1.3),
        ] {
            let (y, h) = if index == 1 {(0.0, 478.0)} else if index == 4 {(480.0, 544.0)} else {((index / 3) as f32 * 512.0, 512.0)};
            let mut sprite = Sprite3D::new_alloc();
            sprite.set_texture(&texture);
            sprite.set_region_enabled(true);
            sprite.set_region_rect(Rect2::new(Vector2::new((index % 3) as f32 * 512.0, y), Vector2::new(512.0, h)));
            sprite.set_pixel_size(height / h);
            sprite.set_position(Vector3::new(x, height * 0.5, z));
            sprite.set_texture_filter(TextureFilter::LINEAR);
            sprite.set_draw_flag(godot::classes::sprite_base_3d::DrawFlags::SHADED, false);
            root.add_child(&sprite);
        }
        let mut gallery = Node3D::new_alloc();
        gallery.set_name("NeighborhoodGallery");
        for (index, id) in ["rose", "archive", "music", "repair", "bread", "garden", "dance", "letter", "mask", "heart", "moth", "compass"].iter().enumerate() {
            let Some(texture) = art_texture(&format!("res://assets/decor/neighbors/{id}.tres")) else { continue; };
            let x = if index % 2 == 0 { -11.5 } else { 11.5 };
            let z = 21.0 + (index / 2) as f32 * 6.0;
            let frame = crate::gfx::make_box(Vector3::new(x, 1.9, z), Vector3::new(2.6, 2.4, 0.18), Color::from_rgb(0.05, 0.04, 0.07), None, 1.0);
            gallery.add_child(&frame);
            let post = crate::gfx::make_box(Vector3::new(x, 0.4, z), Vector3::new(0.16, 0.8, 0.16), Color::from_rgb(0.05, 0.04, 0.07), None, 1.0);
            gallery.add_child(&post);
            for side in [-1.0, 1.0] {
                let mut panel = Sprite3D::new_alloc();
                panel.set_texture(&texture);
                panel.set_pixel_size(2.2 / texture.get_height() as f32);
                panel.set_position(Vector3::new(x, 1.9, z + side * 0.101));
                panel.set_texture_filter(TextureFilter::LINEAR);
                panel.set_draw_flag(godot::classes::sprite_base_3d::DrawFlags::SHADED, false);
                gallery.add_child(&panel);
            }
        }
        root.add_child(&gallery);
        self.base_mut().add_child(&root);
    }
}
