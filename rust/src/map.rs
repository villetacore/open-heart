//! Карты из JSON (`presets/<id>/maps/*.json`) — данные, а не код.
//!
//! Поддерживаемые фигуры: box (с поворотом), ramp (наклонная плита from→to),
//! stairs (лестница from→to из N ступеней), cylinder (колонна/башня).
//! Плюс декларативные «здания» (бокс+вывеска+свет), пропсы-биллборды,
//! плоские спрайты на стенах, источники света, светящиеся плиты (неон-каналы),
//! спавны и портал данжа. Это позволяет строить большие многоярусные карты
//! с рампами и мостами чисто данными — редактор игры правит эти файлы.

use godot::prelude::*;
use godot::classes::{
    CollisionShape3D, CylinderMesh, CylinderShape3D, MeshInstance3D, Node3D,
    StandardMaterial3D, StaticBody3D, FileAccess, file_access::ModeFlags,
};
use godot::classes::base_material_3d::{TextureParam, TextureFilter};
use serde::Deserialize;

use crate::config::LevelCfg;
use crate::gfx::{make_billboard, make_box, make_box_rot, make_flat_sprite, make_glow_slab,
                 make_light, TexCache};

// ── Формат карты ──────────────────────────────────────────────────────────────

fn f_uv1() -> f32 { 1.0 }
fn f_px() -> f32 { 0.02 }

#[derive(Deserialize, Clone, Default)]
pub struct MapEnv {
    #[serde(default)] pub sky:            Option<String>,
    #[serde(default)] pub fog_density:    Option<f32>,
    #[serde(default)] pub ambient:        Option<[f32; 3]>,
    #[serde(default)] pub ambient_energy: Option<f32>,
}

#[derive(Deserialize, Clone)]
pub struct BlockDef {
    pub shape: String,                       // box | ramp | stairs | cylinder
    #[serde(default)] pub pos:    Option<[f32; 3]>,   // box/cylinder
    #[serde(default)] pub size:   Option<[f32; 3]>,   // box
    #[serde(default)] pub rot:    f32,                // box: поворот вокруг Y, градусы
    #[serde(default)] pub from:   Option<[f32; 3]>,   // ramp/stairs
    #[serde(default)] pub to:     Option<[f32; 3]>,
    #[serde(default)] pub width:  f32,                // ramp/stairs
    #[serde(default, deserialize_with = "crate::config::de_u32")] pub steps: u32, // stairs
    #[serde(default)] pub radius: f32,                // cylinder
    #[serde(default)] pub height: f32,                // cylinder
    #[serde(default)] pub tex:    Option<String>,
    #[serde(default = "f_uv1")] pub uv: f32,
}

#[derive(Deserialize, Clone)]
pub struct BuildingDef {
    pub pos:  [f32; 2],
    pub size: [f32; 3],
    pub tex:  String,
    #[serde(default)] pub sign:      Option<String>,
    #[serde(default)] pub sign_side: Option<String>,  // n|s|e|w (куда смотрит вывеска)
}

#[derive(Deserialize, Clone)]
pub struct PropDef {
    pub tex: String,
    pub pos: [f32; 3],
    #[serde(default = "f_px")] pub px: f32,
}

#[derive(Deserialize, Clone)]
pub struct FlatDef {
    pub tex: String,
    pub pos: [f32; 3],
    #[serde(default)] pub rot: f32,          // градусы вокруг Y
    #[serde(default = "f_px")] pub px: f32,
    #[serde(default)] pub glow: bool,        // unshaded-неон
}

#[derive(Deserialize, Clone)]
pub struct LightDef {
    pub pos:    [f32; 3],
    pub color:  [f32; 3],
    pub energy: f32,
    pub range:  f32,
}

#[derive(Deserialize, Clone)]
pub struct GlowDef {
    pub pos:      [f32; 3],
    pub size:     [f32; 3],
    pub tex:      String,
    pub emission: [f32; 3],
    #[serde(default = "f_uv1")] pub uv: f32,
}

#[derive(Deserialize, Clone, Default)]
pub struct GroundDef {
    pub size: f32,
    pub tex:  String,
    #[serde(default = "f_uv1")] pub uv: f32,
    /// Высота стен-границ по периметру (0 = без стен).
    #[serde(default)] pub border_h: f32,
    #[serde(default)] pub border_tex: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct MapDef {
    pub id:       String,
    #[serde(default)] pub name_ru: String,
    #[serde(default)] pub env:     MapEnv,
    pub player_spawn: [f32; 3],
    #[serde(default)] pub gate:    Option<[f32; 3]>,  // портал данжа (строится арка)
    #[serde(default)] pub ground:  Option<GroundDef>,
    #[serde(default)] pub blocks:    Vec<BlockDef>,
    #[serde(default)] pub buildings: Vec<BuildingDef>,
    #[serde(default)] pub props:     Vec<PropDef>,
    #[serde(default)] pub flats:     Vec<FlatDef>,
    #[serde(default)] pub lights:    Vec<LightDef>,
    #[serde(default)] pub glows:     Vec<GlowDef>,
    #[serde(default)] pub spawns:    LevelCfg,
}

pub struct BuiltMap {
    pub root:         Gd<Node3D>,
    pub player_spawn: Vector3,
    pub gate:         Option<Vector3>,
    pub env:          MapEnv,
    pub name_ru:      String,
}

// ── Загрузка ──────────────────────────────────────────────────────────────────

pub fn load_map(preset_base: &str, id: &str) -> Option<MapDef> {
    let path = format!("{preset_base}/maps/{id}.json");
    let f = FileAccess::open(&path, ModeFlags::READ)?;
    match serde_json::from_str::<MapDef>(&f.get_as_text().to_string()) {
        Ok(m) => Some(m),
        Err(e) => {
            godot::global::godot_warn!("map {path}: {e}");
            None
        }
    }
}

// ── Утилиты текстур ───────────────────────────────────────────────────────────

/// Текстуры карты ищутся по короткому имени в стандартных папках.
fn tex_path(name: &str) -> String {
    if name.contains('/') {
        return format!("res://assets/{name}.png");
    }
    if name.starts_with("dtile_") || name.starts_with("liquid_") {
        format!("res://assets/textures/dungeon/{name}.png")
    } else if name.starts_with("sky_") {
        format!("res://assets/textures/sky/{name}.png")
    } else if name.starts_with("neon_") || name.starts_with("street_")
        || name.starts_with("furn_") || name.starts_with("bath_") {
        format!("res://assets/sprites/props/{name}.png")
    } else if name.starts_with("effect_") {
        format!("res://assets/effects/{name}.png")
    } else if name.starts_with("item_") {
        format!("res://assets/sprites/items/{name}.png")
    } else if name.starts_with("ammo_") || name == "soul" || name.starts_with("heart_")
        || name == "grenade" || name == "scroll" {
        format!("res://assets/sprites/pickups/{name}.png")
    } else {
        format!("res://assets/textures/{name}.png")
    }
}

const C_STONE: Color = Color::from_rgba(0.10, 0.07, 0.10, 1.0);
const C_DARK:  Color = Color::from_rgba(0.05, 0.03, 0.06, 1.0);
const PINK:    Color = Color::from_rgba(1.0, 0.5, 0.75, 1.0);

fn v3(a: [f32; 3]) -> Vector3 { Vector3::new(a[0], a[1], a[2]) }

// ── Билдер ────────────────────────────────────────────────────────────────────

pub fn build_map(def: &MapDef, cache: &mut TexCache) -> BuiltMap {
    let mut root = Node3D::new_alloc();

    // Земля плитками (лимит источников света на меш в GL Compatibility)
    if let Some(ref g) = def.ground {
        let tex = cache.get(&tex_path(&g.tex));
        const TILE: f32 = 25.0;
        let n = (g.size / TILE).ceil() as i32;
        for gx in 0..n {
            for gz in 0..n {
                let cx = (gx as f32 + 0.5) * TILE - g.size * 0.5;
                let cz = (gz as f32 + 0.5) * TILE - g.size * 0.5;
                let b = make_box(Vector3::new(cx, -0.15, cz),
                                 Vector3::new(TILE, 0.3, TILE),
                                 C_DARK, tex.as_ref(), TILE / 25.0 * g.uv);
                root.add_child(&b);
            }
        }
        if g.border_h > 0.0 {
            let bt = g.border_tex.as_ref().map(|t| tex_path(t));
            let btex = bt.as_deref().and_then(|p| cache.get(p));
            let half = g.size * 0.5 - 1.0;
            for (px, pz, sx, sz) in [
                (0.0, -half, g.size, 0.8), (0.0, half, g.size, 0.8),
                (-half, 0.0, 0.8, g.size), (half, 0.0, 0.8, g.size),
            ] {
                let w = make_box(Vector3::new(px, g.border_h * 0.5, pz),
                                 Vector3::new(sx, g.border_h, sz),
                                 C_DARK, btex.as_ref(), 24.0);
                root.add_child(&w);
            }
        }
    }

    // Блоки-фигуры
    for b in &def.blocks {
        let tex = b.tex.as_ref().map(|t| tex_path(t));
        let tex = tex.as_deref().and_then(|p| cache.get(p));
        match b.shape.as_str() {
            "box" => {
                let (Some(pos), Some(size)) = (b.pos, b.size) else { continue };
                let node = make_box_rot(v3(pos), v3(size), b.rot.to_radians(),
                                        C_STONE, tex.as_ref(), b.uv);
                root.add_child(&node);
            }
            "ramp" => {
                let (Some(from), Some(to)) = (b.from, b.to) else { continue };
                let (from, to) = (v3(from), v3(to));
                let dir = to - from;
                let horiz = Vector3::new(dir.x, 0.0, dir.z);
                let run = horiz.length().max(0.01);
                let full = (run * run + dir.y * dir.y).sqrt();
                let yaw = (-dir.x).atan2(-dir.z);
                let pitch = (dir.y).atan2(run);
                let mid = (from + to) * 0.5;
                let mut node = make_box(mid, Vector3::new(b.width.max(1.0), 0.3, full + 0.3),
                                        C_STONE, tex.as_ref(), (full / 3.0).max(1.0));
                node.set_rotation(Vector3::new(pitch, yaw, 0.0));
                root.add_child(&node);
            }
            "stairs" => {
                let (Some(from), Some(to)) = (b.from, b.to) else { continue };
                let (from, to) = (v3(from), v3(to));
                let n = b.steps.max(2) as i32;
                let dir = to - from;
                let horiz = Vector3::new(dir.x, 0.0, dir.z);
                let run = horiz.length().max(0.01);
                let step_d = run / n as f32;
                let hn = horiz.normalized();
                let yaw = (-hn.x).atan2(-hn.z);
                for i in 0..n {
                    let h = dir.y * (i + 1) as f32 / n as f32;
                    let center = from + hn * (step_d * (i as f32 + 0.5));
                    let node = make_box_rot(
                        Vector3::new(center.x, from.y + h * 0.5, center.z),
                        Vector3::new(b.width.max(1.0), h.max(0.1), step_d + 0.05),
                        yaw, C_STONE, tex.as_ref(), 1.0);
                    root.add_child(&node);
                }
            }
            "cylinder" => {
                let Some(pos) = b.pos else { continue };
                let mut body = StaticBody3D::new_alloc();
                body.set_position(v3(pos) + Vector3::new(0.0, b.height * 0.5, 0.0));
                let mut mesh = CylinderMesh::new_gd();
                mesh.set_top_radius(b.radius);
                mesh.set_bottom_radius(b.radius);
                mesh.set_height(b.height);
                let mut mi = MeshInstance3D::new_alloc();
                mi.set_mesh(&mesh);
                let mut mat = StandardMaterial3D::new_gd();
                if let Some(ref t) = tex {
                    mat.set_albedo(Color::WHITE);
                    mat.set_texture(TextureParam::ALBEDO, t);
                    mat.set_uv1_scale(Vector3::new(b.uv, b.uv, 1.0));
                    mat.set_texture_filter(TextureFilter::NEAREST_WITH_MIPMAPS);
                } else {
                    mat.set_albedo(C_STONE);
                }
                mi.set_surface_override_material(0, &mat);
                let mut col = CollisionShape3D::new_alloc();
                let mut shape = CylinderShape3D::new_gd();
                shape.set_radius(b.radius);
                shape.set_height(b.height);
                col.set_shape(&shape);
                body.add_child(&mi);
                body.add_child(&col);
                root.add_child(&body);
            }
            other => godot::global::godot_warn!("map block: unknown shape '{other}'"),
        }
    }

    // Здания: бокс + вывеска + подсветка
    for bd in &def.buildings {
        let tex = cache.get(&tex_path(&bd.tex));
        let (w, h, d) = (bd.size[0], bd.size[1], bd.size[2]);
        let (cx, cz) = (bd.pos[0], bd.pos[1]);
        let node = make_box(Vector3::new(cx, h * 0.5, cz), Vector3::new(w, h, d),
                            C_STONE, tex.as_ref(), 3.0);
        root.add_child(&node);

        if let Some(ref sign) = bd.sign {
            let side = bd.sign_side.as_deref().unwrap_or("s");
            let sy = h * 0.62;
            let (spos, rot) = match side {
                "n" => (Vector3::new(cx, sy, cz - d * 0.5 - 0.06), std::f32::consts::PI),
                "e" => (Vector3::new(cx + w * 0.5 + 0.06, sy, cz), std::f32::consts::FRAC_PI_2),
                "w" => (Vector3::new(cx - w * 0.5 - 0.06, sy, cz), -std::f32::consts::FRAC_PI_2),
                _   => (Vector3::new(cx, sy, cz + d * 0.5 + 0.06), 0.0),
            };
            if let Some(sp) = make_flat_sprite(cache, &tex_path(sign), spos, rot, 0.022) {
                // Sprite3D по умолчанию unshaded — неон светится сам.
                root.add_child(&sp);
            }
            let l_off = match side {
                "n" => Vector3::new(0.0, 0.0, -1.2),
                "e" => Vector3::new(1.2, 0.0, 0.0),
                "w" => Vector3::new(-1.2, 0.0, 0.0),
                _   => Vector3::new(0.0, 0.0, 1.2),
            };
            let l = make_light(spos + l_off, PINK, 0.8, 7.0);
            root.add_child(&l);
        }
    }

    // Пропсы-биллборды (высота — из текстуры)
    for p in &def.props {
        let path = tex_path(&p.tex);
        if let Some(tex) = cache.get(&path) {
            let h_m = tex.get_height() as f32 * p.px;
            let pos = v3(p.pos) + Vector3::new(0.0, h_m * 0.5 + 0.02, 0.0);
            if let Some(sp) = make_billboard(cache, &path, pos, p.px) {
                root.add_child(&sp);
            }
        }
    }

    // Плоские спрайты (вывески/декали на стенах)
    for f in &def.flats {
        if let Some(sp) = make_flat_sprite(cache, &tex_path(&f.tex), v3(f.pos),
                                           f.rot.to_radians(), f.px) {
            let _ = f.glow; // Sprite3D unshaded по умолчанию; поле оставлено для будущих материалов
            root.add_child(&sp);
        }
    }

    // Свет
    for l in &def.lights {
        let node = make_light(v3(l.pos),
                              Color::from_rgba(l.color[0], l.color[1], l.color[2], 1.0),
                              l.energy, l.range);
        root.add_child(&node);
    }

    // Светящиеся плиты (неон-каналы, лужи)
    for g in &def.glows {
        let tex = cache.get(&tex_path(&g.tex));
        let slab = make_glow_slab(v3(g.pos), v3(g.size), tex.as_ref(),
                                  Color::from_rgba(g.emission[0], g.emission[1], g.emission[2], 1.0),
                                  g.uv);
        root.add_child(&slab);
    }

    // Врата данжа (арка + портал), если заданы
    let gate = def.gate.map(v3);
    if let Some(gp) = gate {
        let t_boss = cache.get("res://assets/textures/wall_boss.png");
        for side in [-1.0f32, 1.0] {
            let p = make_box(gp + Vector3::new(side * 2.6, 2.4, 0.0),
                             Vector3::new(1.2, 4.8, 1.2), C_STONE, t_boss.as_ref(), 1.5);
            root.add_child(&p);
        }
        let lintel = make_box(gp + Vector3::new(0.0, 5.0, 0.0),
                              Vector3::new(6.4, 1.0, 1.4), C_STONE, t_boss.as_ref(), 2.0);
        root.add_child(&lintel);
        if let Some(sp) = make_flat_sprite(cache, "res://assets/sprites/props/neon_game_over.png",
                                           gp + Vector3::new(0.0, 4.0, 0.75), 0.0, 0.02) {
            root.add_child(&sp);
        }
        if let Some(mut sp) = make_billboard(cache, "res://assets/effects/effect_teleport.png",
                                             gp + Vector3::new(0.0, 1.5, 0.0), 0.024) {
            sp.set_modulate(Color::from_rgba(1.0, 0.4, 0.8, 1.0));
            root.add_child(&sp);
        }
        let gl = make_light(gp + Vector3::new(0.0, 2.0, 0.0),
                            Color::from_rgba(0.9, 0.3, 0.9, 1.0), 2.0, 14.0);
        root.add_child(&gl);
    }

    BuiltMap {
        root,
        player_spawn: v3(def.player_spawn),
        gate,
        env: def.env.clone(),
        name_ru: if def.name_ru.is_empty() { def.id.clone() } else { def.name_ru.clone() },
    }
}
