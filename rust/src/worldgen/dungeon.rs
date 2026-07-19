//! Процедурный генератор данжей — Doom/Quake стиль.
//!
//! Комнаты связываются по близости (MST + петли), высоты пола (0 / 0.8)
//! назначаются по дереву и соединяются пологими пандусами (только когда коридор
//! достаточно длинный). Коридоры шириной 2. Пост-проверка достижимости
//! (flood-fill) не даёт спавнить врагов/лут в отрезанных карманах.

use godot::prelude::*;
use godot::classes::Node3D;

use crate::config::GameConfig;
use crate::gfx::{make_box, make_ramp, make_glow_slab, make_light, make_billboard, Rng, TexCache};
use crate::weapon::{AmmoType, WeaponId};

pub const CELL: f32   = 3.0;
pub const WALL_H: f32 = 3.4;   // высота стен по умолчанию
pub const GRID: usize = 44;

// ── Тема данжа (разрешённая из dungeon.json пресета) ─────────────────────────

struct Theme {
    name:    String,
    wall:    String,
    accent:  String,
    floor:   String,
    ceil:    String,
    lava:    String,
    light:   Color,
}

/// Короткое имя текстуры → путь (dtile_* → textures/dungeon и т.д.);
/// полные res://-пути пропускаются как есть.
fn resolve_tex(name: &str) -> String {
    if name.starts_with("res://") { name.to_string() } else { crate::map::tex_path(name) }
}

impl Theme {
    fn from_cfg(c: &crate::config::ThemeCfg) -> Self {
        Self {
            name:   c.name_ru.clone(),
            wall:   resolve_tex(&c.wall),
            accent: resolve_tex(&c.accent),
            floor:  resolve_tex(&c.floor),
            ceil:   resolve_tex(&c.ceil),
            lava:   resolve_tex(&c.lava),
            light:  Color::from_rgba(c.light[0], c.light[1], c.light[2], 1.0),
        }
    }
}

// ── Результат генерации ───────────────────────────────────────────────────────

pub struct EnemySpawn {
    pub kind:    String,
    pub pos:     Vector3,
    pub mult:    f32,
    pub is_boss: bool,
    /// id аффиксов элиты (пусто = обычный враг).
    pub affixes: Vec<String>,
}

pub struct DungeonPlan {
    pub depth:        u32,
    pub theme_name:   String,
    pub root:         Gd<Node3D>,
    pub player_spawn: Vector3,
    pub exit_portal:  Vector3,
    pub next_portal:  Vector3,
    pub enemies:      Vec<EnemySpawn>,
    pub items:        Vec<(String, Vector3)>,
    pub ammo:         Vec<(AmmoType, u32, Vector3)>,
    pub weapons:      Vec<(WeaponId, Vector3)>,
    pub floor_map:    Vec<bool>,   // GRID×GRID, true = проходимый пол
    pub floor_heights: Vec<f32>,   // GRID×GRID, высота пола клетки (для навигации)
}

// ── Комната ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Room {
    x: i32, z: i32, w: i32, h: i32,
    floor_y: f32,   // высота пола: 0.0 | 0.8 | 1.6
    wall_h:  f32,   // высота стен над полом: 2.4 | 3.4 | 4.8 | 5.5
}

impl Room {
    fn center(&self) -> (i32, i32) { (self.x + self.w / 2, self.z + self.h / 2) }
    fn overlaps(&self, o: &Room, pad: i32) -> bool {
        self.x - pad < o.x + o.w && self.x + self.w + pad > o.x &&
        self.z - pad < o.z + o.h && self.z + self.h + pad > o.z
    }
}

fn cell_at(i: i32, j: i32, y: f32) -> Vector3 {
    Vector3::new(
        (i as f32 + 0.5 - GRID as f32 * 0.5) * CELL,
        y,
        (j as f32 + 0.5 - GRID as f32 * 0.5) * CELL,
    )
}

// ── Вспомогательные функции carve ────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn carve_cell(floor: &mut [bool], fh: &mut [f32], cwh: &mut [f32], is_ramp: &mut [bool],
              is_room: &[bool], i: i32, j: i32, h: f32, wh: f32, ramp: bool) {
    if i < 1 || j < 1 || i >= GRID as i32 - 1 || j >= GRID as i32 - 1 { return; }
    let idx = j as usize * GRID + i as usize;
    floor[idx] = true;
    // Комнаты не перезаписываем (у них своя высота и это не пандус).
    if !is_room[idx] {
        fh[idx]  = h;
        cwh[idx] = wh;
        if ramp { is_ramp[idx] = true; }
    }
}

// ── Генерация ─────────────────────────────────────────────────────────────────

pub fn generate(depth: u32, seed: u64, cache: &mut TexCache, cfg: &GameConfig) -> DungeonPlan {
    let mut rng = Rng::new(seed ^ (depth as u64).wrapping_mul(0x9E3779B97F4A7C15));
    // темы/пулы/настройки — из dungeon.json пресета (config гарантирует непустоту)
    let dc = &cfg.dungeon;
    let theme = Theme::from_cfg(&dc.themes[((depth - 1) as usize) % dc.themes.len()]);

    // Уровни высот пола: только 0.0 и 0.8 — перепад, который пандус (>=3 клетки)
    // проходит пологим склоном (<= RAMP_STEP на клетку). 1.6 убран как источник
    // непроходимых обрывов.
    const CEIL_LEVELS: [f32; 5] = [3.4, 3.4, 4.8, 2.4, 3.4];

    // 1. Комнаты (высоту назначим позже — по дереву связей)
    let mut rooms: Vec<Room> = Vec::new();
    let target = 6 + (depth.min(6) as i32) / 2 + rng.range(0, 2);
    for _ in 0..120 {
        if rooms.len() as i32 >= target { break; }
        let w = rng.range(4, 9);
        let h = rng.range(4, 9);
        let x = rng.range(2, GRID as i32 - w - 3);
        let z = rng.range(2, GRID as i32 - h - 3);
        let wall_h = CEIL_LEVELS[rng.below(5) as usize];
        let r = Room { x, z, w, h, floor_y: 0.0, wall_h };
        if !rooms.iter().any(|o| r.overlaps(o, 2)) {
            rooms.push(r);
        }
    }
    let n = rooms.len();

    // Боссовая комната — самая дальняя от входа, всегда высокий потолок
    let (e_cx, e_cz) = rooms[0].center();
    let mut boss_idx = n - 1;
    let mut best_d = -1i32;
    for (k, r) in rooms.iter().enumerate() {
        if k == 0 { continue; }
        let (cx, cz) = r.center();
        let d = (cx - e_cx).pow(2) + (cz - e_cz).pow(2);
        if d > best_d { best_d = d; boss_idx = k; }
    }
    rooms[boss_idx].wall_h = 5.5;

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
            if !in_tree[a] { continue; }
            for b in 0..n {
                if in_tree[b] { continue; }
                let d = corridor_len(a, b);
                if d < best.0 { best = (d, a, b); }
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
            let b = if u == a { v } else if v == a { u } else { continue };
            if visited[b] { continue; }
            visited[b] = true;
            let can_ramp = corridor_len(a, b) >= 3;
            rooms[b].floor_y = if can_ramp && b != boss_idx && rng.chance(0.4) {
                if rooms[a].floor_y < 0.4 { 0.8 } else { 0.0 }
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
            if links.contains(&(a, b)) || links.contains(&(b, a)) { continue; }
            if (rooms[a].floor_y - rooms[b].floor_y).abs() > 0.05 { continue; }
            extra_cand.push((corridor_len(a, b), a, b));
        }
    }
    extra_cand.sort_by_key(|e| e.0);
    for &(_, a, b) in extra_cand.iter().take((n / 4).max(1)) {
        links.push((a, b));
    }

    // 2. Пол/высоты
    let mut floor   = vec![false; GRID * GRID];
    let mut fh      = vec![0.0f32; GRID * GRID];
    let mut cwh     = vec![WALL_H; GRID * GRID];
    let mut is_room = vec![false; GRID * GRID];
    let mut is_ramp = vec![false; GRID * GRID];

    // Фаза A: комнаты
    for r in &rooms {
        for j in r.z..r.z + r.h {
            for i in r.x..r.x + r.w {
                let ii = i as usize; let jj = j as usize;
                if ii < 1 || jj < 1 || ii >= GRID-1 || jj >= GRID-1 { continue; }
                let idx = jj * GRID + ii;
                floor[idx]   = true;
                fh[idx]      = r.floor_y;
                cwh[idx]     = r.wall_h;
                is_room[idx] = true;
            }
        }
    }

    // Фаза B: коридоры (L-образные, ширина 2). При разной высоте комнат весь
    // коридор — плавный пандус: высота линейно интерполируется по длине пути,
    // а сегменты записываются для отрисовки наклонными слэбами.
    let mut ramp_segs: Vec<(Vector3, Vector3)> = Vec::new(); // (низ, верх) поверхности
    for &(a, b) in &links {
        let ra = rooms[a]; let rb = rooms[b];
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
            carve_cell(&mut floor, &mut fh, &mut cwh, &mut is_ramp, &is_room, cx, az,     h, wh, ramp);
            carve_cell(&mut floor, &mut fh, &mut cwh, &mut is_ramp, &is_room, cx, az + 1, h, wh, ramp);
            cx += (bx - cx).signum();
            step += 1.0;
        }
        // Вертикальный сегмент (widen по X)
        let mut cz = az;
        while cz != bz {
            let h = lerp_h(step);
            carve_cell(&mut floor, &mut fh, &mut cwh, &mut is_ramp, &is_room, bx,     cz, h, wh, ramp);
            carve_cell(&mut floor, &mut fh, &mut cwh, &mut is_ramp, &is_room, bx + 1, cz, h, wh, ramp);
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

    let is_floor = |i: i32, j: i32| -> bool {
        i >= 0 && j >= 0 && (i as usize) < GRID && (j as usize) < GRID
            && floor[j as usize * GRID + i as usize]
    };
    let get_fh = |i: i32, j: i32| -> f32 {
        if i < 0 || j < 0 || i as usize >= GRID || j as usize >= GRID { 0.0 }
        else { fh[j as usize * GRID + i as usize] }
    };
    let get_cwh = |i: i32, j: i32| -> f32 {
        if i < 0 || j < 0 || i as usize >= GRID || j as usize >= GRID { WALL_H }
        else { cwh[j as usize * GRID + i as usize] }
    };
    let is_ramp_at = |i: i32, j: i32| -> bool {
        i >= 0 && j >= 0 && (i as usize) < GRID && (j as usize) < GRID
            && is_ramp[j as usize * GRID + i as usize]
    };

    // 3. Геометрия
    let mut root = Node3D::new_alloc();
    let t_wall   = cache.get(&theme.wall);
    let t_accent = cache.get(&theme.accent);
    let t_floor  = cache.get(&theme.floor);
    let t_ceil   = cache.get(&theme.ceil);
    let t_lava   = cache.get(&theme.lava);
    let c_dark = Color::from_rgba(0.07, 0.04, 0.07, 1.0);
    const T: f32 = 0.3;

    // ── 3a. Пол и потолок — полосами, только одинаковая высота ──────────────
    for j in 0..GRID as i32 {
        let mut i = 0i32;
        while i < GRID as i32 {
            if is_floor(i, j) {
                let start = i;
                let h0  = get_fh(i, j);
                let wh0 = get_cwh(i, j);
                let ramp0 = is_ramp_at(i, j);
                while i < GRID as i32 && is_floor(i, j)
                    && (get_fh(i, j) - h0).abs() < 0.01
                    && (get_cwh(i, j) - wh0).abs() < 0.01
                    && is_ramp_at(i, j) == ramp0
                { i += 1; }
                let len = (i - start) as f32 * CELL;
                let cx = ((start + i) as f32 * 0.5 - GRID as f32 * 0.5) * CELL;
                let cz = (j as f32 + 0.5 - GRID as f32 * 0.5) * CELL;
                let uv = (len / CELL).max(1.0);
                // Пол пандус-клеток заменяет наклонный слэб (см. 3e) — здесь только потолок.
                if !ramp0 {
                    let fl = make_box(Vector3::new(cx, h0 - 0.15, cz),
                                      Vector3::new(len + 0.02, 0.3, CELL + 0.02),
                                      c_dark, t_floor.as_ref(), uv);
                    root.add_child(&fl);
                }
                let ce = make_box(Vector3::new(cx, h0 + wh0 + 0.15, cz),
                                  Vector3::new(len + 0.02, 0.3, CELL + 0.02),
                                  c_dark, t_ceil.as_ref(), uv);
                root.add_child(&ce);
            } else { i += 1; }
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
                    let h0  = get_fh(i, j);
                    let wh0 = get_cwh(i, j);
                    while i < GRID as i32 && is_floor(i, j) && !is_floor(i, j + side)
                        && (get_fh(i, j) - h0).abs() < 0.01
                        && (get_cwh(i, j) - wh0).abs() < 0.01
                    { i += 1; }
                    let len = (i - start) as f32 * CELL;
                    let cx = ((start + i) as f32 * 0.5 - GRID as f32 * 0.5) * CELL;
                    let cz = (j as f32 + 0.5 - GRID as f32 * 0.5) * CELL + side as f32 * CELL * 0.5;
                    let use_accent = rng.chance(0.12);
                    let tex = if use_accent { t_accent.as_ref() } else { t_wall.as_ref() };
                    let w = make_box(Vector3::new(cx, h0 + wh0 * 0.5, cz),
                                     Vector3::new(len + T, wh0, T), c_dark, tex,
                                     (len / CELL).max(1.0));
                    root.add_child(&w);
                } else { i += 1; }
            }
            // --- Ступенчатые стены (два пола на разных высотах) ---
            i = 0;
            while i < GRID as i32 {
                let h_me = get_fh(i, j);
                let h_nb = get_fh(i, j + side);
                // Рисуем ступень только от ВЫСОКОЙ клетки (избегаем дублей)
                if is_floor(i, j) && is_floor(i, j + side) && h_me > h_nb + 0.05
                    && !is_ramp_at(i, j) && !is_ramp_at(i, j + side) {
                    let start = i;
                    let step_h = h_me - h_nb;
                    while i < GRID as i32
                        && is_floor(i, j) && is_floor(i, j + side)
                        && !is_ramp_at(i, j) && !is_ramp_at(i, j + side)
                        && (get_fh(i, j) - h_me).abs() < 0.01
                        && (get_fh(i, j + side) - h_nb).abs() < 0.01
                    { i += 1; }
                    let len = (i - start) as f32 * CELL;
                    let cx = ((start + i) as f32 * 0.5 - GRID as f32 * 0.5) * CELL;
                    let cz = (j as f32 + 0.5 - GRID as f32 * 0.5) * CELL + side as f32 * CELL * 0.5;
                    let sw = make_box(Vector3::new(cx, h_nb + step_h * 0.5, cz),
                                      Vector3::new(len + T, step_h + 0.02, T),
                                      c_dark, t_wall.as_ref(), (len / CELL).max(1.0));
                    root.add_child(&sw);
                } else { i += 1; }
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
                    let h0  = get_fh(i, j);
                    let wh0 = get_cwh(i, j);
                    while j < GRID as i32 && is_floor(i, j) && !is_floor(i + side, j)
                        && (get_fh(i, j) - h0).abs() < 0.01
                        && (get_cwh(i, j) - wh0).abs() < 0.01
                    { j += 1; }
                    let len = (j - start) as f32 * CELL;
                    let cz = ((start + j) as f32 * 0.5 - GRID as f32 * 0.5) * CELL;
                    let cx = (i as f32 + 0.5 - GRID as f32 * 0.5) * CELL + side as f32 * CELL * 0.5;
                    let use_accent = rng.chance(0.12);
                    let tex = if use_accent { t_accent.as_ref() } else { t_wall.as_ref() };
                    let w = make_box(Vector3::new(cx, h0 + wh0 * 0.5, cz),
                                     Vector3::new(T, wh0, len + T), c_dark, tex,
                                     (len / CELL).max(1.0));
                    root.add_child(&w);
                } else { j += 1; }
            }
            // --- Ступенчатые стены ---
            j = 0;
            while j < GRID as i32 {
                let h_me = get_fh(i, j);
                let h_nb = get_fh(i + side, j);
                if is_floor(i, j) && is_floor(i + side, j) && h_me > h_nb + 0.05
                    && !is_ramp_at(i, j) && !is_ramp_at(i + side, j) {
                    let start = j;
                    let step_h = h_me - h_nb;
                    while j < GRID as i32
                        && is_floor(i, j) && is_floor(i + side, j)
                        && !is_ramp_at(i, j) && !is_ramp_at(i + side, j)
                        && (get_fh(i, j) - h_me).abs() < 0.01
                        && (get_fh(i + side, j) - h_nb).abs() < 0.01
                    { j += 1; }
                    let len = (j - start) as f32 * CELL;
                    let cz = ((start + j) as f32 * 0.5 - GRID as f32 * 0.5) * CELL;
                    let cx = (i as f32 + 0.5 - GRID as f32 * 0.5) * CELL + side as f32 * CELL * 0.5;
                    let sw = make_box(Vector3::new(cx, h_nb + step_h * 0.5, cz),
                                      Vector3::new(T, step_h + 0.02, len + T),
                                      c_dark, t_wall.as_ref(), (len / CELL).max(1.0));
                    root.add_child(&sw);
                } else { j += 1; }
            }
        }
    }

    // ── 3e. Слэбы-пандусы (пологие склоны между уровнями) ────────────────────
    for &(low, high) in &ramp_segs {
        let slab = make_ramp(low, high, CELL * 2.0, T, c_dark, t_floor.as_ref());
        root.add_child(&slab);
    }

    // ── 3d. Детали комнат ────────────────────────────────────────────────────
    for (k, r) in rooms.iter().enumerate() {
        let (cx, cz) = r.center();
        let center = cell_at(cx, cz, r.floor_y);
        let is_boss_room = k == boss_idx;

        // Свет под потолком
        let light_y = r.floor_y + r.wall_h - 0.6;
        let range = (r.w.max(r.h) as f32) * CELL * 1.4;
        let energy = if is_boss_room { 2.6 } else { 1.7 };
        let color = if is_boss_room { Color::from_rgba(0.95, 0.12, 0.18, 1.0) } else { theme.light };
        let l = make_light(Vector3::new(center.x, light_y, center.z), color, energy, range);
        root.add_child(&l);

        // Факел у стены
        if rng.chance(0.85) {
            let ti = r.x + 1 + rng.range(0, (r.w - 2).max(0));
            let torch_y = r.floor_y + 1.5;
            let pos = cell_at(ti, r.z, torch_y) + Vector3::new(0.0, 0.0, -CELL * 0.28);
            if let Some(sp) = make_billboard(cache, "res://assets/sprites/pickups/soul.png", pos, 0.012) {
                root.add_child(&sp);
            }
            let tl = make_light(pos, theme.light, 0.9, 7.0);
            root.add_child(&tl);
        }

        // Лужа лавы (только в несунутых комнатах)
        if !is_boss_room && k != 0 && rng.chance(0.35) && r.w >= 5 && r.h >= 5 {
            let lp = cell_at(cx + rng.range(-1, 1), cz + rng.range(-1, 1), r.floor_y);
            let slab = make_glow_slab(
                lp + Vector3::new(0.0, 0.03, 0.0),
                Vector3::new(CELL * 1.6, 0.06, CELL * 1.6),
                t_lava.as_ref(),
                Color::from_rgba(0.9, 0.25, 0.45, 1.0), 1.0,
            );
            root.add_child(&slab);
            let ll = make_light(lp + Vector3::new(0.0, 0.8, 0.0),
                                Color::from_rgba(1.0, 0.3, 0.5, 1.0), 1.1, 6.0);
            root.add_child(&ll);
        }

        // Колонны в больших комнатах
        if r.w >= 6 && r.h >= 6 && rng.chance(0.6) {
            for (dx, dz) in [(-2i32, -2i32), (2, 2), (-2, 2), (2, -2)] {
                let p = cell_at(cx + dx, cz + dz, r.floor_y);
                let col = make_box(p + Vector3::new(0.0, r.wall_h * 0.5, 0.0),
                                   Vector3::new(0.7, r.wall_h, 0.7),
                                   c_dark, t_wall.as_ref(), 1.0);
                root.add_child(&col);
            }
        }

        // Поднятая платформа внутри больших комнат (тактическая геометрия)
        if r.w >= 7 && r.h >= 7 && rng.chance(0.55) && r.floor_y < 1.6 {
            let plat_y = r.floor_y + 0.8;
            let pw = rng.range(2, (r.w - 3).max(2)) as f32 * CELL;
            let ph = rng.range(2, (r.h - 3).max(2)) as f32 * CELL;
            let pc = cell_at(cx + rng.range(-1, 1), cz + rng.range(-1, 1), plat_y);
            let plat = make_box(pc + Vector3::new(0.0, -0.15, 0.0),
                                Vector3::new(pw, 0.3 + plat_y - r.floor_y, ph),
                                c_dark, t_floor.as_ref(), (pw / CELL).max(1.0));
            root.add_child(&plat);
            // Свет под потолком над платформой
            let pl = make_light(pc + Vector3::new(0.0, r.wall_h - 0.5, 0.0),
                                theme.light, 0.7, pw + 2.0);
            root.add_child(&pl);
        }
    }

    // Алтарь в боссовой комнате
    let (b_cx, b_cz) = rooms[boss_idx].center();
    let b_floor = rooms[boss_idx].floor_y;
    let boss_center = cell_at(b_cx, b_cz, b_floor);
    let alt = make_box(boss_center + Vector3::new(0.0, 0.35, 0.0),
                       Vector3::new(2.6, 0.7, 1.4), c_dark, t_accent.as_ref(), 1.0);
    root.add_child(&alt);

    // 4. Спавны (Y = высота пола в точке спавна)
    let mut enemies: Vec<EnemySpawn> = Vec::new();
    let mut items:   Vec<(String, Vector3)> = Vec::new();
    let mut ammo:    Vec<(AmmoType, u32, Vector3)> = Vec::new();
    let mut weapons: Vec<(WeaponId, Vector3)> = Vec::new();

    let st = &dc.settings;
    let mult = 1.0 + (depth - 1) as f32 * st.mult_per_depth;
    // Пул врагов: самый глубокий из подходящих по min_depth
    let pool: &[String] = dc.pools.iter()
        .filter(|p| p.min_depth <= depth)
        .max_by_key(|p| p.min_depth)
        .or_else(|| dc.pools.first())
        .map(|p| p.enemies.as_slice())
        .unwrap_or(&[]);

    // Достижимость: flood-fill от клетки спавна игрока по ТЕМ ЖЕ правилам, что и
    // навигация (перепад <= RAMP_STEP). В недостижимых карманах не спавним ничего
    // — иначе враги оказываются «за стеной»/«на крыше» и до них не дойти.
    let reachable = {
        let mut reach = vec![false; GRID * GRID];
        let (sx, sz) = centers[0];
        let start = sz as usize * GRID + sx as usize;
        if floor.get(start).copied().unwrap_or(false) {
            reach[start] = true;
            let mut stack = vec![start];
            while let Some(c) = stack.pop() {
                let (ci, cj) = ((c % GRID) as i32, (c / GRID) as i32);
                for (di, dj) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (ni, nj) = (ci + di, cj + dj);
                    if ni < 0 || nj < 0 || ni as usize >= GRID || nj as usize >= GRID { continue; }
                    let nidx = nj as usize * GRID + ni as usize;
                    if reach[nidx] || !floor[nidx] { continue; }
                    if (fh[nidx] - fh[c]).abs() > crate::nav::RAMP_STEP { continue; }
                    reach[nidx] = true;
                    stack.push(nidx);
                }
            }
        }
        reach
    };
    let room_reachable = |k: usize| -> bool {
        let (cx, cz) = centers[k];
        reachable.get(cz as usize * GRID + cx as usize).copied().unwrap_or(false)
    };

    for (k, r) in rooms.iter().enumerate() {
        if k == 0 { continue; }
        if !room_reachable(k) { continue; }
        let (cx, cz) = r.center();
        let fy = r.floor_y;
        let is_boss_room = k == boss_idx;

        if is_boss_room {
            enemies.push(EnemySpawn { kind: st.boss.clone(), pos: boss_center,
                                      mult: mult * st.boss_mult, is_boss: true,
                                      affixes: Vec::new() });
            // свита по бокам алтаря
            for (gi, guard) in st.boss_guards.iter().enumerate() {
                let d = if gi % 2 == 0 { -2 - (gi as i32 / 2) } else { 2 + (gi as i32 / 2) };
                enemies.push(EnemySpawn { kind: guard.clone(),
                    pos: cell_at(cx + d, cz, fy), mult, is_boss: false,
                    affixes: Vec::new() });
            }
            // награда рядком за алтарём: центр, слева, справа, дальше наружу
            for (ii, item) in st.boss_items.iter().enumerate() {
                let k = (ii as i32 + 1) / 2;
                let dx = if ii % 2 == 0 { k } else { -k };
                items.push((item.clone(), cell_at(cx + dx, cz + 1, fy)));
            }
            continue;
        }

        // Враги: 2–5 на комнату (растёт с глубиной)
        let n = 2 + rng.range(0, 2 + (depth.min(6) as i32));
        let elite_chance = (st.elite_chance + st.elite_per_depth * (depth - 1) as f32)
            .clamp(0.0, 0.6);
        for idx in 0..n {
            let Some(kind) = (!pool.is_empty()).then(|| rng.pick(pool)) else { break };
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
                pos: cell_at((px + ox).clamp(r.x + 1, r.x + r.w - 2),
                             (pz + oz).clamp(r.z + 1, r.z + r.h - 2), fy),
                mult, is_boss: false, affixes,
            });
        }
        // Патроны: точки комнаты по шансам из loot.json (позиции чередуются)
        let ammo_spots = [(r.x + 1, r.z + r.h - 2), (r.x + r.w - 2, r.z + 1)];
        for (ai, chance) in cfg.loot.settings.room_ammo_chances.iter().enumerate() {
            if rng.chance(*chance) {
                let t = AmmoType::from_idx(rng.below(4) as usize);
                let (sx, sz) = ammo_spots[ai % ammo_spots.len()];
                ammo.push((t, t.pack_size(), cell_at(sx, sz, fy)));
            }
        }
        // Предметы комнаты — таблица room_items из loot.json
        let item_spots = [
            (r.x + r.w - 2, r.z + 1), (cx, cz + 1), (cx - 1, cz - 1),
            (cx - 1, cz),             (cx + 1, cz - 1),
        ];
        for (li, entry) in cfg.loot.room_items.iter().enumerate() {
            if rng.chance(entry.chance) {
                let (sx, sz) = item_spots[li % item_spots.len()];
                items.push((entry.id.clone(), cell_at(sx, sz, fy)));
            }
        }
    }

    // Оружейный тайник (пул из dungeon.json; неизвестные id пропускаются)
    let cache_pool: Vec<WeaponId> = st.weapon_cache.iter()
        .filter_map(|s| WeaponId::from_id(s))
        .collect();
    if rooms.len() > 2 && !cache_pool.is_empty() {
        let wk = 1 + rng.below((rooms.len() - 1) as u32) as usize;
        let (cx, cz) = rooms[wk].center();
        let fy = rooms[wk].floor_y;
        let w = *rng.pick(&cache_pool);
        weapons.push((w, cell_at(cx, cz.max(1), fy)));
        if let Some((t, _)) = crate::weapon::weapon_def(w).ammo {
            ammo.push((t, t.pack_size(), cell_at(cx + 1, cz, fy)));
        }
    }

    // 5. Порталы
    let (ex, ez) = rooms[0].center();
    let entry_fy  = rooms[0].floor_y;
    let player_spawn = cell_at(ex, ez, entry_fy + 1.1);
    let exit_portal  = cell_at(rooms[0].x + 1, rooms[0].z + 1, entry_fy);
    let next_portal  = cell_at(b_cx, b_cz - (rooms[boss_idx].h / 2 - 1).max(1), b_floor);

    for (pos, col) in [
        (exit_portal, Color::from_rgba(0.4, 0.9, 1.0, 1.0)),
        (next_portal, Color::from_rgba(1.0, 0.35, 0.75, 1.0)),
    ] {
        if let Some(mut sp) = make_billboard(cache, "res://assets/effects/effect_teleport.png",
                                             pos + Vector3::new(0.0, 1.3, 0.0), 0.02) {
            sp.set_modulate(col);
            root.add_child(&sp);
        }
        let pl = make_light(pos + Vector3::new(0.0, 1.6, 0.0), col, 1.4, 6.5);
        root.add_child(&pl);
    }

    DungeonPlan {
        depth, theme_name: theme.name, root,
        player_spawn, exit_portal, next_portal,
        enemies, items, ammo, weapons,
        floor_map: floor,
        floor_heights: fh,
    }
}
