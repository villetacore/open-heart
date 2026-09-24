//! Генерация данжа: план.
//!
//! Комнаты, коридоры, пандусы, сетка проходимости и высоты — чистая функция от
//! `(depth, seed, config)`. Геометрию по этому плану строит клиент
//! (`client/src/worldgen/dungeon.rs`), а сервер берёт отсюда сетку для навигации
//! врагов (docs/MULTIPLAYER.md §5).

use crate::config::{GameConfig, RoomArchetypeCfg};
use crate::weapon::{AmmoType, WeaponId};
use crate::math::Vec3;
use crate::rng::Rng;
use crate::worldgen::{CELL, GRID};

/// Высота стен по умолчанию.
pub const WALL_H: f32 = 3.4;

/// План данжа: всё, что нужно и клиенту (строить геометрию), и серверу
/// (строить навигацию и водить врагов).
pub struct DungeonLayout {
    pub depth: u32,
    /// Индекс темы в `dungeon.json` — сами текстуры разрешает клиент.
    pub theme_index: usize,
    pub rooms: Vec<Room>,
    /// Индекс комнаты-арены босса в `rooms`.
    pub boss_idx: usize,
    /// Враги, лут, порталы и события — то, что раздаёт сервер.
    pub spawns: Spawns,
    /// Центры комнат в клетках сетки.
    pub centers: Vec<(i32, i32)>,
    /// GRID×GRID: true — проходимый пол.
    pub floor: Vec<bool>,
    /// GRID×GRID: высота пола клетки.
    pub floor_heights: Vec<f32>,
    /// GRID×GRID: высота стен над полом клетки.
    pub ceil_heights: Vec<f32>,
    /// GRID×GRID: клетка принадлежит пандусу.
    pub is_ramp: Vec<bool>,
    /// GRID×GRID: клетка принадлежит комнате (не коридору).
    pub is_room: Vec<bool>,
    /// Отрезки наклонных плит пандусов (от нижней точки к верхней).
    pub ramp_segs: Vec<(Vec3, Vec3)>,
    /// GRID×GRID: достижимо ли из стартовой комнаты.
    pub reachable: Vec<bool>,
    /// Состояние генератора после планирования: клиент продолжает поток с него,
    /// поэтому декор и спавны не меняются от того, что план уехал в ядро.
    pub rng: Rng,
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

    /// Цвет зоны в RGB; движковый Color из него делает клиент.
    pub fn color(self) -> [f32; 3] {
        match self {
            Self::Blood => [0.95, 0.16, 0.36],
            Self::Void => [0.62, 0.18, 1.0],
            Self::Electric => [0.1, 0.8, 1.0],
            Self::Embers => [1.0, 0.34, 0.08],
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

// ── Комната ───────────────────────────────────────────────────────────────────

/// Форма комнаты в пределах её bounding-box (разнообразит «простые прямоугольники»).
#[derive(Clone, Copy, PartialEq)]
pub enum Shape {
    Rect,
    Round,
    Octagon,
    Cross,
}

#[derive(Clone, Copy, PartialEq)]
pub enum RoomRole {
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
    pub fn id(self) -> &'static str {
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
pub enum RoomDecor {
    Basic,
    Pipes,
    Ossuary,
    Crystals,
    Machinery,
    Archive,
    Sanctuary,
}

#[derive(Clone, Copy)]
pub enum FocalPattern {
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
    pub fn id(self) -> &'static str {
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
pub struct Room {
    pub x: i32,
    pub z: i32,
    pub w: i32,
    pub h: i32,
    pub floor_y: f32, // высота пола: 0.0 | 0.8
    pub wall_h: f32,  // высота стен над полом: 2.4 | 3.4 | 4.8 | 5.5
    pub shape: Shape,
    pub role: RoomRole,
    pub decor: RoomDecor,
    pub focal_pattern: FocalPattern,
    pub focal_color: [f32; 3],
    pub focal_scale: f32,
    pub focal_height: f32,
    pub hazard_chance: f32,
    pub hazard_dps: f32,
    pub hazard_radius: f32,
    pub hazard_kind: HazardKind,
    pub special_event: bool,
}

impl Room {
    pub fn center(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.z + self.h / 2)
    }
    pub fn overlaps(&self, o: &Room, pad: i32) -> bool {
        self.x - pad < o.x + o.w
            && self.x + self.w + pad > o.x
            && self.z - pad < o.z + o.h
            && self.z + self.h + pad > o.z
    }

    /// Входит ли клетка (i,j) в форму комнаты. Центр и центральные ось/ряд всегда
    /// внутри — коридоры цепляются к центру, форма не должна их отрезать.
    pub fn contains(&self, i: i32, j: i32) -> bool {
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

pub fn cell_at(i: i32, j: i32, y: f32) -> Vec3 {
    Vec3::new(
        (i as f32 + 0.5 - GRID as f32 * 0.5) * CELL,
        y,
        (j as f32 + 0.5 - GRID as f32 * 0.5) * CELL,
    )
}

pub fn flood_walkable(floor: &[bool], heights: &[f32], start: (i32, i32)) -> Vec<bool> {
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
pub fn carve_cell(
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

/// Построить план данжа: комнаты, коридоры, сетку проходимости и высоты.
///
/// Чистая функция от `(depth, seed, cfg)` — сервер и клиент получают из одного
/// сида один и тот же данж.
pub fn plan(depth: u32, seed: u64, cfg: &GameConfig) -> DungeonLayout {
    let mut rng = Rng::new(seed ^ (depth as u64).wrapping_mul(0x9E3779B97F4A7C15));
    // темы/пулы/настройки — из dungeon.json пресета (config гарантирует непустоту)
    let dc = &cfg.dungeon;
    let theme_index = ((depth - 1) as usize) % dc.themes.len();

    // Уровни высот пола: только 0.0 и 0.8 — перепад, который пандус (>=3 клетки)
    // проходит пологим склоном (<= RAMP_STEP на клетку). 1.6 убран как источник
    // непроходимых обрывов.
    const CEIL_LEVELS: [f32; 5] = [3.4, 3.4, 4.8, 2.4, 3.4];

    // 1. Комнаты (высоту назначим позже — по дереву связей)
    let mut rooms: Vec<Room> = Vec::new();
    let target = 8 + depth.min(3) as i32 + rng.range(0, 2);
    for _ in 0..480 {
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
    // Every mandatory beat needs a distinct room. Crowded random packing used
    // to silently omit story/puzzle rooms, making their quests impossible.
    if rooms.len() < 8 {
        let template = rooms[0];
        rooms.clear();
        for index in 0..9 {
            let mut room = template;
            room.x = 3 + (index % 3) * 13;
            room.z = 3 + (index / 3) * 13;
            room.w = 8;
            room.h = 8;
            room.shape = Shape::Rect;
            room.role = RoomRole::Arena;
            rooms.push(room);
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
    let mut ramp_segs: Vec<(Vec3, Vec3)> = Vec::new(); // (низ, верх) поверхности
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
            let off_z = Vec3::new(0.0, 0.0, CELL * 0.5);
            let off_x = Vec3::new(CELL * 0.5, 0.0, 0.0);
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

    let spawns = populate(
        depth, cfg, &mut rng, &rooms, &centers, boss_idx, &floor, &fh, &reachable,
    );

    DungeonLayout {
        depth,
        theme_index,
        rooms,
        boss_idx,
        spawns,
        centers,
        floor,
        floor_heights: fh,
        ceil_heights: cwh,
        is_ramp,
        is_room,
        ramp_segs,
        reachable,
        rng,
    }
}

// ── Спавны ───────────────────────────────────────────────────────────────────

/// Точка появления врага.
#[derive(Clone, Debug)]
pub struct EnemySpawn {
    pub kind: String,
    pub pos: Vec3,
    /// Множитель статов от глубины (и отдельно — от «босс»-тира).
    pub mult: f32,
    pub is_boss: bool,
    /// id аффиксов элиты (пусто = обычный враг).
    pub affixes: Vec<String>,
}

/// Интерактивное событие комнаты (переключатель головоломки, эхо-сцена).
#[derive(Clone, Debug)]
pub struct DungeonEventSpawn {
    pub kind: String,
    pub group: u32,
    pub step: u32,
    pub pos: Vec3,
}

/// Всё, что появляется в данже: враги, лут, порталы, события.
///
/// Считается в ядре, потому что раздавать это обязан сервер — иначе у каждого
/// клиента был бы свой набор врагов (docs/MULTIPLAYER.md §5).
#[derive(Clone, Debug, Default)]
pub struct Spawns {
    pub enemies: Vec<EnemySpawn>,
    pub items: Vec<(String, Vec3)>,
    pub ammo: Vec<(AmmoType, u32, Vec3)>,
    pub weapons: Vec<(WeaponId, Vec3)>,
    pub events: Vec<DungeonEventSpawn>,
    pub player_spawn: Vec3,
    pub exit_portal: Vec3,
    pub next_portal: Vec3,
    pub room_roles: Vec<String>,
    pub room_markers: Vec<(String, Vec3)>,
}

#[allow(clippy::too_many_arguments)]
fn populate(
    depth: u32,
    cfg: &GameConfig,
    rng: &mut Rng,
    rooms: &[Room],
    centers: &[(i32, i32)],
    boss_idx: usize,
    floor: &[bool],
    fh: &[f32],
    reachable: &[bool],
) -> Spawns {
    let dc = &cfg.dungeon;
    let (b_cx, b_cz) = rooms[boss_idx].center();
    let b_floor = rooms[boss_idx].floor_y;
    let boss_center = cell_at(b_cx, b_cz, b_floor);
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

    // 4. Спавны (Y = высота пола в точке спавна)
    let mut enemies: Vec<EnemySpawn> = Vec::new();
    let mut items: Vec<(String, Vec3)> = Vec::new();
    let mut ammo: Vec<(AmmoType, u32, Vec3)> = Vec::new();
    let mut weapons: Vec<(WeaponId, Vec3)> = Vec::new();

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
    let spawn_at = |i: i32, j: i32| -> Vec3 {
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

    let (ex, ez) = rooms[0].center();
    let entry_fy = rooms[0].floor_y;
    let player_spawn = cell_at(ex, ez, entry_fy + 1.1);
    let exit_portal = cell_at(rooms[0].x + 1, rooms[0].z + 1, entry_fy);
    let next_portal = cell_at(b_cx, b_cz - (rooms[boss_idx].h / 2 - 1).max(1), b_floor);

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

    Spawns {
        enemies,
        items,
        ammo,
        weapons,
        events,
        player_spawn,
        exit_portal,
        next_portal,
        room_roles: rooms.iter().map(|room| room.role.id().to_string()).collect(),
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
    }
}
