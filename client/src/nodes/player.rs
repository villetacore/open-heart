//! Игрок: FPS-контроллер (WASD + мышь + прыжок + спринт).
//! Параметры (скорость, HP) задаются классом персонажа через Game3D.
//!
//! Контроллер **не читает ввод сам**: он двигает тело по [`PlayerInput`], которую
//! заполняет либо локальный сборщик (клавиатура и мышь), либо сеть — для чужих
//! игроков в кооперативе (docs/MULTIPLAYER.md §12, этап N0).

use godot::classes::input::MouseMode;
use godot::classes::{
    Camera3D, CharacterBody3D, ICharacterBody3D, Input, InputEvent, InputEventMouseMotion,
};
use godot::prelude::*;

use openheart_core::sim::buttons;

const GRAVITY: f32 = -20.0;
const JUMP_SPEED: f32 = 7.0;
const MOUSE_SENS: f32 = 0.002;
const SPRINT_MULT: f32 = 1.42;

pub const MAX_HP: f32 = 100.0;

/// Ввод игрока за кадр — единственный источник команд для контроллера.
///
/// Локальный игрок собирает её в [`PlayerInput::collect_local`], удалённые получают
/// по сети. Биты кнопок общие с сервером ([`openheart_core::sim::buttons`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerInput {
    /// Направление движения в локальных осях: x — вправо, y — вперёд. Длина ≤ 1.
    pub move_dir: Vector2,
    pub yaw: f32,
    pub pitch: f32,
    pub buttons: u16,
}

impl PlayerInput {
    #[inline]
    pub fn pressed(self, bit: u16) -> bool {
        self.buttons & bit != 0
    }

    /// Собрать ввод с клавиатуры. Обзор мышью приходит отдельно
    /// (`unhandled_input`), поэтому yaw/pitch передаются снаружи.
    pub fn collect_local(yaw: f32, pitch: f32) -> Self {
        let input = Input::singleton();
        let mut move_dir = Vector2::ZERO;
        if input.is_action_pressed("move_forward") {
            move_dir.y += 1.0;
        }
        if input.is_action_pressed("move_back") {
            move_dir.y -= 1.0;
        }
        if input.is_action_pressed("move_right") {
            move_dir.x += 1.0;
        }
        if input.is_action_pressed("move_left") {
            move_dir.x -= 1.0;
        }
        if move_dir.length_squared() > 1.0 {
            move_dir = move_dir.normalized();
        }

        let mut bits = 0u16;
        for (action, bit) in [
            ("jump", buttons::JUMP),
            ("sprint", buttons::SPRINT),
            ("shoot", buttons::FIRE),
            ("alt_shoot", buttons::ALT_FIRE),
            ("reload", buttons::RELOAD),
            ("interact", buttons::USE),
        ] {
            if input.is_action_pressed(action) {
                bits |= bit;
            }
        }

        Self { move_dir, yaw, pitch, buttons: bits }
    }

    /// Перевод в сетевой формат: то, что уйдёт серверу вместе с позицией.
    pub fn to_core(self, tick: u32, pos: Vector3, vel: Vector3, on_floor: bool) -> openheart_core::sim::Input {
        openheart_core::sim::Input {
            tick,
            move_dir: openheart_core::math::Vec2::new(self.move_dir.x, self.move_dir.y),
            yaw: self.yaw,
            pitch: self.pitch,
            buttons: self.buttons,
            pos: openheart_core::math::Vec3::new(pos.x, pos.y, pos.z),
            vel: openheart_core::math::Vec3::new(vel.x, vel.y, vel.z),
            on_floor,
        }
    }
}

#[derive(GodotClass)]
#[class(base = CharacterBody3D)]
pub struct Player {
    base: Base<CharacterBody3D>,
    cam: Option<Gd<Camera3D>>,
    cam_base_pos: Vector3,
    base_fov: f32,
    move_time: f32,
    yaw: f32,
    pitch: f32,
    /// Ввод текущего кадра.
    input: PlayerInput,
    /// Кнопки прошлого кадра — для срабатываний «нажали только что» (прыжок).
    prev_buttons: u16,
    /// true — игрок за этим клиентом (ввод собирается локально, есть камера);
    /// false — сетевая копия чужого игрока, её ввод приходит из `set_input`.
    local: bool,
    pub hp: f32,
    pub max_hp: f32,
    pub speed: f32,
    /// Транзитный множитель скорости (замедление-статус); Game3D ставит каждый кадр.
    pub speed_mult: f32,
    /// Оглушён статусом (stun): движение остановлено; стрельбу гейтит Game3D.
    pub stunned: bool,
    pub dead: bool,
    pub frozen: bool, // ввод отключён (меню выбора класса и т.п.)
    pub moving: bool, // для покачивания оружия
    pub sprinting: bool,
}

#[godot_api]
impl ICharacterBody3D for Player {
    fn init(base: Base<CharacterBody3D>) -> Self {
        Self {
            base,
            cam: None,
            cam_base_pos: Vector3::ZERO,
            base_fov: 75.0,
            move_time: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            input: PlayerInput::default(),
            prev_buttons: 0,
            local: true,
            hp: MAX_HP,
            max_hp: MAX_HP,
            speed: 5.0,
            speed_mult: 1.0,
            stunned: false,
            dead: false,
            frozen: false,
            moving: false,
            sprinting: false,
        }
    }

    fn ready(&mut self) {
        let cam = self.base().get_node_as::<Camera3D>("Camera3D");
        self.cam_base_pos = cam.get_position();
        self.base_fov = cam.get_fov();
        self.cam = Some(cam);
        self.base_mut().add_to_group("player");
        if self.local {
            Input::singleton().set_mouse_mode(MouseMode::CAPTURED);
        }
    }

    fn physics_process(&mut self, delta: f64) {
        if self.dead || self.frozen {
            self.moving = false;
            self.sprinting = false;
            self.prev_buttons = 0;
            self.input.buttons = 0;
            return;
        }
        let dt = delta as f32;

        // Локальный игрок собирает ввод сам; чужому его уже положили в set_input.
        if self.local {
            self.input = PlayerInput::collect_local(self.yaw, self.pitch);
        } else {
            self.yaw = self.input.yaw;
            self.pitch = self.input.pitch;
            let yaw = self.yaw;
            self.base_mut().set_rotation(Vector3::new(0.0, yaw, 0.0));
        }
        let input = self.input;
        // Прыжок — по фронту нажатия: удержание не даёт прыгать бесконечно.
        let jump_edge = input.pressed(buttons::JUMP) && self.prev_buttons & buttons::JUMP == 0;
        self.prev_buttons = input.buttons;

        let mut vel = self.base().get_velocity();
        if !self.base().is_on_floor() {
            vel.y += GRAVITY * dt;
        }

        let (sin_y, cos_y) = (self.yaw.sin(), self.yaw.cos());
        let fwd = Vector3::new(-sin_y, 0.0, -cos_y);
        let right = Vector3::new(cos_y, 0.0, -sin_y);

        let mut dir = Vector3::ZERO;
        // оглушение: ввод движения/прыжка игнорируется (мышь-обзор остаётся)
        if !self.stunned {
            dir = fwd * input.move_dir.y + right * input.move_dir.x;
            if jump_edge && self.base().is_on_floor() {
                vel.y = JUMP_SPEED;
            }
        }

        self.sprinting = input.pressed(buttons::SPRINT) && dir.length_squared() > 0.001;
        let sprint = if self.sprinting { SPRINT_MULT } else { 1.0 };

        if dir.length_squared() > 0.001 {
            dir = dir.normalized();
        }
        self.moving = dir.length_squared() > 0.001;
        vel.x = dir.x * self.speed * sprint * self.speed_mult;
        vel.z = dir.z * self.speed * sprint * self.speed_mult;
        self.base_mut().set_velocity(vel);
        self.base_mut().move_and_slide();

        self.move_time += dt * if self.sprinting { 13.0 } else { 9.0 };
        if let Some(ref mut cam) = self.cam {
            let target_fov = self.base_fov + if self.sprinting { 7.0 } else { 0.0 };
            let current_fov = cam.get_fov();
            cam.set_fov(current_fov.lerp(target_fov, (dt * 8.0).min(1.0)));
            let amount = if self.moving {
                if self.sprinting {
                    0.055
                } else {
                    0.032
                }
            } else {
                0.0
            };
            let target = self.cam_base_pos
                + Vector3::new(
                    self.move_time.sin() * amount * 0.55,
                    (self.move_time * 2.0).sin().abs() * amount,
                    0.0,
                );
            let current_position = cam.get_position();
            cam.set_position(current_position.lerp(target, (dt * 12.0).min(1.0)));
        }
    }

    fn unhandled_input(&mut self, event: Gd<InputEvent>) {
        if self.frozen || !self.local {
            return;
        }
        if let Ok(m) = event.try_cast::<InputEventMouseMotion>() {
            if Input::singleton().get_mouse_mode() != MouseMode::CAPTURED {
                return;
            }
            let rel = m.get_relative();
            self.yaw -= rel.x * MOUSE_SENS;
            self.pitch = (self.pitch - rel.y * MOUSE_SENS).clamp(-1.4, 1.4);
            let (yaw, pitch) = (self.yaw, self.pitch);
            self.base_mut().set_rotation(Vector3::new(0.0, yaw, 0.0));
            if let Some(ref mut cam) = self.cam {
                cam.set_rotation(Vector3::new(pitch, 0.0, 0.0));
            }
        }
        // Esc обрабатывает Game3D (меню паузы); прежний тумблер мыши убран,
        // чтобы не конфликтовать с ним.
    }
}

impl Player {
    /// Ввод этого кадра: клиент отправляет его серверу вместе с позицией.
    pub fn current_input(&self) -> PlayerInput {
        self.input
    }

    /// Положить ввод извне — для сетевой копии чужого игрока.
    pub fn set_input(&mut self, input: PlayerInput) {
        self.input = input;
    }

    /// Локальный игрок сам читает клавиатуру и владеет камерой.
    pub fn set_local(&mut self, local: bool) {
        self.local = local;
        if !local {
            self.input = PlayerInput::default();
            self.prev_buttons = 0;
        }
    }

    pub fn is_local(&self) -> bool {
        self.local
    }

    pub fn take_damage(&mut self, amount: f32) {
        if self.dead {
            return;
        }
        self.hp = (self.hp - amount).max(0.0);
        if self.hp == 0.0 {
            self.dead = true;
        }
    }

    pub fn heal(&mut self, amount: f32) {
        self.hp = (self.hp + amount).min(self.max_hp);
    }

    /// Направление взгляда по горизонтали.
    pub fn facing_dir(&self) -> Vector3 {
        Vector3::new(-self.yaw.sin(), 0.0, -self.yaw.cos())
    }

    /// Полное направление взгляда (с учётом наклона камеры).
    pub fn aim_dir(&self) -> Vector3 {
        let (p, y) = (self.pitch, self.yaw);
        Vector3::new(-y.sin() * p.cos(), p.sin(), -y.cos() * p.cos()).normalized()
    }

    /// Точка глаз (камеры) в мировых координатах.
    pub fn eye_pos(&self) -> Vector3 {
        self.base().get_global_position() + Vector3::new(0.0, 0.75, 0.0)
    }

    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    pub fn apply_weapon_recoil(&mut self, recoil: f32) {
        self.pitch = (self.pitch + recoil.max(0.0) * 0.0008).clamp(-1.4, 1.4);
        if let Some(camera) = self.cam.as_mut() {
            camera.set_rotation(Vector3::new(self.pitch, 0.0, 0.0));
        }
    }

    /// Телепорт с сохранением взгляда.
    pub fn teleport(&mut self, pos: Vector3) {
        self.base_mut().set_global_position(pos);
        self.base_mut().set_velocity(Vector3::ZERO);
    }
}
