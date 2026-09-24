//! Формат карты: `presets/<id>/maps/*.json` — данные, а не код.
//!
//! Поддерживаемые фигуры: box (с поворотом), ramp (наклонная плита from→to),
//! stairs (лестница из N ступеней), cylinder (колонна/башня). Плюс декларативные
//! «здания» (бокс + вывеска + свет), пропсы-биллборды, плоские спрайты, свет,
//! светящиеся плиты, спавны и портал данжа.
//!
//! Модель живёт в ядре: по этим же данным сервер строит грубую геометрию для
//! проверки попаданий, а клиент — меши (`client/src/worldgen/map.rs`).

use serde::Deserialize;

use crate::config::LevelCfg;

// ── Формат карты ──────────────────────────────────────────────────────────────

fn f_uv1() -> f32 {
    1.0
}
fn f_px() -> f32 {
    0.02
}
fn f_cluster_radius() -> f32 {
    8.0
}
fn f_cluster_density() -> u32 {
    10
}
fn f_ambient_bob() -> f32 {
    0.22
}
fn f_ambient_speed() -> f32 {
    1.0
}

#[derive(Deserialize, Clone, Default)]
pub struct MapEnv {
    #[serde(default)]
    pub sky: Option<String>,
    #[serde(default)]
    pub fog_density: Option<f32>,
    #[serde(default)]
    pub ambient: Option<[f32; 3]>,
    #[serde(default)]
    pub ambient_energy: Option<f32>,
}

#[derive(Deserialize, Clone)]
pub struct BlockDef {
    pub shape: String, // box | ramp | stairs | cylinder
    #[serde(default)]
    pub pos: Option<[f32; 3]>, // box/cylinder
    #[serde(default)]
    pub size: Option<[f32; 3]>, // box
    #[serde(default)]
    pub rot: f32, // box: поворот вокруг Y, градусы
    #[serde(default)]
    pub from: Option<[f32; 3]>, // ramp/stairs
    #[serde(default)]
    pub to: Option<[f32; 3]>,
    #[serde(default)]
    pub width: f32, // ramp/stairs
    #[serde(default, deserialize_with = "crate::config::de_u32")]
    pub steps: u32, // stairs
    #[serde(default)]
    pub radius: f32, // cylinder
    #[serde(default)]
    pub height: f32, // cylinder
    #[serde(default)]
    pub tex: Option<String>,
    #[serde(default = "f_uv1")]
    pub uv: f32,
}

#[derive(Deserialize, Clone)]
pub struct BuildingDef {
    pub pos: [f32; 2],
    pub size: [f32; 3],
    pub tex: String,
    #[serde(default)]
    pub sign: Option<String>,
    #[serde(default)]
    pub sign_side: Option<String>, // n|s|e|w (куда смотрит вывеска)
}

#[derive(Deserialize, Clone)]
pub struct PropDef {
    pub tex: String,
    pub pos: [f32; 3],
    #[serde(default = "f_px")]
    pub px: f32,
}

#[derive(Deserialize, Clone)]
pub struct FlatDef {
    pub tex: String,
    pub pos: [f32; 3],
    #[serde(default)]
    pub rot: f32, // градусы вокруг Y
    #[serde(default = "f_px")]
    pub px: f32,
    #[serde(default)]
    pub glow: bool, // unshaded-неон
}

#[derive(Deserialize, Clone)]
pub struct LightDef {
    pub pos: [f32; 3],
    pub color: [f32; 3],
    pub energy: f32,
    pub range: f32,
}

#[derive(Deserialize, Clone)]
pub struct GlowDef {
    pub pos: [f32; 3],
    pub size: [f32; 3],
    pub tex: String,
    pub emission: [f32; 3],
    #[serde(default = "f_uv1")]
    pub uv: f32,
}

#[derive(Deserialize, Clone)]
pub struct DistrictDef {
    pub id: String,
    pub name_ru: String,
    pub name_en: String,
    pub center: [f32; 2],
    pub radius: f32,
    pub color: [f32; 3],
    #[serde(default)]
    pub outline_tex: Option<String>,
    #[serde(default)]
    pub landmark: Option<PropDef>,
}

#[derive(Deserialize, Clone)]
pub struct DecorClusterDef {
    pub id: String,
    pub center: [f32; 2],
    #[serde(default = "f_cluster_radius")]
    pub radius: f32,
    #[serde(
        default = "f_cluster_density",
        deserialize_with = "crate::config::de_u32"
    )]
    pub density: u32,
    pub color: [f32; 3],
    #[serde(default)]
    pub props: Vec<String>,
    #[serde(default)]
    pub glow_tex: Option<String>,
    #[serde(default = "f_ambient_bob")]
    pub bob: f32,
    #[serde(default)]
    pub spin: f32,
    #[serde(default = "f_ambient_speed")]
    pub speed: f32,
}

#[derive(Deserialize, Clone)]
pub struct RouteLayerDef {
    pub id: String,
    pub points: Vec<[f32; 3]>,
    #[serde(default = "f_route_width")]
    pub width: f32,
    pub color: [f32; 3],
    #[serde(default)]
    pub glow_tex: Option<String>,
    #[serde(default = "f_uv1")]
    pub uv: f32,
}

fn f_route_width() -> f32 {
    1.2
}

#[derive(Deserialize, Clone)]
pub struct SkylineBeaconDef {
    pub id: String,
    pub pos: [f32; 2],
    pub height: f32,
    pub color: [f32; 3],
    #[serde(default)]
    pub glow_tex: Option<String>,
    #[serde(default)]
    pub crown: Option<String>,
    #[serde(default = "f_px")]
    pub crown_px: f32,
}

#[derive(Deserialize, Clone, Default)]
pub struct GroundDef {
    pub size: f32,
    pub tex: String,
    #[serde(default = "f_uv1")]
    pub uv: f32,
    /// Высота стен-границ по периметру (0 = без стен).
    #[serde(default)]
    pub border_h: f32,
    #[serde(default)]
    pub border_tex: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct MapDef {
    pub id: String,
    #[serde(default)]
    pub name_ru: String,
    #[serde(default)]
    pub name_en: String,
    #[serde(default)]
    pub env: MapEnv,
    pub player_spawn: [f32; 3],
    #[serde(default)]
    pub gate: Option<[f32; 3]>, // портал данжа (строится арка)
    #[serde(default)]
    pub ground: Option<GroundDef>,
    #[serde(default)]
    pub blocks: Vec<BlockDef>,
    #[serde(default)]
    pub buildings: Vec<BuildingDef>,
    #[serde(default)]
    pub props: Vec<PropDef>,
    #[serde(default)]
    pub flats: Vec<FlatDef>,
    #[serde(default)]
    pub lights: Vec<LightDef>,
    #[serde(default)]
    pub glows: Vec<GlowDef>,
    #[serde(default)]
    pub districts: Vec<DistrictDef>,
    #[serde(default)]
    pub decor_clusters: Vec<DecorClusterDef>,
    #[serde(default)]
    pub route_layers: Vec<RouteLayerDef>,
    #[serde(default)]
    pub skyline_beacons: Vec<SkylineBeaconDef>,
    /// Интерактивные станции (верстак крафта и т.п.): зоны, где доступны
    /// станочные рецепты. Геометрию станции рисуют blocks/props карты.
    #[serde(default)]
    pub stations: Vec<StationDef>,
    #[serde(default)]
    pub spawns: LevelCfg,
}

fn d_station_radius() -> f32 {
    4.0
}

/// Станция крафта на карте: тип (`kind`, напр. "bench") и зона взаимодействия.
#[derive(Deserialize, Clone)]
pub struct StationDef {
    pub kind: String,
    pub pos: [f32; 2],
    #[serde(default = "d_station_radius")]
    pub radius: f32,
}

// ── Утилиты текстур ───────────────────────────────────────────────────────────

/// Текстуры карты ищутся по короткому имени в стандартных папках.
pub fn tex_path(name: &str) -> String {
    if name.contains('/') {
        return format!("res://assets/{name}.png");
    }
    if name.starts_with("dtile_") || name.starts_with("liquid_") {
        format!("res://assets/textures/dungeon/{name}.png")
    } else if name.starts_with("sky_") {
        format!("res://assets/textures/sky/{name}.png")
    } else if name.starts_with("neon_")
        || name.starts_with("street_")
        || name.starts_with("furn_")
        || name.starts_with("bath_")
    {
        format!("res://assets/sprites/props/{name}.png")
    } else if name.starts_with("effect_") {
        format!("res://assets/effects/{name}.png")
    } else if name.starts_with("item_") {
        format!("res://assets/sprites/items/{name}.png")
    } else if name.starts_with("ammo_")
        || name == "soul"
        || name.starts_with("heart_")
        || name == "grenade"
        || name == "scroll"
    {
        format!("res://assets/sprites/pickups/{name}.png")
    } else {
        format!("res://assets/textures/{name}.png")
    }
}

/// Разобрать карту из JSON. Читает файл хост: клиент — движком, сервер — с диска.
pub fn parse_map(text: &str) -> Result<MapDef, String> {
    serde_json::from_str::<MapDef>(text).map_err(|error| error.to_string())
}
