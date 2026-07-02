//! Enemy — CharacterBody3D с AI: патруль → преследование → атака.
//! Визуал: Sprite3D billboard с анимацией (DOOM-стиль), как NPC.

use godot::prelude::*;
use godot::classes::{
    CapsuleShape3D, CharacterBody3D, CollisionShape3D,
    ICharacterBody3D, Image, ImageTexture, Sprite3D, Texture2D,
};
use godot::classes::base_material_3d::BillboardMode;
use godot::classes::sprite_base_3d::AlphaCutMode;

// ── Стандартный формат спрайтшита (512×256, 4 фрейма по 128×256) ─────────────
//    Фреймы: 0-1 = idle, 2-3 = walk

const FRAME_W:    f32 = 128.0;
const FRAME_H:    f32 = 256.0;
const IDLE_FRAMES: [(f32, f32, f32, f32); 2] = [
    (0.0,   0.0, 128.0, 256.0),
    (128.0, 0.0, 128.0, 256.0),
];
const WALK_FRAMES: [(f32, f32, f32, f32); 2] = [
    (256.0, 0.0, 128.0, 256.0),
    (384.0, 0.0, 128.0, 256.0),
];

// Новые пути — assets/sprites/characters/enemy_<id>.png
// Fallback на старые спрайты до момента генерации новых.
fn enemy_tex(id: &str) -> &'static str {
    match id {
        "grunt"   => "res://assets/sprites/characters/enemy_grunt.png",
        "fast"    => "res://assets/sprites/characters/enemy_fast.png",
        "heavy"   => "res://assets/sprites/characters/enemy_heavy.png",
        "brute"   => "res://assets/sprites/characters/enemy_brute.png",
        "sniper"  => "res://assets/sprites/characters/enemy_sniper.png",
        "cultist" => "res://assets/sprites/characters/enemy_cultist.png",
        _         => "res://assets/sprites/characters/enemy_grunt.png",
    }
}

fn enemy_tex_fallback(id: &str) -> &'static str {
    match id {
        "cultist" => "res://assets/sprites/femboy_pink.png",
        _         => "res://assets/sprites/femboy_dark1.png",
    }
}

// ── Состояния AI ──────────────────────────────────────────────────────────────

#[derive(PartialEq, Clone, Copy)]
enum EState { Patrol, Chase, Attack, Dead }

// ── Структура врага ───────────────────────────────────────────────────────────

#[derive(GodotClass)]
#[class(base = CharacterBody3D)]
pub struct Enemy {
    base: Base<CharacterBody3D>,

    // конфиг
    pub cfg_id:       GString,
    pub hp:           f32,
    pub max_hp:       f32,
    speed:            f32,
    pub atk_damage:   f32,
    atk_range:        f32,
    atk_cooldown:     f32,
    chase_range:      f32,
    patrol_radius:    f32,

    // runtime
    state:            EState,
    atk_timer:        f32,
    patrol_target:    Vector3,
    patrol_wait:      f32,
    patrol_counter:   u32,
    spawn_pos:        Vector3,

    player:           Option<Gd<CharacterBody3D>>,
    pub alive:        bool,
    pub pending_dmg:  f32,

    // визуал
    pending_color:    Color,
    tex_path:         &'static str,
    sprite:           Option<Gd<Sprite3D>>,
    anim_timer:       f32,
    anim_frame:       usize,
}

// ── Публичный API ─────────────────────────────────────────────────────────────

impl Enemy {
    pub fn configure(
        &mut self,
        id: &str, hp: f32, speed: f32, damage: f32,
        atk_range: f32, cooldown: f32, chase: f32, patrol: f32,
        color: Color, spawn: Vector3,
    ) {
        self.cfg_id        = GString::from(id);
        self.hp            = hp;
        self.max_hp        = hp;
        self.speed         = speed;
        self.atk_damage    = damage;
        self.atk_range     = atk_range;
        self.atk_cooldown  = cooldown;
        self.chase_range   = chase;
        self.patrol_radius = patrol;
        self.spawn_pos     = spawn;
        self.patrol_target = spawn;
        self.alive         = true;
        self.pending_color = color;
        self.tex_path      = enemy_tex(id);
    }

    pub fn take_damage(&mut self, amount: f32) {
        if !self.alive { return; }
        self.hp -= amount;
        if self.hp <= 0.0 {
            self.hp    = 0.0;
            self.state = EState::Dead;
            self.alive = false;
            self.base_mut().queue_free();
        }
    }

    pub fn set_player(&mut self, player: Gd<CharacterBody3D>) {
        self.player = Some(player);
    }
}

// ── GodotClass callbacks ──────────────────────────────────────────────────────

#[godot_api]
impl ICharacterBody3D for Enemy {
    fn init(base: Base<CharacterBody3D>) -> Self {
        Self {
            base,
            cfg_id: GString::new(),
            hp: 50.0, max_hp: 50.0,
            speed: 2.5, atk_damage: 10.0,
            atk_range: 1.8, atk_cooldown: 1.5,
            chase_range: 8.0, patrol_radius: 3.0,
            state: EState::Patrol,
            atk_timer: 0.0,
            patrol_target: Vector3::ZERO,
            patrol_wait: 0.0,
            patrol_counter: 0,
            spawn_pos: Vector3::ZERO,
            player: None,
            alive: true,
            pending_dmg: 0.0,
            pending_color: Color::from_rgba(0.85, 0.1, 0.1, 1.0),
            tex_path: "res://assets/sprites/femboy_dark1.png",
            sprite: None,
            anim_timer: 0.0,
            anim_frame: 0,
        }
    }

    fn ready(&mut self) {
        // ── Sprite3D (billboard) ──────────────────────────────────────────────
        let color = self.pending_color;
        let tex_path = self.tex_path;

        let mut sp = Sprite3D::new_alloc();
        sp.set_pixel_size(0.010);
        sp.set_billboard_mode(BillboardMode::ENABLED);
        sp.set_alpha_cut_mode(AlphaCutMode::DISCARD);
        sp.set_position(Vector3::new(0.0, 0.8, 0.0));

        // Пробуем новый путь, fallback на старый спрайтшит
        let loaded = Image::load_from_file(tex_path)
            .or_else(|| Image::load_from_file(enemy_tex_fallback(&self.cfg_id.to_string())));
        if let Some(img) = loaded {
            if let Some(itex) = ImageTexture::create_from_image(&img) {
                sp.set_texture(&itex.upcast::<Texture2D>());
                sp.set_region_enabled(true);
                let (x, y, w, h) = IDLE_FRAMES[0];
                sp.set_region_rect(Rect2::new(Vector2::new(x, y), Vector2::new(w, h)));
            }
        }
        sp.set_modulate(color);
        let sp_clone = sp.clone();
        self.base_mut().add_child(&sp);
        self.sprite = Some(sp_clone);

        // ── Коллайдер ─────────────────────────────────────────────────────────
        let mut col = CollisionShape3D::new_alloc();
        let mut cap = CapsuleShape3D::new_gd();
        cap.set_radius(0.3);
        cap.set_height(1.6);
        col.set_shape(&cap);
        col.set_position(Vector3::new(0.0, 0.8, 0.0));
        self.base_mut().add_child(&col);

        self.base_mut().add_to_group("enemies");
        self.spawn_pos     = self.base().get_global_position();
        self.patrol_target = self.spawn_pos;
    }

    fn physics_process(&mut self, delta: f64) {
        if !self.alive || self.state == EState::Dead { return; }
        let dt = delta as f32;

        let player_pos = match self.player.as_ref() {
            Some(p) => p.get_global_position(),
            None    => return,
        };

        let my_pos = self.base().get_global_position();
        let dist   = Vector3::new(player_pos.x - my_pos.x, 0.0, player_pos.z - my_pos.z).length();

        // ── Переходы состояний ────────────────────────────────────────────────
        self.state = match self.state {
            EState::Patrol => {
                if dist < self.chase_range { EState::Chase } else { EState::Patrol }
            }
            EState::Chase => {
                if dist < self.atk_range          { self.atk_timer = self.atk_cooldown; EState::Attack }
                else if dist > self.chase_range * 1.5 { EState::Patrol }
                else { EState::Chase }
            }
            EState::Attack => {
                if dist > self.atk_range * 1.4 { EState::Chase } else { EState::Attack }
            }
            EState::Dead => EState::Dead,
        };

        // ── Поведение ─────────────────────────────────────────────────────────
        let mut vel = Vector3::ZERO;
        match self.state {
            EState::Patrol => {
                if self.patrol_wait > 0.0 {
                    self.patrol_wait -= dt;
                } else {
                    let flat = Vector3::new(
                        self.patrol_target.x - my_pos.x,
                        0.0,
                        self.patrol_target.z - my_pos.z,
                    );
                    if flat.length() < 0.6 {
                        self.patrol_wait = 1.5 + (self.patrol_counter % 3) as f32 * 0.7;
                        let angle = (self.patrol_counter as f32 * 2.399) % (2.0 * std::f32::consts::PI);
                        let r = self.patrol_radius * 0.4
                            + (self.patrol_counter % 5) as f32 * self.patrol_radius * 0.12;
                        self.patrol_target = Vector3::new(
                            self.spawn_pos.x + angle.cos() * r,
                            0.0,
                            self.spawn_pos.z + angle.sin() * r,
                        );
                        self.patrol_counter += 1;
                    } else {
                        vel = flat.normalized() * self.speed * 0.55;
                    }
                }
            }
            EState::Chase => {
                let dir = Vector3::new(
                    player_pos.x - my_pos.x, 0.0, player_pos.z - my_pos.z,
                ).normalized();
                vel = dir * self.speed;
            }
            EState::Attack => {
                self.atk_timer -= dt;
                if self.atk_timer <= 0.0 {
                    self.atk_timer   = self.atk_cooldown;
                    self.pending_dmg += self.atk_damage;
                }
                let dir = Vector3::new(
                    player_pos.x - my_pos.x, 0.0, player_pos.z - my_pos.z,
                ).normalized();
                vel = dir * 0.15;
            }
            EState::Dead => return,
        }

        // ── Физика ────────────────────────────────────────────────────────────
        let mut full_vel = vel;
        if !self.base().is_on_floor() { full_vel.y = -9.8 * dt; }
        self.base_mut().set_velocity(full_vel);
        self.base_mut().move_and_slide();

        // ── Анимация спрайта ──────────────────────────────────────────────────
        let is_moving = vel.length_squared() > 0.1;
        let fps    = if is_moving { 7.0 } else { 2.0 };
        let frames = if is_moving { &WALK_FRAMES } else { &IDLE_FRAMES };
        self.anim_timer += dt;
        if self.anim_timer >= 1.0 / fps {
            self.anim_timer = 0.0;
            self.anim_frame = (self.anim_frame + 1) % frames.len();
            let (x, y, w, h) = frames[self.anim_frame];
            if let Some(ref mut sp) = self.sprite {
                sp.set_region_rect(Rect2::new(Vector2::new(x, y), Vector2::new(w, h)));
            }
        }
    }
}
