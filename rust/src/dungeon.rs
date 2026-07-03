//! Процедурный генератор данжей.
//!
//! Комнаты + L-коридоры на сетке, темы по глубине, спавны врагов/лута,
//! босс в дальней комнате, портал выхода и портал «глубже».

use godot::prelude::*;
use godot::classes::Node3D;

use crate::gfx::{make_box, make_glow_slab, make_light, make_billboard, Rng, TexCache};
use crate::weapon::{AmmoType, WeaponId};

pub const CELL: f32   = 3.0;
pub const WALL_H: f32 = 3.4;
const GRID: usize     = 44;

// ── Тема данжа ────────────────────────────────────────────────────────────────

struct Theme {
    name:    &'static str,
    wall:    &'static str,
    accent:  &'static str,
    floor:   &'static str,
    ceil:    &'static str,
    lava:    &'static str,
    light:   Color,
}

const THEMES: [Theme; 3] = [
    Theme {
        name: "Катакомбы Сердец",
        wall:   "res://assets/textures/dungeon/dtile_00.png",
        accent: "res://assets/textures/dungeon/dtile_06.png",
        floor:  "res://assets/textures/dungeon/dtile_02.png",
        ceil:   "res://assets/textures/dungeon/dtile_65.png",
        lava:   "res://assets/textures/dungeon/dtile_74.png",
        light:  Color::from_rgba(1.0, 0.45, 0.72, 1.0),
    },
    Theme {
        name: "Неоновый Подвал",
        wall:   "res://assets/textures/dungeon/dtile_05.png",
        accent: "res://assets/textures/dungeon/dtile_33.png",
        floor:  "res://assets/textures/dungeon/dtile_19.png",
        ceil:   "res://assets/textures/dungeon/dtile_72.png",
        lava:   "res://assets/textures/dungeon/dtile_21.png",
        light:  Color::from_rgba(0.62, 0.38, 0.95, 1.0),
    },
    Theme {
        name: "Алтарь Боли",
        wall:   "res://assets/textures/dungeon/dtile_30.png",
        accent: "res://assets/textures/dungeon/dtile_63.png",
        floor:  "res://assets/textures/dungeon/dtile_58.png",
        ceil:   "res://assets/textures/dungeon/dtile_68.png",
        lava:   "res://assets/textures/dungeon/liquid_red.png",
        light:  Color::from_rgba(0.95, 0.18, 0.22, 1.0),
    },
];

// ── Результат генерации ───────────────────────────────────────────────────────

pub struct EnemySpawn {
    pub kind:    String,
    pub pos:     Vector3,
    pub mult:    f32,
    pub is_boss: bool,
}

pub struct DungeonPlan {
    pub depth:        u32,
    pub theme_name:   &'static str,
    pub root:         Gd<Node3D>,
    pub player_spawn: Vector3,
    pub exit_portal:  Vector3,   // назад в мир (стартовая комната)
    pub next_portal:  Vector3,   // глубже (комната босса)
    pub enemies:      Vec<EnemySpawn>,
    pub items:        Vec<(String, Vector3)>,          // kind из items.json
    pub ammo:         Vec<(AmmoType, u32, Vector3)>,
    pub weapons:      Vec<(WeaponId, Vector3)>,
}

#[derive(Clone, Copy)]
struct Room { x: i32, z: i32, w: i32, h: i32 }

impl Room {
    fn center(&self) -> (i32, i32) { (self.x + self.w / 2, self.z + self.h / 2) }
    fn overlaps(&self, o: &Room, pad: i32) -> bool {
        self.x - pad < o.x + o.w && self.x + self.w + pad > o.x &&
        self.z - pad < o.z + o.h && self.z + self.h + pad > o.z
    }
}

fn cell_world(i: i32, j: i32) -> Vector3 {
    Vector3::new(
        (i as f32 + 0.5 - GRID as f32 * 0.5) * CELL,
        0.0,
        (j as f32 + 0.5 - GRID as f32 * 0.5) * CELL,
    )
}

// ── Генерация ─────────────────────────────────────────────────────────────────

pub fn generate(depth: u32, seed: u64, cache: &mut TexCache) -> DungeonPlan {
    let mut rng = Rng::new(seed ^ (depth as u64).wrapping_mul(0x9E3779B97F4A7C15));
    let theme = &THEMES[((depth - 1) % 3) as usize];

    // 1. Комнаты
    let mut rooms: Vec<Room> = Vec::new();
    let target = 6 + (depth.min(6) as i32) / 2 + rng.range(0, 2); // 6..10
    for _ in 0..80 {
        if rooms.len() as i32 >= target { break; }
        let w = rng.range(4, 8);
        let h = rng.range(4, 8);
        let x = rng.range(2, GRID as i32 - w - 3);
        let z = rng.range(2, GRID as i32 - h - 3);
        let r = Room { x, z, w, h };
        if !rooms.iter().any(|o| r.overlaps(o, 2)) {
            rooms.push(r);
        }
    }

    // 2. Пол: комнаты + коридоры
    let mut floor = vec![false; GRID * GRID];
    let mut carve = |i: i32, j: i32| {
        if i >= 1 && j >= 1 && i < GRID as i32 - 1 && j < GRID as i32 - 1 {
            floor[j as usize * GRID + i as usize] = true;
        }
    };
    for r in &rooms {
        for j in r.z..r.z + r.h {
            for i in r.x..r.x + r.w {
                carve(i, j);
            }
        }
    }
    // L-коридоры последовательно + замыкающий
    let mut links: Vec<(usize, usize)> = (1..rooms.len()).map(|i| (i - 1, i)).collect();
    if rooms.len() > 3 { links.push((0, rooms.len() - 1)); }
    for (a, b) in links {
        let (ax, az) = rooms[a].center();
        let (bx, bz) = rooms[b].center();
        let wide = rng.chance(0.4);
        let (mut cx, mut cz) = (ax, az);
        while cx != bx {
            carve(cx, cz);
            if wide { carve(cx, cz + 1); }
            cx += (bx - cx).signum();
        }
        while cz != bz {
            carve(cx, cz);
            if wide { carve(cx + 1, cz); }
            cz += (bz - cz).signum();
        }
    }

    let is_floor = |i: i32, j: i32| -> bool {
        i >= 0 && j >= 0 && (i as usize) < GRID && (j as usize) < GRID
            && floor[j as usize * GRID + i as usize]
    };

    // 3. Геометрия
    let mut root = Node3D::new_alloc();
    let t_wall   = cache.get(theme.wall);
    let t_accent = cache.get(theme.accent);
    let t_floor  = cache.get(theme.floor);
    let t_ceil   = cache.get(theme.ceil);
    let t_lava   = cache.get(theme.lava);

    let c_dark = Color::from_rgba(0.07, 0.04, 0.07, 1.0);

    // Пол и потолок — полосами по рядам пола (лимит света на меш в GL Compatibility)
    for j in 0..GRID as i32 {
        let mut i = 0i32;
        while i < GRID as i32 {
            if is_floor(i, j) {
                let start = i;
                while i < GRID as i32 && is_floor(i, j) { i += 1; }
                let len = (i - start) as f32 * CELL;
                let cx = ((start + i) as f32 * 0.5 - GRID as f32 * 0.5) * CELL;
                let cz = (j as f32 + 0.5 - GRID as f32 * 0.5) * CELL;
                let fl = make_box(Vector3::new(cx, -0.15, cz),
                                  Vector3::new(len + 0.02, 0.3, CELL + 0.02),
                                  c_dark, t_floor.as_ref(), (len / CELL).max(1.0));
                root.add_child(&fl);
                let ce = make_box(Vector3::new(cx, WALL_H + 0.15, cz),
                                  Vector3::new(len + 0.02, 0.3, CELL + 0.02),
                                  c_dark, t_ceil.as_ref(), (len / CELL).max(1.0));
                root.add_child(&ce);
            } else {
                i += 1;
            }
        }
    }

    // Стены: слитые горизонтальные/вертикальные сегменты по границам пола
    const T: f32 = 0.3;
    // северные/южные грани (перебор по строкам)
    for j in 0..GRID as i32 {
        for side in [-1i32, 1] {
            let mut i = 0i32;
            while i < GRID as i32 {
                let need = is_floor(i, j) && !is_floor(i, j + side);
                if need {
                    let start = i;
                    while i < GRID as i32 && is_floor(i, j) && !is_floor(i, j + side) { i += 1; }
                    let len = (i - start) as f32 * CELL;
                    let cx = ((start + i) as f32 * 0.5 - GRID as f32 * 0.5) * CELL;
                    let cz = (j as f32 + 0.5 - GRID as f32 * 0.5) * CELL + side as f32 * CELL * 0.5;
                    let use_accent = rng.chance(0.12);
                    let tex = if use_accent { t_accent.as_ref() } else { t_wall.as_ref() };
                    let w = make_box(Vector3::new(cx, WALL_H * 0.5, cz),
                                     Vector3::new(len + T, WALL_H, T), c_dark, tex,
                                     (len / CELL).max(1.0));
                    root.add_child(&w);
                } else {
                    i += 1;
                }
            }
        }
    }
    // западные/восточные грани (перебор по столбцам)
    for i in 0..GRID as i32 {
        for side in [-1i32, 1] {
            let mut j = 0i32;
            while j < GRID as i32 {
                let need = is_floor(i, j) && !is_floor(i + side, j);
                if need {
                    let start = j;
                    while j < GRID as i32 && is_floor(i, j) && !is_floor(i + side, j) { j += 1; }
                    let len = (j - start) as f32 * CELL;
                    let cz = ((start + j) as f32 * 0.5 - GRID as f32 * 0.5) * CELL;
                    let cx = (i as f32 + 0.5 - GRID as f32 * 0.5) * CELL + side as f32 * CELL * 0.5;
                    let use_accent = rng.chance(0.12);
                    let tex = if use_accent { t_accent.as_ref() } else { t_wall.as_ref() };
                    let w = make_box(Vector3::new(cx, WALL_H * 0.5, cz),
                                     Vector3::new(T, WALL_H, len + T), c_dark, tex,
                                     (len / CELL).max(1.0));
                    root.add_child(&w);
                } else {
                    j += 1;
                }
            }
        }
    }

    // 4. Комнатные детали: свет, души-факелы, лава, колонны
    let entry_idx = 0usize;
    // босс — самая дальняя комната от входа
    let (e_cx, e_cz) = rooms[entry_idx].center();
    let mut boss_idx = rooms.len() - 1;
    let mut best_d = -1i32;
    for (k, r) in rooms.iter().enumerate() {
        if k == entry_idx { continue; }
        let (cx, cz) = r.center();
        let d = (cx - e_cx).pow(2) + (cz - e_cz).pow(2);
        if d > best_d { best_d = d; boss_idx = k; }
    }

    for (k, r) in rooms.iter().enumerate() {
        let (cx, cz) = r.center();
        let center = cell_world(cx, cz);
        let is_boss_room = k == boss_idx;

        // свет комнаты
        let range = (r.w.max(r.h) as f32) * CELL * 1.4;
        let energy = if is_boss_room { 2.6 } else { 1.7 };
        let color = if is_boss_room {
            Color::from_rgba(0.95, 0.12, 0.18, 1.0)
        } else {
            theme.light
        };
        let l = make_light(center + Vector3::new(0.0, WALL_H - 0.6, 0.0), color, energy, range);
        root.add_child(&l);

        // душа-факел у стены
        if rng.chance(0.85) {
            let ti = r.x + 1 + rng.range(0, (r.w - 2).max(0));
            let pos = cell_world(ti, r.z) + Vector3::new(0.0, 1.5, -CELL * 0.28);
            if let Some(sp) = make_billboard(cache, "res://assets/sprites/pickups/soul.png", pos, 0.012) {
                root.add_child(&sp);
            }
            let tl = make_light(pos, theme.light, 0.9, 7.0);
            root.add_child(&tl);
        }

        // лужа лавы (декор + свет)
        if !is_boss_room && k != entry_idx && rng.chance(0.35) && r.w >= 5 && r.h >= 5 {
            let lp = cell_world(cx + rng.range(-1, 1), cz + rng.range(-1, 1));
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

        // колонны в больших комнатах
        if r.w >= 6 && r.h >= 6 && rng.chance(0.6) {
            for (dx, dz) in [(-2, -2), (2, 2), (-2, 2), (2, -2)] {
                let p = cell_world(cx + dx, cz + dz);
                let col = make_box(p + Vector3::new(0.0, WALL_H * 0.5, 0.0),
                                   Vector3::new(0.7, WALL_H, 0.7),
                                   c_dark, t_wall.as_ref(), 1.0);
                root.add_child(&col);
            }
        }
    }

    // алтарь в комнате босса
    let (b_cx, b_cz) = rooms[boss_idx].center();
    let boss_center = cell_world(b_cx, b_cz);
    let alt = make_box(boss_center + Vector3::new(0.0, 0.35, 0.0),
                       Vector3::new(2.6, 0.7, 1.4), c_dark, t_accent.as_ref(), 1.0);
    root.add_child(&alt);

    // 5. Спавны
    let mut enemies: Vec<EnemySpawn> = Vec::new();
    let mut items:   Vec<(String, Vector3)> = Vec::new();
    let mut ammo:    Vec<(AmmoType, u32, Vector3)> = Vec::new();
    let mut weapons: Vec<(WeaponId, Vector3)> = Vec::new();

    let mult = 1.0 + (depth - 1) as f32 * 0.18;
    let pool_early: [&str; 3] = ["grunt", "fast", "cultist"];
    let pool_late:  [&str; 5] = ["grunt", "fast", "cultist", "heavy", "sniper"];

    for (k, r) in rooms.iter().enumerate() {
        if k == entry_idx { continue; }
        let (cx, cz) = r.center();
        let is_boss_room = k == boss_idx;

        if is_boss_room {
            enemies.push(EnemySpawn {
                kind: "brute".into(),
                pos: boss_center,
                mult: mult * 1.25,
                is_boss: true,
            });
            for d in [-2i32, 2] {
                enemies.push(EnemySpawn {
                    kind: "cultist".into(),
                    pos: cell_world(cx + d, cz),
                    mult,
                    is_boss: false,
                });
            }
            // сокровища
            items.push(("ancient_ruby".into(), cell_world(cx, cz + 1)));
            items.push(("gold_stack".into(),   cell_world(cx - 1, cz + 1)));
            items.push(("heart_1up".into(),    cell_world(cx + 1, cz + 1)));
            continue;
        }

        // обычная комната
        let n = 1 + rng.range(0, 1 + (depth.min(5) as i32) / 2);
        for _ in 0..n {
            let kind = if depth >= 2 { *rng.pick(&pool_late) } else { *rng.pick(&pool_early) };
            let px = r.x + 1 + rng.range(0, (r.w - 2).max(0));
            let pz = r.z + 1 + rng.range(0, (r.h - 2).max(0));
            enemies.push(EnemySpawn {
                kind: kind.into(),
                pos: cell_world(px, pz),
                mult,
                is_boss: false,
            });
        }
        // лут
        if rng.chance(0.55) {
            let t = AmmoType::from_idx(rng.below(4) as usize);
            ammo.push((t, t.pack_size(), cell_world(r.x + 1, r.z + r.h - 2)));
        }
        if rng.chance(0.4) {
            items.push(("medkit".into(), cell_world(r.x + r.w - 2, r.z + 1)));
        }
        if rng.chance(0.25) {
            items.push(("gold_coin".into(), cell_world(cx, cz + 1)));
        }
        if rng.chance(0.12) {
            items.push(("potion".into(), cell_world(cx - 1, cz)));
        }
    }

    // оружейный тайник — одна случайная не-стартовая комната
    if rooms.len() > 2 {
        let wk = 1 + rng.below((rooms.len() - 1) as u32) as usize;
        let (cx, cz) = rooms[wk].center();
        let pool: [WeaponId; 5] = [WeaponId::Shotgun, WeaponId::Rifle, WeaponId::Nailgun,
                                   WeaponId::Plasma, WeaponId::Rocket];
        let w = *rng.pick(&pool);
        weapons.push((w, cell_world(cx, cz.max(1))));
        // и патроны к нему
        if let Some((t, _)) = crate::weapon::weapon_def(w).ammo {
            ammo.push((t, t.pack_size(), cell_world(cx + 1, cz)));
        }
    }

    // 6. Порталы
    let (ex, ez) = rooms[entry_idx].center();
    let player_spawn = cell_world(ex, ez);
    let exit_portal  = cell_world(rooms[entry_idx].x + 1, rooms[entry_idx].z + 1);
    let next_portal  = cell_world(b_cx, b_cz - (rooms[boss_idx].h / 2 - 1).max(1));

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
    }
}
