//! Enemy — CharacterBody3D с AI: патруль → преследование → атака.
//! Визуал: Sprite3D billboard с анимацией (DOOM-стиль).
//!
//! Обнаружение: зрение (радиус + прямая видимость — сквозь стены не агрятся),
//! слух (Game3D будит врагов вокруг выстрела/взрыва через alert()).
//! В данже преследование идёт по A*-пути (nav.rs), в мире — напрямик.
//! behavior "ranged" держит дистанцию; "melee" идёт в контакт.

use std::sync::Arc;

use godot::classes::base_material_3d::{BillboardMode, TextureFilter};
use godot::classes::sprite_base_3d::AlphaCutMode;
use godot::classes::{
    CapsuleShape3D, CharacterBody3D, CollisionShape3D, CylinderMesh, ICharacterBody3D,
    MeshInstance3D, Node3D, PhysicsRayQueryParameters3D, ResourceLoader, Sprite3D,
    StandardMaterial3D, Texture2D,
};
use godot::prelude::*;

use crate::config::{AbilityCfg, AffixCfg, EnemyAnimationCfg};
use crate::gfx::Rng;
use crate::nav::NavGrid;
use crate::weapon::DmgType;

/// Запрос вражеского выстрела — Game3D собирает их и спавнит снаряды.
pub struct ShotReq {
    pub origin: Vector3,
    pub dir: Vector3,
    pub speed: f32,
    pub damage: f32,
    pub count: u32,
    pub spread: f32,
    pub color: Color,
    /// Статус на игрока при попадании снаряда (id способности).
    pub status: Option<String>,
}

/// Запрос призыва миньонов (kind × count рядом с кастером, mult кастера).
pub struct SummonReq {
    pub kind: String,
    pub count: u32,
    pub pos: Vector3,
    pub mult: f32,
}

/// Запрос лечения союзников в радиусе.
pub struct HealReq {
    pub pos: Vector3,
    pub amount: f32,
    pub radius: f32,
}

pub struct PhaseReq {
    pub pos: Vector3,
    pub color: Color,
    pub boss_name: String,
    pub phase: usize,
    pub total: usize,
}

/// Текущий каст: способность + оставшийся телеграф + цвет подсветки.
struct Cast {
    ability: usize, // индекс в self.abilities
    timer: f32,
    color: Color,
    scale: f32,
}

/// Способность в рантайме: конфиг + кулдаун.
struct AbilityRt {
    cfg: AbilityCfg,
    cd: f32,
}

pub struct BossPhaseDef {
    pub threshold: f32,
    pub abilities: Vec<AbilityCfg>,
    pub damage_mult: f32,
    pub speed_mult: f32,
    pub tint: Option<[f32; 3]>,
}

struct BossPhaseRt {
    threshold: f32,
    abilities: Vec<AbilityCfg>,
    damage_mult: f32,
    speed_mult: f32,
    tint: Option<[f32; 3]>,
}

// ── Спрайтшит 512×256: 4 кадра по 128×256 (idle×2, walk×2) ───────────────────

const IDLE_FRAMES: [(f32, f32, f32, f32); 8] = [
    (0.0, 0.0, 128.0, 256.0),
    (128.0, 0.0, 128.0, 256.0),
    (256.0, 0.0, 128.0, 256.0),
    (384.0, 0.0, 128.0, 256.0),
    (512.0, 0.0, 128.0, 256.0),
    (640.0, 0.0, 128.0, 256.0),
    (768.0, 0.0, 128.0, 256.0),
    (896.0, 0.0, 128.0, 256.0),
];

fn animation_frame(row: usize, frame: usize) -> (f32, f32, f32, f32) {
    ((frame % 8) as f32 * 128.0, row as f32 * 256.0, 128.0, 256.0)
}

/// Путь спрайт-листа по имени (без префикса пути): "grunt" → enemy_grunt.png.
fn enemy_tex(sprite: &str) -> String {
    format!("res://assets/sprites/characters/enemy_{}.png", sprite)
}
const ENEMY_TEX_FALLBACK: &str = "res://assets/sprites/characters/enemy_grunt.png";

#[derive(PartialEq, Clone, Copy)]
enum EState {
    Patrol,
    Chase,
    Attack,
    Dead,
}

#[derive(PartialEq, Clone, Copy)]
enum EnemyAnimState {
    Idle,
    Move,
    Alert,
    Attack,
    Pain,
    Cast,
    Charge,
    Phase,
    Death,
}

/// Боевое поведение (enemies.json: "behavior", по умолчанию melee).
#[derive(PartialEq, Clone, Copy)]
pub enum Behavior {
    Melee,
    Ranged,
}

impl Behavior {
    pub fn from_id(s: &str) -> Self {
        if s == "ranged" {
            Self::Ranged
        } else {
            Self::Melee
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum TacticalRole {
    Pursuer,
    Tank,
    Artillery,
    Support,
    Summoner,
    Controller,
    Commander,
}

impl TacticalRole {
    pub fn from_id(id: &str) -> Self {
        match id {
            "tank" => Self::Tank,
            "artillery" => Self::Artillery,
            "support" => Self::Support,
            "summoner" => Self::Summoner,
            "controller" => Self::Controller,
            "commander" => Self::Commander,
            _ => Self::Pursuer,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Pursuer => "pursuer",
            Self::Tank => "tank",
            Self::Artillery => "artillery",
            Self::Support => "support",
            Self::Summoner => "summoner",
            Self::Controller => "controller",
            Self::Commander => "commander",
        }
    }

    fn prefers_range(self) -> bool {
        matches!(
            self,
            Self::Artillery | Self::Support | Self::Summoner | Self::Controller
        )
    }

    fn retreat_ratio(self) -> f32 {
        match self {
            Self::Support | Self::Summoner => 0.72,
            Self::Controller => 0.62,
            Self::Artillery => 0.52,
            _ => 0.45,
        }
    }

    fn ability_cooldown_mult(self) -> f32 {
        match self {
            Self::Commander => 0.72,
            Self::Support | Self::Summoner => 0.78,
            Self::Controller => 0.86,
            _ => 1.0,
        }
    }
}

#[derive(GodotClass)]
#[class(base = CharacterBody3D)]
pub struct Enemy {
    base: Base<CharacterBody3D>,

    // конфиг
    pub cfg_id: GString,
    display_name: String,
    pub hp: f32,
    pub max_hp: f32,
    speed: f32,
    pub atk_damage: f32,
    atk_range: f32,
    atk_cooldown: f32,
    chase_range: f32,
    patrol_radius: f32,
    pub xp_value: f32,
    pub is_boss: bool,
    resist: [f32; 4], // резисты по DmgType::idx
    vis_scale: f32,   // масштаб спрайта/коллайдера
    behavior: Behavior,
    tactical_role: TacticalRole,

    // runtime
    state: EState,
    atk_timer: f32,
    attack_anim: f32,
    attack_hit_timer: f32,
    patrol_target: Vector3,
    patrol_wait: f32,
    patrol_counter: u32,
    spawn_pos: Vector3,

    // навигация (только в данже)
    nav: Option<Arc<NavGrid>>,
    nav_offset: Vector3,   // DUNGEON_OFFSET: мир → локальные координаты
    path: Vec<(i32, i32)>, // клетки до игрока (первая — ближайшая)
    path_timer: f32,       // перерасчёт пути
    los_timer: f32,        // троттлинг рейкаста зрения
    los_cached: bool,
    alert_timer: f32, // тревога от шума: пока > 0, поводок не отпускает

    // способности (abilities.json)
    abilities: Vec<AbilityRt>,
    boss_phases: Vec<BossPhaseRt>,
    boss_phase_idx: usize,
    phase_transition: f32,
    cast: Option<Cast>, // активный телеграф
    charge_timer: f32,  // остаток рывка
    charge_dir: Vector3,
    charge_speed: f32,
    charge_damage: f32,
    charge_hit: bool, // урон рывка нанесён (один раз за рывок)
    stagger: f32,     // >0 — оглушён (pain)
    pain_chance: f32,
    spawn_mult: f32, // множитель силы (наследуют миньоны)
    weak_point_height: f32,
    weak_point_multiplier: f32,

    // элита (аффиксы)
    elite_prefix: String,                // «Быстрый Вампирический» (для HUD)
    lifesteal: f32,                      // доля урона игроку → своё HP
    pub death_blast: Option<(f32, f32)>, // (урон, радиус) при смерти
    rng: Rng,
    pub shot_reqs: Vec<ShotReq>,
    pub summon_reqs: Vec<SummonReq>,
    pub heal_reqs: Vec<HealReq>,
    pub phase_reqs: Vec<PhaseReq>,

    player: Option<Gd<CharacterBody3D>>,
    pub alive: bool,
    /// Мир «на паузе» (игрок в меню/диалоге): AI не двигается и не атакует.
    pub frozen: bool,
    pub pending_dmg: f32,
    /// Статус, который враг наложит на игрока при попадании (мили/рывок).
    pub pending_status: Option<String>,
    attack_status: Option<(String, f32)>, // (id, шанс) — из enemies.json

    // статусы урона (горение/кровь/замедление/оглушение/уязвимость)
    statuses: crate::status::StatusSet,

    // визуал
    pending_color: Color,
    tex_path: String,
    sprite: Option<Gd<Sprite3D>>,
    telegraph_disc: Option<Gd<MeshInstance3D>>,
    anim_timer: f32,
    anim_frame: usize,
    anim_row: usize,
    anim_state: EnemyAnimState,
    anim_cfg: EnemyAnimationCfg,
    hurt_flash: f32,
    alert_anim: f32,
    pub death_timer: f32,
}

impl Enemy {
    #[allow(clippy::too_many_arguments)]
    pub fn configure(
        &mut self,
        id: &str,
        display_name: &str,
        hp: f32,
        speed: f32,
        damage: f32,
        atk_range: f32,
        cooldown: f32,
        chase: f32,
        patrol: f32,
        _color: Color,
        spawn: Vector3,
        xp: f32,
        mult: f32,
        is_boss: bool,
        resist: [f32; 4],
        sprite: &str,
        scale: f32,
        behavior: Behavior,
        tactical_role: TacticalRole,
        mut animation: EnemyAnimationCfg,
    ) {
        self.behavior = if tactical_role.prefers_range() {
            Behavior::Ranged
        } else {
            behavior
        };
        self.tactical_role = tactical_role;
        match tactical_role {
            TacticalRole::Pursuer => {
                animation.attack_fps.get_or_insert(14.0);
                animation.pain_fps.get_or_insert(12.0);
            }
            TacticalRole::Tank => {
                animation.attack_fps.get_or_insert(7.0);
                animation.pain_fps.get_or_insert(6.0);
                animation.death_fps.get_or_insert(7.0);
            }
            TacticalRole::Artillery => {
                animation.attack_fps.get_or_insert(9.0);
                animation.cast_fps.get_or_insert(8.0);
            }
            TacticalRole::Support | TacticalRole::Summoner => {
                animation.cast_fps.get_or_insert(6.0);
                animation.alert_fps.get_or_insert(7.0);
            }
            TacticalRole::Controller => {
                animation.cast_fps.get_or_insert(9.0);
                animation.pain_fps.get_or_insert(8.0);
            }
            TacticalRole::Commander => {
                animation.attack_fps.get_or_insert(8.0);
                animation.cast_fps.get_or_insert(7.0);
                animation.death_fps.get_or_insert(6.0);
            }
        }
        self.cfg_id = GString::from(id);
        self.display_name = display_name.to_string();
        self.hp = hp * mult;
        self.max_hp = hp * mult;
        self.speed = speed;
        self.atk_damage = damage * mult;
        self.atk_range = atk_range;
        self.atk_cooldown = cooldown;
        self.chase_range = chase;
        self.patrol_radius = patrol;
        self.spawn_pos = spawn;
        self.patrol_target = spawn;
        self.alive = true;
        self.death_timer = 0.0;
        self.pending_color = Color::WHITE;
        self.tex_path = enemy_tex(if sprite.is_empty() { id } else { sprite });
        self.xp_value = xp * mult;
        self.is_boss = is_boss;
        self.resist = resist;
        self.vis_scale = if is_boss { scale.max(1.35) } else { 1.0 };
        self.spawn_mult = mult;
        self.anim_cfg = animation;
    }

    fn animation_duration(frames: usize, fps: f32) -> f32 {
        frames.clamp(1, 8) as f32 / fps.max(0.1)
    }

    fn begin_death(&mut self) {
        self.hp = 0.0;
        self.state = EState::Dead;
        self.alive = false;
        self.death_timer =
            Self::animation_duration(self.anim_cfg.death_frames, self.anim_cfg.death_fps())
                .clamp(0.45, 1.6);
    }

    fn tick_visual_animation(
        &mut self,
        state: EnemyAnimState,
        row: usize,
        frame_count: usize,
        fps: f32,
        dt: f32,
        looping: bool,
    ) {
        let frame_count = frame_count.clamp(1, 8);
        if state != self.anim_state || row != self.anim_row {
            self.anim_state = state;
            self.anim_row = row;
            self.anim_frame = 0;
            self.anim_timer = 0.0;
        }
        self.anim_timer += dt;
        let frame_time = 1.0 / fps.max(0.1);
        while self.anim_timer >= frame_time {
            self.anim_timer -= frame_time;
            self.anim_frame = if looping {
                (self.anim_frame + 1) % frame_count
            } else {
                (self.anim_frame + 1).min(frame_count - 1)
            };
        }
        let (x, y, w, h) = animation_frame(row, self.anim_frame);
        let progress = self.anim_frame as f32 / frame_count.saturating_sub(1).max(1) as f32;
        let phase = progress * std::f32::consts::TAU;
        let base_y = 1.2 * self.vis_scale;
        let (offset_y, scale, rotation_z) = match state {
            EnemyAnimState::Idle => (phase.sin() * 0.025 * self.vis_scale, 1.0, 0.0),
            EnemyAnimState::Move => (phase.sin().abs() * 0.075 * self.vis_scale, 1.0, 0.0),
            EnemyAnimState::Alert => (phase.sin().abs() * 0.04 * self.vis_scale, 1.02, 0.0),
            EnemyAnimState::Attack => (
                (progress * std::f32::consts::PI).sin() * 0.06 * self.vis_scale,
                1.03,
                0.0,
            ),
            EnemyAnimState::Pain => (-0.045 * self.vis_scale, 0.97, -0.035),
            EnemyAnimState::Cast => (phase.sin().abs() * 0.07 * self.vis_scale, 1.05, 0.0),
            EnemyAnimState::Charge => (0.025 * self.vis_scale, 1.08, 0.0),
            EnemyAnimState::Phase => (phase.sin().abs() * 0.1 * self.vis_scale, 1.12, 0.0),
            EnemyAnimState::Death => (-progress * 0.28 * self.vis_scale, 1.0, progress * 0.12),
        };
        if let Some(ref mut sprite) = self.sprite {
            sprite.set_region_rect(Rect2::new(Vector2::new(x, y), Vector2::new(w, h)));
            sprite.set_position(Vector3::new(0.0, base_y + offset_y, 0.0));
            sprite.set_scale(Vector3::new(scale, scale, scale));
            sprite.set_rotation(Vector3::new(0.0, 0.0, rotation_z));
        }
    }

    #[cfg(debug_assertions)]
    pub fn runtime_smoke_animation_profile(&mut self) -> (usize, bool, bool, bool, u32) {
        let states = [
            (
                EnemyAnimState::Idle,
                self.anim_cfg.idle_row,
                self.anim_cfg.idle_frames,
                self.anim_cfg.idle_fps,
                true,
            ),
            (
                EnemyAnimState::Move,
                self.anim_cfg.move_row,
                self.anim_cfg.move_frames,
                self.anim_cfg.action_fps,
                true,
            ),
            (
                EnemyAnimState::Alert,
                self.anim_cfg.alert_row,
                self.anim_cfg.alert_frames,
                self.anim_cfg.alert_fps(),
                false,
            ),
            (
                EnemyAnimState::Attack,
                self.anim_cfg.attack_row,
                self.anim_cfg.attack_frames,
                self.anim_cfg.attack_fps(),
                false,
            ),
            (
                EnemyAnimState::Pain,
                self.anim_cfg.pain_row,
                self.anim_cfg.pain_frames,
                self.anim_cfg.pain_fps(),
                false,
            ),
            (
                EnemyAnimState::Cast,
                self.anim_cfg.cast_row,
                self.anim_cfg.cast_frames,
                self.anim_cfg.cast_fps(),
                true,
            ),
            (
                EnemyAnimState::Charge,
                self.anim_cfg.attack_row,
                self.anim_cfg.attack_frames,
                self.anim_cfg.attack_fps() * 1.15,
                true,
            ),
            (
                EnemyAnimState::Phase,
                self.anim_cfg.cast_row,
                self.anim_cfg.cast_frames,
                self.anim_cfg.cast_fps() * 0.8,
                true,
            ),
            (
                EnemyAnimState::Death,
                self.anim_cfg.death_row,
                self.anim_cfg.death_frames,
                self.anim_cfg.death_fps(),
                false,
            ),
        ];
        let mut transitioned = 0;
        let mut cast_animated = false;
        let mut pain_animated = false;
        for (state, row, frames, fps, looping) in states {
            self.tick_visual_animation(state, row, frames, fps, 1.0 / fps.max(0.1) + 0.01, looping);
            if self.anim_state == state && self.anim_row == row {
                transitioned += 1;
            }
            if state == EnemyAnimState::Cast {
                cast_animated = frames <= 1 || self.anim_frame > 0;
            }
            if state == EnemyAnimState::Pain {
                pain_animated = frames <= 1 || self.anim_frame > 0;
            }
        }
        self.tick_visual_animation(
            EnemyAnimState::Attack,
            self.anim_cfg.attack_row,
            self.anim_cfg.attack_frames,
            self.anim_cfg.attack_fps(),
            10.0,
            false,
        );
        let one_shot_clamped = self.anim_frame == self.anim_cfg.attack_frames.clamp(1, 8) - 1;
        self.tick_visual_animation(
            EnemyAnimState::Idle,
            self.anim_cfg.idle_row,
            self.anim_cfg.idle_frames,
            self.anim_cfg.idle_fps,
            0.0,
            true,
        );
        let signature = (self.anim_cfg.attack_fps() * 10.0) as u32 * 10_000
            + (self.anim_cfg.cast_fps() * 10.0) as u32 * 100
            + (self.anim_cfg.death_fps() * 10.0) as u32;
        (
            transitioned,
            cast_animated,
            pain_animated,
            one_shot_clamped,
            signature,
        )
    }

    /// Применить аффиксы элиты: мультипликаторы статов, tint, спец-механики.
    /// Вызывать ПОСЛЕ configure и set_combat_extras (модифицирует pain_chance).
    pub fn apply_affixes(&mut self, affixes: &[AffixCfg]) {
        for a in affixes {
            self.hp *= a.hp_mult;
            self.max_hp *= a.hp_mult;
            self.atk_damage *= a.dmg_mult;
            self.speed *= a.speed_mult;
            self.xp_value *= a.xp_mult;
            self.pain_chance *= a.pain_mult;
            self.lifesteal += a.lifesteal;
            if let Some([dmg, r]) = a.death_blast {
                // урон взрыва масштабируется силой врага, как и остальной урон
                self.death_blast = Some((dmg * self.spawn_mult, r));
            }
            if !self.elite_prefix.is_empty() {
                self.elite_prefix.push(' ');
            }
            self.elite_prefix.push_str(&a.name_ru);
        }
    }

    /// Имя для HUD: «Быстрый grunt» у элит, просто id — у обычных.
    pub fn display_name(&self) -> String {
        if self.elite_prefix.is_empty() {
            self.display_name.clone()
        } else {
            format!("{} {}", self.elite_prefix, self.display_name)
        }
    }

    /// Элита? (для HUD-подсветки)
    pub fn is_elite(&self) -> bool {
        !self.elite_prefix.is_empty()
    }

    /// Способности из abilities.json + pain_chance. Отдельно от configure —
    /// нужен доступ к GameConfig; сид рандомизирует кулдауны/pain.
    pub fn set_combat_extras(&mut self, abilities: Vec<AbilityCfg>, pain_chance: f32, seed: u64) {
        self.rng = Rng::new(seed | 1);
        self.pain_chance = pain_chance
            * match self.tactical_role {
                TacticalRole::Tank => 0.35,
                TacticalRole::Commander => 0.6,
                _ => 1.0,
            };
        let cooldown_mult = self.tactical_role.ability_cooldown_mult();
        self.abilities = abilities
            .into_iter()
            .map(|cfg| {
                // стартовый кулдаун случайный — толпа не кастует синхронно
                let cd = cfg.cooldown * cooldown_mult * (0.3 + self.rng.f32() * 0.7);
                AbilityRt { cfg, cd }
            })
            .collect();
    }

    pub fn tactical_role_id(&self) -> &'static str {
        self.tactical_role.id()
    }

    fn ability_telegraph_scale(&self, cfg: &AbilityCfg) -> f32 {
        match cfg.kind.as_str() {
            "charge" => 1.45,
            "summon" => 1.8,
            "heal_pulse" => (cfg.radius / (1.25 * self.vis_scale)).clamp(1.0, 5.5),
            "projectile_burst" => (1.0 + cfg.spread * 1.8).clamp(1.0, 1.8),
            _ => 1.0,
        }
    }

    pub fn strongest_telegraph_scale(&self) -> f32 {
        self.abilities
            .iter()
            .map(|ability| self.ability_telegraph_scale(&ability.cfg))
            .fold(1.0, f32::max)
    }

    pub fn set_boss_phases(&mut self, mut phases: Vec<BossPhaseDef>) {
        phases.sort_by(|a, b| {
            b.threshold
                .partial_cmp(&a.threshold)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.boss_phases = phases
            .into_iter()
            .map(|phase| BossPhaseRt {
                threshold: phase.threshold.clamp(0.0, 1.0),
                abilities: phase.abilities,
                damage_mult: phase.damage_mult.max(0.1),
                speed_mult: phase.speed_mult.max(0.1),
                tint: phase.tint,
            })
            .collect();
        self.boss_phase_idx = 0;
    }

    pub fn boss_phase_index(&self) -> usize {
        self.boss_phase_idx
    }

    pub fn boss_phase_count(&self) -> usize {
        self.boss_phases.len()
    }

    fn update_boss_phase(&mut self) {
        let health_ratio = if self.max_hp > 0.0 {
            self.hp / self.max_hp
        } else {
            0.0
        };
        while let Some(phase) = self.boss_phases.get(self.boss_phase_idx) {
            if health_ratio > phase.threshold {
                break;
            }
            let abilities = phase.abilities.clone();
            let damage_mult = phase.damage_mult;
            let speed_mult = phase.speed_mult;
            let tint = phase.tint;
            self.boss_phase_idx += 1;
            self.atk_damage *= damage_mult;
            self.speed *= speed_mult;
            for cfg in abilities {
                if self
                    .abilities
                    .iter()
                    .any(|ability| ability.cfg.id == cfg.id)
                {
                    continue;
                }
                self.abilities.push(AbilityRt {
                    cd: cfg.cooldown.min(0.8),
                    cfg,
                });
            }
            if let Some([r, g, b]) = tint {
                self.pending_color = Color::from_rgba(r, g, b, 1.0);
            }
            self.cast = None;
            self.charge_timer = 0.0;
            self.phase_transition = self.phase_transition.max(0.9);
            self.stagger = 0.0;
            self.alert_anim =
                Self::animation_duration(self.anim_cfg.alert_frames, self.anim_cfg.alert_fps());
            self.hurt_flash =
                Self::animation_duration(self.anim_cfg.pain_frames, self.anim_cfg.pain_fps());
            self.phase_reqs.push(PhaseReq {
                pos: self.base().get_global_position(),
                color: self.pending_color,
                boss_name: self.display_name(),
                phase: self.boss_phase_idx,
                total: self.boss_phases.len(),
            });
        }
    }

    /// Статус на игрока при атаке (из enemies.json).
    pub fn set_attack_status(&mut self, status: Option<(String, f32)>) {
        self.attack_status = status;
    }

    /// Наложить статус (из оружия/способности/самого себя).
    pub fn apply_status(&mut self, cfg: &crate::config::StatusCfg) {
        self.statuses.apply(cfg);
    }

    /// Ролл статуса на игрока при попадании атакой; ставит pending_status,
    /// который Game3D снимет вместе с pending_dmg.
    fn roll_attack_status(&mut self) {
        if let Some((id, chance)) = self.attack_status.clone() {
            if self.rng.f32() < chance {
                self.pending_status = Some(id);
            }
        }
    }

    /// Урон-по-тику (DoT): резист и уязвимость учитываются (как у take_damage —
    /// «ослабь, потом подожги» работает), но без pain-стаггера/пробуждения —
    /// иначе горящий враг был бы вечно оглушён.
    fn take_dot(&mut self, amount: f32, dmg_type: DmgType) {
        if !self.alive {
            return;
        }
        let dealt =
            (amount * (1.0 - self.resist[dmg_type.idx()])).max(0.0) * self.statuses.vuln_mult();
        self.hp -= dealt;
        self.update_boss_phase();
        if self.hp <= 0.0 {
            self.begin_death();
        }
    }

    /// Урон с учётом типа, резиста и уязвимости. Возвращает фактически нанесённый.
    pub fn take_damage(&mut self, amount: f32, dmg_type: DmgType) -> f32 {
        if !self.alive {
            return 0.0;
        }
        // уязвимость (weakened) усиливает входящий урон после резиста
        let dealt =
            (amount * (1.0 - self.resist[dmg_type.idx()])).max(0.0) * self.statuses.vuln_mult();
        self.hp -= dealt;
        self.update_boss_phase();
        self.hurt_flash =
            Self::animation_duration(self.anim_cfg.pain_frames, self.anim_cfg.pain_fps())
                .clamp(0.16, 0.65);
        self.alert_anim =
            Self::animation_duration(self.anim_cfg.alert_frames, self.anim_cfg.alert_fps())
                .clamp(0.2, 1.0);
        // проснуться при уроне (и не отпускать поводок, даже если игрок далеко)
        self.alert_timer = if self.tactical_role == TacticalRole::Commander {
            10.0
        } else {
            6.0
        };
        if self.state == EState::Patrol {
            self.state = EState::Chase;
        }
        // pain: шанс стаггера — прерывает телеграф и рывок (контр-игра против кастеров)
        if self.pain_chance > 0.0 && self.rng.f32() < self.pain_chance {
            self.stagger = 0.4;
            self.cast = None;
            self.charge_timer = 0.0;
        }
        if self.hp <= 0.0 {
            self.begin_death();
        }
        dealt
    }

    pub fn set_player(&mut self, player: Gd<CharacterBody3D>) {
        self.player = Some(player);
    }

    pub fn set_weak_point(&mut self, height: f32, multiplier: f32) {
        self.weak_point_height = height.clamp(0.5, 0.95);
        self.weak_point_multiplier = multiplier.clamp(1.0, 3.0);
    }

    pub fn weak_point_multiplier(&self) -> f32 {
        self.weak_point_multiplier
    }

    pub fn weak_point_multiplier_at(&self, world_hit: Vector3) -> f32 {
        let body_height = 1.6 * self.vis_scale;
        let local_height = world_hit.y - self.base().get_global_position().y;
        if local_height >= body_height * self.weak_point_height {
            self.weak_point_multiplier
        } else {
            1.0
        }
    }

    /// Навигационная сетка данжа (offset — позиция корня данжа в мире).
    pub fn set_nav(&mut self, nav: Arc<NavGrid>, offset: Vector3) {
        self.nav = Some(nav);
        self.nav_offset = offset;
    }

    /// Разбудить (шум выстрела/взрыва, тревога от соседа): патруль → погоня.
    /// Таймер тревоги не даёт поводку (chase_range×1.8) сразу отпустить врага,
    /// который услышал шум издалека.
    /// Возвращает `true`, если враг только что перешёл из патруля в погоню
    /// (используется, чтобы проиграть звук «пробуждения» один раз).
    pub fn alert(&mut self) -> bool {
        if !self.alive {
            return false;
        }
        self.alert_timer = if self.tactical_role == TacticalRole::Commander {
            10.0
        } else {
            6.0
        };
        if self.state == EState::Patrol {
            self.state = EState::Chase;
            self.alert_anim =
                Self::animation_duration(self.anim_cfg.alert_frames, self.anim_cfg.alert_fps())
                    .clamp(0.2, 1.0);
            return true;
        }
        false
    }

    /// Отталкивание от других врагов рядом — стая не слипается в колонну.
    fn separation(&self, my_pos: Vector3) -> Vector3 {
        let tree = self.base().get_tree();
        let my_id = self.base().instance_id();
        let mut push = Vector3::ZERO;
        for node in tree.get_nodes_in_group("enemies").iter_shared() {
            if node.instance_id() == my_id {
                continue;
            }
            let Ok(other) = node.try_cast::<Node3D>() else {
                continue;
            };
            let d = my_pos - other.get_global_position();
            let flat = Vector3::new(d.x, 0.0, d.z);
            let len = flat.length();
            if len > 0.01 && len < 1.7 {
                push += flat / len * (1.7 - len);
            }
        }
        push
    }

    /// Направление очередного шага по A*-пути к игроку (данж).
    /// None — сетки нет или путь не найден (вызывающий идёт напрямик).
    fn chase_dir(&mut self, my_pos: Vector3, player_pos: Vector3) -> Option<Vector3> {
        use crate::dungeon::CELL;
        let nav = self.nav.as_ref()?.clone();
        let my_local = my_pos - self.nav_offset;
        let pl_local = player_pos - self.nav_offset;

        // перерасчёт строго по таймеру: пустой РЕЗУЛЬТАТ (пути нет) не должен
        // гонять полный A* каждый кадр; исчерпанный путь → straight-фолбэк
        if self.path_timer <= 0.0 {
            self.path = nav
                .astar(NavGrid::cell_of(my_local), NavGrid::cell_of(pl_local))
                .unwrap_or_default();
            self.path_timer = if self.path.is_empty() { 0.35 } else { 0.7 };
        }
        // выкидываем достигнутые вейпоинты
        while let Some(&wp) = self.path.first() {
            let c = NavGrid::center_of(wp.0, wp.1);
            if Vector3::new(c.x - my_local.x, 0.0, c.z - my_local.z).length() < CELL * 0.35 {
                self.path.remove(0);
            } else {
                break;
            }
        }
        let wp = self.path.first()?;
        let c = NavGrid::center_of(wp.0, wp.1);
        let d = Vector3::new(c.x - my_local.x, 0.0, c.z - my_local.z);
        (d.length() > 0.01).then(|| d.normalized())
    }

    /// Применить скорость с гравитацией и шагнуть физикой (ранние ветки ИИ).
    fn apply_velocity(&mut self, vel: Vector3) {
        let mut fv = vel;
        if !self.base().is_on_floor() {
            fv.y = -9.8;
        }
        self.base_mut().set_velocity(fv);
        self.base_mut().move_and_slide();
    }

    /// Эффект способности по окончании телеграфа.
    fn fire_ability(&mut self, idx: usize, my_pos: Vector3, player_pos: Vector3) {
        let Some(rt) = self.abilities.get(idx) else {
            return;
        };
        let cfg = rt.cfg.clone();
        let flat = Vector3::new(player_pos.x - my_pos.x, 0.0, player_pos.z - my_pos.z);
        let dir = if flat.length() > 0.01 {
            flat.normalized()
        } else {
            Vector3::new(0.0, 0.0, -1.0)
        };
        let color = cfg
            .color
            .map(|c| Color::from_rgba(c[0], c[1], c[2], 1.0))
            .unwrap_or(Color::from_rgba(1.0, 0.5, 0.8, 1.0));
        match cfg.kind.as_str() {
            "projectile_burst" => {
                let origin = my_pos + Vector3::new(0.0, 1.15 * self.vis_scale, 0.0) + dir * 0.6;
                let aim = (player_pos + Vector3::new(0.0, 1.0, 0.0) - origin).normalized();
                // статус способности накладывается по шансу при попадании
                let status = cfg
                    .status
                    .as_ref()
                    .filter(|s| self.rng.f32() < s.chance)
                    .map(|s| s.id.clone());
                self.shot_reqs.push(ShotReq {
                    origin,
                    dir: aim,
                    speed: cfg.proj_speed.max(4.0),
                    damage: cfg.damage * self.spawn_mult,
                    count: cfg.count.max(1),
                    spread: cfg.spread,
                    color,
                    status,
                });
            }
            "charge" => {
                self.charge_dir = dir;
                self.charge_speed = self.speed * cfg.speed_mult.max(1.2);
                self.charge_damage = cfg.damage * self.spawn_mult;
                self.charge_timer = cfg.duration.max(0.2);
                self.charge_hit = false;
            }
            "summon" => {
                if let Some(minion) = cfg.minion.clone() {
                    self.summon_reqs.push(SummonReq {
                        kind: minion,
                        count: cfg.count.max(1),
                        pos: my_pos,
                        mult: self.spawn_mult,
                    });
                }
            }
            "heal_pulse" => {
                self.heal_reqs.push(HealReq {
                    pos: my_pos,
                    amount: cfg.heal,
                    radius: cfg.radius.max(1.0),
                });
            }
            other => {
                godot_warn!("[ability] '{}': неизвестный kind '{other}'", cfg.id);
            }
        }
    }

    /// Забрать накопленные запросы способностей (Game3D, раз в кадр).
    pub fn drain_requests(
        &mut self,
    ) -> (Vec<ShotReq>, Vec<SummonReq>, Vec<HealReq>, Vec<PhaseReq>) {
        (
            std::mem::take(&mut self.shot_reqs),
            std::mem::take(&mut self.summon_reqs),
            std::mem::take(&mut self.heal_reqs),
            std::mem::take(&mut self.phase_reqs),
        )
    }

    pub fn phase_transition_active(&self) -> bool {
        self.phase_transition > 0.0
    }

    pub fn pending_phase_events(&self) -> usize {
        self.phase_reqs.len()
    }

    /// Лечение от heal_pulse союзника.
    pub fn heal_hp(&mut self, amount: f32) {
        if self.alive {
            self.hp = (self.hp + amount).min(self.max_hp);
        }
    }

    /// Строка статусов для HUD (иконки).
    pub fn status_summary(&self) -> String {
        self.statuses.summary()
    }

    pub fn is_casting(&self) -> bool {
        self.cast.is_some() || self.attack_hit_timer >= 0.0
    }

    /// Забрать статус, который враг наложит на игрока (сбрасывает).
    pub fn take_pending_status(&mut self) -> Option<String> {
        self.pending_status.take()
    }

    /// Есть ли прямая видимость до игрока (для дальнобойных атак).
    fn has_los(&mut self, player_pos: Vector3) -> bool {
        let from = self.base().get_global_position() + Vector3::new(0.0, 1.3, 0.0);
        let to = player_pos + Vector3::new(0.0, 1.3, 0.0);
        let Some(world) = self.base().get_world_3d() else {
            return true;
        };
        let Some(mut space) = world.clone().get_direct_space_state() else {
            return true;
        };
        let Some(mut query) = PhysicsRayQueryParameters3D::create(from, to) else {
            return true;
        };
        let mut excl: godot::builtin::Array<Rid> = godot::builtin::Array::new();
        excl.push(self.base().get_rid());
        query.set_exclude(&excl);
        let hit = space.intersect_ray(&query);
        if hit.is_empty() {
            return true;
        }
        if let Some(cv) = hit.get("collider") {
            if let Ok(node) = cv.try_to::<Gd<godot::classes::Node>>() {
                if let Some(ref p) = self.player {
                    return node.instance_id() == p.instance_id();
                }
            }
        }
        false
    }
}

#[godot_api]
impl ICharacterBody3D for Enemy {
    fn init(base: Base<CharacterBody3D>) -> Self {
        Self {
            base,
            cfg_id: GString::new(),
            display_name: String::new(),
            hp: 50.0,
            max_hp: 50.0,
            speed: 2.5,
            atk_damage: 10.0,
            atk_range: 1.8,
            atk_cooldown: 1.5,
            chase_range: 8.0,
            patrol_radius: 3.0,
            xp_value: 10.0,
            is_boss: false,
            resist: [0.0; 4],
            behavior: Behavior::Melee,
            tactical_role: TacticalRole::Pursuer,
            state: EState::Patrol,
            atk_timer: 0.0,
            attack_anim: 0.0,
            attack_hit_timer: -1.0,
            patrol_target: Vector3::ZERO,
            patrol_wait: 0.0,
            patrol_counter: 0,
            spawn_pos: Vector3::ZERO,
            nav: None,
            nav_offset: Vector3::ZERO,
            path: Vec::new(),
            path_timer: 0.0,
            los_timer: 0.0,
            los_cached: false,
            alert_timer: 0.0,
            abilities: Vec::new(),
            boss_phases: Vec::new(),
            boss_phase_idx: 0,
            phase_transition: 0.0,
            cast: None,
            charge_timer: 0.0,
            charge_dir: Vector3::ZERO,
            charge_speed: 0.0,
            charge_damage: 0.0,
            charge_hit: false,
            stagger: 0.0,
            pain_chance: 0.0,
            spawn_mult: 1.0,
            weak_point_height: 0.7,
            weak_point_multiplier: 1.6,
            elite_prefix: String::new(),
            lifesteal: 0.0,
            death_blast: None,
            rng: Rng::new(0x0E0E_0E0E),
            shot_reqs: Vec::new(),
            summon_reqs: Vec::new(),
            heal_reqs: Vec::new(),
            phase_reqs: Vec::new(),
            player: None,
            alive: true,
            frozen: false,
            pending_dmg: 0.0,
            pending_status: None,
            attack_status: None,
            statuses: crate::status::StatusSet::new(),
            pending_color: Color::from_rgba(1.0, 1.0, 1.0, 1.0),
            tex_path: ENEMY_TEX_FALLBACK.to_string(),
            sprite: None,
            telegraph_disc: None,
            anim_timer: 0.0,
            anim_frame: 0,
            anim_row: 0,
            anim_state: EnemyAnimState::Idle,
            anim_cfg: EnemyAnimationCfg::default(),
            hurt_flash: 0.0,
            alert_anim: 0.0,
            death_timer: 0.0,
            vis_scale: 1.0,
        }
    }

    fn ready(&mut self) {
        let color = self.pending_color;
        let tex_path = self.tex_path.clone();
        let s = self.vis_scale;
        let px = 0.010 * s;

        let mut sp = Sprite3D::new_alloc();
        sp.set_pixel_size(px);
        sp.set_billboard_mode(BillboardMode::ENABLED);
        sp.set_alpha_cut_mode(AlphaCutMode::DISCARD);
        sp.set_texture_filter(TextureFilter::NEAREST);
        sp.set_position(Vector3::new(0.0, 1.2 * s, 0.0));

        let texture = ResourceLoader::singleton()
            .load(&tex_path)
            .and_then(|resource| resource.try_cast::<Texture2D>().ok())
            .or_else(|| {
                ResourceLoader::singleton()
                    .load(ENEMY_TEX_FALLBACK)
                    .and_then(|resource| resource.try_cast::<Texture2D>().ok())
            });
        if let Some(texture) = texture {
            sp.set_texture(&texture);
            sp.set_region_enabled(true);
            let (x, y, w, h) = IDLE_FRAMES[0];
            sp.set_region_rect(Rect2::new(Vector2::new(x, y), Vector2::new(w, h)));
        }
        sp.set_modulate(color);
        let sp_clone = sp.clone();
        self.base_mut().add_child(&sp);
        self.sprite = Some(sp_clone);

        let mut disc = MeshInstance3D::new_alloc();
        let mut mesh = CylinderMesh::new_gd();
        mesh.set_top_radius(1.25 * s);
        mesh.set_bottom_radius(1.25 * s);
        mesh.set_height(0.025);
        let mut material = StandardMaterial3D::new_gd();
        material.set_albedo(Color::from_rgba(1.0, 0.25, 0.45, 0.72));
        material.set_emission(Color::from_rgba(1.0, 0.1, 0.35, 1.0));
        disc.set_mesh(&mesh);
        disc.set_material_override(&material);
        disc.set_position(Vector3::new(0.0, 0.025, 0.0));
        disc.set_visible(false);
        let disc_clone = disc.clone();
        self.base_mut().add_child(&disc);
        self.telegraph_disc = Some(disc_clone);

        let mut col = CollisionShape3D::new_alloc();
        let mut cap = CapsuleShape3D::new_gd();
        cap.set_radius(0.3 * s);
        cap.set_height(1.6 * s);
        col.set_shape(&cap);
        col.set_position(Vector3::new(0.0, 0.8 * s, 0.0));
        self.base_mut().add_child(&col);

        self.base_mut().add_to_group("enemies");
        self.spawn_pos = self.base().get_global_position();
        self.patrol_target = self.spawn_pos;
    }

    fn physics_process(&mut self, delta: f64) {
        let dt = delta as f32;
        if self.state == EState::Dead {
            self.death_timer = (self.death_timer - dt).max(0.0);
            self.tick_visual_animation(
                EnemyAnimState::Death,
                self.anim_cfg.death_row,
                self.anim_cfg.death_frames,
                self.anim_cfg.death_fps(),
                dt,
                false,
            );
            return;
        }
        if !self.alive || self.frozen {
            return;
        }
        if self.cast.is_none() {
            if let Some(ref mut disc) = self.telegraph_disc {
                disc.set_visible(false);
            }
        }

        // Статусы: DoT-урон, истечение таймеров. Тикают до всего остального,
        // чтобы горение/кровь добивали даже стоящего/оглушённого врага.
        for (dot, dtype) in self.statuses.tick(dt) {
            self.take_dot(dot, dtype);
        }
        if !self.alive || self.state == EState::Dead {
            return;
        }
        let status_tint = self.statuses.tint(); // после тика — истёкшие не подсвечивают

        // флэш урона поверх тинта; вне флэша — тинт активного статуса или базовый цвет
        // (сбрасываем каждый кадр, чтобы тинт снимался по истечении статуса).
        if self.hurt_flash > 0.0 {
            self.hurt_flash -= dt;
            let c = if self.hurt_flash > 0.0 {
                Color::from_rgba(1.0, 0.25, 0.25, 1.0)
            } else {
                status_tint.unwrap_or(self.pending_color)
            };
            if let Some(ref mut sp) = self.sprite {
                sp.set_modulate(c);
            }
        } else {
            let c = status_tint.unwrap_or(self.pending_color);
            if let Some(ref mut sp) = self.sprite {
                sp.set_modulate(c);
            }
        }

        let player_pos = match self.player.as_ref() {
            Some(p) => p.get_global_position(),
            None => return,
        };

        let my_pos = self.base().get_global_position();
        let dist = Vector3::new(player_pos.x - my_pos.x, 0.0, player_pos.z - my_pos.z).length();

        // Зрение: рейкаст троттлится; актуален только когда игрок в радиусе интереса
        self.los_timer -= dt;
        self.path_timer -= dt;
        self.alert_timer -= dt;
        self.alert_anim = (self.alert_anim - dt).max(0.0);
        self.attack_anim = (self.attack_anim - dt).max(0.0);
        if self.los_timer <= 0.0 && dist < self.chase_range * 2.0 {
            self.los_timer = 0.18;
            self.los_cached = self.has_los(player_pos);
        }
        let sees = self.los_cached;
        let to_player =
            Vector3::new(player_pos.x - my_pos.x, 0.0, player_pos.z - my_pos.z).normalized();

        // кулдауны способностей тикают всегда
        for a in self.abilities.iter_mut() {
            a.cd -= dt;
        }

        if self.phase_transition > 0.0 {
            self.phase_transition = (self.phase_transition - dt).max(0.0);
            self.cast = None;
            self.attack_hit_timer = -1.0;
            self.charge_timer = 0.0;
            self.apply_velocity(Vector3::ZERO);
            if let Some(ref mut sprite) = self.sprite {
                let pulse = 0.72 + (self.phase_transition * 14.0).sin().abs() * 0.28;
                sprite.set_modulate(Color::from_rgba(
                    self.pending_color.r * pulse,
                    self.pending_color.g * pulse,
                    self.pending_color.b * pulse,
                    1.0,
                ));
            }
            self.tick_visual_animation(
                EnemyAnimState::Phase,
                self.anim_cfg.cast_row,
                self.anim_cfg.cast_frames,
                self.anim_cfg.cast_fps() * 0.8,
                dt,
                true,
            );
            return;
        }

        // Стаггер (pain) ИЛИ оглушение (stun-статус): стоит без действий
        if self.stagger > 0.0 || self.statuses.stunned() {
            if self.stagger > 0.0 {
                self.stagger -= dt;
            }
            self.cast = None; // оглушение прерывает телеграф
            self.attack_hit_timer = -1.0;
            self.charge_timer = 0.0; // и рывок
            self.apply_velocity(Vector3::ZERO);
            self.tick_visual_animation(
                EnemyAnimState::Pain,
                self.anim_cfg.pain_row,
                self.anim_cfg.pain_frames,
                self.anim_cfg.pain_fps(),
                dt,
                false,
            );
            return;
        }

        if self.attack_hit_timer >= 0.0 {
            self.attack_hit_timer -= dt;
            if self.attack_hit_timer <= 0.0 {
                self.attack_hit_timer = -1.0;
                if dist <= self.atk_range * 1.2 && (dist <= 3.0 || self.has_los(player_pos)) {
                    self.pending_dmg += self.atk_damage;
                    if self.lifesteal > 0.0 {
                        self.hp = (self.hp + self.atk_damage * self.lifesteal).min(self.max_hp);
                    }
                    self.roll_attack_status();
                }
            }
        }

        // Рывок (charge): несёмся по прямой; контакт наносит урон один раз
        if self.charge_timer > 0.0 {
            self.charge_timer -= dt;
            if !self.charge_hit && dist < 1.5 {
                self.pending_dmg += self.charge_damage;
                if self.lifesteal > 0.0 {
                    self.hp = (self.hp + self.charge_damage * self.lifesteal).min(self.max_hp);
                }
                self.roll_attack_status();
                self.charge_hit = true;
                self.charge_timer = 0.0;
            }
            let dir = self.charge_dir;
            let spd = self.charge_speed;
            self.apply_velocity(dir * spd);
            self.tick_visual_animation(
                EnemyAnimState::Charge,
                self.anim_cfg.attack_row,
                self.anim_cfg.attack_frames,
                self.anim_cfg.attack_fps() * 1.15,
                dt,
                true,
            );
            return;
        }

        // Телеграф каста: стоим подсвеченными; по истечении — эффект
        if let Some(c) = self.cast.take() {
            let t2 = c.timer - dt;
            if t2 <= 0.0 {
                let col = self.pending_color;
                if let Some(ref mut sp) = self.sprite {
                    sp.set_modulate(col);
                }
                if let Some(ref mut disc) = self.telegraph_disc {
                    disc.set_visible(false);
                }
                self.fire_ability(c.ability, my_pos, player_pos);
                // выйти сразу: charge не должен наложиться на старт нового
                // телеграфа в этом же кадре (рывок «под чужой подсветкой»)
                self.apply_velocity(Vector3::ZERO);
                self.tick_visual_animation(
                    EnemyAnimState::Cast,
                    self.anim_cfg.cast_row,
                    self.anim_cfg.cast_frames,
                    self.anim_cfg.cast_fps(),
                    dt,
                    false,
                );
                return;
            } else {
                let col = c.color;
                if let Some(ref mut sp) = self.sprite {
                    sp.set_modulate(col);
                }
                if let Some(ref mut disc) = self.telegraph_disc {
                    disc.set_visible(true);
                    disc.set_scale(Vector3::new(
                        c.scale * (1.0 + (t2 * 8.0).sin().abs() * 0.2),
                        1.0,
                        c.scale * (1.0 + (t2 * 8.0).sin().abs() * 0.2),
                    ));
                    if let Some(material) = disc.get_material_override() {
                        if let Ok(mut standard) = material.try_cast::<StandardMaterial3D>() {
                            standard.set_albedo(col);
                            standard.set_emission(col);
                        }
                    }
                }
                self.cast = Some(Cast { timer: t2, ..c });
                self.apply_velocity(Vector3::ZERO);
                self.tick_visual_animation(
                    EnemyAnimState::Cast,
                    self.anim_cfg.cast_row,
                    self.anim_cfg.cast_frames,
                    self.anim_cfg.cast_fps(),
                    dt,
                    true,
                );
                return;
            }
        }

        self.state = match self.state {
            EState::Patrol => {
                // агро только по ЗРЕНИЮ — сквозь стены не видит (слух — через alert())
                if dist < self.chase_range && sees {
                    EState::Chase
                } else {
                    EState::Patrol
                }
            }
            EState::Chase => {
                if dist < self.atk_range {
                    self.atk_timer = self.atk_cooldown * 0.5;
                    EState::Attack
                }
                // поводок не отпускает, пока действует тревога от шума/урона
                else if dist > self.chase_range * 1.8 && self.alert_timer <= 0.0 {
                    EState::Patrol
                } else {
                    EState::Chase
                }
            }
            EState::Attack => {
                if dist > self.atk_range * 1.4 {
                    EState::Chase
                } else {
                    EState::Attack
                }
            }
            EState::Dead => EState::Dead,
        };

        // Старт каста: видим игрока, свободны — первая готовая способность в диапазоне
        if sees
            && self.charge_timer <= 0.0
            && self.attack_hit_timer < 0.0
            && matches!(self.state, EState::Chase | EState::Attack)
        {
            let ready = self
                .abilities
                .iter()
                .position(|a| a.cd <= 0.0 && dist >= a.cfg.min_range && dist <= a.cfg.max_range);
            if let Some(idx) = ready {
                let cfg = &self.abilities[idx].cfg;
                let color = cfg
                    .color
                    .map(|c| Color::from_rgba(c[0], c[1], c[2], 1.0))
                    .unwrap_or(Color::from_rgba(1.0, 0.85, 0.3, 1.0));
                let timer = cfg.telegraph.max(0.05);
                let scale = self.ability_telegraph_scale(cfg);
                self.abilities[idx].cd = cfg.cooldown;
                self.cast = Some(Cast {
                    ability: idx,
                    timer,
                    color,
                    scale,
                });
                if let Some(ref mut sp) = self.sprite {
                    sp.set_modulate(color);
                }
                if let Some(ref mut disc) = self.telegraph_disc {
                    disc.set_visible(true);
                    disc.set_scale(Vector3::new(scale, 1.0, scale));
                }
                self.apply_velocity(Vector3::ZERO);
                return;
            }
        }

        // Замедление (slow-статус) масштабирует всю скорость передвижения
        let eff_speed = self.speed * self.statuses.slow_mult();

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
                        let angle =
                            (self.patrol_counter as f32 * 2.399) % (2.0 * std::f32::consts::PI);
                        let r = self.patrol_radius * 0.4
                            + (self.patrol_counter % 5) as f32 * self.patrol_radius * 0.12;
                        self.patrol_target = Vector3::new(
                            self.spawn_pos.x + angle.cos() * r,
                            0.0,
                            self.spawn_pos.z + angle.sin() * r,
                        );
                        self.patrol_counter += 1;
                    } else {
                        vel = flat.normalized() * eff_speed * 0.55;
                    }
                }
            }
            EState::Chase => {
                // (отступление ranged живёт в Attack: Chase работает при dist ≥ atk_range)
                let dir = if sees || self.nav.is_none() {
                    // видит (или мир без сетки) — напрямик
                    self.path.clear();
                    self.path_timer = 0.0; // потеряет из виду — путь строится сразу
                    to_player
                } else {
                    // не видит: A* по данжу до игрока
                    self.chase_dir(my_pos, player_pos).unwrap_or(to_player)
                };
                vel = dir * eff_speed;
            }
            EState::Attack => {
                self.atk_timer -= dt;
                if self.atk_timer <= 0.0 {
                    self.atk_timer = self.atk_cooldown * (0.88 + self.rng.f32() * 0.24);
                    // атака в упор проходит всегда; издали — только при видимости
                    if dist <= 3.0 || self.has_los(player_pos) {
                        self.attack_anim = Self::animation_duration(
                            self.anim_cfg.attack_frames,
                            self.anim_cfg.attack_fps(),
                        )
                        .min(self.atk_cooldown * 0.75)
                        .clamp(0.2, 1.0);
                        self.attack_hit_timer =
                            self.attack_anim * self.anim_cfg.attack_hit_ratio.clamp(0.1, 0.9);
                    }
                }
                // melee дожимает вплотную; ranged отходит, если игрок налез
                vel = if self.behavior == Behavior::Ranged
                    && dist < self.atk_range * self.tactical_role.retreat_ratio()
                {
                    -to_player * eff_speed * 0.6
                } else {
                    to_player * 0.15
                };
            }
            EState::Dead => return,
        }

        // сепарация: в движении стая расходится, а не строится в колонну
        if self.state == EState::Chase || self.state == EState::Attack {
            let push = self.separation(my_pos);
            if push.length_squared() > 0.001 {
                vel += push * eff_speed * 0.45;
                let l = vel.length();
                if l > eff_speed {
                    vel = vel / l * eff_speed;
                }
            }
        }

        let mut full_vel = vel;
        if !self.base().is_on_floor() {
            full_vel.y = -9.8;
        }
        self.base_mut().set_velocity(full_vel);
        self.base_mut().move_and_slide();

        // анимация
        let is_moving = vel.length_squared() > 0.1;
        let (anim_state, row, frame_count, fps, looping) = if self.hurt_flash > 0.0 {
            (
                EnemyAnimState::Pain,
                self.anim_cfg.pain_row,
                self.anim_cfg.pain_frames,
                self.anim_cfg.pain_fps(),
                false,
            )
        } else if self.alert_anim > 0.0 {
            (
                EnemyAnimState::Alert,
                self.anim_cfg.alert_row,
                self.anim_cfg.alert_frames,
                self.anim_cfg.alert_fps(),
                false,
            )
        } else if self.attack_anim > 0.0 {
            (
                EnemyAnimState::Attack,
                self.anim_cfg.attack_row,
                self.anim_cfg.attack_frames,
                self.anim_cfg.attack_fps(),
                false,
            )
        } else if is_moving {
            let side = to_player.cross(vel.normalized()).y;
            if side > 0.25 {
                (
                    EnemyAnimState::Move,
                    self.anim_cfg.move_left_row,
                    self.anim_cfg.move_frames,
                    self.anim_cfg.action_fps.max(0.1),
                    true,
                )
            } else if side < -0.25 {
                (
                    EnemyAnimState::Move,
                    self.anim_cfg.move_right_row,
                    self.anim_cfg.move_frames,
                    self.anim_cfg.action_fps.max(0.1),
                    true,
                )
            } else {
                (
                    EnemyAnimState::Move,
                    self.anim_cfg.move_row,
                    self.anim_cfg.move_frames,
                    self.anim_cfg.action_fps.max(0.1),
                    true,
                )
            }
        } else {
            (
                EnemyAnimState::Idle,
                self.anim_cfg.idle_row,
                self.anim_cfg.idle_frames,
                self.anim_cfg.idle_fps.max(0.1),
                true,
            )
        };
        self.tick_visual_animation(anim_state, row, frame_count, fps, dt, looping);
    }
}
