//! Графические утилиты: боксы, спрайты-биллборды, свет, кэш текстур.

use godot::prelude::*;
use godot::classes::{
    BoxShape3D, CollisionShape3D, Image, ImageTexture, MeshInstance3D, BoxMesh,
    OmniLight3D, Sprite3D, StandardMaterial3D, StaticBody3D, Texture2D,
};
use godot::classes::base_material_3d::{BillboardMode, TextureParam, TextureFilter, Feature, Flags};
use godot::classes::sprite_base_3d::AlphaCutMode;
use std::collections::HashMap;

/// Кэш текстур на время построения мира.
#[derive(Default)]
pub struct TexCache {
    map: HashMap<String, Option<Gd<Texture2D>>>,
}

impl TexCache {
    pub fn new() -> Self { Self { map: HashMap::new() } }

    pub fn get(&mut self, path: &str) -> Option<Gd<Texture2D>> {
        if let Some(t) = self.map.get(path) {
            return t.clone();
        }
        // Генерируем мип-мапы: убирает мерцание/алиасинг вдали и на мелких
        // спрайтах, и оживляет фильтры с мип-мапами у окружения.
        let tex = Image::load_from_file(path)
            .and_then(|mut img| {
                img.generate_mipmaps();
                ImageTexture::create_from_image(&img)
            })
            .map(|t| t.upcast::<Texture2D>());
        self.map.insert(path.to_string(), tex.clone());
        tex
    }
}

/// Мировой размер одного повтора текстуры (м). Квадратные тексели на всех
/// гранях достигаются triplanar-проекцией — параметр `uv` в make_box больше
/// не используется для масштаба (оставлен для совместимости вызовов).
pub const TEXEL_M: f32 = 3.0;

/// Текстурированный (или цветной) бокс со столкновением.
///
/// Текстуры кладутся мировым triplanar-маппингом: одинаковый квадратный тексель
/// на полу/стене/потолке независимо от пропорций бокса, и бесшовно между боксами.
pub fn make_box(
    pos: Vector3, size: Vector3, color: Color,
    tex: Option<&Gd<Texture2D>>, _uv: f32,
) -> Gd<StaticBody3D> {
    let mut body = StaticBody3D::new_alloc();
    body.set_position(pos);

    let mut mi = MeshInstance3D::new_alloc();
    let mut mesh = BoxMesh::new_gd();
    mesh.set_size(size);
    mi.set_mesh(&mesh);

    let mut mat = StandardMaterial3D::new_gd();
    if let Some(t) = tex {
        mat.set_albedo(Color::WHITE);
        mat.set_texture(TextureParam::ALBEDO, t);
        mat.set_uv1_scale(Vector3::splat(1.0 / TEXEL_M));
        mat.set_flag(Flags::UV1_USE_TRIPLANAR, true);
        mat.set_flag(Flags::UV1_USE_WORLD_TRIPLANAR, true);
        mat.set_texture_filter(TextureFilter::NEAREST_WITH_MIPMAPS_ANISOTROPIC);
    } else {
        mat.set_albedo(color);
    }
    mi.set_surface_override_material(0, &mat);

    let mut col = CollisionShape3D::new_alloc();
    let mut shape = BoxShape3D::new_gd();
    shape.set_size(size);
    col.set_shape(&shape);

    body.add_child(&mi);
    body.add_child(&col);
    body
}

/// Бокс с поворотом вокруг Y.
pub fn make_box_rot(
    pos: Vector3, size: Vector3, rot_y: f32, color: Color,
    tex: Option<&Gd<Texture2D>>, uv: f32,
) -> Gd<StaticBody3D> {
    let mut b = make_box(pos, size, color, tex, uv);
    b.set_rotation(Vector3::new(0.0, rot_y, 0.0));
    b
}

/// Наклонный слэб-пандус с коллизией: соединяет два уровня пола плавным склоном.
/// `low`/`high` — мировые точки поверхности пола на нижнем и верхнем концах,
/// `width` — ширина пандуса, `thick` — толщина плиты.
pub fn make_ramp(
    low: Vector3, high: Vector3, width: f32, thick: f32,
    color: Color, tex: Option<&Gd<Texture2D>>,
) -> Gd<StaticBody3D> {
    let dir = high - low;
    let len = dir.length().max(0.01);
    let dirn = dir / len;
    // ось ширины — перпендикуляр к направлению склона в горизонтали
    let right = Vector3::UP.cross(dirn).normalized();
    let up = dirn.cross(right).normalized();
    let uv = (len / 3.0).max(1.0);
    // размеры запекаем в меш/шейп; basis — чистое вращение (ортонормированный)
    let mut body = make_box(Vector3::ZERO, Vector3::new(width, thick, len), color, tex, uv);
    let basis = Basis::from_cols(right, up, dirn);
    body.set_transform(Transform3D {
        basis,
        origin: (low + high) * 0.5 - up * (thick * 0.5),
    });
    body
}

/// Светящийся декоративный бокс (без коллизии), например лужа лавы.
pub fn make_glow_slab(
    pos: Vector3, size: Vector3, tex: Option<&Gd<Texture2D>>, emission: Color, uv: f32,
) -> Gd<MeshInstance3D> {
    let mut mi = MeshInstance3D::new_alloc();
    let mut mesh = BoxMesh::new_gd();
    mesh.set_size(size);
    mi.set_mesh(&mesh);
    mi.set_position(pos);
    let mut mat = StandardMaterial3D::new_gd();
    if let Some(t) = tex {
        mat.set_albedo(Color::WHITE);
        mat.set_texture(TextureParam::ALBEDO, t);
        mat.set_uv1_scale(Vector3::new(uv, uv, 1.0));
        mat.set_texture_filter(TextureFilter::NEAREST_WITH_MIPMAPS_ANISOTROPIC);
    }
    mat.set_feature(Feature::EMISSION, true);
    mat.set_emission(emission);
    mat.set_emission_energy_multiplier(0.9);
    mi.set_surface_override_material(0, &mat);
    mi
}

/// Спрайт-биллборд (всегда лицом к камере) из файла.
pub fn make_billboard(
    cache: &mut TexCache, path: &str, pos: Vector3, pixel_size: f32,
) -> Option<Gd<Sprite3D>> {
    let tex = cache.get(path)?;
    let mut sp = Sprite3D::new_alloc();
    sp.set_position(pos);
    sp.set_pixel_size(pixel_size);
    sp.set_billboard_mode(BillboardMode::ENABLED);
    sp.set_alpha_cut_mode(AlphaCutMode::DISCARD);
    sp.set_texture_filter(TextureFilter::NEAREST_WITH_MIPMAPS);
    sp.set_texture(&tex);
    Some(sp)
}

/// Плоский спрайт на стене (без биллборда), повёрнут на rot_y.
pub fn make_flat_sprite(
    cache: &mut TexCache, path: &str, pos: Vector3, rot_y: f32, pixel_size: f32,
) -> Option<Gd<Sprite3D>> {
    let tex = cache.get(path)?;
    let mut sp = Sprite3D::new_alloc();
    sp.set_position(pos);
    sp.set_rotation(Vector3::new(0.0, rot_y, 0.0));
    sp.set_pixel_size(pixel_size);
    sp.set_alpha_cut_mode(AlphaCutMode::DISCARD);
    sp.set_texture_filter(TextureFilter::NEAREST_WITH_MIPMAPS);
    sp.set_texture(&tex);
    Some(sp)
}

/// Точечный свет.
pub fn make_light(pos: Vector3, color: Color, energy: f32, range: f32) -> Gd<OmniLight3D> {
    use godot::classes::light_3d::Param;
    let mut l = OmniLight3D::new_alloc();
    l.set_position(pos);
    l.set_color(color);
    l.set_param(Param::ENERGY, energy);
    l.set_param(Param::RANGE, range);
    l.set_param(Param::ATTENUATION, 0.5);
    l
}

/// Простой xorshift64* — детерминированный RNG для генерации.
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self { Self(seed.max(1)) }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    /// [0, n)
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 { return 0; }
        (self.next() >> 33) as u32 % n
    }

    /// [lo, hi] включительно
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo { return lo; }
        lo + self.below((hi - lo + 1) as u32) as i32
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u32) as usize]
    }

    pub fn f32(&mut self) -> f32 {
        ((self.next() >> 40) as f32) / ((1u64 << 24) as f32)
    }
}
