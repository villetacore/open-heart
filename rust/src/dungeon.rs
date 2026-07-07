//! Процедурный генератор данжей — Doom/Quake стиль.
//!
//! Комнаты на разных высотах (0, 0.8, 1.6), переменная высота потолка,
//! ступенчатые стены на границах уровней, платформы в больших комнатах.

use godot::prelude::*;
use godot::classes::Node3D;

use crate::config::GameConfig;
use crate::gfx::{make_box, make_glow_slab, make_light, make_billboard, Rng, TexCache};
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
fn carve_cell(floor: &mut [bool], fh: &mut [f32], cwh: &mut [f32],
              is_room: &[bool], i: i32, j: i32, h: f32, wh: f32) {
    if i < 1 || j < 1 || i >= GRID as i32 - 1 || j >= GRID as i32 - 1 { return; }
    let idx = j as usize * GRID + i as usize;
    floor[idx] = true;
    if !is_room[idx] {
        fh[idx]  = h;
        cwh[idx] = wh;
    }
}

// ── Генерация ─────────────────────────────────────────────────────────────────

pub fn generate(depth: u32, seed: u64, cache: &mut TexCache, cfg: &GameConfig) -> DungeonPlan {
    let mut rng = Rng::new(seed ^ (depth as u64).wrapping_mul(0x9E3779B97F4A7C15));
    // темы/пулы/настройки — из dungeon.json пресета (config гарантирует непустоту)
    let dc = &cfg.dungeon;
    let theme = Theme::from_cfg(&dc.themes[((depth - 1) as usize) % dc.themes.len()]);

    // Возможные высоты пола: сильный перевес в пользу 0.0
    const FLOOR_LEVELS: [f32; 7] = [0.0, 0.0, 0.0, 0.8, 0.8, 1.6, 0.0];
    // Возможные высоты потолка
    const CEIL_LEVELS:  [f32; 5] = [3.4, 3.4, 4.8, 2.4, 3.4];

    // 1. Комнаты
    let mut rooms: Vec<Room> = Vec::new();
    let target = 6 + (depth.min(6) as i32) / 2 + rng.range(0, 2);
    for _ in 0..120 {
        if rooms.len() as i32 >= target { break; }
        let w = rng.range(4, 9);
        let h = rng.range(4, 9);
        let x = rng.range(2, GRID as i32 - w - 3);
        let z = rng.range(2, GRID as i32 - h - 3);
        // Стартовая комната всегда на полу 0.0
        let floor_y = if rooms.is_empty() {
            0.0
        } else {
            FLOOR_LEVELS[rng.below(7) as usize]
        };
        let wall_h = CEIL_LEVELS[rng.below(5) as usize];
        let r = Room { x, z, w, h, floor_y, wall_h };
        if !rooms.iter().any(|o| r.overlaps(o, 2)) {
            rooms.push(r);
        }
    }

    // Боссовая комната — самая дальняя от входа, всегда высокий потолок
    let (e_cx, e_cz) = rooms[0].center();
    let mut boss_idx = rooms.len() - 1;
    let mut best_d = -1i32;
    for (k, r) in rooms.iter().enumerate() {
        if k == 0 { continue; }
        let (cx, cz) = r.center();
        let d = (cx - e_cx).pow(2) + (cz - e_cz).pow(2);
        if d > best_d { best_d = d; boss_idx = k; }
    }
    rooms[boss_idx].wall_h = 5.5;

    // 2. Пол/высоты — два прохода: комнаты, потом коридоры
    let mut floor   = vec![false; GRID * GRID];
    let mut fh      = vec![0.0f32; GRID * GRID];
    let mut cwh     = vec![WALL_H; GRID * GRID];
    let mut is_room = vec![false; GRID * GRID];

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

    // Фаза B: коридоры (L-образные)
    let mut links: Vec<(usize, usize)> = (1..rooms.len()).map(|i| (i - 1, i)).collect();
    if rooms.len() > 3 { links.push((0, rooms.len() - 1)); }

    for (a, b) in &links {
        let ra = rooms[*a]; let rb = rooms[*b];
        let (ax, az) = ra.center();
        let (bx, bz) = rb.center();
        let wide = rng.chance(0.35);
        // Горизонтальный сегмент → высота комнаты A
        let h_a = ra.floor_y; let wh_a = ra.wall_h.min(rb.wall_h);
        // Вертикальный сегмент → высота комнаты B
        let h_b = rb.floor_y;
        let mut cx = ax;
        while cx != bx {
            carve_cell(&mut floor, &mut fh, &mut cwh, &is_room, cx, az, h_a, wh_a);
            if wide { carve_cell(&mut floor, &mut fh, &mut cwh, &is_room, cx, az + 1, h_a, wh_a); }
            cx += (bx - cx).signum();
        }
        let mut cz = az;
        while cz != bz {
            carve_cell(&mut floor, &mut fh, &mut cwh, &is_room, bx, cz, h_b, wh_a);
            if wide { carve_cell(&mut floor, &mut fh, &mut cwh, &is_room, bx + 1, cz, h_b, wh_a); }
            cz += (bz - cz).signum();
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
                while i < GRID as i32 && is_floor(i, j)
                    && (get_fh(i, j) - h0).abs() < 0.01
                    && (get_cwh(i, j) - wh0).abs() < 0.01
                { i += 1; }
                let len = (i - start) as f32 * CELL;
                let cx = ((start + i) as f32 * 0.5 - GRID as f32 * 0.5) * CELL;
                let cz = (j as f32 + 0.5 - GRID as f32 * 0.5) * CELL;
                let uv = (len / CELL).max(1.0);
                let fl = make_box(Vector3::new(cx, h0 - 0.15, cz),
                                  Vector3::new(len + 0.02, 0.3, CELL + 0.02),
                                  c_dark, t_floor.as_ref(), uv);
                root.add_child(&fl);
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
                if is_floor(i, j) && is_floor(i, j + side) && h_me > h_nb + 0.05 {
                    let start = i;
                    let step_h = h_me - h_nb;
                    while i < GRID as i32
                        && is_floor(i, j) && is_floor(i, j + side)
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
                if is_floor(i, j) && is_floor(i + side, j) && h_me > h_nb + 0.05 {
                    let start = j;
                    let step_h = h_me - h_nb;
                    while j < GRID as i32
                        && is_floor(i, j) && is_floor(i + side, j)
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

    for (k, r) in rooms.iter().enumerate() {
        if k == 0 { continue; }
        let (cx, cz) = r.center();
        let fy = r.floor_y;
        let is_boss_room = k == boss_idx;

        if is_boss_room {
            enemies.push(EnemySpawn { kind: st.boss.clone(), pos: boss_center,
                                      mult: mult * st.boss_mult, is_boss: true });
            // свита по бокам алтаря
            for (gi, guard) in st.boss_guards.iter().enumerate() {
                let d = if gi % 2 == 0 { -2 - (gi as i32 / 2) } else { 2 + (gi as i32 / 2) };
                enemies.push(EnemySpawn { kind: guard.clone(),
                    pos: cell_at(cx + d, cz, fy), mult, is_boss: false });
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
        for idx in 0..n {
            let Some(kind) = (!pool.is_empty()).then(|| rng.pick(pool)) else { break };
            let px = r.x + 1 + rng.range(0, (r.w - 2).max(1));
            let pz = r.z + 1 + rng.range(0, (r.h - 2).max(1));
            // Небольшой разброс по комнате чтобы не стояли в кучке
            let ox = if idx % 2 == 0 { 0 } else { rng.range(-1, 1) };
            let oz = if idx % 3 == 0 { 0 } else { rng.range(-1, 1) };
            enemies.push(EnemySpawn {
                kind: kind.clone(),
                pos: cell_at((px + ox).clamp(r.x + 1, r.x + r.w - 2),
                             (pz + oz).clamp(r.z + 1, r.z + r.h - 2), fy),
                mult, is_boss: false,
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
    }
}
