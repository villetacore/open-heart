//! Открытый мир: центральная площадь-хаб с NPC и неоном, пустоши с руинами,
//! врата процедурного данжа на севере. Статичная карта, собирается кодом.

use godot::prelude::*;
use godot::classes::Node3D;

use crate::gfx::{make_box, make_box_rot, make_billboard, make_flat_sprite, make_light, Rng, TexCache};

pub const WORLD_SIZE: f32 = 200.0;
pub const GATE_POS: Vector3 = Vector3::new(0.0, 0.0, -62.0);

pub struct WorldPlan {
    pub root:         Gd<Node3D>,
    pub player_spawn: Vector3,
    pub gate_portal:  Vector3, // точка входа в данж (интеракт)
}

const C_STONE: Color = Color::from_rgba(0.10, 0.07, 0.10, 1.0);
const C_DARK:  Color = Color::from_rgba(0.05, 0.03, 0.06, 1.0);
const PINK:    Color = Color::from_rgba(1.0, 0.5, 0.75, 1.0);

pub fn build_world(cache: &mut TexCache) -> WorldPlan {
    let mut root = Node3D::new_alloc();
    let mut rng = Rng::new(0xC0FFEE);

    let t_ground = cache.get("res://assets/textures/floor_main.png");
    let t_market = cache.get("res://assets/textures/wall_market.png");
    let t_main   = cache.get("res://assets/textures/wall_main.png");
    let t_lab    = cache.get("res://assets/textures/wall_lab.png");
    let t_arch   = cache.get("res://assets/textures/wall_archive.png");
    let t_arena  = cache.get("res://assets/textures/wall_arena.png");
    let t_boss   = cache.get("res://assets/textures/wall_boss.png");

    // ── Земля (плитками — из-за лимита источников света на меш) и границы ───
    const TILE: f32 = 25.0;
    let n = (WORLD_SIZE / TILE) as i32;
    for gx in 0..n {
        for gz in 0..n {
            let cx = (gx as f32 + 0.5) * TILE - WORLD_SIZE * 0.5;
            let cz = (gz as f32 + 0.5) * TILE - WORLD_SIZE * 0.5;
            let g = make_box(Vector3::new(cx, -0.15, cz),
                             Vector3::new(TILE, 0.3, TILE),
                             C_DARK, t_ground.as_ref(), TILE / 6.0);
            root.add_child(&g);
        }
    }

    let half = WORLD_SIZE * 0.5 - 1.0;
    for (px, pz, sx, sz) in [
        (0.0, -half, WORLD_SIZE, 0.8), (0.0, half, WORLD_SIZE, 0.8),
        (-half, 0.0, 0.8, WORLD_SIZE), (half, 0.0, 0.8, WORLD_SIZE),
    ] {
        let w = make_box(Vector3::new(px, 2.5, pz), Vector3::new(sx, 5.0, sz),
                         C_DARK, t_arena.as_ref(), 24.0);
        root.add_child(&w);
    }

    // ── Хаб: здания вокруг площади ───────────────────────────────────────────
    // (cx, cz, w, d, h, текстура, вывеска, поворот вывески к площади)
    let buildings: [(f32, f32, f32, f32, f32, u8, &str); 8] = [
        (-24.0, -18.0, 14.0, 9.0, 5.2, 0, "res://assets/sprites/props/neon_femboy_club.png"),
        ( 24.0, -18.0, 14.0, 9.0, 4.6, 1, "res://assets/sprites/props/neon_boys.png"),
        (-26.0,   6.0, 10.0, 12.0, 4.4, 2, "res://assets/sprites/props/neon_kawaii.png"),
        ( 26.0,   6.0, 10.0, 12.0, 4.8, 3, "res://assets/sprites/props/neon_good_boy.png"),
        (-16.0,  26.0, 12.0, 10.0, 5.0, 1, "res://assets/sprites/props/neon_love_wins.png"),
        ( 16.0,  26.0, 12.0, 10.0, 4.5, 0, "res://assets/sprites/props/neon_traps.png"),
        (  0.0,  32.0, 10.0, 8.0,  5.4, 2, "res://assets/sprites/props/neon_heart.png"),
        (-30.0, -34.0, 12.0, 10.0, 4.2, 3, "res://assets/sprites/props/neon_uwu.png"),
    ];
    let texs = [&t_market, &t_main, &t_lab, &t_arch];
    for (cx, cz, w, d, h, ti, sign) in buildings {
        let b = make_box(Vector3::new(cx, h * 0.5, cz), Vector3::new(w, h, d),
                         C_STONE, texs[ti as usize].as_ref(), 3.0);
        root.add_child(&b);
        // вывеска на стене, обращённой к центру площади
        let toward_center_z = if cz > 0.0 { -1.0 } else { 1.0 };
        let sy = h * 0.62;
        let (spos, rot) = if cx.abs() > cz.abs() {
            // вывеска на боковой грани, смотрящей к оси X=0
            let side = if cx > 0.0 { -1.0 } else { 1.0 };
            (Vector3::new(cx + side * (w * 0.5 + 0.06), sy, cz),
             if side > 0.0 { std::f32::consts::FRAC_PI_2 } else { -std::f32::consts::FRAC_PI_2 })
        } else {
            (Vector3::new(cx, sy, cz + toward_center_z * (d * 0.5 + 0.06)),
             if toward_center_z > 0.0 { 0.0 } else { std::f32::consts::PI })
        };
        if let Some(sp) = make_flat_sprite(cache, sign, spos, rot, 0.022) {
            root.add_child(&sp);
        }
        let l_off = match rot {
            r if r == 0.0 => Vector3::new(0.0, 0.0, 1.2),
            r if r == std::f32::consts::PI => Vector3::new(0.0, 0.0, -1.2),
            r if r > 0.0 => Vector3::new(1.2, 0.0, 0.0),
            _ => Vector3::new(-1.2, 0.0, 0.0),
        };
        let l = make_light(spos + l_off, PINK, 0.8, 7.0);
        root.add_child(&l);
    }

    // ── Фонтан в центре ──────────────────────────────────────────────────────
    let rim = make_box(Vector3::new(0.0, 0.3, 0.0), Vector3::new(2.4, 0.6, 2.4),
                       C_STONE, t_main.as_ref(), 1.0);
    root.add_child(&rim);
    if let Some(sp) = make_billboard(cache, "res://assets/sprites/props/street_fountain.png",
                                     Vector3::new(0.0, 1.35, 0.0), 0.022) {
        root.add_child(&sp);
    }
    let fl = make_light(Vector3::new(0.0, 2.2, 0.0), PINK, 1.5, 16.0);
    root.add_child(&fl);

    // ── Уличные пропсы на площади ────────────────────────────────────────────
    let props: [(&str, f32, f32, f32); 9] = [
        ("street_bench",       -7.0, -5.0, 0.020),
        ("street_bench",        7.0,  5.0, 0.020),
        ("street_dumpster",   -19.0, -11.0, 0.024),
        ("street_trashcan",     9.0, -9.0, 0.018),
        ("street_vending",     20.0,  1.0, 0.022),
        ("street_phone",       21.0, -10.0, 0.022),
        ("street_bags",       -17.5, -9.0, 0.018),
        ("street_grate_table", -9.0,  9.0, 0.018),
        ("street_cone",         3.0, -12.0, 0.016),
    ];
    for (name, px, pz, ps) in props {
        let path = format!("res://assets/sprites/props/{}.png", name);
        if let Some(tex) = cache.get(&path) {
            let h_m = tex.get_height() as f32 * ps;
            if let Some(sp) = make_billboard(cache, &path,
                                             Vector3::new(px, h_m * 0.5 + 0.02, pz), ps) {
                root.add_child(&sp);
            }
        }
    }

    // ── Фонари площади ───────────────────────────────────────────────────────
    for (px, pz) in [(-12.0f32, -12.0f32), (12.0, -12.0), (-12.0, 12.0), (12.0, 12.0)] {
        let pole = make_box(Vector3::new(px, 1.6, pz), Vector3::new(0.16, 3.2, 0.16),
                            C_DARK, None, 1.0);
        root.add_child(&pole);
        if let Some(sp) = make_billboard(cache, "res://assets/sprites/pickups/soul.png",
                                         Vector3::new(px, 3.4, pz), 0.010) {
            root.add_child(&sp);
        }
        let l = make_light(Vector3::new(px, 3.3, pz), PINK, 1.1, 12.0);
        root.add_child(&l);
    }

    // ── Дорога к вратам данжа ────────────────────────────────────────────────
    for k in 0..6 {
        let z = -20.0 - k as f32 * 7.0;
        let side = if k % 2 == 0 { -2.6 } else { 2.6 };
        if let Some(sp) = make_billboard(cache, "res://assets/sprites/pickups/soul.png",
                                         Vector3::new(side, 1.0, z), 0.008) {
            root.add_child(&sp);
        }
        let l = make_light(Vector3::new(side, 1.2, z),
                           Color::from_rgba(0.8, 0.4, 0.95, 1.0), 0.7, 6.0);
        root.add_child(&l);
    }

    // ── Врата данжа ──────────────────────────────────────────────────────────
    let gz = GATE_POS.z;
    for side in [-1.0f32, 1.0] {
        let p = make_box(Vector3::new(side * 2.6, 2.4, gz), Vector3::new(1.2, 4.8, 1.2),
                         C_STONE, t_boss.as_ref(), 1.5);
        root.add_child(&p);
    }
    let lintel = make_box(Vector3::new(0.0, 5.0, gz), Vector3::new(6.4, 1.0, 1.4),
                          C_STONE, t_boss.as_ref(), 2.0);
    root.add_child(&lintel);
    if let Some(sp) = make_flat_sprite(cache, "res://assets/sprites/props/neon_game_over.png",
                                       Vector3::new(0.0, 4.0, gz + 0.75), 0.0, 0.02) {
        root.add_child(&sp);
    }
    if let Some(mut sp) = make_billboard(cache, "res://assets/effects/effect_teleport.png",
                                         GATE_POS + Vector3::new(0.0, 1.5, 0.0), 0.024) {
        sp.set_modulate(Color::from_rgba(1.0, 0.4, 0.8, 1.0));
        root.add_child(&sp);
    }
    let gl = make_light(GATE_POS + Vector3::new(0.0, 2.0, 0.0),
                        Color::from_rgba(0.9, 0.3, 0.9, 1.0), 2.0, 14.0);
    root.add_child(&gl);

    // ── Пустоши: руины, камни, души ──────────────────────────────────────────
    for _ in 0..12 {
        let ang = rng.f32() * std::f32::consts::TAU;
        let r = 46.0 + rng.f32() * 44.0;
        let (px, pz) = (ang.cos() * r, ang.sin() * r);
        if pz < -46.0 && px.abs() < 12.0 { continue; } // не загораживаем врата
        let len = 3.0 + rng.f32() * 5.0;
        let h = 1.2 + rng.f32() * 2.4;
        let w = make_box_rot(Vector3::new(px, h * 0.5, pz),
                             Vector3::new(len, h, 0.5),
                             rng.f32() * std::f32::consts::PI,
                             C_STONE, t_arena.as_ref(), 2.0);
        root.add_child(&w);
    }
    for _ in 0..16 {
        let ang = rng.f32() * std::f32::consts::TAU;
        let r = 42.0 + rng.f32() * 50.0;
        let (px, pz) = (ang.cos() * r, ang.sin() * r);
        let s = 0.7 + rng.f32() * 1.8;
        let rock = make_box_rot(Vector3::new(px, s * 0.4, pz),
                                Vector3::new(s, s * 0.8, s * (0.7 + rng.f32() * 0.6)),
                                rng.f32() * std::f32::consts::PI,
                                C_DARK, None, 1.0);
        root.add_child(&rock);
    }
    for _ in 0..7 {
        let ang = rng.f32() * std::f32::consts::TAU;
        let r = 40.0 + rng.f32() * 46.0;
        let (px, pz) = (ang.cos() * r, ang.sin() * r);
        if let Some(sp) = make_billboard(cache, "res://assets/sprites/pickups/soul.png",
                                         Vector3::new(px, 1.0, pz), 0.010) {
            root.add_child(&sp);
        }
        let l = make_light(Vector3::new(px, 1.3, pz),
                           Color::from_rgba(0.9, 0.5, 0.9, 1.0), 0.8, 8.0);
        root.add_child(&l);
    }

    WorldPlan {
        root,
        player_spawn: Vector3::new(0.0, 1.1, 10.0),
        gate_portal: GATE_POS,
    }
}
