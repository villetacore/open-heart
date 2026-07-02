//! NPC: точка в мире с именем, цветом и «ролью». Логику диалога/квестов держит Game.

use godot::classes::{Label, Node2D, INode2D};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base = Node2D)]
pub struct Npc {
    base: Base<Node2D>,
    pub npc_name: GString,
    pub role: GString,
    pub color: Color,
}

#[godot_api]
impl INode2D for Npc {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            base,
            npc_name: GString::from("NPC"),
            role: GString::from("villager"),
            color: Color::from_rgb(0.9, 0.4, 0.4),
        }
    }

    fn ready(&mut self) {
        // Подпись с именем над персонажем.
        let mut label = Label::new_alloc();
        label.set_text(&self.npc_name.to_string());
        label.set_position(Vector2::new(-40.0, -42.0));
        label.set_size(Vector2::new(80.0, 18.0));
        label.set_horizontal_alignment(godot::global::HorizontalAlignment::CENTER);
        label.add_theme_color_override("font_color", Color::from_rgb(1.0, 1.0, 0.85));
        self.base_mut().add_child(&label);
        self.base_mut().queue_redraw();
    }

    fn draw(&mut self) {
        let col = self.color;
        self.base_mut().draw_rect(
            Rect2::new(Vector2::new(-14.0, -14.0), Vector2::new(28.0, 28.0)),
            col,
        );
        self.base_mut()
            .draw_rect_ex(
                Rect2::new(Vector2::new(-14.0, -14.0), Vector2::new(28.0, 28.0)),
                Color::from_rgb(0.15, 0.1, 0.1),
            )
            .filled(false)
            .width(2.0)
            .done();
    }
}

impl Npc {
    /// Настроить NPC при спавне (вызывается из Game через bind_mut).
    pub fn configure(&mut self, name: &str, role: &str, color: Color) {
        self.npc_name = GString::from(name);
        self.role = GString::from(role);
        self.color = color;
    }

    pub fn role_str(&self) -> String {
        self.role.to_string()
    }

    pub fn name_str(&self) -> String {
        self.npc_name.to_string()
    }
}
