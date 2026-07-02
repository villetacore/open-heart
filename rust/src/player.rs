use godot::prelude::*;
use godot::classes::{CharacterBody3D, ICharacterBody3D, Camera3D,
                     InputEvent, InputEventMouseMotion, Input};
use godot::classes::input::MouseMode;

const SPEED:      f32 = 5.0;
const GRAVITY:    f32 = -20.0;
const JUMP_SPEED: f32 = 7.0;
const MOUSE_SENS: f32 = 0.002;

pub const MAX_HP: f32 = 100.0;

#[derive(GodotClass)]
#[class(base = CharacterBody3D)]
pub struct Player {
    base:    Base<CharacterBody3D>,
    cam:     Option<Gd<Camera3D>>,
    yaw:     f32,
    pitch:   f32,
    pub hp:     f32,
    pub max_hp: f32,
    pub dead:   bool,
}

#[godot_api]
impl ICharacterBody3D for Player {
    fn init(base: Base<CharacterBody3D>) -> Self {
        Self { base, cam: None, yaw: 0.0, pitch: 0.0,
               hp: MAX_HP, max_hp: MAX_HP, dead: false }
    }

    fn ready(&mut self) {
        let cam = self.base().get_node_as::<Camera3D>("Camera3D");
        self.cam = Some(cam);
        self.base_mut().add_to_group("player");
        Input::singleton().set_mouse_mode(MouseMode::CAPTURED);
    }

    fn physics_process(&mut self, delta: f64) {
        if self.dead { return; }
        let input = Input::singleton();
        let dt = delta as f32;

        let mut vel = self.base().get_velocity();
        if !self.base().is_on_floor() { vel.y += GRAVITY * dt; }

        let (sin_y, cos_y) = (self.yaw.sin(), self.yaw.cos());
        let fwd   = Vector3::new(-sin_y, 0.0, -cos_y);
        let right = Vector3::new( cos_y, 0.0, -sin_y);

        let mut dir = Vector3::ZERO;
        if input.is_action_pressed("move_forward") { dir += fwd; }
        if input.is_action_pressed("move_back")    { dir -= fwd; }
        if input.is_action_pressed("move_right")   { dir += right; }
        if input.is_action_pressed("move_left")    { dir -= right; }

        if input.is_action_just_pressed("jump") && self.base().is_on_floor() {
            vel.y = JUMP_SPEED;
        }

        if dir.length_squared() > 0.001 { dir = dir.normalized(); }
        vel.x = dir.x * SPEED;
        vel.z = dir.z * SPEED;
        self.base_mut().set_velocity(vel);
        self.base_mut().move_and_slide();
    }

    fn unhandled_input(&mut self, event: Gd<InputEvent>) {
        if let Ok(m) = event.try_cast::<InputEventMouseMotion>() {
            let rel = m.get_relative();
            self.yaw  -= rel.x * MOUSE_SENS;
            self.pitch  = (self.pitch - rel.y * MOUSE_SENS).clamp(-1.4, 1.4);
            let (yaw, pitch) = (self.yaw, self.pitch);
            self.base_mut().set_rotation(Vector3::new(0.0, yaw, 0.0));
            if let Some(ref mut cam) = self.cam {
                cam.set_rotation(Vector3::new(pitch, 0.0, 0.0));
            }
        }
        if Input::singleton().is_action_just_pressed("escape") {
            let mode = Input::singleton().get_mouse_mode();
            let new_mode = if mode == MouseMode::CAPTURED { MouseMode::VISIBLE } else { MouseMode::CAPTURED };
            Input::singleton().set_mouse_mode(new_mode);
        }
    }
}

impl Player {
    pub fn take_damage(&mut self, amount: f32) {
        if self.dead { return; }
        self.hp = (self.hp - amount).max(0.0);
        if self.hp == 0.0 { self.dead = true; }
    }

    pub fn heal(&mut self, amount: f32) {
        self.hp = (self.hp + amount).min(self.max_hp);
    }

    pub fn facing_dir(&self) -> Vector3 {
        Vector3::new(-self.yaw.sin(), 0.0, -self.yaw.cos())
    }

    pub fn yaw(&self) -> f32 { self.yaw }
}
