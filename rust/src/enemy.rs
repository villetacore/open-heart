//! Enemy — CharacterBody3D с AI: патруль → преследование → атака.
//! Визуал: Sprite3D billboard с анимацией (DOOM-стиль).
//!
//! Обнаружение: зрение (радиус + прямая видимость — сквозь стены не агрятся),
//! слух (Game3D будит врагов вокруг выстрела/взрыва через alert()).
//! В данже преследование идёт по A*-пути (nav.rs), в мире — напрямик.
//! behavior "ranged" держит дистанцию; "melee" идёт в контакт.

use std::sync::Arc;

use godot::prelude::*;
use godot::classes::{
    CapsuleShape3D, CharacterBody3D, CollisionShape3D,
    ICharacterBody3D, Image, ImageTexture, Node3D, PhysicsRayQueryParameters3D,
    Sprite3D, Texture2D,
};
use godot::classes::base_material_3d::{BillboardMode, TextureFilter};
use godot::classes::sprite_base_3d::AlphaCutMode;

use crate::config::{AbilityCfg, AffixCfg};
use crate::gfx::Rng;
use crate::nav::NavGrid;
use crate::weapon::DmgType;

/// Запрос вражеского выстрела — Game3D собирает их и спавнит снаряды.
pub struct ShotReq {
    pub origin:   Vector3,
    pub dir:      Vector3,
    pub speed:    f32,
    pub damage:   f32,
    pub count:    u32,
    pub spread:   f32,
    pub color:    Color,
}

/// Запрос призыва миньонов (kind × count рядом с кастером, mult кастера).
pub struct SummonReq {
    pub kind:  String,
    pub count: u32,
    pub pos:   Vector3,
    pub mult:  f32,
}

/// Запрос лечения союзников в радиусе.
pub struct HealReq {
    pub pos:    Vector3,
    pub amount: f32,
    pub radius: f32,
}

/// Текущий каст: способность + оставшийся телеграф + цвет подсветки.
struct Cast {
    ability: usize,   // индекс в self.abilities
    timer:   f32,
    color:   Color,
}

/// Способность в рантайме: конфиг + кулдаун.
struct AbilityRt {
    cfg: AbilityCfg,
    cd:  f32,
}

// ── Спрайтшит 512×256: 4 кадра по 128×256 (idle×2, walk×2) ───────────────────

const IDLE_FRAMES: [(f32, f32, f32, f32); 2] = [
    (0.0,   0.0, 128.0, 256.0),
    (128.0, 0.0, 128.0, 256.0),
];
const WALK_FRAMES: [(f32, f32, f32, f32); 2] = [
    (256.0, 0.0, 128.0, 256.0),
    (384.0, 0.0, 128.0, 256.0),
];

/// Путь спрайт-листа по имени (без префикса пути): "grunt" → enemy_grunt.png.
fn enemy_tex(sprite: &str) -> String {
    format!("res://assets/sprites/characters/enemy_{}.png", sprite)
}
const ENEMY_TEX_FALLBACK: &str = "res://assets/sprites/characters/enemy_grunt.png";

#[derive(PartialEq, Clone, Copy)]
enum EState { Patrol, Chase, Attack, Dead }

/// Боевое поведение (enemies.json: "behavior", по умолчанию melee).
#[derive(PartialEq, Clone, Copy)]
pub enum Behavior { Melee, Ranged }

impl Behavior {
    pub fn from_id(s: &str) -> Self {
        if s == "ranged" { Self::Ranged } else { Self::Melee }
    }
}

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
    pub xp_value:     f32,
    pub is_boss:      bool,
    resist:           [f32; 4],   // резисты по DmgType::idx
    vis_scale:        f32,        // масштаб спрайта/коллайдера
    behavior:         Behavior,

    // runtime
    state:            EState,
    atk_timer:        f32,
    patrol_target:    Vector3,
    patrol_wait:      f32,
    patrol_counter:   u32,
    spawn_pos:        Vector3,

    // навигация (только в данже)
    nav:              Option<Arc<NavGrid>>,
    nav_offset:       Vector3,          // DUNGEON_OFFSET: мир → локальные координаты
    path:             Vec<(i32, i32)>,  // клетки до игрока (первая — ближайшая)
    path_timer:       f32,              // перерасчёт пути
    los_timer:        f32,              // троттлинг рейкаста зрения
    los_cached:       bool,
    alert_timer:      f32,              // тревога от шума: пока > 0, поводок не отпускает

    // способности (abilities.json)
    abilities:        Vec<AbilityRt>,
    cast:             Option<Cast>,     // активный телеграф
    charge_timer:     f32,              // остаток рывка
    charge_dir:       Vector3,
    charge_speed:     f32,
    charge_damage:    f32,
    charge_hit:       bool,             // урон рывка нанесён (один раз за рывок)
    stagger:          f32,              // >0 — оглушён (pain)
    pain_chance:      f32,
    spawn_mult:       f32,              // множитель силы (наследуют миньоны)

    // элита (аффиксы)
    elite_prefix:     String,           // «Быстрый Вампирический» (для HUD)
    lifesteal:        f32,              // доля урона игроку → своё HP
    pub death_blast:  Option<(f32, f32)>,   // (урон, радиус) при смерти
    rng:              Rng,
    pub shot_reqs:    Vec<ShotReq>,
    pub summon_reqs:  Vec<SummonReq>,
    pub heal_reqs:    Vec<HealReq>,

    player:           Option<Gd<CharacterBody3D>>,
    pub alive:        bool,
    /// Мир «на паузе» (игрок в меню/диалоге): AI не двигается и не атакует.
    pub frozen:       bool,
    pub pending_dmg:  f32,

    // визуал
    pending_color:    Color,
    tex_path:         String,
    sprite:           Option<Gd<Sprite3D>>,
    anim_timer:       f32,
    anim_frame:       usize,
    hurt_flash:       f32,
}

impl Enemy {
    #[allow(clippy::too_many_arguments)]
    pub fn configure(
        &mut self,
        id: &str, hp: f32, speed: f32, damage: f32,
        atk_range: f32, cooldown: f32, chase: f32, patrol: f32,
        color: Color, spawn: Vector3, xp: f32, mult: f32, is_boss: bool,
        resist: [f32; 4], sprite: &str, scale: f32, behavior: Behavior,
    ) {
        self.behavior = behavior;
        self.cfg_id        = GString::from(id);
        self.hp            = hp * mult;
        self.max_hp        = hp * mult;
        self.speed         = speed;
        self.atk_damage    = damage * mult;
        self.atk_range     = atk_range;
        self.atk_cooldown  = cooldown;
        self.chase_range   = chase;
        self.patrol_radius = patrol;
        self.spawn_pos     = spawn;
        self.patrol_target = spawn;
        self.alive         = true;
        self.pending_color = color;
        self.tex_path      = enemy_tex(if sprite.is_empty() { id } else { sprite });
        self.xp_value      = xp * mult;
        self.is_boss       = is_boss;
        self.resist        = resist;
        self.vis_scale     = if is_boss { scale.max(1.35) } else { scale.max(0.5) };
        self.spawn_mult    = mult;
    }

    /// Применить аффиксы элиты: мультипликаторы статов, tint, спец-механики.
    /// Вызывать ПОСЛЕ configure и set_combat_extras (модифицирует pain_chance).
    pub fn apply_affixes(&mut self, affixes: &[AffixCfg]) {
        for a in affixes {
            self.hp          *= a.hp_mult;
            self.max_hp      *= a.hp_mult;
            self.atk_damage  *= a.dmg_mult;
            self.speed       *= a.speed_mult;
            self.xp_value    *= a.xp_mult;
            self.pain_chance *= a.pain_mult;
            self.lifesteal   += a.lifesteal;
            if let Some([dmg, r]) = a.death_blast {
                // урон взрыва масштабируется силой врага, как и остальной урон
                self.death_blast = Some((dmg * self.spawn_mult, r));
            }
            if let Some(t) = a.tint {
                // подмешиваем цвет аффикса — элиту видно издалека
                let tint = Color::from_rgba(t[0], t[1], t[2], 1.0);
                self.pending_color = self.pending_color.lerp(tint, 0.6);
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
            self.cfg_id.to_string()
        } else {
            format!("{} {}", self.elite_prefix, self.cfg_id)
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
        self.pain_chance = pain_chance;
        self.abilities = abilities.into_iter().map(|cfg| {
            // стартовый кулдаун случайный — толпа не кастует синхронно
            let cd = cfg.cooldown * (0.3 + self.rng.f32() * 0.7);
            AbilityRt { cfg, cd }
        }).collect();
    }

    /// Урон с учётом типа и резиста. Возвращает фактически нанесённый урон.
    pub fn take_damage(&mut self, amount: f32, dmg_type: DmgType) -> f32 {
        if !self.alive { return 0.0; }
        let dealt = (amount * (1.0 - self.resist[dmg_type.idx()])).max(0.0);
        self.hp -= dealt;
        self.hurt_flash = 0.15;
        // проснуться при уроне (и не отпускать поводок, даже если игрок далеко)
        self.alert_timer = 6.0;
        if self.state == EState::Patrol { self.state = EState::Chase; }
        // pain: шанс стаггера — прерывает телеграф и рывок (контр-игра против кастеров)
        if self.pain_chance > 0.0 && self.rng.f32() < self.pain_chance {
            self.stagger = 0.4;
            self.cast = None;
            self.charge_timer = 0.0;
        }
        if self.hp <= 0.0 {
            self.hp    = 0.0;
            self.state = EState::Dead;
            self.alive = false;
        }
        dealt
    }

    pub fn set_player(&mut self, player: Gd<CharacterBody3D>) {
        self.player = Some(player);
    }

    /// Навигационная сетка данжа (offset — позиция корня данжа в мире).
    pub fn set_nav(&mut self, nav: Arc<NavGrid>, offset: Vector3) {
        self.nav = Some(nav);
        self.nav_offset = offset;
    }

    /// Разбудить (шум выстрела/взрыва, тревога от соседа): патруль → погоня.
    /// Таймер тревоги не даёт поводку (chase_range×1.8) сразу отпустить врага,
    /// который услышал шум издалека.
    pub fn alert(&mut self) {
        if !self.alive { return; }
        self.alert_timer = 6.0;
        if self.state == EState::Patrol {
            self.state = EState::Chase;
        }
    }

    /// Отталкивание от других врагов рядом — стая не слипается в колонну.
    fn separation(&self, my_pos: Vector3) -> Vector3 {
        let tree = self.base().get_tree();
        let my_id = self.base().instance_id();
        let mut push = Vector3::ZERO;
        for node in tree.get_nodes_in_group("enemies").iter_shared() {
            if node.instance_id() == my_id { continue; }
            let Ok(other) = node.try_cast::<Node3D>() else { continue };
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
        if !self.base().is_on_floor() { fv.y = -9.8; }
        self.base_mut().set_velocity(fv);
        self.base_mut().move_and_slide();
    }

    /// Эффект способности по окончании телеграфа.
    fn fire_ability(&mut self, idx: usize, my_pos: Vector3, player_pos: Vector3) {
        let Some(rt) = self.abilities.get(idx) else { return };
        let cfg = rt.cfg.clone();
        let flat = Vector3::new(player_pos.x - my_pos.x, 0.0, player_pos.z - my_pos.z);
        let dir = if flat.length() > 0.01 { flat.normalized() } else { Vector3::new(0.0, 0.0, -1.0) };
        let color = cfg.color
            .map(|c| Color::from_rgba(c[0], c[1], c[2], 1.0))
            .unwrap_or(Color::from_rgba(1.0, 0.5, 0.8, 1.0));
        match cfg.kind.as_str() {
            "projectile_burst" => {
                let origin = my_pos + Vector3::new(0.0, 1.15 * self.vis_scale, 0.0) + dir * 0.6;
                let aim = (player_pos + Vector3::new(0.0, 1.0, 0.0) - origin).normalized();
                self.shot_reqs.push(ShotReq {
                    origin,
                    dir: aim,
                    speed:  cfg.proj_speed.max(4.0),
                    damage: cfg.damage * self.spawn_mult,
                    count:  cfg.count.max(1),
                    spread: cfg.spread,
                    color,
                });
            }
            "charge" => {
                self.charge_dir    = dir;
                self.charge_speed  = self.speed * cfg.speed_mult.max(1.2);
                self.charge_damage = cfg.damage * self.spawn_mult;
                self.charge_timer  = cfg.duration.max(0.2);
                self.charge_hit    = false;
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
                    pos: my_pos, amount: cfg.heal, radius: cfg.radius.max(1.0),
                });
            }
            other => {
                godot_warn!("[ability] '{}': неизвестный kind '{other}'", cfg.id);
            }
        }
    }

    /// Забрать накопленные запросы способностей (Game3D, раз в кадр).
    pub fn drain_requests(&mut self) -> (Vec<ShotReq>, Vec<SummonReq>, Vec<HealReq>) {
        (
            std::mem::take(&mut self.shot_reqs),
            std::mem::take(&mut self.summon_reqs),
            std::mem::take(&mut self.heal_reqs),
        )
    }

    /// Лечение от heal_pulse союзника.
    pub fn heal_hp(&mut self, amount: f32) {
        if self.alive {
            self.hp = (self.hp + amount).min(self.max_hp);
        }
    }

    /// Есть ли прямая видимость до игрока (для дальнобойных атак).
    fn has_los(&mut self, player_pos: Vector3) -> bool {
        let from = self.base().get_global_position() + Vector3::new(0.0, 1.3, 0.0);
        let to   = player_pos + Vector3::new(0.0, 1.3, 0.0);
        let Some(world) = self.base().get_world_3d() else { return true };
        let Some(mut space) = world.clone().get_direct_space_state() else { return true };
        let Some(mut query) = PhysicsRayQueryParameters3D::create(from, to) else { return true };
        let mut excl: godot::builtin::Array<Rid> = godot::builtin::Array::new();
        excl.push(self.base().get_rid());
        query.set_exclude(&excl);
        let hit = space.intersect_ray(&query);
        if hit.is_empty() { return true; }
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
            hp: 50.0, max_hp: 50.0,
            speed: 2.5, atk_damage: 10.0,
            atk_range: 1.8, atk_cooldown: 1.5,
            chase_range: 8.0, patrol_radius: 3.0,
            xp_value: 10.0, is_boss: false,
            resist: [0.0; 4],
            behavior: Behavior::Melee,
            state: EState::Patrol,
            atk_timer: 0.0,
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
            cast: None,
            charge_timer: 0.0,
            charge_dir: Vector3::ZERO,
            charge_speed: 0.0,
            charge_damage: 0.0,
            charge_hit: false,
            stagger: 0.0,
            pain_chance: 0.0,
            spawn_mult: 1.0,
            elite_prefix: String::new(),
            lifesteal: 0.0,
            death_blast: None,
            rng: Rng::new(0x0E0E_0E0E),
            shot_reqs: Vec::new(),
            summon_reqs: Vec::new(),
            heal_reqs: Vec::new(),
            player: None,
            alive: true,
            frozen: false,
            pending_dmg: 0.0,
            pending_color: Color::from_rgba(1.0, 1.0, 1.0, 1.0),
            tex_path: ENEMY_TEX_FALLBACK.to_string(),
            sprite: None,
            anim_timer: 0.0,
            anim_frame: 0,
            hurt_flash: 0.0,
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

        let img = Image::load_from_file(&tex_path)
            .or_else(|| Image::load_from_file(ENEMY_TEX_FALLBACK));
        if let Some(img) = img {
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

        let mut col = CollisionShape3D::new_alloc();
        let mut cap = CapsuleShape3D::new_gd();
        cap.set_radius(0.3 * s);
        cap.set_height(1.6 * s);
        col.set_shape(&cap);
        col.set_position(Vector3::new(0.0, 0.8 * s, 0.0));
        self.base_mut().add_child(&col);

        self.base_mut().add_to_group("enemies");
        self.spawn_pos     = self.base().get_global_position();
        self.patrol_target = self.spawn_pos;
    }

    fn physics_process(&mut self, delta: f64) {
        if !self.alive || self.frozen || self.state == EState::Dead { return; }
        let dt = delta as f32;

        // флэш урона
        if self.hurt_flash > 0.0 {
            self.hurt_flash -= dt;
            let c = if self.hurt_flash > 0.0 {
                Color::from_rgba(1.0, 0.25, 0.25, 1.0)
            } else {
                self.pending_color
            };
            if let Some(ref mut sp) = self.sprite { sp.set_modulate(c); }
        }

        let player_pos = match self.player.as_ref() {
            Some(p) => p.get_global_position(),
            None    => return,
        };

        let my_pos = self.base().get_global_position();
        let dist   = Vector3::new(player_pos.x - my_pos.x, 0.0, player_pos.z - my_pos.z).length();

        // Зрение: рейкаст троттлится; актуален только когда игрок в радиусе интереса
        self.los_timer -= dt;
        self.path_timer -= dt;
        self.alert_timer -= dt;
        if self.los_timer <= 0.0 && dist < self.chase_range * 2.0 {
            self.los_timer = 0.18;
            self.los_cached = self.has_los(player_pos);
        }
        let sees = self.los_cached;
        let to_player = Vector3::new(
            player_pos.x - my_pos.x, 0.0, player_pos.z - my_pos.z,
        ).normalized();

        // кулдауны способностей тикают всегда
        for a in self.abilities.iter_mut() { a.cd -= dt; }

        // Стаггер (pain): оглушён — стоит, каст/рывок уже сброшены в take_damage
        if self.stagger > 0.0 {
            self.stagger -= dt;
            self.apply_velocity(Vector3::ZERO);
            return;
        }

        // Рывок (charge): несёмся по прямой; контакт наносит урон один раз
        if self.charge_timer > 0.0 {
            self.charge_timer -= dt;
            if !self.charge_hit && dist < 1.5 {
                self.pending_dmg += self.charge_damage;
                if self.lifesteal > 0.0 {
                    self.hp = (self.hp + self.charge_damage * self.lifesteal).min(self.max_hp);
                }
                self.charge_hit = true;
                self.charge_timer = 0.0;
            }
            let dir = self.charge_dir;
            let spd = self.charge_speed;
            self.apply_velocity(dir * spd);
            return;
        }

        // Телеграф каста: стоим подсвеченными; по истечении — эффект
        if let Some(c) = self.cast.take() {
            let t2 = c.timer - dt;
            if t2 <= 0.0 {
                let col = self.pending_color;
                if let Some(ref mut sp) = self.sprite { sp.set_modulate(col); }
                self.fire_ability(c.ability, my_pos, player_pos);
                // выйти сразу: charge не должен наложиться на старт нового
                // телеграфа в этом же кадре (рывок «под чужой подсветкой»)
                self.apply_velocity(Vector3::ZERO);
                return;
            } else {
                let col = c.color;
                if let Some(ref mut sp) = self.sprite { sp.set_modulate(col); }
                self.cast = Some(Cast { timer: t2, ..c });
                self.apply_velocity(Vector3::ZERO);
                return;
            }
        }

        self.state = match self.state {
            EState::Patrol => {
                // агро только по ЗРЕНИЮ — сквозь стены не видит (слух — через alert())
                if dist < self.chase_range && sees { EState::Chase } else { EState::Patrol }
            }
            EState::Chase => {
                if dist < self.atk_range { self.atk_timer = self.atk_cooldown * 0.5; EState::Attack }
                // поводок не отпускает, пока действует тревога от шума/урона
                else if dist > self.chase_range * 1.8 && self.alert_timer <= 0.0 { EState::Patrol }
                else { EState::Chase }
            }
            EState::Attack => {
                if dist > self.atk_range * 1.4 { EState::Chase } else { EState::Attack }
            }
            EState::Dead => EState::Dead,
        };

        // Старт каста: видим игрока, свободны — первая готовая способность в диапазоне
        if sees && self.charge_timer <= 0.0
            && matches!(self.state, EState::Chase | EState::Attack) {
            let ready = self.abilities.iter().position(|a|
                a.cd <= 0.0 && dist >= a.cfg.min_range && dist <= a.cfg.max_range);
            if let Some(idx) = ready {
                let cfg = &self.abilities[idx].cfg;
                let color = cfg.color
                    .map(|c| Color::from_rgba(c[0], c[1], c[2], 1.0))
                    .unwrap_or(Color::from_rgba(1.0, 0.85, 0.3, 1.0));
                let timer = cfg.telegraph.max(0.05);
                self.abilities[idx].cd = cfg.cooldown;
                self.cast = Some(Cast { ability: idx, timer, color });
                if let Some(ref mut sp) = self.sprite { sp.set_modulate(color); }
                self.apply_velocity(Vector3::ZERO);
                return;
            }
        }

        let mut vel = Vector3::ZERO;
        match self.state {
            EState::Patrol => {
                if self.patrol_wait > 0.0 {
                    self.patrol_wait -= dt;
                } else {
                    let flat = Vector3::new(
                        self.patrol_target.x - my_pos.x, 0.0,
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
                // (отступление ranged живёт в Attack: Chase работает при dist ≥ atk_range)
                let dir = if sees || self.nav.is_none() {
                    // видит (или мир без сетки) — напрямик
                    self.path.clear();
                    self.path_timer = 0.0;   // потеряет из виду — путь строится сразу
                    to_player
                } else {
                    // не видит: A* по данжу до игрока
                    self.chase_dir(my_pos, player_pos).unwrap_or(to_player)
                };
                vel = dir * self.speed;
            }
            EState::Attack => {
                self.atk_timer -= dt;
                if self.atk_timer <= 0.0 {
                    self.atk_timer = self.atk_cooldown;
                    // атака в упор проходит всегда; издали — только при видимости
                    if dist <= 3.0 || self.has_los(player_pos) {
                        self.pending_dmg += self.atk_damage;
                        // вампирический аффикс: лечится от нанесённого урона
                        if self.lifesteal > 0.0 {
                            self.hp = (self.hp + self.atk_damage * self.lifesteal).min(self.max_hp);
                        }
                    }
                }
                // melee дожимает вплотную; ranged отходит, если игрок налез
                vel = if self.behavior == Behavior::Ranged && dist < self.atk_range * 0.45 {
                    -to_player * self.speed * 0.6
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
                vel += push * self.speed * 0.45;
                let l = vel.length();
                if l > self.speed { vel = vel / l * self.speed; }
            }
        }

        let mut full_vel = vel;
        if !self.base().is_on_floor() { full_vel.y = -9.8; }
        self.base_mut().set_velocity(full_vel);
        self.base_mut().move_and_slide();

        // анимация
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
