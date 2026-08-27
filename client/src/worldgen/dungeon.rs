//! Процедурный генератор данжей — Doom/Quake стиль.
//!
//! Комнаты связываются по близости (MST + петли), высоты пола (0 / 0.8)
//! назначаются по дереву и соединяются пологими пандусами (только когда коридор
//! достаточно длинный). Коридоры шириной 2. Пост-проверка достижимости
//! (flood-fill) не даёт спавнить врагов/лут в отрезанных карманах.

use godot::classes::Node3D;
use godot::prelude::*;

use crate::config::GameConfig;
use crate::gfx::{make_billboard, make_box, make_glow_slab, make_light, make_ramp, TexCache};
use crate::weapon::{AmmoType, WeaponId};

// Размер клетки и сторона сетки живут в ядре: по ним ходит серверная
// навигация врагов, значения обязаны совпадать.
pub use openheart_core::worldgen::{CELL, GRID};

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

// Типы плана и сам планировщик — в ядре: сервер строит по ним навигацию.
pub use openheart_core::worldgen::dungeon::{
    plan, DungeonLayout, FocalPattern, HazardKind, HazardPhase, Room, RoomDecor, RoomRole, Shape,
    WALL_H,
};

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

/// Центр клетки сетки в координатах движка. Ядро считает в своей математике,
/// геометрии удобнее сразу в `Vector3`.
fn cell_at(i: i32, j: i32, y: f32) -> Vector3 {
    crate::convert::to_godot(openheart_core::worldgen::dungeon::cell_at(i, j, y))
}

// ── Генерация: план из ядра + геометрия здесь ────────────────────────────────

pub fn generate(
    depth: u32,
    seed: u64,
    cache: &mut TexCache,
    cfg: &GameConfig,
    lang: &str,
) -> DungeonPlan {
    let layout = plan(depth, seed, cfg);
    let dc = &cfg.dungeon;
    let theme = Theme::from_cfg(&dc.themes[layout.theme_index], lang);

    // Продолжаем тот же поток RNG, на котором остановился план: декор и спавны
    // обязаны получиться те же, что до выноса плана в ядро.
    let mut rng = layout.rng;
    let rooms = layout.rooms;
    let boss_idx = layout.boss_idx;
    let floor = layout.floor;
    let fh = layout.floor_heights;
    let cwh = layout.ceil_heights;
    let is_ramp = layout.is_ramp;
    let ramp_segs: Vec<(Vector3, Vector3)> = layout
        .ramp_segs
        .iter()
        .map(|(a, b)| (crate::convert::to_godot(*a), crate::convert::to_godot(*b)))
        .collect();

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
                let hazard_color = crate::convert::color(r.hazard_kind.color());
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

    // 4. Спавны и порталы посчитало ядро — здесь только их визуал.
    let spawns = layout.spawns;
    let to_world = crate::convert::to_godot;
    let player_spawn = to_world(spawns.player_spawn);
    let exit_portal = to_world(spawns.exit_portal);
    let next_portal = to_world(spawns.next_portal);

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

    DungeonPlan {
        depth,
        theme_name: theme.name,
        root,
        player_spawn,
        exit_portal,
        next_portal,
        // Спавны посчитаны ядром в его математике — переводим в координаты движка.
        enemies: spawns
            .enemies
            .iter()
            .map(|e| EnemySpawn {
                kind: e.kind.clone(),
                pos: to_world(e.pos),
                mult: e.mult,
                is_boss: e.is_boss,
                affixes: e.affixes.clone(),
            })
            .collect(),
        items: spawns
            .items
            .iter()
            .map(|(id, pos)| (id.clone(), to_world(*pos)))
            .collect(),
        ammo: spawns
            .ammo
            .iter()
            .map(|(kind, count, pos)| (*kind, *count, to_world(*pos)))
            .collect(),
        weapons: spawns
            .weapons
            .iter()
            .map(|(id, pos)| (*id, to_world(*pos)))
            .collect(),
        hazards,
        room_roles: spawns.room_roles.clone(),
        room_markers: spawns
            .room_markers
            .iter()
            .map(|(role, pos)| (role.clone(), to_world(*pos)))
            .collect(),
        dressing_nodes,
        dressing_variants,
        events: spawns
            .events
            .iter()
            .map(|e| DungeonEventSpawn {
                kind: e.kind.clone(),
                group: e.group,
                step: e.step,
                pos: to_world(e.pos),
            })
            .collect(),
        floor_map: floor,
        floor_heights: fh,
    }
}
