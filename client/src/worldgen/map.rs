//! Построение карты в сцене движка: меши, коллайдеры, свет, биллборды.
//!
//! Формат и разбор карты — в ядре (`openheart_core::worldgen::map_def`),
//! он нужен и серверу; здесь только геометрия.

use godot::classes::base_material_3d::{Flags, TextureFilter, TextureParam};
use godot::classes::{
    file_access::ModeFlags, CollisionShape3D, CylinderMesh, CylinderShape3D, FileAccess,
    MeshInstance3D, Node3D, StandardMaterial3D, StaticBody3D,
};
use godot::prelude::*;

use crate::gfx::{
    make_billboard, make_box, make_box_rot, make_flat_sprite, make_glow_slab, make_light, TexCache,
};

// Формат карты и раскладка текстур — общие с сервером.
pub use openheart_core::worldgen::map_def::*;

pub struct BuiltMap {
    pub root: Gd<Node3D>,
    pub player_spawn: Vector3,
    pub gate: Option<Vector3>,
    pub env: MapEnv,
    pub name_ru: String,
    pub name_en: String,
    pub districts: Vec<MapDistrict>,
    pub ambient: Vec<MapAmbient>,
    /// Зоны станций крафта: (kind, центр, радиус). Геометрию рисуют blocks/props.
    pub stations: Vec<(String, Vector3, f32)>,
}

pub struct MapAmbient {
    pub node: Gd<Node3D>,
    pub origin: Vector3,
    pub phase: f32,
    pub speed: f32,
    pub bob: f32,
    pub spin: f32,
}

#[derive(Clone)]
pub struct MapDistrict {
    pub id: String,
    pub name_ru: String,
    pub name_en: String,
    pub center: Vector3,
    pub radius: f32,
}

impl MapDistrict {
    pub fn name(&self, lang: &str) -> &str {
        if lang == "en" && !self.name_en.is_empty() {
            &self.name_en
        } else {
            &self.name_ru
        }
    }
}

// ── Загрузка ──────────────────────────────────────────────────────────────────

// ── Загрузка ──────────────────────────────────────────────────────────────────

pub fn load_map(preset_base: &str, id: &str) -> Option<MapDef> {
    let path = format!("{preset_base}/maps/{id}.json");
    let f = FileAccess::open(&path, ModeFlags::READ)?;
    match parse_map(&f.get_as_text().to_string()) {
        Ok(m) => Some(m),
        Err(e) => {
            godot::global::godot_warn!("map {path}: {e}");
            None
        }
    }
}

const C_STONE: Color = Color::from_rgba(0.10, 0.07, 0.10, 1.0);
const C_DARK: Color = Color::from_rgba(0.05, 0.03, 0.06, 1.0);
const PINK: Color = Color::from_rgba(1.0, 0.5, 0.75, 1.0);

fn v3(a: [f32; 3]) -> Vector3 {
    Vector3::new(a[0], a[1], a[2])
}

// ── Билдер ────────────────────────────────────────────────────────────────────

pub fn build_map(def: &MapDef, cache: &mut TexCache) -> BuiltMap {
    let mut root = Node3D::new_alloc();
    let mut ambient = Vec::new();

    // Земля плитками (лимит источников света на меш в GL Compatibility)
    if let Some(ref g) = def.ground {
        let tex = cache.get(&tex_path(&g.tex));
        const TILE: f32 = 25.0;
        let n = (g.size / TILE).ceil() as i32;
        for gx in 0..n {
            for gz in 0..n {
                let cx = (gx as f32 + 0.5) * TILE - g.size * 0.5;
                let cz = (gz as f32 + 0.5) * TILE - g.size * 0.5;
                let b = make_box(
                    Vector3::new(cx, -0.15, cz),
                    Vector3::new(TILE, 0.3, TILE),
                    C_DARK,
                    tex.as_ref(),
                    TILE / 25.0 * g.uv,
                );
                root.add_child(&b);
            }
        }
        if g.border_h > 0.0 {
            let bt = g.border_tex.as_ref().map(|t| tex_path(t));
            let btex = bt.as_deref().and_then(|p| cache.get(p));
            let half = g.size * 0.5 - 1.0;
            for (px, pz, sx, sz) in [
                (0.0, -half, g.size, 0.8),
                (0.0, half, g.size, 0.8),
                (-half, 0.0, 0.8, g.size),
                (half, 0.0, 0.8, g.size),
            ] {
                let w = make_box(
                    Vector3::new(px, g.border_h * 0.5, pz),
                    Vector3::new(sx, g.border_h, sz),
                    C_DARK,
                    btex.as_ref(),
                    24.0,
                );
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
                let (Some(pos), Some(size)) = (b.pos, b.size) else {
                    continue;
                };
                let node = make_box_rot(
                    v3(pos),
                    v3(size),
                    b.rot.to_radians(),
                    C_STONE,
                    tex.as_ref(),
                    b.uv,
                );
                root.add_child(&node);
            }
            "ramp" => {
                let (Some(from), Some(to)) = (b.from, b.to) else {
                    continue;
                };
                let (from, to) = (v3(from), v3(to));
                let dir = to - from;
                let horiz = Vector3::new(dir.x, 0.0, dir.z);
                let run = horiz.length().max(0.01);
                let full = (run * run + dir.y * dir.y).sqrt();
                let yaw = (-dir.x).atan2(-dir.z);
                let pitch = (dir.y).atan2(run);
                let mid = (from + to) * 0.5;
                let mut node = make_box(
                    mid,
                    Vector3::new(b.width.max(1.0), 0.3, full + 0.3),
                    C_STONE,
                    tex.as_ref(),
                    (full / 3.0).max(1.0),
                );
                node.set_rotation(Vector3::new(pitch, yaw, 0.0));
                root.add_child(&node);
            }
            "stairs" => {
                let (Some(from), Some(to)) = (b.from, b.to) else {
                    continue;
                };
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
                        yaw,
                        C_STONE,
                        tex.as_ref(),
                        1.0,
                    );
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
                    // World projection also covers the top cap: scaling UVs by
                    // circumference/height stretched the plaza into long stripes.
                    mat.set_uv1_scale(Vector3::splat(1.0 / crate::gfx::TEXEL_M));
                    mat.set_flag(Flags::UV1_USE_TRIPLANAR, true);
                    mat.set_flag(Flags::UV1_USE_WORLD_TRIPLANAR, true);
                    mat.set_texture_filter(TextureFilter::LINEAR_WITH_MIPMAPS_ANISOTROPIC);
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
            other => {
                godot::global::godot_warn!("map block: unknown shape '{other}'")
            }
        }
    }

    // Здания: бокс + вывеска + подсветка
    for bd in &def.buildings {
        let tex = cache.get(&tex_path(&bd.tex));
        let (w, h, d) = (bd.size[0], bd.size[1], bd.size[2]);
        let (cx, cz) = (bd.pos[0], bd.pos[1]);
        let node = make_box(
            Vector3::new(cx, h * 0.5, cz),
            Vector3::new(w, h, d),
            C_STONE,
            tex.as_ref(),
            3.0,
        );
        root.add_child(&node);

        if let Some(ref sign) = bd.sign {
            let side = bd.sign_side.as_deref().unwrap_or("s");
            let sy = h * 0.62;
            let (spos, rot) = match side {
                "n" => (
                    Vector3::new(cx, sy, cz - d * 0.5 - 0.06),
                    std::f32::consts::PI,
                ),
                "e" => (
                    Vector3::new(cx + w * 0.5 + 0.06, sy, cz),
                    std::f32::consts::FRAC_PI_2,
                ),
                "w" => (
                    Vector3::new(cx - w * 0.5 - 0.06, sy, cz),
                    -std::f32::consts::FRAC_PI_2,
                ),
                _ => (Vector3::new(cx, sy, cz + d * 0.5 + 0.06), 0.0),
            };
            if let Some(sp) = make_flat_sprite(cache, &tex_path(sign), spos, rot, 0.022) {
                // Sprite3D по умолчанию unshaded — неон светится сам.
                root.add_child(&sp);
            }
            let l_off = match side {
                "n" => Vector3::new(0.0, 0.0, -1.2),
                "e" => Vector3::new(1.2, 0.0, 0.0),
                "w" => Vector3::new(-1.2, 0.0, 0.0),
                _ => Vector3::new(0.0, 0.0, 1.2),
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
        if let Some(sp) = make_flat_sprite(
            cache,
            &tex_path(&f.tex),
            v3(f.pos),
            f.rot.to_radians(),
            f.px,
        ) {
            let _ = f.glow; // Sprite3D unshaded по умолчанию; поле оставлено для будущих материалов
            root.add_child(&sp);
        }
    }

    // Свет
    for l in &def.lights {
        let node = make_light(
            v3(l.pos),
            Color::from_rgba(l.color[0], l.color[1], l.color[2], 1.0),
            l.energy,
            l.range,
        );
        root.add_child(&node);
    }

    // Светящиеся плиты (неон-каналы, лужи)
    for g in &def.glows {
        let tex = cache.get(&tex_path(&g.tex));
        let slab = make_glow_slab(
            v3(g.pos),
            v3(g.size),
            tex.as_ref(),
            Color::from_rgba(g.emission[0], g.emission[1], g.emission[2], 1.0),
            g.uv,
        );
        root.add_child(&slab);
    }

    // Районы: тонкая световая рамка, локальная палитра и уникальный landmark.
    for route in &def.route_layers {
        let mut route_root = Node3D::new_alloc();
        route_root.set_name(&format!("Route_{}", route.id));
        route_root.add_to_group("map_route_layers");
        let color = Color::from_rgba(route.color[0], route.color[1], route.color[2], 1.0);
        let glow_path = route
            .glow_tex
            .as_deref()
            .map(tex_path)
            .unwrap_or_else(|| "res://assets/textures/dungeon/liquid_purple.png".to_string());
        let texture = cache.get(&glow_path);
        let width = route.width.clamp(0.25, 4.0);
        for pair in route.points.windows(2) {
            let from = v3(pair[0]);
            let to = v3(pair[1]);
            let delta = to - from;
            let horizontal = Vector3::new(delta.x, 0.0, delta.z);
            let length = horizontal.length();
            if length < 0.25 {
                continue;
            }
            let mut segment = make_glow_slab(
                (from + to) * 0.5 + Vector3::new(0.0, 0.025, 0.0),
                Vector3::new(width, 0.018, length),
                texture.as_ref(),
                color,
                (length / 4.0 * route.uv).max(1.0),
            );
            segment.set_rotation(Vector3::new(0.0, (-horizontal.x).atan2(-horizontal.z), 0.0));
            route_root.add_child(&segment);
        }
        for point in &route.points {
            route_root.add_child(&make_glow_slab(
                v3(*point) + Vector3::new(0.0, 0.03, 0.0),
                Vector3::new(width * 1.28, 0.022, width * 1.28),
                texture.as_ref(),
                color,
                1.0,
            ));
        }
        root.add_child(&route_root);
    }

    for beacon in &def.skyline_beacons {
        let mut beacon_root = Node3D::new_alloc();
        beacon_root.set_name(&format!("SkylineBeacon_{}", beacon.id));
        beacon_root.add_to_group("map_skyline_beacons");
        let color = Color::from_rgba(beacon.color[0], beacon.color[1], beacon.color[2], 1.0);
        let glow_path = beacon
            .glow_tex
            .as_deref()
            .map(tex_path)
            .unwrap_or_else(|| "res://assets/textures/dungeon/liquid_pink.png".to_string());
        let texture = cache.get(&glow_path);
        let height = beacon.height.clamp(3.0, 30.0);
        let base = Vector3::new(beacon.pos[0], 0.0, beacon.pos[1]);
        beacon_root.add_child(&make_glow_slab(
            base + Vector3::new(0.0, height * 0.5, 0.0),
            Vector3::new(0.18, height, 0.18),
            texture.as_ref(),
            color,
            (height / 3.0).max(1.0),
        ));
        for size in [Vector3::new(2.4, 0.12, 0.14), Vector3::new(0.14, 0.12, 2.4)] {
            beacon_root.add_child(&make_glow_slab(
                base + Vector3::new(0.0, height, 0.0),
                size,
                texture.as_ref(),
                color,
                1.0,
            ));
        }
        if let Some(crown) = beacon.crown.as_deref() {
            let path = tex_path(crown);
            if let Some(mut sprite) = make_billboard(
                cache,
                &path,
                base + Vector3::new(0.0, height + 1.4, 0.0),
                beacon.crown_px.clamp(0.008, 0.05),
            ) {
                sprite.set_modulate(Color::from_rgba(
                    0.7 + color.r * 0.3,
                    0.7 + color.g * 0.3,
                    0.7 + color.b * 0.3,
                    1.0,
                ));
                beacon_root.add_child(&sprite);
            }
        }
        beacon_root.add_child(&make_light(
            base + Vector3::new(0.0, height, 0.0),
            color,
            1.1,
            10.0,
        ));
        root.add_child(&beacon_root);
    }

    for district in &def.districts {
        let center = Vector3::new(district.center[0], 0.035, district.center[1]);
        let radius = district.radius.max(4.0);
        let color = Color::from_rgba(district.color[0], district.color[1], district.color[2], 1.0);
        let outline_path = district
            .outline_tex
            .as_deref()
            .map(tex_path)
            .unwrap_or_else(|| "res://assets/textures/dungeon/liquid_purple.png".to_string());
        let outline = cache.get(&outline_path);
        for (offset, size) in [
            (
                Vector3::new(0.0, 0.0, -radius * 0.58),
                Vector3::new(radius * 1.16, 0.025, 0.16),
            ),
            (
                Vector3::new(0.0, 0.0, radius * 0.58),
                Vector3::new(radius * 1.16, 0.025, 0.16),
            ),
            (
                Vector3::new(-radius * 0.58, 0.0, 0.0),
                Vector3::new(0.16, 0.025, radius * 1.16),
            ),
            (
                Vector3::new(radius * 0.58, 0.0, 0.0),
                Vector3::new(0.16, 0.025, radius * 1.16),
            ),
        ] {
            root.add_child(&make_glow_slab(
                center + offset,
                size,
                outline.as_ref(),
                color,
                2.0,
            ));
        }
        root.add_child(&make_light(
            center + Vector3::new(0.0, 3.2, 0.0),
            color,
            0.55,
            radius * 0.72,
        ));
        if let Some(landmark) = &district.landmark {
            let path = tex_path(&landmark.tex);
            if let Some(texture) = cache.get(&path) {
                let height = texture.get_height() as f32 * landmark.px;
                let position = v3(landmark.pos) + Vector3::new(0.0, height * 0.5 + 0.05, 0.0);
                if let Some(sprite) = make_billboard(cache, &path, position, landmark.px) {
                    root.add_child(&sprite);
                }
                root.add_child(&make_light(
                    position + Vector3::new(0.0, 1.0, 0.0),
                    color,
                    1.25,
                    10.0,
                ));
            }
        }
    }

    // Плотные тематические кластеры: детерминированные кольца пропсов и вертикального неона.
    for cluster in &def.decor_clusters {
        let mut cluster_root = Node3D::new_alloc();
        cluster_root.set_name(&format!("DecorCluster_{}", cluster.id));
        cluster_root.add_to_group("map_decor_clusters");
        let center = Vector3::new(cluster.center[0], 0.0, cluster.center[1]);
        let radius = cluster.radius.max(2.0);
        let density = cluster.density.clamp(3, 32) as usize;
        let color = Color::from_rgba(cluster.color[0], cluster.color[1], cluster.color[2], 1.0);
        let seed = cluster.id.bytes().fold(0x9E37_79B9u32, |hash, byte| {
            hash.rotate_left(5) ^ byte as u32
        });
        let glow_path = cluster
            .glow_tex
            .as_deref()
            .map(tex_path)
            .unwrap_or_else(|| "res://assets/textures/dungeon/liquid_purple.png".to_string());
        let glow_texture = cache.get(&glow_path);

        for index in 0..density {
            let noise = seed
                .wrapping_add(index as u32 * 0x45D_9F3B)
                .rotate_left((index % 17) as u32);
            let jitter = (noise & 1023) as f32 / 1023.0;
            let angle =
                std::f32::consts::TAU * (index as f32 / density as f32) + (jitter - 0.5) * 0.38;
            let distance = radius * (0.48 + ((noise >> 10) & 255) as f32 / 255.0 * 0.46);
            let local = Vector3::new(angle.cos() * distance, 0.0, angle.sin() * distance);

            if !cluster.props.is_empty() {
                let prop_id = &cluster.props[(noise as usize) % cluster.props.len()];
                let path = tex_path(prop_id);
                if let Some(texture) = cache.get(&path) {
                    let px = 0.015 + ((noise >> 18) & 31) as f32 / 31.0 * 0.006;
                    let height = texture.get_height() as f32 * px;
                    if let Some(mut sprite) = make_billboard(
                        cache,
                        &path,
                        center + local + Vector3::new(0.0, height * 0.5 + 0.03, 0.0),
                        px,
                    ) {
                        sprite.set_modulate(Color::from_rgba(
                            0.72 + color.r * 0.28,
                            0.72 + color.g * 0.28,
                            0.72 + color.b * 0.28,
                            1.0,
                        ));
                        cluster_root.add_child(&sprite);
                    }
                }
            }

            if index % 2 == 0 {
                let height = 1.4 + ((noise >> 23) & 15) as f32 * 0.12;
                cluster_root.add_child(&make_glow_slab(
                    center + local * 0.82 + Vector3::new(0.0, height * 0.5, 0.0),
                    Vector3::new(0.11, height, 0.11),
                    glow_texture.as_ref(),
                    color,
                    1.0,
                ));
            }
            if index % 4 == 0 {
                cluster_root.add_child(&make_light(
                    center + local * 0.76 + Vector3::new(0.0, 2.2, 0.0),
                    color,
                    0.7,
                    6.5,
                ));
            }
        }

        if let Some(ambient_prop) = cluster
            .props
            .iter()
            .find(|prop| prop.starts_with("neon_"))
            .or_else(|| cluster.props.first())
        {
            let path = tex_path(ambient_prop);
            let origin = center + Vector3::new(0.0, 4.2, 0.0);
            if let Some(mut sprite) = make_billboard(cache, &path, origin, 0.022) {
                sprite.set_name(&format!("Ambient_{}", cluster.id));
                sprite.add_to_group("map_ambient");
                sprite.set_modulate(Color::from_rgba(
                    0.72 + color.r * 0.28,
                    0.72 + color.g * 0.28,
                    0.72 + color.b * 0.28,
                    0.9,
                ));
                sprite.add_child(&make_light(Vector3::ZERO, color, 1.1, 8.0));
                let node: Gd<Node3D> = sprite.clone().upcast();
                ambient.push(MapAmbient {
                    node,
                    origin,
                    phase: (seed & 1023) as f32 / 1023.0 * std::f32::consts::TAU,
                    speed: cluster.speed.clamp(0.15, 4.0),
                    bob: cluster.bob.clamp(0.0, 1.5),
                    spin: cluster.spin.to_radians().clamp(-2.0, 2.0),
                });
                cluster_root.add_child(&sprite);
            }
        }
        root.add_child(&cluster_root);
    }

    // Врата данжа (арка + портал), если заданы
    let gate = def.gate.map(v3);
    if let Some(gp) = gate {
        let t_boss = cache.get("res://assets/textures/wall_boss.png");
        for side in [-1.0f32, 1.0] {
            let p = make_box(
                gp + Vector3::new(side * 2.6, 2.4, 0.0),
                Vector3::new(1.2, 4.8, 1.2),
                C_STONE,
                t_boss.as_ref(),
                1.5,
            );
            root.add_child(&p);
        }
        let lintel = make_box(
            gp + Vector3::new(0.0, 5.0, 0.0),
            Vector3::new(6.4, 1.0, 1.4),
            C_STONE,
            t_boss.as_ref(),
            2.0,
        );
        root.add_child(&lintel);
        if let Some(sp) = make_flat_sprite(
            cache,
            "res://assets/sprites/props/neon_game_over.png",
            gp + Vector3::new(0.0, 4.0, 0.75),
            0.0,
            0.02,
        ) {
            root.add_child(&sp);
        }
        if let Some(mut sp) = make_billboard(
            cache,
            "res://assets/effects/effect_teleport.png",
            gp + Vector3::new(0.0, 1.5, 0.0),
            0.024,
        ) {
            sp.set_modulate(Color::from_rgba(1.0, 0.4, 0.8, 1.0));
            root.add_child(&sp);
        }
        let gl = make_light(
            gp + Vector3::new(0.0, 2.0, 0.0),
            Color::from_rgba(0.9, 0.3, 0.9, 1.0),
            2.0,
            14.0,
        );
        root.add_child(&gl);
    }

    BuiltMap {
        root,
        player_spawn: v3(def.player_spawn),
        gate,
        env: def.env.clone(),
        name_ru: if def.name_ru.is_empty() {
            def.id.clone()
        } else {
            def.name_ru.clone()
        },
        name_en: def.name_en.clone(),
        districts: def
            .districts
            .iter()
            .map(|district| MapDistrict {
                id: district.id.clone(),
                name_ru: district.name_ru.clone(),
                name_en: district.name_en.clone(),
                center: Vector3::new(district.center[0], 0.0, district.center[1]),
                radius: district.radius.max(4.0),
            })
            .collect(),
        ambient,
        stations: def
            .stations
            .iter()
            .map(|s| (s.kind.clone(), Vector3::new(s.pos[0], 0.0, s.pos[1]), s.radius))
            .collect(),
    }
}
