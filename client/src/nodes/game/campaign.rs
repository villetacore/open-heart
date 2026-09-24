use super::*;
use godot::classes::Button;

impl Game3D {
    /// Returning home after the final boss completes the authored campaign.
    pub(super) fn maybe_end_campaign(&mut self) -> bool {
        if self.creative { return false; }
        if self.preset != "core" || self.loc != Loc::World || self.net_ready() { return false; }
        if !self.state.as_ref().is_some_and(|s| s.has("boss_defeated_heart_tyrant") && !s.has("campaign_completed")) { return false; }
        self.mode=Mode::Epilogue;
        self.freeze_player(true);
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
        if self.epilogue_panel.is_none() {
            let mut layer=CanvasLayer::new_alloc();
            layer.set_layer(8);
            let mut panel=Panel::new_alloc();
            place(&panel,0.5,0.5,360.0,180.0,1200.0,720.0);
            panel.add_theme_stylebox_override("panel",&make_style(C_UI_BG,C_PINK,2));
            let mut title=Label::new_alloc();
            title.set_text(if self.settings.lang=="en" {"THE HEART BEATS AGAIN"} else {"СЕРДЦЕ СНОВА БЬЁТСЯ"});
            title.set_position(Vector2::new(54.0,50.0));
            title.add_theme_font_size_override("font_size",44);
            title.add_theme_color_override("font_color",C_PINK);
            panel.add_child(&title);
            let mut body=Label::new_alloc();
            body.set_position(Vector2::new(54.0,148.0));
            body.set_size(Vector2::new(1090.0,400.0));
            body.set_autowrap_mode(godot::classes::text_server::AutowrapMode::WORD_SMART);
            body.add_theme_font_size_override("font_size",26);
            body.set_text(if self.settings.lang=="en" {
                "The Tyrant has fallen. For the first time, the pulse beneath the city belongs to its people.\n\nThe city has a chance to rebuild. Its people still need your help, but the pulse that held them captive is silent. Your victories will remember the price of that freedom.\n\nOPENHEART\nCreated by the OpenHeart team and contributors.\nBuilt with Godot and Rust. Thank you for playing.\n\nThe story is complete. Unfinished quests and deeper expeditions remain available."
            } else {
                "Тиран повержен. Впервые пульс под городом принадлежит его жителям.\n\nУ города появился шанс на новую жизнь. Его жителям ещё нужна твоя помощь, но пульс, державший их в плену, затих. Твои победы сохранят память о цене этой свободы.\n\nOPENHEART\nСоздано командой OpenHeart и участниками проекта.\nGodot · Rust. Спасибо за игру.\n\nИстория завершена. Незаконченные задания и новые экспедиции остаются доступны."
            });
            panel.add_child(&body);
            let mut b=Button::new_alloc();
            b.set_text(if self.settings.lang=="en" {"RETURN TO THE CITY  [E]"} else {"ВЕРНУТЬСЯ В ГОРОД  [E]"});
            b.set_position(Vector2::new(54.0,618.0));
            b.set_size(Vector2::new(1090.0,58.0));
            b.add_theme_font_size_override("font_size",24);
            b.connect("pressed",&self.to_gd().callable("ui_finish_campaign"));
            panel.add_child(&b);
            layer.add_child(&panel);
            self.base_mut().add_child(&layer);
            self.epilogue_panel=Some(panel);
        }
        if let Some(p)=self.epilogue_panel.as_mut() {p.set_visible(true);}
        true
    }

    pub(super) fn finish_epilogue(&mut self) {
        if self.mode!=Mode::Epilogue {return;}
        if let Some(state)=self.state.as_mut() {state.flags.insert("campaign_completed".into());}
        if let Some(panel)=self.epilogue_panel.as_mut() {panel.set_visible(false);}
        self.set_mode_explore();
        self.auto_save();
    }
}
