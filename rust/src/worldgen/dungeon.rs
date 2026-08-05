//! Процедурный генератор данжей — Doom/Quake стиль.
//!
//! Комнаты связываются по близости (MST + петли), высоты пола (0 / 0.8)
//! назначаются по дереву и соединяются пологими пандусами (только когда коридор
//! достаточно длинный). Коридоры шириной 2. Пост-проверка достижимости
//! (flood-fill) не даёт спавнить врагов/лут в отрезанных карманах.

use godot::classes::Node3D;
use godot::prelude::*;

use crate::config::{GameConfig, RoomArchetypeCfg};
use crate::gfx::{make_billboard, make_box, make_glow_slab, make_light, make_ramp, Rng, TexCache};
use crate::weapon::{AmmoType, WeaponId};

pub const CELL: f32 = 3.0;
pub const WALL_H: f32 = 3.4; // высота стен по умолчанию
pub const GRID: usize = 44;

// ── Тема данжа (разрешённая из dungeon.json пресета) ─────────────────────────

struct Theme {
    name: String,
    wall: String,
    accent: String,
    floor: String,
    ceil: String,
    lava: String,
    light: Color,
}

/// Короткое имя текстуры → путь (dtile_* → textures/dungeon и т.д.);
/// полные res://-пути пропускаются как есть.
fn resolve_tex(name: &str) -> String {
    if name.starts_with("res://") {
        name.to_string()
    } else {
        crate::map::tex_path(name)
    }
}

impl Theme {
    fn from_cfg(c: &crate::config::ThemeCfg, lang: &str) -> Self {
        Self {
            name: if lang == "en" && !c.name_en.is_empty() {
                c.name_en.clone()
            } else {
                c.name_ru.clone()
            },
            wall: resolve_tex(&c.wall),
            accent: resolve_tex(&c.accent),
            floor: resolve_tex(&c.floor),
            ceil: resolve_tex(&c.ceil),
            lava: resolve_tex(&c.lava),
            light: Color::from_rgba(c.light[0], c.light[1], c.light[2], 1.0),
        }
    }
}

// ── Результат генерации ───────────────────────────────────────────────────────

pub struct EnemySpawn {
    pub kind: String,
    pub pos: Vector3,
    pub mult: f32,
    pub is_boss: bool,
    /// id аффиксов элиты (пусто = обычный враг).
    pub affixes: Vec<String>,
}

pub struct DungeonEventSpawn {
    pub kind: String,
    pub group: u32,
    pub step: u32,
    pub pos: Vector3,
}

#[derive(Clone, Copy)]
pub enum HazardKind {
    Blood,
    Void,
    Electric,
    Embers,
}

#[derive(Clone, Copy, PartialEq)]
pub enum HazardPhase {
    Dormant,
    Warning,
    Active,
}

impl HazardKind {
    pub fn id(self) -> &'static str {
        match self {
            Self::Blood => "blood",
            Self::Void => "void",
            Self::Electric => "electric",
            Self::Embers => "embers",
        }
    }

    fn from_id(id: &str) -> Self {
        match id {
            "void" => Self::Void,
            "electric" => Self::Electric,
            "embers" => Self::Embers,
            _ => Self::Blood,
        }
    }

    pub fn color(self) -> Color {
        match self {
            Self::Blood => Color::from_rgba(0.95, 0.16, 0.36, 1.0),
            Self::Void => Color::from_rgba(0.62, 0.18, 1.0, 1.0),
            Self::Electric => Color::from_rgba(0.1, 0.8, 1.0, 1.0),
            Self::Embers => Color::from_rgba(1.0, 0.34, 0.08, 1.0),
        }
    }

    pub fn phase(self, time: f32) -> HazardPhase {
        let (period, warning, active) = match self {
            Self::Blood => return HazardPhase::Active,
            Self::Void => (2.6, 0.7, 0.7),
            Self::Electric => (1.8, 0.45, 0.25),
            Self::Embers => (1.4, 0.3, 0.9),
        };
        let cycle = time.rem_euclid(period);
        if cycle < active {
            HazardPhase::Active
        } else if cycle >= period - warning {
            HazardPhase::Warning
        } else {
            HazardPhase::Dormant
        }
    }

    pub fn damage_mult(self) -> f32 {
        match self {
            Self::Blood => 0.8,
            Self::Void => 1.8,
            Self::Electric => 3.2,
            Self::Embers => 1.2,
        }
    }

    pub fn status(self) -> &'static str {
        match self {
            Self::Blood => "bleed",
            Self::Void => "weakened",
            Self::Electric => "stun",
            Self::Embers => "burning",
        }
    }

    pub fn warning(self, lang: &str) -> &'static str {
        match (self, lang == "en") {
            (Self::Blood, true) => "BLOOD CORRUPTION — MOVE!",
            (Self::Blood, false) => "КРОВАВАЯ СКВЕРНА — УХОДИ!",
            (Self::Void, true) => "VOID RIFT — MOVE!",
            (Self::Void, false) => "РАЗЛОМ ПУСТОТЫ — УХОДИ!",
            (Self::Electric, true) => "LIVE CURRENT — MOVE!",
            (Self::Electric, false) => "ЭЛЕКТРИЧЕСКИЙ РАЗРЯД — УХОДИ!",
            (Self::Embers, true) => "BURNING FLOOR — MOVE!",
            (Self::Embers, false) => "ГОРЯЩИЙ ПОЛ — УХОДИ!",
        }
    }
}

#[derive(Clone, Copy)]
pub struct HazardZone {
    pub pos: Vector3,
    pub radius: f32,
    pub dps: f32,
    pub kind: HazardKind,
    pub phase_offset: f32,
    pub phase: HazardPhase,
    pub player_inside: bool,
}

pub struct DungeonPlan {
    pub depth: u32,
    pub theme_name: String,
    pub root: Gd<Node3D>,
    pub player_spawn: Vector3,
    pub exit_portal: Vector3,
    pub next_portal: Vector3,
    pub enemies: Vec<EnemySpawn>,
    pub items: Vec<(String, Vector3)>,
    pub ammo: Vec<(AmmoType, u32, Vector3)>,
    pub weapons: Vec<(WeaponId, Vector3)>,
    pub hazards: Vec<HazardZone>,
    pub room_roles: Vec<String>,
    pub room_markers: Vec<(String, Vector3)>,
    pub dressing_nodes: usize,
    pub dressing_variants: Vec<u8>,
    pub events: Vec<DungeonEventSpawn>,
    pub floor_map: Vec<bool>,    // GRID×GRID, true = проходимый пол
    pub floor_heights: Vec<f32>, // GRID×GRID, высота пола клетки (для навигации)
}

// ── Комната ───────────────────────────────────────────────────────────────────

/// Форма комнаты в пределах её bounding-box (разнообразит «простые прямоугольники»).
#[derive(Clone, Copy, PartialEq)]
enum Shape {
    Rect,
    Round,
    Octagon,
    Cross,
}

#[derive(Clone, Copy, PartialEq)]
enum RoomRole {
    Safe,
    Arena,
    Gallery,
    Ritual,
    Treasure,
    Ambush,
    Traversal,
    Puzzle,
    Story,
    Antechamber,
}

impl RoomRole {
    fn id(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Arena => "arena",
            Self::Gallery => "gallery",
            Self::Ritual => "ritual",
            Self::Treasure => "treasure",
            Self::Ambush => "ambush",
            Self::Traversal => "traversal",
            Self::Puzzle => "puzzle",
            Self::Story => "story",
            Self::Antechamber => "antechamber",
        }
    }
}

#[derive(Clone, Copy)]
enum RoomDecor {
    Basic,
    Pipes,
    Ossuary,
    Crystals,
    Machinery,
    Archive,
    Sanctuary,
}

#[derive(Clone, Copy)]
enum FocalPattern {
    Spire,
    Ring,
    Cross,
    Aisle,
    Altar,
    Well,
    Archive,
    Gate,
}

impl FocalPattern {
    fn id(self) -> &'static str {
        match self {
            Self::Spire => "spire",
            Self::Ring => "ring",
            Self::Cross => "cross",
            Self::Aisle => "aisle",
            Self::Altar => "altar",
            Self::Well => "well",
            Self::Archive => "archive",
            Self::Gate => "gate",
        }
    }
}

#[derive(Clone, Copy)]
struct Room {
    x: i32,
    z: i32,
    w: i32,
    h: i32,
    floor_y: f32, // высота пола: 0.0 | 0.8
    wall_h: f32,  // высота стен над полом: 2.4 | 3.4 | 4.8 | 5.5
    shape: Shape,
    role: RoomRole,
    decor: RoomDecor,
    focal_pattern: FocalPattern,
    focal_color: [f32; 3],
    focal_scale: f32,
    focal_height: f32,
    hazard_chance: f32,
    hazard_dps: f32,
    hazard_radius: f32,
    hazard_kind: HazardKind,
    special_event: bool,
}

impl Room {
    fn center(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.z + self.h / 2)
    }
    fn overlaps(&self, o: &Room, pad: i32) -> bool {
        self.x - pad < o.x + o.w
            && self.x + self.w + pad > o.x
            && self.z - pad < o.z + o.h
            && self.z + self.h + pad > o.z
    }

    /// Входит ли клетка (i,j) в форму комнаты. Центр и центральные ось/ряд всегда
    /// внутри — коридоры цепляются к центру, форма не должна их отрезать.
    fn contains(&self, i: i32, j: i32) -> bool {
        if i < self.x || j < self.z || i >= self.x + self.w || j >= self.z + self.h {
            return false;
        }
        let (li, lj) = (i - self.x, j - self.z);
        let (cx, cz) = ((self.w - 1) as f32 * 0.5, (self.h - 1) as f32 * 0.5);
        match self.shape {
            Shape::Rect => true,
            Shape::Round => {
                let (rx, rz) = ((self.w as f32) * 0.5, (self.h as f32) * 0.5);
                let dx = (li as f32 - cx) / rx;
                let dz = (lj as f32 - cz) / rz;
                dx * dx + dz * dz <= 1.05
            }
            Shape::Octagon => {
                // срез углов треугольниками размера c
                let c = (self.w.min(self.h)) / 3;
                let far_i = self.w - 1 - li;
                let far_j = self.h - 1 - lj;
                (li + lj) >= c && (far_i + lj) >= c && (li + far_j) >= c && (far_i + far_j) >= c
            }
            Shape::Cross => {
                let in_col = li >= self.w / 3 && li < self.w - self.w / 3;
                let in_row = lj >= self.h / 3 && lj < self.h - self.h / 3;
                in_col || in_row
            }
        }
    }
}

fn shape_from_id(id: &str) -> Shape {
    match id {
        "round" => Shape::Round,
        "octagon" => Shape::Octagon,
        "cross" => Shape::Cross,
        _ => Shape::Rect,
    }
}

fn role_from_id(id: &str) -> RoomRole {
    match id {
        "safe" => RoomRole::Safe,
        "gallery" => RoomRole::Gallery,
        "ritual" => RoomRole::Ritual,
        "treasure" => RoomRole::Treasure,
        "ambush" => RoomRole::Ambush,
        "traversal" => RoomRole::Traversal,
        "puzzle" => RoomRole::Puzzle,
        "story" => RoomRole::Story,
        "antechamber" => RoomRole::Antechamber,
        _ => RoomRole::Arena,
    }
}

fn decor_from_id(id: &str) -> RoomDecor {
    match id {
        "pipes" => RoomDecor::Pipes,
        "ossuary" => RoomDecor::Ossuary,
        "crystals" => RoomDecor::Crystals,
        "machinery" => RoomDecor::Machinery,
        "archive" => RoomDecor::Archive,
        "sanctuary" => RoomDecor::Sanctuary,
        _ => RoomDecor::Basic,
    }
}

fn focal_from_id(id: &str) -> FocalPattern {
    match id {
        "ring" => FocalPattern::Ring,
        "cross" => FocalPattern::Cross,
        "aisle" => FocalPattern::Aisle,
        "altar" => FocalPattern::Altar,
        "well" => FocalPattern::Well,
        "archive" => FocalPattern::Archive,
        "gate" => FocalPattern::Gate,
        _ => FocalPattern::Spire,
    }
}

fn pick_room_archetype<'a>(
    rng: &mut Rng,
    archetypes: &'a [RoomArchetypeCfg],
    avoid_role: Option<RoomRole>,
) -> Option<&'a RoomArchetypeCfg> {
    if archetypes.is_empty() {
        return None;
    }
    let has_alternative = avoid_role.is_some_and(|role| {
        archetypes
            .iter()
            .any(|candidate| role_from_id(&candidate.role) != role)
    });
    let eligible = |candidate: &&RoomArchetypeCfg| {
        !has_alternative || avoid_role.is_none_or(|role| role_from_id(&candidate.role) != role)
    };
    let total: u32 = archetypes
        .iter()
        .filter(eligible)
        .map(|candidate| candidate.weight.max(1))
        .sum();
    let mut roll = rng.below(total);
    for candidate in archetypes.iter().filter(eligible) {
        let weight = candidate.weight.max(1);
        if roll < weight {
            return Some(candidate);
        }
        roll -= weight;
    }
    archetypes.first()
}

fn apply_archetype(room: &mut Room, profile: &RoomArchetypeCfg, special_event: bool) {
    room.shape = shape_from_id(&profile.shape);
    room.role = role_from_id(&profile.role);
    room.decor = decor_from_id(&profile.decor);
    room.focal_pattern = focal_from_id(&profile.focal_pattern);
    room.focal_color = profile.focal_color;
    room.focal_scale = profile.focal_scale;
    room.focal_height = profile.focal_height;
    room.wall_h = profile.wall_height.unwrap_or(room.wall_h);
    room.hazard_chance = profile.hazard_chance;
    room.hazard_dps = profile.hazard_dps;
    room.hazard_radius = profile.hazard_radius;
    room.hazard_kind = HazardKind::from_id(&profile.hazard_kind);
    room.special_event = special_event;
}

fn cell_at(i: i32, j: i32, y: f32) -> Vector3 {
    Vector3::new(
        (i as f32 + 0.5 - GRID as f32 * 0.5) * CELL,
        y,
        (j as f32 + 0.5 - GRID as f32 * 0.5) * CELL,
    )
}

fn flood_walkable(floor: &[bool], heights: &[f32], start: (i32, i32)) -> Vec<bool> {
    let mut reachable = vec![false; GRID * GRID];
    let (start_x, start_z) = start;
    if start_x < 0 || start_z < 0 || start_x as usize >= GRID || start_z as usize >= GRID {
        return reachable;
    }
    let start_index = start_z as usize * GRID + start_x as usize;
    if !floor.get(start_index).copied().unwrap_or(false) {
        return reachable;
    }

    reachable[start_index] = true;
    let mut stack = vec![start_index];
    while let Some(index) = stack.pop() {
        let (cell_x, cell_z) = ((index % GRID) as i32, (index / GRID) as i32);
        for (offset_x, offset_z) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (next_x, next_z) = (cell_x + offset_x, cell_z + offset_z);
            if next_x < 0 || next_z < 0 || next_x as usize >= GRID || next_z as usize >= GRID {
                continue;
            }
            let next_index = next_z as usize * GRID + next_x as usize;
            if reachable[next_index] || !floor[next_index] {
                continue;
            }
            if (heights[next_index] - heights[index]).abs() > crate::nav::RAMP_STEP {
                continue;
            }
            reachable[next_index] = true;
            stack.push(next_index);
        }
    }
    reachable
}

// ── Вспомогательные функции carve ────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn carve_cell(
    floor: &mut [bool],
    fh: &mut [f32],
    cwh: &mut [f32],
    is_ramp: &mut [bool],
    is_room: &[bool],
    i: i32,
    j: i32,
    h: f32,
    wh: f32,
    ramp: bool,
) {
    if i < 1 || j < 1 || i >= GRID as i32 - 1 || j >= GRID as i32 - 1 {
        return;
    }
    let idx = j as usize * GRID + i as usize;
    floor[idx] = true;
    // Комнаты не перезаписываем (у них своя высота и это не пандус).
    if !is_room[idx] {
        fh[idx] = h;
        cwh[idx] = wh;
        if ramp {
            is_ramp[idx] = true;
        }
    }
}

// ── Генерация ─────────────────────────────────────────────────────────────────

pub fn generate(
    depth: u32,
    seed: u64,
    cache: &mut TexCache,
    cfg: &GameConfig,
    lang: &str,
) -> DungeonPlan {
    let mut rng = Rng::new(seed ^ (depth as u64).wrapping_mul(0x9E3779B97F4A7C15));
    // темы/пулы/настройки — из dungeon.json пресета (config гарантирует непустоту)
    let dc = &cfg.dungeon;
    let theme = Theme::from_cfg(&dc.themes[((depth - 1) as usize) % dc.themes.len()], lang);

    // Уровни высот пола: только 0.0 и 0.8 — перепад, который пандус (>=3 клетки)
    // проходит пологим склоном (<= RAMP_STEP на клетку). 1.6 убран как источник
    // непроходимых обрывов.
    const CEIL_LEVELS: [f32; 5] = [3.4, 3.4, 4.8, 2.4, 3.4];

    // 1. Комнаты (высоту назначим позже — по дереву связей)
    let mut rooms: Vec<Room> = Vec::new();
    let target = 8 + depth.min(3) as i32 + rng.range(0, 2);
    for _ in 0..120 {
        if rooms.len() as i32 >= target {
            break;
        }
        let archetype = pick_room_archetype(
            &mut rng,
            &dc.room_archetypes,
            rooms.last().map(|room| room.role),
        );
        let (w, h) = if let Some(profile) = archetype {
            let min_size = profile.min_size.clamp(4, 10) as i32;
            let max_size = profile.max_size.max(profile.min_size).clamp(4, 11) as i32;
            (
                rng.range(min_size, max_size + 1),
                rng.range(min_size, max_size + 1),
            )
        } else {
            (rng.range(4, 9), rng.range(4, 9))
        };
        let x = rng.range(2, GRID as i32 - w - 3);
        let z = rng.range(2, GRID as i32 - h - 3);
        let wall_h = archetype
            .and_then(|profile| profile.wall_height)
            .unwrap_or(CEIL_LEVELS[rng.below(5) as usize]);
        // Форма: крупные комнаты разнообразим, мелкие оставляем прямоугольными
        let shape = if let Some(profile) = archetype {
            shape_from_id(&profile.shape)
        } else if w >= 6 && h >= 6 {
            match rng.below(5) {
                0 => Shape::Round,
                1 => Shape::Octagon,
                2 => Shape::Cross,
                _ => Shape::Rect,
            }
        } else {
            Shape::Rect
        };
        let r = Room {
            x,
            z,
            w,
            h,
            floor_y: 0.0,
            wall_h,
            shape,
            role: archetype
                .map(|profile| role_from_id(&profile.role))
                .unwrap_or(RoomRole::Arena),
            decor: archetype
                .map(|profile| decor_from_id(&profile.decor))
                .unwrap_or(RoomDecor::Basic),
            focal_pattern: archetype
                .map(|profile| focal_from_id(&profile.focal_pattern))
                .unwrap_or(FocalPattern::Spire),
            focal_color: archetype
                .map(|profile| profile.focal_color)
                .unwrap_or([0.8, 0.3, 0.9]),
            focal_scale: archetype.map(|profile| profile.focal_scale).unwrap_or(1.0),
            focal_height: archetype.map(|profile| profile.focal_height).unwrap_or(1.0),
            hazard_chance: archetype
                .map(|profile| profile.hazard_chance)
                .unwrap_or(0.2),
            hazard_dps: archetype.map(|profile| profile.hazard_dps).unwrap_or(16.0),
            hazard_radius: archetype
                .map(|profile| profile.hazard_radius)
                .unwrap_or(2.5),
            hazard_kind: archetype
                .map(|profile| HazardKind::from_id(&profile.hazard_kind))
                .unwrap_or(HazardKind::Blood),
            special_event: archetype
                .map(|profile| rng.chance(profile.event_chance.clamp(0.0, 1.0)))
                .unwrap_or(false),
        };
        if !rooms.iter().any(|o| r.overlaps(o, 2)) {
            rooms.push(r);
        }
    }
    let n = rooms.len();
    if let Some(profile) = dc
        .room_archetypes
        .iter()
        .find(|profile| role_from_id(&profile.role) == RoomRole::Safe)
    {
        let special_event = rng.chance(profile.event_chance.clamp(0.0, 1.0));
        apply_archetype(&mut rooms[0], profile, special_event);
    }
    rooms[0].shape = Shape::Rect; // стартовая комната — чистый прямоугольник
    rooms[0].role = RoomRole::Safe;

    // Боссовая комната — самая дальняя от входа, всегда высокий потолок
    let (e_cx, e_cz) = rooms[0].center();
    let mut boss_idx = n - 1;
    let mut best_d = -1i32;
    for (k, r) in rooms.iter().enumerate() {
        if k == 0 {
            continue;
        }
        let (cx, cz) = r.center();
        let d = (cx - e_cx).pow(2) + (cz - e_cz).pow(2);
        if d > best_d {
            best_d = d;
            boss_idx = k;
        }
    }
    rooms[boss_idx].wall_h = 5.5;
    rooms[boss_idx].role = RoomRole::Arena;

    let mut reserved = vec![0, boss_idx];
    let antechamber_idx = (1..n).filter(|idx| *idx != boss_idx).min_by_key(|idx| {
        let (room_x, room_z) = rooms[*idx].center();
        let (boss_x, boss_z) = rooms[boss_idx].center();
        (room_x - boss_x).pow(2) + (room_z - boss_z).pow(2)
    });
    if let Some(idx) = antechamber_idx {
        if let Some(profile) = dc
            .room_archetypes
            .iter()
            .find(|profile| role_from_id(&profile.role) == RoomRole::Antechamber)
        {
            let special_event = rng.chance(profile.event_chance.clamp(0.0, 1.0));
            apply_archetype(&mut rooms[idx], profile, special_event);
            reserved.push(idx);
        }
    }

    let mut required_roles = vec![RoomRole::Treasure, RoomRole::Ritual];
    if depth >= 2 {
        required_roles.extend([RoomRole::Traversal, RoomRole::Puzzle]);
    }
    if depth >= 3 {
        required_roles.push(RoomRole::Story);
    }
    for required_role in required_roles {
        let already_present = rooms.iter().enumerate().find_map(|(idx, room)| {
            (idx != boss_idx && !reserved.contains(&idx) && room.role == required_role)
                .then_some(idx)
        });
        if let Some(idx) = already_present {
            reserved.push(idx);
            continue;
        }
        let Some(profile) = dc
            .room_archetypes
            .iter()
            .find(|profile| role_from_id(&profile.role) == required_role)
        else {
            continue;
        };
        let Some(idx) = (1..n).find(|idx| !reserved.contains(idx)) else {
            continue;
        };
        let special_event = rng.chance(profile.event_chance.clamp(0.0, 1.0));
        apply_archetype(&mut rooms[idx], profile, special_event);
        reserved.push(idx);
    }

    // Центры комнат кэшируем отдельно — высоты будем менять, а центры нет,
    // и это развязывает заимствования при назначении высот.
    let centers: Vec<(i32, i32)> = rooms.iter().map(|r| r.center()).collect();
    // Манхэттенское расстояние между центрами комнат (длина L-коридора в клетках)
    let corridor_len = |a: usize, b: usize| -> i32 {
        let (ax, az) = centers[a];
        let (bx, bz) = centers[b];
        (ax - bx).abs() + (az - bz).abs()
    };

    // 1b. Связь: MST (Прим) по центрам комнат — соединяем БЛИЖАЙШИЕ, не по индексу.
    let mut in_tree = vec![false; n];
    let mut tree: Vec<(usize, usize)> = Vec::new();
    in_tree[0] = true;
    for _ in 1..n {
        let mut best = (i32::MAX, 0usize, 0usize);
        #[allow(clippy::needless_range_loop)]
        for a in 0..n {
            if !in_tree[a] {
                continue;
            }
            for b in 0..n {
                if in_tree[b] {
                    continue;
                }
                let d = corridor_len(a, b);
                if d < best.0 {
                    best = (d, a, b);
                }
            }
        }
        in_tree[best.2] = true;
        tree.push((best.1, best.2));
    }

    // 1c. Высоты комнат: BFS по дереву. Поднимаем/опускаем только когда коридор
    //     достаточно длинный для пологого пандуса — иначе высота как у родителя.
    let mut visited = vec![false; n];
    visited[0] = true;
    let mut queue = std::collections::VecDeque::from([0usize]);
    while let Some(a) = queue.pop_front() {
        for &(u, v) in &tree {
            let b = if u == a {
                v
            } else if v == a {
                u
            } else {
                continue;
            };
            if visited[b] {
                continue;
            }
            visited[b] = true;
            let can_ramp = corridor_len(a, b) >= 3;
            rooms[b].floor_y = if can_ramp && b != boss_idx && rng.chance(0.4) {
                if rooms[a].floor_y < 0.4 {
                    0.8
                } else {
                    0.0
                }
            } else {
                rooms[a].floor_y
            };
            queue.push_back(b);
        }
    }
    rooms[0].floor_y = 0.0;

    // 1d. Петли: добавим 1–2 коротких доп-ребра между комнатами ОДНОЙ высоты
    //     (чтобы не плодить крутые пандусы) для более связной топологии.
    let mut links = tree.clone();
    let mut extra_cand: Vec<(i32, usize, usize)> = Vec::new();
    for a in 0..n {
        for b in (a + 1)..n {
            if links.contains(&(a, b)) || links.contains(&(b, a)) {
                continue;
            }
            if (rooms[a].floor_y - rooms[b].floor_y).abs() > 0.05 {
                continue;
            }
            extra_cand.push((corridor_len(a, b), a, b));
        }
    }
    extra_cand.sort_by_key(|e| e.0);
    for &(_, a, b) in extra_cand.iter().take((n / 4).max(1)) {
        links.push((a, b));
    }

    // 2. Пол/высоты
    let mut floor = vec![false; GRID * GRID];
    let mut fh = vec![0.0f32; GRID * GRID];
    let mut cwh = vec![WALL_H; GRID * GRID];
    let mut is_room = vec![false; GRID * GRID];
    let mut is_ramp = vec![false; GRID * GRID];

    // Фаза A: комнаты (по форме архетипа)
    for r in &rooms {
        for j in r.z..r.z + r.h {
            for i in r.x..r.x + r.w {
                let ii = i as usize;
                let jj = j as usize;
                if ii < 1 || jj < 1 || ii >= GRID - 1 || jj >= GRID - 1 {
                    continue;
                }
                if !r.contains(i, j) {
                    continue;
                }
                let idx = jj * GRID + ii;
                floor[idx] = true;
                fh[idx] = r.floor_y;
                cwh[idx] = r.wall_h;
                is_room[idx] = true;
            }
        }
    }

    // Фаза B: коридоры (L-образные, ширина 2). При разной высоте комнат весь
    // коридор — плавный пандус: высота линейно интерполируется по длине пути,
    // а сегменты записываются для отрисовки наклонными слэбами.
    let mut ramp_segs: Vec<(Vector3, Vector3)> = Vec::new(); // (низ, верх) поверхности
    for &(a, b) in &links {
        let ra = rooms[a];
        let rb = rooms[b];
        let (ax, az) = ra.center();
        let (bx, bz) = rb.center();
        let wh = ra.wall_h.min(rb.wall_h);
        let dx = (ax - bx).abs();
        let dz = (az - bz).abs();
        let total = (dx + dz).max(1) as f32;
        let ramp = (ra.floor_y - rb.floor_y).abs() > 0.05;
        let lerp_h = |step: f32| ra.floor_y + (rb.floor_y - ra.floor_y) * (step / total);
        // Горизонтальный сегмент (widen по Z)
        let mut cx = ax;
        let mut step = 0.0f32;
        while cx != bx {
            let h = lerp_h(step);
            carve_cell(
                &mut floor,
                &mut fh,
                &mut cwh,
                &mut is_ramp,
                &is_room,
                cx,
                az,
                h,
                wh,
                ramp,
            );
            carve_cell(
                &mut floor,
                &mut fh,
                &mut cwh,
                &mut is_ramp,
                &is_room,
                cx,
                az + 1,
                h,
                wh,
                ramp,
            );
            cx += (bx - cx).signum();
            step += 1.0;
        }
        // Вертикальный сегмент (widen по X)
        let mut cz = az;
        while cz != bz {
            let h = lerp_h(step);
            carve_cell(
                &mut floor,
                &mut fh,
                &mut cwh,
                &mut is_ramp,
                &is_room,
                bx,
                cz,
                h,
                wh,
                ramp,
            );
            carve_cell(
                &mut floor,
                &mut fh,
                &mut cwh,
                &mut is_ramp,
                &is_room,
                bx + 1,
                cz,
                h,
                wh,
                ramp,
            );
            cz += (bz - cz).signum();
            step += 1.0;
        }
        // Записываем наклонные слэбы (центр 2-клеточного коридора)
        if ramp {
            let off_z = Vector3::new(0.0, 0.0, CELL * 0.5);
            let off_x = Vector3::new(CELL * 0.5, 0.0, 0.0);
            if dx > 0 {
                let p0 = cell_at(ax, az, ra.floor_y) + off_z;
                let p1 = cell_at(bx, az, lerp_h(dx as f32)) + off_z;
                ramp_segs.push(if p0.y <= p1.y { (p0, p1) } else { (p1, p0) });
            }
            if dz > 0 {
                let p0 = cell_at(bx, az, lerp_h(dx as f32)) + off_x;
                let p1 = cell_at(bx, bz, rb.floor_y) + off_x;
                ramp_segs.push(if p0.y <= p1.y { (p0, p1) } else { (p1, p0) });
            }
        }
    }

    let mut reachable = flood_walkable(&floor, &fh, centers[0]);
    let disconnected_layout = centers
        .iter()
        .any(|(center_x, center_z)| !reachable[*center_z as usize * GRID + *center_x as usize]);
    if disconnected_layout {
        for room in &mut rooms {
            room.floor_y = 0.0;
        }
        for (index, is_floor) in floor.iter().copied().enumerate() {
            if is_floor {
                fh[index] = 0.0;
                is_ramp[index] = false;
            }
        }
        ramp_segs.clear();
        reachable = flood_walkable(&floor, &fh, centers[0]);
        debug_assert!(centers.iter().all(|(center_x, center_z)| {
            reachable[*center_z as usize * GRID + *center_x as usize]
        }));
    }

    let is_floor = |i: i32, j: i32| -> bool {
        i >= 0
            && j >= 0
            && (i as usize) < GRID
            && (j as usize) < GRID
            && floor[j as usize * GRID + i as usize]
    };
    let get_fh = |i: i32, j: i32| -> f32 {
        if i < 0 || j < 0 || i as usize >= GRID || j as usize >= GRID {
            0.0
        } else {
            fh[j as usize * GRID + i as usize]
        }
    };
    let get_cwh = |i: i32, j: i32| -> f32 {
        if i < 0 || j < 0 || i as usize >= GRID || j as usize >= GRID {
            WALL_H
        } else {
            cwh[j as usize * GRID + i as usize]
        }
    };
    let is_ramp_at = |i: i32, j: i32| -> bool {
        i >= 0
            && j >= 0
            && (i as usize) < GRID
            && (j as usize) < GRID
            && is_ramp[j as usize * GRID + i as usize]
    };

    // 3. Геометрия
    let mut root = Node3D::new_alloc();
    let t_wall = cache.get(&theme.wall);
    let t_accent = cache.get(&theme.accent);
    let t_floor = cache.get(&theme.floor);
    let t_ceil = cache.get(&theme.ceil);
    let t_lava = cache.get(&theme.lava);
    let c_dark = Color::from_rgba(0.07, 0.04, 0.07, 1.0);
    const T: f32 = 0.3;

    // ── 3a. Пол и потолок — полосами, только одинаковая высота ──────────────
    for j in 0..GRID as i32 {
        let mut i = 0i32;
        while i < GRID as i32 {
            if is_floor(i, j) {
                let start = i;
                let h0 = get_fh(i, j);
                let wh0 = get_cwh(i, j);
                let ramp0 = is_ramp_at(i, j);
                while i < GRID as i32
                    && is_floor(i, j)
                    && (get_fh(i, j) - h0).abs() < 0.01
                    && (get_cwh(i, j) - wh0).abs() < 0.01
                    && is_ramp_at(i, j) == ramp0
                {
                    i += 1;
                }
                let len = (i - start) as f32 * CELL;
                let cx = ((start + i) as f32 * 0.5 - GRID as f32 * 0.5) * CELL;
                let cz = (j as f32 + 0.5 - GRID as f32 * 0.5) * CELL;
                let uv = (len / CELL).max(1.0);
                // Пол пандус-клеток заменяет наклонный слэб (см. 3e) — здесь только потолок.
                if !ramp0 {
                    let fl = make_box(
                        Vector3::new(cx, h0 - 0.15, cz),
                        Vector3::new(len + 0.02, 0.3, CELL + 0.02),
                        c_dark,
                        t_floor.as_ref(),
                        uv,
                    );
                    root.add_child(&fl);
                }
                let ce = make_box(
                    Vector3::new(cx, h0 + wh0 + 0.15, cz),
                    Vector3::new(len + 0.02, 0.3, CELL + 0.02),
                    c_dark,
                    t_ceil.as_ref(),
                    uv,
                );
                root.add_child(&ce);
            } else {
                i += 1;
            }
        }
    }

    // ── 3b. Стены: N/S грани (перебор по строкам) ────────────────────────────
    for j in 0..GRID as i32 {
        for side in [-1i32, 1] {
            let mut i = 0i32;
            while i < GRID as i32 {
                // --- Полная стена (пол рядом с пустотой) ---
                if is_floor(i, j) && !is_floor(i, j + side) {
                    let start = i;
                    let h0 = get_fh(i, j);
                    let wh0 = get_cwh(i, j);
                    while i < GRID as i32
                        && is_floor(i, j)
                        && !is_floor(i, j + side)
                        && (get_fh(i, j) - h0).abs() < 0.01
                        && (get_cwh(i, j) - wh0).abs() < 0.01
                    {
                        i += 1;
                    }
                    let len = (i - start) as f32 * CELL;
                    let cx = ((start + i) as f32 * 0.5 - GRID as f32 * 0.5) * CELL;
                    let cz = (j as f32 + 0.5 - GRID as f32 * 0.5) * CELL + side as f32 * CELL * 0.5;
                    let use_accent = rng.chance(0.12);
                    let tex = if use_accent {
                        t_accent.as_ref()
                    } else {
                        t_wall.as_ref()
                    };
                    let w = make_box(
                        Vector3::new(cx, h0 + wh0 * 0.5, cz),
                        Vector3::new(len + T, wh0, T),
                        c_dark,
                        tex,
                        (len / CELL).max(1.0),
                    );
                    root.add_child(&w);
                } else {
                    i += 1;
                }
            }
            // --- Ступенчатые стены (два пола на разных высотах) ---
            i = 0;
            while i < GRID as i32 {
                let h_me = get_fh(i, j);
                let h_nb = get_fh(i, j + side);
                // Рисуем ступень только от ВЫСОКОЙ клетки (избегаем дублей)
                if is_floor(i, j)
                    && is_floor(i, j + side)
                    && h_me > h_nb + 0.05
                    && !is_ramp_at(i, j)
                    && !is_ramp_at(i, j + side)
                {
                    let start = i;
                    let step_h = h_me - h_nb;
                    while i < GRID as i32
                        && is_floor(i, j)
                        && is_floor(i, j + side)
                        && !is_ramp_at(i, j)
                        && !is_ramp_at(i, j + side)
                        && (get_fh(i, j) - h_me).abs() < 0.01
                        && (get_fh(i, j + side) - h_nb).abs() < 0.01
                    {
                        i += 1;
                    }
                    let len = (i - start) as f32 * CELL;
                    let cx = ((start + i) as f32 * 0.5 - GRID as f32 * 0.5) * CELL;
                    let cz = (j as f32 + 0.5 - GRID as f32 * 0.5) * CELL + side as f32 * CELL * 0.5;
                    let sw = make_box(
                        Vector3::new(cx, h_nb + step_h * 0.5, cz),
                        Vector3::new(len + T, step_h + 0.02, T),
                        c_dark,
                        t_wall.as_ref(),
                        (len / CELL).max(1.0),
                    );
                    root.add_child(&sw);
                } else {
                    i += 1;
                }
            }
        }
    }

    // ── 3c. Стены: W/E грани (перебор по столбцам) ──────────────────────────
    for i in 0..GRID as i32 {
        for side in [-1i32, 1] {
            let mut j = 0i32;
            while j < GRID as i32 {
                // --- Полная стена ---
                if is_floor(i, j) && !is_floor(i + side, j) {
                    let start = j;
                    let h0 = get_fh(i, j);
                    let wh0 = get_cwh(i, j);
                    while j < GRID as i32
                        && is_floor(i, j)
                        && !is_floor(i + side, j)
                        && (get_fh(i, j) - h0).abs() < 0.01
                        && (get_cwh(i, j) - wh0).abs() < 0.01
                    {
                        j += 1;
                    }
                    let len = (j - start) as f32 * CELL;
                    let cz = ((start + j) as f32 * 0.5 - GRID as f32 * 0.5) * CELL;
                    let cx = (i as f32 + 0.5 - GRID as f32 * 0.5) * CELL + side as f32 * CELL * 0.5;
                    let use_accent = rng.chance(0.12);
                    let tex = if use_accent {
                        t_accent.as_ref()
                    } else {
                        t_wall.as_ref()
                    };
                    let w = make_box(
                        Vector3::new(cx, h0 + wh0 * 0.5, cz),
                        Vector3::new(T, wh0, len + T),
                        c_dark,
                        tex,
                        (len / CELL).max(1.0),
                    );
                    root.add_child(&w);
                } else {
                    j += 1;
                }
            }
            // --- Ступенчатые стены ---
            j = 0;
            while j < GRID as i32 {
                let h_me = get_fh(i, j);
                let h_nb = get_fh(i + side, j);
                if is_floor(i, j)
                    && is_floor(i + side, j)
                    && h_me > h_nb + 0.05
                    && !is_ramp_at(i, j)
                    && !is_ramp_at(i + side, j)
                {
                    let start = j;
                    let step_h = h_me - h_nb;
                    while j < GRID as i32
                        && is_floor(i, j)
                        && is_floor(i + side, j)
                        && !is_ramp_at(i, j)
                        && !is_ramp_at(i + side, j)
                        && (get_fh(i, j) - h_me).abs() < 0.01
                        && (get_fh(i + side, j) - h_nb).abs() < 0.01
                    {
                        j += 1;
                    }
                    let len = (j - start) as f32 * CELL;
                    let cz = ((start + j) as f32 * 0.5 - GRID as f32 * 0.5) * CELL;
                    let cx = (i as f32 + 0.5 - GRID as f32 * 0.5) * CELL + side as f32 * CELL * 0.5;
                    let sw = make_box(
                        Vector3::new(cx, h_nb + step_h * 0.5, cz),
                        Vector3::new(T, step_h + 0.02, len + T),
                        c_dark,
                        t_wall.as_ref(),
                        (len / CELL).max(1.0),
                    );
                    root.add_child(&sw);
                } else {
                    j += 1;
                }
            }
        }
    }

    // ── 3e. Слэбы-пандусы (пологие склоны между уровнями) ────────────────────
    for &(low, high) in &ramp_segs {
        let slab = make_ramp(low, high, CELL * 2.0, T, c_dark, t_floor.as_ref());
        root.add_child(&slab);
    }

    // ── 3d. Детали комнат ────────────────────────────────────────────────────
    let mut hazards: Vec<HazardZone> = Vec::new();
    let mut dressing_nodes = 0usize;
    let mut dressing_variants = Vec::with_capacity(rooms.len());
    for (k, r) in rooms.iter().enumerate() {
        let (cx, cz) = r.center();
        let center = cell_at(cx, cz, r.floor_y);
        let is_boss_room = k == boss_idx;

        // Свет под потолком
        let light_y = r.floor_y + r.wall_h - 0.6;
        let range = (r.w.max(r.h) as f32) * CELL * 1.4;
        let energy = if is_boss_room { 2.6 } else { 1.7 };
        let color = if is_boss_room {
            Color::from_rgba(0.95, 0.12, 0.18, 1.0)
        } else {
            theme.light
        };
        let l = make_light(
            Vector3::new(center.x, light_y, center.z),
            color,
            energy,
            range,
        );
        root.add_child(&l);

        // Факел у стены
        if rng.chance(0.85) {
            let ti = r.x + 1 + rng.range(0, (r.w - 2).max(0));
            let torch_y = r.floor_y + 1.5;
            let pos = cell_at(ti, r.z, torch_y) + Vector3::new(0.0, 0.0, -CELL * 0.28);
            if let Some(sp) =
                make_billboard(cache, "res://assets/sprites/pickups/soul.png", pos, 0.012)
            {
                root.add_child(&sp);
            }
            let tl = make_light(pos, theme.light, 0.9, 7.0);
            root.add_child(&tl);
        }

        // Лужа лавы (только в несунутых комнатах)
        if !is_boss_room
            && k != 0
            && r.hazard_dps > 0.0
            && rng.chance(r.hazard_chance.clamp(0.0, 1.0))
            && r.w >= 5
            && r.h >= 5
        {
            let (lx, lz) = (cx + rng.range(-1, 1), cz + rng.range(-1, 1));
            if r.contains(lx, lz) {
                let lp = cell_at(lx, lz, r.floor_y);
                let hazard_color = r.hazard_kind.color();
                let slab = make_glow_slab(
                    lp + Vector3::new(0.0, 0.03, 0.0),
                    Vector3::new(CELL * 1.6, 0.06, CELL * 1.6),
                    t_lava.as_ref(),
                    hazard_color,
                    1.0,
                );
                root.add_child(&slab);
                let ll = make_light(lp + Vector3::new(0.0, 0.8, 0.0), hazard_color, 1.1, 6.0);
                root.add_child(&ll);
                hazards.push(HazardZone {
                    pos: lp,
                    radius: r.hazard_radius.max(0.5),
                    dps: r.hazard_dps + depth as f32 * 2.0,
                    kind: r.hazard_kind,
                    phase_offset: rng.f32() * 3.0,
                    phase: HazardPhase::Dormant,
                    player_inside: false,
                });
            }
        }

        // Колонны в больших комнатах
        if r.w >= 6 && r.h >= 6 && rng.chance(0.6) {
            for (dx, dz) in [(-2i32, -2i32), (2, 2), (-2, 2), (2, -2)] {
                if !r.contains(cx + dx, cz + dz) {
                    continue;
                }
                let p = cell_at(cx + dx, cz + dz, r.floor_y);
                let col = make_box(
                    p + Vector3::new(0.0, r.wall_h * 0.5, 0.0),
                    Vector3::new(0.7, r.wall_h, 0.7),
                    c_dark,
                    t_wall.as_ref(),
                    1.0,
                );
                root.add_child(&col);
            }
        }

        // Поднятая платформа — только в прямоугольных комнатах (в фигурных
        // выпирала бы в пустоту).
        if r.shape == Shape::Rect
            && r.w >= 7
            && r.h >= 7
            && !matches!(
                r.role,
                RoomRole::Safe
                    | RoomRole::Traversal
                    | RoomRole::Puzzle
                    | RoomRole::Story
                    | RoomRole::Antechamber
            )
            && rng.chance(0.55)
        {
            let plat_y = r.floor_y + 0.8;
            let pw = rng.range(2, (r.w - 3).max(2)) as f32 * CELL;
            let ph = rng.range(2, (r.h - 3).max(2)) as f32 * CELL;
            let pc = cell_at(cx + rng.range(-1, 1), cz + rng.range(-1, 1), plat_y);
            let plat = make_box(
                pc + Vector3::new(0.0, -0.15, 0.0),
                Vector3::new(pw, 0.3 + plat_y - r.floor_y, ph),
                c_dark,
                t_floor.as_ref(),
                (pw / CELL).max(1.0),
            );
            root.add_child(&plat);
            // Свет под потолком над платформой
            let pl = make_light(
                pc + Vector3::new(0.0, r.wall_h - 0.5, 0.0),
                theme.light,
                0.7,
                pw + 2.0,
            );
            root.add_child(&pl);
        }

        match r.role {
            RoomRole::Safe => {
                root.add_child(&make_glow_slab(
                    center + Vector3::new(0.0, 0.05, 0.0),
                    Vector3::new(CELL * 1.4, 0.08, CELL * 1.4),
                    t_accent.as_ref(),
                    theme.light,
                    1.0,
                ));
            }
            RoomRole::Ritual => {
                root.add_child(&make_box(
                    center + Vector3::new(0.0, 0.45, 0.0),
                    Vector3::new(2.8, 0.9, 2.8),
                    c_dark,
                    t_accent.as_ref(),
                    1.0,
                ));
                for (dx, dz) in [(-2, 0), (2, 0), (0, -2), (0, 2)] {
                    if r.contains(cx + dx, cz + dz) {
                        let p = cell_at(cx + dx, cz + dz, r.floor_y + 0.7);
                        root.add_child(&make_light(p, theme.light, 1.25, 6.0));
                    }
                }
            }
            RoomRole::Gallery => {
                for offset in [-2, 0, 2] {
                    if r.contains(cx + offset, cz) {
                        root.add_child(&make_box(
                            cell_at(cx + offset, cz, r.floor_y) + Vector3::new(0.0, 0.65, 0.0),
                            Vector3::new(CELL * 0.65, 1.3, 0.65),
                            c_dark,
                            t_wall.as_ref(),
                            1.0,
                        ));
                    }
                }
            }
            RoomRole::Treasure => {
                root.add_child(&make_box(
                    center + Vector3::new(0.0, 0.3, 0.0),
                    Vector3::new(CELL * 2.2, 0.6, CELL * 1.4),
                    c_dark,
                    t_floor.as_ref(),
                    2.0,
                ));
            }
            RoomRole::Ambush => {
                for (dx, dz) in [(-2, -2), (2, -2), (-2, 2), (2, 2)] {
                    if r.contains(cx + dx, cz + dz) {
                        root.add_child(&make_box(
                            cell_at(cx + dx, cz + dz, r.floor_y) + Vector3::new(0.0, 0.9, 0.0),
                            Vector3::new(1.2, 1.8, 1.2),
                            c_dark,
                            t_accent.as_ref(),
                            1.0,
                        ));
                    }
                }
            }
            RoomRole::Traversal => {
                for side in [-1.0, 1.0] {
                    let bridge_center = center + Vector3::new(side * CELL * 1.25, 0.28, 0.0);
                    root.add_child(&make_box(
                        bridge_center,
                        Vector3::new(CELL * 0.85, 0.55, CELL * 3.6),
                        c_dark,
                        t_floor.as_ref(),
                        3.0,
                    ));
                    root.add_child(&make_glow_slab(
                        bridge_center + Vector3::new(0.0, 0.31, 0.0),
                        Vector3::new(CELL * 0.72, 0.05, CELL * 3.2),
                        t_accent.as_ref(),
                        Color::from_rgba(0.12, 0.78, 1.0, 1.0),
                        3.0,
                    ));
                }
                for z_offset in [-2.2, 2.2] {
                    root.add_child(&make_light(
                        center + Vector3::new(0.0, 1.3, z_offset * CELL),
                        Color::from_rgba(0.12, 0.78, 1.0, 1.0),
                        0.9,
                        5.0,
                    ));
                }
            }
            RoomRole::Puzzle => {
                for (dx, dz) in [(-2, -2), (2, -2), (-2, 2), (2, 2)] {
                    if !r.contains(cx + dx, cz + dz) {
                        continue;
                    }
                    let switch = cell_at(cx + dx, cz + dz, r.floor_y);
                    root.add_child(&make_box(
                        switch + Vector3::new(0.0, 0.75, 0.0),
                        Vector3::new(1.0, 1.5, 1.0),
                        c_dark,
                        t_wall.as_ref(),
                        1.0,
                    ));
                    root.add_child(&make_glow_slab(
                        switch + Vector3::new(0.0, 1.53, 0.0),
                        Vector3::new(0.62, 0.06, 0.62),
                        t_accent.as_ref(),
                        Color::from_rgba(0.2, 1.0, 0.68, 1.0),
                        1.0,
                    ));
                }
                root.add_child(&make_glow_slab(
                    center + Vector3::new(0.0, 0.06, 0.0),
                    Vector3::new(CELL * 1.6, 0.08, CELL * 1.6),
                    t_accent.as_ref(),
                    Color::from_rgba(0.2, 1.0, 0.68, 1.0),
                    1.5,
                ));
            }
            RoomRole::Story => {
                for (dx, height) in [(-2, 1.65), (0, 2.2), (2, 1.65)] {
                    if r.contains(cx + dx, cz + 2) {
                        root.add_child(&make_box(
                            cell_at(cx + dx, cz + 2, r.floor_y)
                                + Vector3::new(0.0, height * 0.5, 0.0),
                            Vector3::new(CELL * 0.8, height, 0.28),
                            c_dark,
                            t_accent.as_ref(),
                            1.0,
                        ));
                    }
                }
                if let Some(mut memory) = make_billboard(
                    cache,
                    "res://assets/sprites/pickups/soul.png",
                    center + Vector3::new(0.0, 1.45, 0.0),
                    0.018,
                ) {
                    memory.set_modulate(Color::from_rgba(0.72, 0.56, 1.0, 0.95));
                    root.add_child(&memory);
                }
            }
            RoomRole::Antechamber => {
                for (dx, dz) in [(-2, -2), (2, -2), (-2, 0), (2, 0), (-2, 2), (2, 2)] {
                    if r.contains(cx + dx, cz + dz) {
                        let column = cell_at(cx + dx, cz + dz, r.floor_y);
                        root.add_child(&make_box(
                            column + Vector3::new(0.0, 1.35, 0.0),
                            Vector3::new(0.85, 2.7, 0.85),
                            c_dark,
                            t_accent.as_ref(),
                            1.0,
                        ));
                    }
                }
                root.add_child(&make_glow_slab(
                    center + Vector3::new(0.0, 0.055, 0.0),
                    Vector3::new(CELL * 1.1, 0.06, CELL * 4.0),
                    t_accent.as_ref(),
                    Color::from_rgba(1.0, 0.28, 0.48, 1.0),
                    3.0,
                ));
            }
            RoomRole::Arena => {}
        }
        match r.decor {
            RoomDecor::Basic => {}
            RoomDecor::Pipes => {
                for offset in [-2, 2] {
                    if r.contains(cx, cz + offset) {
                        root.add_child(&make_box(
                            cell_at(cx, cz + offset, r.floor_y + r.wall_h - 0.9),
                            Vector3::new((r.w.min(7) as f32) * CELL * 0.72, 0.32, 0.42),
                            c_dark,
                            t_wall.as_ref(),
                            3.0,
                        ));
                    }
                }
            }
            RoomDecor::Ossuary => {
                for (dx, dz) in [(-2, 0), (2, 0), (0, -2), (0, 2)] {
                    if r.contains(cx + dx, cz + dz) {
                        root.add_child(&make_box(
                            cell_at(cx + dx, cz + dz, r.floor_y + 0.65),
                            Vector3::new(0.7, 1.3, 0.7),
                            c_dark,
                            t_accent.as_ref(),
                            1.0,
                        ));
                    }
                }
            }
            RoomDecor::Crystals => {
                for (dx, dz) in [(-2, -1), (2, 1), (0, 2)] {
                    if r.contains(cx + dx, cz + dz) {
                        let crystal = cell_at(cx + dx, cz + dz, r.floor_y + 0.55);
                        root.add_child(&make_box(
                            crystal,
                            Vector3::new(0.45, 1.1 + (dx.abs() as f32 * 0.15), 0.45),
                            c_dark,
                            t_accent.as_ref(),
                            1.0,
                        ));
                        root.add_child(&make_light(crystal, theme.light, 0.8, 4.5));
                    }
                }
            }
            RoomDecor::Machinery => {
                for (dx, dz) in [(-2, -2), (2, -2), (-2, 2), (2, 2)] {
                    if r.contains(cx + dx, cz + dz) {
                        let machine = cell_at(cx + dx, cz + dz, r.floor_y + 0.75);
                        root.add_child(&make_box(
                            machine,
                            Vector3::new(1.5, 1.5, 1.5),
                            c_dark,
                            t_wall.as_ref(),
                            1.0,
                        ));
                        root.add_child(&make_light(
                            machine + Vector3::new(0.0, 0.55, 0.0),
                            Color::from_rgba(0.1, 0.8, 1.0, 1.0),
                            0.65,
                            3.5,
                        ));
                    }
                }
            }
            RoomDecor::Archive => {
                for dx in [-2, 2] {
                    if r.contains(cx + dx, cz) {
                        root.add_child(&make_box(
                            cell_at(cx + dx, cz, r.floor_y + 1.0),
                            Vector3::new(0.55, 2.0, (r.h.min(6) as f32) * CELL * 0.65),
                            c_dark,
                            t_wall.as_ref(),
                            2.0,
                        ));
                    }
                }
            }
            RoomDecor::Sanctuary => {
                for (dx, dz, sx, sz) in [
                    (0, -2, 0.65, 2.2),
                    (0, 2, 0.65, 2.2),
                    (-2, 0, 2.2, 0.65),
                    (2, 0, 2.2, 0.65),
                ] {
                    if r.contains(cx + dx, cz + dz) {
                        root.add_child(&make_glow_slab(
                            cell_at(cx + dx, cz + dz, r.floor_y + 0.06),
                            Vector3::new(sx, 0.06, sz),
                            t_accent.as_ref(),
                            theme.light,
                            1.0,
                        ));
                    }
                }
            }
        }

        // Второй seeded dressing-слой: одна из трёх ориентаций на комнату.
        // Только бесколлизионные меши/спрайты/lights — навигационная сетка остаётся честной.
        let variant = ((k as u64 + depth as u64 + (seed & 0xff)) % 3) as u8;
        dressing_variants.push(variant);
        let dressing_color = match r.role {
            RoomRole::Safe => Color::from_rgba(0.3, 1.0, 0.75, 1.0),
            RoomRole::Arena => Color::from_rgba(1.0, 0.2, 0.32, 1.0),
            RoomRole::Gallery => Color::from_rgba(0.35, 0.72, 1.0, 1.0),
            RoomRole::Ritual => Color::from_rgba(0.9, 0.2, 1.0, 1.0),
            RoomRole::Treasure => Color::from_rgba(1.0, 0.78, 0.2, 1.0),
            RoomRole::Ambush => Color::from_rgba(1.0, 0.12, 0.2, 1.0),
            RoomRole::Traversal => Color::from_rgba(0.12, 0.78, 1.0, 1.0),
            RoomRole::Puzzle => Color::from_rgba(0.2, 1.0, 0.68, 1.0),
            RoomRole::Story => Color::from_rgba(0.72, 0.56, 1.0, 1.0),
            RoomRole::Antechamber => Color::from_rgba(1.0, 0.28, 0.48, 1.0),
        };
        let base_pattern: &[(i32, i32)] = match r.role {
            RoomRole::Safe => &[(-2, 0), (0, 2), (2, 0)],
            RoomRole::Arena => &[(-2, -2), (0, -2), (2, -2), (-2, 2), (0, 2), (2, 2)],
            RoomRole::Gallery => &[(-3, 0), (-1, 0), (1, 0), (3, 0)],
            RoomRole::Ritual => &[(-2, 0), (-1, -1), (0, -2), (1, -1), (2, 0)],
            RoomRole::Treasure => &[(-2, -1), (0, -2), (2, -1), (-1, 2), (1, 2)],
            RoomRole::Ambush => &[(-3, -3), (3, -3), (-3, 3), (3, 3)],
            RoomRole::Traversal => &[(-2, -3), (-2, 0), (-2, 3), (2, -3), (2, 0), (2, 3)],
            RoomRole::Puzzle => &[(-2, -2), (2, -2), (2, 2), (-2, 2), (0, 0)],
            RoomRole::Story => &[(-2, 2), (0, 3), (2, 2), (0, -2)],
            RoomRole::Antechamber => &[(-2, -3), (2, -3), (-2, 0), (2, 0), (-2, 3), (2, 3)],
        };
        for (index, (base_x, base_z)) in base_pattern.iter().enumerate() {
            let (offset_x, offset_z) = match variant {
                1 => (-*base_z, *base_x),
                2 => (-*base_x, -*base_z),
                _ => (*base_x, *base_z),
            };
            if !r.contains(cx + offset_x, cz + offset_z) {
                continue;
            }
            let position = cell_at(cx + offset_x, cz + offset_z, r.floor_y);
            root.add_child(&make_glow_slab(
                position + Vector3::new(0.0, 0.065, 0.0),
                Vector3::new(0.52 + index as f32 * 0.03, 0.045, 0.52),
                t_accent.as_ref(),
                dressing_color,
                1.0,
            ));
            dressing_nodes += 1;
            if index % 2 == 0 {
                let stele_height = 0.8 + variant as f32 * 0.18;
                root.add_child(&make_glow_slab(
                    position + Vector3::new(0.0, stele_height * 0.5, 0.0),
                    Vector3::new(0.09, stele_height, 0.09),
                    t_accent.as_ref(),
                    dressing_color,
                    1.0,
                ));
                dressing_nodes += 1;
            }
        }
        if matches!(r.role, RoomRole::Safe | RoomRole::Ritual | RoomRole::Story) {
            if let Some(mut soul) = make_billboard(
                cache,
                "res://assets/sprites/pickups/soul.png",
                center + Vector3::new(0.0, 1.0 + variant as f32 * 0.22, 0.0),
                0.009 + variant as f32 * 0.002,
            ) {
                soul.set_modulate(dressing_color);
                root.add_child(&soul);
                dressing_nodes += 1;
            }
        }
        root.add_child(&make_light(
            center + Vector3::new(0.0, 1.15, 0.0),
            dressing_color,
            0.35 + variant as f32 * 0.12,
            (r.w.min(r.h) as f32 * CELL * 0.42).max(4.0),
        ));
        dressing_nodes += 1;

        let mut focal_root = Node3D::new_alloc();
        focal_root.set_name(&format!("RoomFocal_{}_{}", r.focal_pattern.id(), k));
        focal_root.add_to_group("dungeon_room_focals");
        let focal_color = if r.focal_color.iter().any(|channel| *channel > 0.01) {
            Color::from_rgba(r.focal_color[0], r.focal_color[1], r.focal_color[2], 1.0)
        } else {
            dressing_color
        };
        let focal_scale = r.focal_scale.clamp(0.55, 1.8);
        let focal_height = r.focal_height.clamp(0.5, 4.0);
        match r.focal_pattern {
            FocalPattern::Spire => {
                focal_root.add_child(&make_glow_slab(
                    center + Vector3::new(0.0, focal_height * 0.5, 0.0),
                    Vector3::new(0.16, focal_height, 0.16) * focal_scale,
                    t_accent.as_ref(),
                    focal_color,
                    1.0,
                ));
                for (x, z) in [(-1.2, 0.0), (1.2, 0.0), (0.0, -1.2), (0.0, 1.2)] {
                    focal_root.add_child(&make_glow_slab(
                        center + Vector3::new(x * focal_scale, 0.075, z * focal_scale),
                        Vector3::new(0.42, 0.05, 0.42) * focal_scale,
                        t_accent.as_ref(),
                        focal_color,
                        1.0,
                    ));
                }
            }
            FocalPattern::Ring => {
                for index in 0..8 {
                    let angle = std::f32::consts::TAU * index as f32 / 8.0;
                    focal_root.add_child(&make_glow_slab(
                        center
                            + Vector3::new(
                                angle.cos() * 1.45 * focal_scale,
                                0.075,
                                angle.sin() * 1.45 * focal_scale,
                            ),
                        Vector3::new(0.38, 0.05, 0.38) * focal_scale,
                        t_accent.as_ref(),
                        focal_color,
                        1.0,
                    ));
                }
            }
            FocalPattern::Cross => {
                for (offset, size) in [
                    (
                        Vector3::new(0.0, 0.075, -1.1),
                        Vector3::new(0.42, 0.05, 1.8),
                    ),
                    (Vector3::new(0.0, 0.075, 1.1), Vector3::new(0.42, 0.05, 1.8)),
                    (
                        Vector3::new(-1.1, 0.075, 0.0),
                        Vector3::new(1.8, 0.05, 0.42),
                    ),
                    (Vector3::new(1.1, 0.075, 0.0), Vector3::new(1.8, 0.05, 0.42)),
                ] {
                    focal_root.add_child(&make_glow_slab(
                        center + offset * focal_scale,
                        size * focal_scale,
                        t_accent.as_ref(),
                        focal_color,
                        1.0,
                    ));
                }
            }
            FocalPattern::Aisle => {
                for z in [-1.8, 0.0, 1.8] {
                    for x in [-0.9, 0.9] {
                        focal_root.add_child(&make_glow_slab(
                            center + Vector3::new(x, 0.075, z) * focal_scale,
                            Vector3::new(0.34, 0.05, 0.72) * focal_scale,
                            t_accent.as_ref(),
                            focal_color,
                            1.0,
                        ));
                    }
                }
            }
            FocalPattern::Altar => {
                for (index, size) in [2.4, 1.6, 0.85].into_iter().enumerate() {
                    focal_root.add_child(&make_glow_slab(
                        center + Vector3::new(0.0, 0.06 + index as f32 * 0.045, 0.0),
                        Vector3::new(size, 0.04, size * 0.55) * focal_scale,
                        t_accent.as_ref(),
                        focal_color,
                        1.0,
                    ));
                }
                focal_root.add_child(&make_glow_slab(
                    center + Vector3::new(0.0, focal_height * 0.25, 0.0),
                    Vector3::new(0.28, focal_height * 0.5, 0.28) * focal_scale,
                    t_accent.as_ref(),
                    focal_color,
                    1.0,
                ));
            }
            FocalPattern::Well => {
                for (offset, size) in [
                    (
                        Vector3::new(0.0, 0.075, -1.25),
                        Vector3::new(2.6, 0.05, 0.22),
                    ),
                    (
                        Vector3::new(0.0, 0.075, 1.25),
                        Vector3::new(2.6, 0.05, 0.22),
                    ),
                    (
                        Vector3::new(-1.25, 0.075, 0.0),
                        Vector3::new(0.22, 0.05, 2.6),
                    ),
                    (
                        Vector3::new(1.25, 0.075, 0.0),
                        Vector3::new(0.22, 0.05, 2.6),
                    ),
                ] {
                    focal_root.add_child(&make_glow_slab(
                        center + offset * focal_scale,
                        size * focal_scale,
                        t_accent.as_ref(),
                        focal_color,
                        1.0,
                    ));
                }
            }
            FocalPattern::Archive => {
                for x in [-1.1, 0.0, 1.1] {
                    focal_root.add_child(&make_glow_slab(
                        center + Vector3::new(x * focal_scale, focal_height * 0.5, 0.0),
                        Vector3::new(0.12, focal_height, 0.75) * focal_scale,
                        t_accent.as_ref(),
                        focal_color,
                        1.0,
                    ));
                }
            }
            FocalPattern::Gate => {
                for x in [-1.0, 1.0] {
                    focal_root.add_child(&make_glow_slab(
                        center + Vector3::new(x * focal_scale, focal_height * 0.5, 0.0),
                        Vector3::new(0.14, focal_height, 0.14) * focal_scale,
                        t_accent.as_ref(),
                        focal_color,
                        1.0,
                    ));
                }
                focal_root.add_child(&make_glow_slab(
                    center + Vector3::new(0.0, focal_height * focal_scale, 0.0),
                    Vector3::new(2.2, 0.14, 0.14) * focal_scale,
                    t_accent.as_ref(),
                    focal_color,
                    1.0,
                ));
            }
        }
        focal_root.add_child(&make_light(
            center + Vector3::new(0.0, focal_height.min(2.2), 0.0),
            focal_color,
            0.55,
            4.2 * focal_scale,
        ));
        root.add_child(&focal_root);

        if r.special_event {
            let event_color = match r.role {
                RoomRole::Safe => Color::from_rgba(0.3, 1.0, 0.75, 1.0),
                RoomRole::Treasure => Color::from_rgba(1.0, 0.78, 0.2, 1.0),
                RoomRole::Ritual => Color::from_rgba(0.9, 0.2, 1.0, 1.0),
                RoomRole::Ambush => Color::from_rgba(1.0, 0.15, 0.25, 1.0),
                RoomRole::Traversal => Color::from_rgba(0.12, 0.78, 1.0, 1.0),
                RoomRole::Puzzle => Color::from_rgba(0.2, 1.0, 0.68, 1.0),
                RoomRole::Story => Color::from_rgba(0.72, 0.56, 1.0, 1.0),
                RoomRole::Antechamber => Color::from_rgba(1.0, 0.28, 0.48, 1.0),
                _ => theme.light,
            };
            root.add_child(&make_light(
                center + Vector3::new(0.0, 1.4, 0.0),
                event_color,
                2.1,
                (r.w.max(r.h) as f32) * CELL,
            ));
            if let Some(mut marker) = make_billboard(
                cache,
                "res://assets/sprites/pickups/soul.png",
                center + Vector3::new(0.0, 1.35, 0.0),
                0.014,
            ) {
                marker.set_modulate(event_color);
                root.add_child(&marker);
            }
            let event_pattern: &[(f32, f32)] = match r.role {
                RoomRole::Safe => &[(0.0, -1.8), (1.8, 0.0), (0.0, 1.8), (-1.8, 0.0)],
                RoomRole::Treasure => &[(-1.4, -1.4), (1.4, -1.4), (1.4, 1.4), (-1.4, 1.4)],
                RoomRole::Ritual => &[(-2.0, 0.0), (-1.0, 0.0), (1.0, 0.0), (2.0, 0.0)],
                RoomRole::Ambush => &[(-2.2, -2.2), (2.2, -2.2), (-2.2, 2.2), (2.2, 2.2)],
                RoomRole::Gallery => &[(-2.4, 0.0), (0.0, 0.0), (2.4, 0.0)],
                RoomRole::Arena => &[(0.0, -2.4), (0.0, 0.0), (0.0, 2.4)],
                RoomRole::Traversal => &[(-2.2, -2.4), (-2.2, 0.0), (2.2, 0.0), (2.2, 2.4)],
                RoomRole::Puzzle => &[(-1.8, -1.8), (1.8, -1.8), (-1.8, 1.8), (1.8, 1.8)],
                RoomRole::Story => &[(-2.0, 1.8), (0.0, 2.2), (2.0, 1.8)],
                RoomRole::Antechamber => &[(-1.8, -2.4), (1.8, -2.4), (-1.8, 2.4), (1.8, 2.4)],
            };
            for (offset_x, offset_z) in event_pattern {
                root.add_child(&make_glow_slab(
                    center + Vector3::new(*offset_x, 0.07, *offset_z),
                    Vector3::new(0.65, 0.05, 0.65),
                    t_accent.as_ref(),
                    event_color,
                    1.0,
                ));
            }
        }
    }

    // Алтарь в боссовой комнате
    let (b_cx, b_cz) = rooms[boss_idx].center();
    let b_floor = rooms[boss_idx].floor_y;
    let boss_center = cell_at(b_cx, b_cz, b_floor);
    let alt = make_box(
        boss_center + Vector3::new(0.0, 0.35, 0.0),
        Vector3::new(2.6, 0.7, 1.4),
        c_dark,
        t_accent.as_ref(),
        1.0,
    );
    root.add_child(&alt);

    // 4. Спавны (Y = высота пола в точке спавна)
    let mut enemies: Vec<EnemySpawn> = Vec::new();
    let mut items: Vec<(String, Vector3)> = Vec::new();
    let mut ammo: Vec<(AmmoType, u32, Vector3)> = Vec::new();
    let mut weapons: Vec<(WeaponId, Vector3)> = Vec::new();

    let st = &dc.settings;
    let boss_tier = st
        .boss_roster
        .iter()
        .filter(|tier| tier.min_depth <= depth)
        .max_by_key(|tier| tier.min_depth);
    let boss_kind = boss_tier.map(|tier| &tier.boss).unwrap_or(&st.boss);
    let boss_mult = boss_tier.map(|tier| tier.mult).unwrap_or(st.boss_mult);
    let boss_guards = boss_tier
        .map(|tier| tier.guards.as_slice())
        .unwrap_or(st.boss_guards.as_slice());
    let boss_items = boss_tier
        .map(|tier| tier.items.as_slice())
        .unwrap_or(st.boss_items.as_slice());
    let mult = 1.0 + (depth - 1) as f32 * st.mult_per_depth;
    // Пул врагов: самый глубокий из подходящих по min_depth
    let pool: &[String] = dc
        .pools
        .iter()
        .filter(|p| p.min_depth <= depth)
        .max_by_key(|p| p.min_depth)
        .or_else(|| dc.pools.first())
        .map(|p| p.enemies.as_slice())
        .unwrap_or(&[]);

    // Достижимость: flood-fill от клетки спавна игрока по ТЕМ ЖЕ правилам, что и
    // навигация (перепад <= RAMP_STEP). В недостижимых карманах не спавним ничего
    // — иначе враги оказываются «за стеной»/«на крыше» и до них не дойти.
    let reachable = reachable;
    let room_reachable = |k: usize| -> bool {
        let (cx, cz) = centers[k];
        reachable
            .get(cz as usize * GRID + cx as usize)
            .copied()
            .unwrap_or(false)
    };
    // Спавн-точки: клетка должна быть достижимым полом (в фигурных комнатах углы —
    // пустота). snap ищет ближайшую годную клетку; spawn_at даёт мировую позицию
    // на реальной высоте пола этой клетки.
    let cell_ok = |i: i32, j: i32| -> bool {
        is_floor(i, j)
            && reachable
                .get(j as usize * GRID + i as usize)
                .copied()
                .unwrap_or(false)
    };
    let snap = |i: i32, j: i32| -> (i32, i32) {
        if cell_ok(i, j) {
            return (i, j);
        }
        for rad in 1..=3 {
            for dj in -rad..=rad {
                for di in -rad..=rad {
                    if cell_ok(i + di, j + dj) {
                        return (i + di, j + dj);
                    }
                }
            }
        }
        (i, j)
    };
    let spawn_at = |i: i32, j: i32| -> Vector3 {
        let (si, sj) = snap(i, j);
        cell_at(si, sj, get_fh(si, sj))
    };

    for (k, r) in rooms.iter().enumerate() {
        if k == 0 {
            continue;
        }
        if !room_reachable(k) {
            continue;
        }
        let (cx, cz) = r.center();
        let is_boss_room = k == boss_idx;

        if is_boss_room {
            enemies.push(EnemySpawn {
                kind: boss_kind.clone(),
                pos: boss_center,
                mult: mult * boss_mult,
                is_boss: true,
                affixes: Vec::new(),
            });
            // свита по бокам алтаря
            for (gi, guard) in boss_guards.iter().enumerate() {
                let d = if gi % 2 == 0 {
                    -2 - (gi as i32 / 2)
                } else {
                    2 + (gi as i32 / 2)
                };
                enemies.push(EnemySpawn {
                    kind: guard.clone(),
                    pos: spawn_at(cx + d, cz),
                    mult,
                    is_boss: false,
                    affixes: Vec::new(),
                });
            }
            // награда рядком за алтарём: центр, слева, справа, дальше наружу
            for (ii, item) in boss_items.iter().enumerate() {
                let k = (ii as i32 + 1) / 2;
                let dx = if ii % 2 == 0 { k } else { -k };
                items.push((item.clone(), spawn_at(cx + dx, cz + 1)));
            }
            continue;
        }

        if r.role == RoomRole::Safe {
            let ammo_type = AmmoType::from_idx(rng.below(4) as usize);
            ammo.push((ammo_type, ammo_type.pack_size(), spawn_at(cx + 1, cz)));
            if cfg.item("medkit").is_some() {
                items.push(("medkit".to_string(), spawn_at(cx - 1, cz)));
                if r.special_event {
                    items.push(("medkit".to_string(), spawn_at(cx, cz + 1)));
                }
            }
            continue;
        }
        if r.role == RoomRole::Story {
            let ammo_type = AmmoType::from_idx(rng.below(4) as usize);
            ammo.push((ammo_type, ammo_type.pack_size(), spawn_at(cx - 1, cz + 1)));
            if cfg.item("void_relic").is_some() && r.special_event {
                items.push(("void_relic".to_string(), spawn_at(cx + 1, cz + 1)));
            }
            continue;
        }
        // Враги: роль комнаты меняет давление и композицию боя.
        let role_bonus = match r.role {
            RoomRole::Ambush => 2,
            RoomRole::Ritual | RoomRole::Arena => 1,
            RoomRole::Gallery => 0,
            RoomRole::Treasure => -1,
            RoomRole::Traversal => 0,
            RoomRole::Puzzle => -1,
            RoomRole::Story => -2,
            RoomRole::Antechamber => 1,
            RoomRole::Safe => 0,
        } + i32::from(r.special_event && r.role == RoomRole::Ambush);
        let n = (2 + role_bonus + rng.range(0, 2 + (depth.min(6) as i32))).max(1);
        let elite_chance = (st.elite_chance
            + st.elite_per_depth * (depth - 1) as f32
            + if r.special_event && r.role == RoomRole::Ritual {
                0.3
            } else {
                0.0
            })
        .clamp(0.0, 0.85);
        let role_pool: Vec<&String> = pool
            .iter()
            .filter(|enemy_id| {
                let Some(enemy) = cfg.enemy(enemy_id) else {
                    return false;
                };
                match r.role {
                    RoomRole::Gallery => enemy.behavior.as_deref() == Some("ranged"),
                    RoomRole::Ambush => {
                        enemy.behavior.as_deref() != Some("ranged") && enemy.speed >= 3.0
                    }
                    RoomRole::Ritual => {
                        enemy.behavior.as_deref() == Some("ranged") && !enemy.abilities.is_empty()
                    }
                    RoomRole::Traversal => {
                        matches!(enemy.role.as_str(), "pursuer" | "artillery")
                    }
                    RoomRole::Puzzle => {
                        matches!(enemy.role.as_str(), "controller" | "support")
                            || !enemy.abilities.is_empty()
                    }
                    RoomRole::Antechamber => {
                        matches!(enemy.role.as_str(), "tank" | "commander" | "support")
                    }
                    _ => true,
                }
            })
            .collect();
        for idx in 0..n {
            let kind = if role_pool.is_empty() {
                (!pool.is_empty()).then(|| rng.pick(pool))
            } else {
                Some(*rng.pick(&role_pool))
            };
            let Some(kind) = kind else {
                break;
            };
            let px = r.x + 1 + rng.range(0, (r.w - 2).max(1));
            let pz = r.z + 1 + rng.range(0, (r.h - 2).max(1));
            // Небольшой разброс по комнате чтобы не стояли в кучке
            let ox = if idx % 2 == 0 { 0 } else { rng.range(-1, 1) };
            let oz = if idx % 3 == 0 { 0 } else { rng.range(-1, 1) };
            // Элита: 1..max случайных РАЗНЫХ аффиксов (комбинаторика видов)
            let mut affixes: Vec<String> = Vec::new();
            if !cfg.affixes.is_empty() && rng.chance(elite_chance) {
                let take = 1 + rng.below(st.elite_affixes_max.max(1)) as usize;
                for _ in 0..take.min(cfg.affixes.len()) {
                    let a = &cfg.affixes[rng.below(cfg.affixes.len() as u32) as usize].id;
                    if !affixes.contains(a) {
                        affixes.push(a.clone());
                    }
                }
            }
            enemies.push(EnemySpawn {
                kind: kind.clone(),
                pos: spawn_at(
                    (px + ox).clamp(r.x + 1, r.x + r.w - 2),
                    (pz + oz).clamp(r.z + 1, r.z + r.h - 2),
                ),
                mult,
                is_boss: false,
                affixes,
            });
        }
        // Патроны: точки комнаты по шансам из loot.json (позиции чередуются)
        let ammo_spots = [(r.x + 1, r.z + r.h - 2), (r.x + r.w - 2, r.z + 1)];
        for (ai, chance) in cfg.loot.settings.room_ammo_chances.iter().enumerate() {
            if rng.chance(*chance) {
                let t = AmmoType::from_idx(rng.below(4) as usize);
                let (sx, sz) = ammo_spots[ai % ammo_spots.len()];
                ammo.push((t, t.pack_size(), spawn_at(sx, sz)));
            }
        }
        // Предметы комнаты — таблица room_items из loot.json
        let item_spots = [
            (r.x + r.w - 2, r.z + 1),
            (cx, cz + 1),
            (cx - 1, cz - 1),
            (cx - 1, cz),
            (cx + 1, cz - 1),
        ];
        for (li, entry) in cfg.loot.room_items.iter().enumerate() {
            if rng.chance(entry.chance) {
                let (sx, sz) = item_spots[li % item_spots.len()];
                items.push((entry.id.clone(), spawn_at(sx, sz)));
            }
        }
        if r.role == RoomRole::Treasure {
            if let Some(entry) = cfg
                .loot
                .room_items
                .get(rng.below(cfg.loot.room_items.len().max(1) as u32) as usize)
            {
                items.push((entry.id.clone(), spawn_at(cx, cz)));
            }
        }
    }

    // Оружейный тайник (пул из dungeon.json; неизвестные id пропускаются)
    let cache_pool: Vec<WeaponId> = st
        .weapon_cache
        .iter()
        .filter_map(|s| WeaponId::from_id(s))
        .collect();
    if rooms.len() > 2 && !cache_pool.is_empty() {
        let event_vaults: Vec<usize> = rooms
            .iter()
            .enumerate()
            .filter(|(index, room)| {
                *index != 0 && room.role == RoomRole::Treasure && room.special_event
            })
            .map(|(index, _)| index)
            .collect();
        let wk = if event_vaults.is_empty() {
            1 + rng.below((rooms.len() - 1) as u32) as usize
        } else {
            *rng.pick(&event_vaults)
        };
        let (cx, cz) = rooms[wk].center();
        let w = *rng.pick(&cache_pool);
        weapons.push((w, spawn_at(cx, cz.max(1))));
        if let Some((t, _)) = crate::weapon::weapon_def(w).ammo {
            ammo.push((t, t.pack_size(), spawn_at(cx + 1, cz)));
        }
    }

    // 5. Порталы
    let (ex, ez) = rooms[0].center();
    let entry_fy = rooms[0].floor_y;
    let player_spawn = cell_at(ex, ez, entry_fy + 1.1);
    let exit_portal = cell_at(rooms[0].x + 1, rooms[0].z + 1, entry_fy);
    let next_portal = cell_at(b_cx, b_cz - (rooms[boss_idx].h / 2 - 1).max(1), b_floor);

    for (pos, col) in [
        (exit_portal, Color::from_rgba(0.4, 0.9, 1.0, 1.0)),
        (next_portal, Color::from_rgba(1.0, 0.35, 0.75, 1.0)),
    ] {
        if let Some(mut sp) = make_billboard(
            cache,
            "res://assets/effects/effect_teleport.png",
            pos + Vector3::new(0.0, 1.3, 0.0),
            0.02,
        ) {
            sp.set_modulate(col);
            root.add_child(&sp);
        }
        let pl = make_light(pos + Vector3::new(0.0, 1.6, 0.0), col, 1.4, 6.5);
        root.add_child(&pl);
    }

    let mut events = Vec::new();
    for (room_index, room) in rooms.iter().enumerate() {
        let (center_x, center_z) = room.center();
        match room.role {
            RoomRole::Puzzle => {
                for (step, (offset_x, offset_z)) in
                    [(-1, -1), (1, -1), (0, 1)].into_iter().enumerate()
                {
                    if room.contains(center_x + offset_x, center_z + offset_z) {
                        events.push(DungeonEventSpawn {
                            kind: "puzzle_switch".to_string(),
                            group: room_index as u32,
                            step: step as u32,
                            pos: cell_at(
                                center_x + offset_x,
                                center_z + offset_z,
                                room.floor_y + 1.7,
                            ),
                        });
                    }
                }
            }
            RoomRole::Story => events.push(DungeonEventSpawn {
                kind: "story_echo".to_string(),
                group: room_index as u32,
                step: 0,
                pos: cell_at(center_x, center_z, room.floor_y + 1.25),
            }),
            _ => {}
        }
    }

    DungeonPlan {
        depth,
        theme_name: theme.name,
        root,
        player_spawn,
        exit_portal,
        next_portal,
        enemies,
        items,
        ammo,
        weapons,
        hazards,
        room_roles: rooms
            .iter()
            .map(|room| room.role.id().to_string())
            .collect(),
        room_markers: rooms
            .iter()
            .map(|room| {
                let (center_x, center_z) = room.center();
                (
                    room.role.id().to_string(),
                    cell_at(center_x, center_z, room.floor_y),
                )
            })
            .collect(),
        dressing_nodes,
        dressing_variants,
        events,
        floor_map: floor,
        floor_heights: fh,
    }
}
