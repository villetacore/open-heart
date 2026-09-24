//! Арсенал: типы боеприпасов, описания оружия, состояние оружия игрока.
//!
//! Оружие — data-driven: параметры грузятся из `presets/<id>/weapons.json` через [`load`].
//! Если файл сломан/отсутствует — используется встроенная копия (`include_str!`),
//! так что игра никогда не падает из-за опечатки в конфиге.
//!
//! Хранилище перезагружаемое (смена пресета в меню): таблица кладётся в `RwLock`
//! через `Box::leak`, чтобы сохранить `&'static`-API. Утечка пары КБ на смену
//! пресета — осознанный трейд-офф (переключений за сессию единицы).

use serde::Deserialize;
use std::sync::RwLock;

// ── Боеприпасы ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum AmmoType {
    Bullets,
    Shells,
    Rockets,
    Cells,
}

impl AmmoType {
    pub const ALL: [AmmoType; 4] = [
        AmmoType::Bullets,
        AmmoType::Shells,
        AmmoType::Rockets,
        AmmoType::Cells,
    ];

    pub fn idx(&self) -> usize {
        match self {
            Self::Bullets => 0,
            Self::Shells => 1,
            Self::Rockets => 2,
            Self::Cells => 3,
        }
    }
    pub fn from_idx(i: usize) -> Self {
        Self::ALL[i % 4]
    }

    /// Разбор строкового id из конфигов.
    pub fn from_id(s: &str) -> Option<Self> {
        Some(match s {
            "bullets" => Self::Bullets,
            "shells" => Self::Shells,
            "rockets" => Self::Rockets,
            "cells" => Self::Cells,
            _ => return None,
        })
    }

    pub fn name_ru(&self) -> &'static str {
        match self {
            Self::Bullets => "Патроны",
            Self::Shells => "Дробь",
            Self::Rockets => "Ракеты",
            Self::Cells => "Энергия",
        }
    }
    pub fn name(&self, lang: &str) -> &'static str {
        if lang != "en" {
            return self.name_ru();
        }
        match self {
            Self::Bullets => "Bullets",
            Self::Shells => "Shells",
            Self::Rockets => "Rockets",
            Self::Cells => "Energy Cells",
        }
    }
    pub fn max(&self) -> u32 {
        match self {
            Self::Bullets => 240,
            Self::Shells => 60,
            Self::Rockets => 24,
            Self::Cells => 180,
        }
    }
    /// Спрайт пикапа в мире.
    pub fn pickup_tex(&self) -> &'static str {
        match self {
            Self::Bullets => "res://assets/sprites/pickups/ammo_bullets.png",
            Self::Shells => "res://assets/sprites/pickups/ammo_shells.png",
            Self::Rockets => "res://assets/sprites/pickups/ammo_rockets.png",
            Self::Cells => "res://assets/sprites/pickups/ammo_cells.png",
        }
    }
    /// Стандартная пачка при подборе.
    pub fn pack_size(&self) -> u32 {
        match self {
            Self::Bullets => 30,
            Self::Shells => 8,
            Self::Rockets => 4,
            Self::Cells => 30,
        }
    }
}

// ── Типы урона ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum DmgType {
    Physical,
    Fire,
    Energy,
    Void,
}

impl DmgType {
    pub const ALL: [DmgType; 4] = [
        DmgType::Physical,
        DmgType::Fire,
        DmgType::Energy,
        DmgType::Void,
    ];

    pub fn idx(&self) -> usize {
        match self {
            Self::Physical => 0,
            Self::Fire => 1,
            Self::Energy => 2,
            Self::Void => 3,
        }
    }
    pub fn from_id(s: &str) -> Option<Self> {
        Some(match s {
            "physical" => Self::Physical,
            "fire" => Self::Fire,
            "energy" => Self::Energy,
            "void" => Self::Void,
            _ => return None,
        })
    }
    pub fn name_ru(&self) -> &'static str {
        match self {
            Self::Physical => "Физический",
            Self::Fire => "Огонь",
            Self::Energy => "Энергия",
            Self::Void => "Пустота",
        }
    }
    pub fn name(&self, lang: &str) -> &'static str {
        if lang != "en" {
            return self.name_ru();
        }
        match self {
            Self::Physical => "Physical",
            Self::Fire => "Fire",
            Self::Energy => "Energy",
            Self::Void => "Void",
        }
    }
}

// ── Оружие ────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum WeaponId {
    Sword,
    Chainsaw,
    Pistol,
    Shotgun,
    Rifle,
    Nailgun,
    Plasma,
    Rocket,
}

impl WeaponId {
    pub const ALL: [WeaponId; 8] = [
        WeaponId::Sword,
        WeaponId::Chainsaw,
        WeaponId::Pistol,
        WeaponId::Shotgun,
        WeaponId::Rifle,
        WeaponId::Nailgun,
        WeaponId::Plasma,
        WeaponId::Rocket,
    ];
    pub fn slot(&self) -> usize {
        match self {
            Self::Sword => 0,
            Self::Chainsaw => 1,
            Self::Pistol => 2,
            Self::Shotgun => 3,
            Self::Rifle => 4,
            Self::Nailgun => 5,
            Self::Plasma => 6,
            Self::Rocket => 7,
        }
    }
    pub fn from_slot(i: usize) -> Self {
        Self::ALL[i % 8]
    }

    /// Разбор строкового id из конфигов.
    pub fn from_id(s: &str) -> Option<Self> {
        Some(match s {
            "sword" => Self::Sword,
            "chainsaw" => Self::Chainsaw,
            "pistol" => Self::Pistol,
            "shotgun" => Self::Shotgun,
            "rifle" => Self::Rifle,
            "nailgun" => Self::Nailgun,
            "plasma" => Self::Plasma,
            "rocket" => Self::Rocket,
            _ => return None,
        })
    }

    pub fn id(&self) -> &'static str {
        match self {
            Self::Sword => "sword",
            Self::Chainsaw => "chainsaw",
            Self::Pistol => "pistol",
            Self::Shotgun => "shotgun",
            Self::Rifle => "rifle",
            Self::Nailgun => "nailgun",
            Self::Plasma => "plasma",
            Self::Rocket => "rocket",
        }
    }
}

/// Тип выстрела.
#[derive(Clone, Copy, PartialEq)]
pub enum FireKind {
    Melee,                                  // ближний удар (дуга перед собой)
    Hitscan { pellets: u32, spread: f32 },  // мгновенные лучи
    Projectile { speed: f32, splash: f32 }, // летящий снаряд (сплэш > 0 — взрыв)
}

#[derive(Clone, Copy, Debug)]
pub struct WeaponFeedback {
    pub muzzle_color: [f32; 3],
    pub muzzle_energy: f32,
    pub muzzle_range: f32,
    pub muzzle_duration: f32,
    pub impact_color: [f32; 3],
    pub impact_scale: f32,
    pub impact_energy: f32,
    pub impact_duration: f32,
    pub tracer_color: [f32; 3],
    pub tracer_scale: f32,
    pub tracer_duration: f32,
}

#[derive(Clone, Debug)]
pub struct WeaponAudio {
    pub fire_sfx: Vec<String>,
    pub impact_sfx: Vec<String>,
    pub reload_sfx: Vec<String>,
    pub fire_pitch: [f32; 2],
    pub impact_pitch: [f32; 2],
    pub reload_pitch: [f32; 2],
    pub fire_volume_db: f32,
    pub impact_volume_db: f32,
    pub reload_volume_db: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct WeaponAccuracy {
    pub bloom_per_shot: f32,
    pub max_bloom: f32,
    pub recovery: f32,
    pub move_penalty: f32,
    pub crosshair_scale: f32,
}

pub struct AltFireDef {
    pub name_ru: String,
    pub name_en: String,
    pub kind: FireKind,
    pub damage_mult: f32,
    pub cooldown_mult: f32,
    pub range_mult: f32,
    pub recoil_mult: f32,
    pub bloom_mult: f32,
    pub animation_frames: Vec<usize>,
    pub animation_fps: f32,
    pub hit_ratio: f32,
}

#[derive(Clone, Deserialize)]
pub struct WeaponModDef {
    pub id: String,
    pub weapon: String,
    pub branch: u8,
    pub name_ru: String,
    pub name_en: String,
    pub desc_ru: String,
    pub desc_en: String,
    #[serde(default = "one_f32")]
    pub damage_mult: f32,
    #[serde(default = "one_f32")]
    pub cooldown_mult: f32,
    #[serde(default = "one_f32")]
    pub range_mult: f32,
    #[serde(default = "one_f32")]
    pub recoil_mult: f32,
    #[serde(default = "one_f32")]
    pub bloom_mult: f32,
    #[serde(default)]
    pub feedback: WeaponModFeedback,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(default)]
pub struct WeaponModFeedback {
    pub tint: [f32; 3],
    pub color_mix: f32,
    pub muzzle_mult: f32,
    pub tracer_mult: f32,
    pub impact_mult: f32,
    pub pitch_mult: f32,
}

impl Default for WeaponModFeedback {
    fn default() -> Self {
        Self {
            tint: [1.0, 1.0, 1.0],
            color_mix: 0.0,
            muzzle_mult: 1.0,
            tracer_mult: 1.0,
            impact_mult: 1.0,
            pitch_mult: 1.0,
        }
    }
}

impl WeaponModDef {
    pub fn name(&self, lang: &str) -> &str {
        if lang == "en" {
            &self.name_en
        } else {
            &self.name_ru
        }
    }

    pub fn description(&self, lang: &str) -> &str {
        if lang == "en" {
            &self.desc_en
        } else {
            &self.desc_ru
        }
    }
}

/// Рантайм-описание оружия (владеет строками — грузится из JSON).
pub struct WeaponDef {
    pub id: WeaponId,
    pub name_ru: String,
    pub name_en: String,
    pub damage: f32,
    pub dmg_type: DmgType,
    pub cooldown: f32,
    pub range: f32,
    pub kind: FireKind,
    pub ammo: Option<(AmmoType, u32)>, // тип и расход за выстрел
    pub auto: bool,
    // FP-спрайт
    pub sheet: String,
    pub frame_h: f32,
    pub idle_frames: Vec<usize>,
    pub fire_frames: Vec<usize>,
    pub fire_fps: f32,
    pub fire_hit_ratio: f32,
    pub reload_frames: Vec<usize>,
    pub reload_fps: f32,
    pub switch_frames: Vec<usize>,
    pub switch_fps: f32,
    pub magazine: u32,
    pub reload_time: f32,
    pub idle_fps: f32,
    pub view_scale: f32,
    pub view_offset: [f32; 2],
    pub bob_amount: f32,
    pub bob_speed: f32,
    pub recoil: f32,
    pub switch_time: f32,
    pub feedback: WeaponFeedback,
    pub audio: WeaponAudio,
    pub accuracy: WeaponAccuracy,
    pub alt_fire: Option<AltFireDef>,
    /// Статус, накладываемый на врага при попадании: (id, шанс).
    pub status: Option<(String, f32)>,
    /// Шанс критического попадания [0..1]. 0 — криты выключены.
    pub crit_chance: f32,
    /// Множитель урона при крите (>= 1.0). По умолчанию ×1.5.
    pub crit_mult: f32,
}

impl WeaponDef {
    pub fn name(&self, lang: &str) -> &str {
        if lang == "en" && !self.name_en.is_empty() {
            &self.name_en
        } else {
            &self.name_ru
        }
    }

    pub fn attack_frames(&self, secondary: bool) -> &[usize] {
        self.alt_fire
            .as_ref()
            .filter(|_| secondary)
            .map(|alt| alt.animation_frames.as_slice())
            .filter(|frames| !frames.is_empty())
            .unwrap_or(&self.fire_frames)
    }

    pub fn attack_fps(&self, secondary: bool) -> f32 {
        self.alt_fire
            .as_ref()
            .filter(|_| secondary)
            .map(|alt| alt.animation_fps)
            .filter(|fps| *fps > 0.0)
            .unwrap_or(self.fire_fps)
    }

    pub fn attack_hit_ratio(&self, secondary: bool) -> f32 {
        self.alt_fire
            .as_ref()
            .filter(|_| secondary)
            .map(|alt| alt.hit_ratio)
            .unwrap_or(self.fire_hit_ratio)
    }
}

pub const FRAME_W: f32 = 84.0;

// ── Загрузка из JSON ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct FireRaw {
    kind: String,
    #[serde(default)]
    pellets: f64,
    #[serde(default)]
    spread: f32,
    #[serde(default)]
    speed: f32,
    #[serde(default)]
    splash: f32,
}

#[derive(Deserialize)]
struct AmmoRaw {
    #[serde(rename = "type")]
    ty: String,
    per_shot: f64,
}

#[derive(Deserialize)]
struct WeaponRaw {
    id: String,
    name_ru: String,
    #[serde(default)]
    name_en: String,
    damage: f32,
    #[serde(default)]
    dmg_type: Option<String>,
    cooldown: f32,
    range: f32,
    fire: FireRaw,
    #[serde(default)]
    ammo: Option<AmmoRaw>,
    auto: bool,
    sheet: String,
    frame_h: f32,
    idle_frames: Vec<f64>,
    fire_frames: Vec<f64>,
    fire_fps: f32,
    #[serde(default = "default_hit_ratio")]
    fire_hit_ratio: f32,
    #[serde(default)]
    reload_frames: Vec<f64>,
    #[serde(default = "default_reload_fps")]
    reload_fps: f32,
    #[serde(default)]
    switch_frames: Vec<f64>,
    #[serde(default = "default_switch_fps")]
    switch_fps: f32,
    #[serde(default)]
    magazine: u32,
    #[serde(default = "default_reload_time")]
    reload_time: f32,
    #[serde(default = "default_idle_fps")]
    idle_fps: f32,
    #[serde(default = "default_view_scale")]
    view_scale: f32,
    #[serde(default)]
    view_offset: [f32; 2],
    #[serde(default = "default_bob_amount")]
    bob_amount: f32,
    #[serde(default = "default_bob_speed")]
    bob_speed: f32,
    #[serde(default = "default_recoil")]
    recoil: f32,
    #[serde(default = "default_switch_time")]
    switch_time: f32,
    #[serde(default)]
    feedback: WeaponFeedbackRaw,
    #[serde(default)]
    audio: WeaponAudioRaw,
    #[serde(default)]
    accuracy: WeaponAccuracyRaw,
    #[serde(default)]
    alt_fire: Option<AltFireRaw>,
    #[serde(default)]
    status: Option<StatusRaw>,
    #[serde(default)]
    crit_chance: f32,
    #[serde(default = "default_crit_mult")]
    crit_mult: f32,
}

#[derive(Deserialize)]
#[serde(default)]
struct WeaponFeedbackRaw {
    muzzle_color: [f32; 3],
    muzzle_energy: f32,
    muzzle_range: f32,
    muzzle_duration: f32,
    impact_color: [f32; 3],
    impact_scale: f32,
    impact_energy: f32,
    impact_duration: f32,
    tracer_color: [f32; 3],
    tracer_scale: f32,
    tracer_duration: f32,
}

impl Default for WeaponFeedbackRaw {
    fn default() -> Self {
        Self {
            muzzle_color: [1.0, 0.6, 0.8],
            muzzle_energy: 1.6,
            muzzle_range: 7.0,
            muzzle_duration: 0.09,
            impact_color: [1.0, 0.45, 0.58],
            impact_scale: 1.0,
            impact_energy: 0.8,
            impact_duration: 0.18,
            tracer_color: [1.0, 0.82, 0.55],
            tracer_scale: 1.0,
            tracer_duration: 0.055,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct WeaponAudioRaw {
    fire_sfx: Vec<String>,
    impact_sfx: Vec<String>,
    reload_sfx: Vec<String>,
    fire_pitch: [f32; 2],
    impact_pitch: [f32; 2],
    reload_pitch: [f32; 2],
    fire_volume_db: f32,
    impact_volume_db: f32,
    reload_volume_db: f32,
}

impl Default for WeaponAudioRaw {
    fn default() -> Self {
        Self {
            fire_sfx: vec!["res://assets/sounds/Plasma Gun Shot.wav".into()],
            impact_sfx: vec!["res://assets/sounds/Plasma Sword Strike.wav".into()],
            reload_sfx: Vec::new(),
            fire_pitch: [0.96, 1.04],
            impact_pitch: [0.96, 1.04],
            reload_pitch: [0.96, 1.04],
            fire_volume_db: -3.0,
            impact_volume_db: -7.0,
            reload_volume_db: -8.0,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct WeaponAccuracyRaw {
    bloom_per_shot: f32,
    max_bloom: f32,
    recovery: f32,
    move_penalty: f32,
    crosshair_scale: f32,
}

impl Default for WeaponAccuracyRaw {
    fn default() -> Self {
        Self {
            bloom_per_shot: 0.18,
            max_bloom: 1.0,
            recovery: 1.8,
            move_penalty: 0.25,
            crosshair_scale: 1.0,
        }
    }
}

#[derive(Deserialize)]
struct AltFireRaw {
    name_ru: String,
    #[serde(default)]
    name_en: String,
    fire: FireRaw,
    damage_mult: f32,
    cooldown_mult: f32,
    range_mult: f32,
    recoil_mult: f32,
    bloom_mult: f32,
    #[serde(default)]
    animation_frames: Vec<f64>,
    #[serde(default)]
    animation_fps: f32,
    #[serde(default = "default_hit_ratio")]
    hit_ratio: f32,
}

fn parse_fire(weapon_id: &str, fire: &FireRaw) -> Result<FireKind, String> {
    let pellets = (fire.pellets as u32).max(1);
    match fire.kind.as_str() {
        "melee" => Ok(FireKind::Melee),
        "hitscan" => Ok(FireKind::Hitscan {
            pellets,
            spread: fire.spread,
        }),
        "projectile" => Ok(FireKind::Projectile {
            speed: fire.speed,
            splash: fire.splash,
        }),
        other => Err(format!(
            "weapon '{}': unknown fire kind '{}'",
            weapon_id, other
        )),
    }
}

#[derive(Deserialize)]
struct StatusRaw {
    id: String,
    #[serde(default = "one_f32")]
    chance: f32,
}

fn one_f32() -> f32 {
    1.0
}
fn default_hit_ratio() -> f32 {
    0.5
}
fn default_idle_fps() -> f32 {
    6.25
}
fn default_view_scale() -> f32 {
    6.0
}
fn default_bob_amount() -> f32 {
    10.0
}
fn default_bob_speed() -> f32 {
    9.0
}
fn default_recoil() -> f32 {
    18.0
}
fn default_switch_time() -> f32 {
    0.22
}
fn default_switch_fps() -> f32 {
    14.0
}
fn default_reload_fps() -> f32 {
    10.0
}
fn default_reload_time() -> f32 {
    1.2
}
fn default_crit_mult() -> f32 {
    1.5
}

impl WeaponRaw {
    fn into_def(self, id: WeaponId) -> Result<WeaponDef, String> {
        let kind = parse_fire(&self.id, &self.fire)?;
        let ammo = match self.ammo {
            None => None,
            Some(a) => {
                let t = AmmoType::from_id(&a.ty)
                    .ok_or_else(|| format!("weapon '{}': unknown ammo '{}'", self.id, a.ty))?;
                Some((t, a.per_shot as u32))
            }
        };
        let dmg_type = match self.dmg_type.as_deref() {
            None | Some("") => DmgType::Physical,
            Some(s) => DmgType::from_id(s)
                .ok_or_else(|| format!("weapon '{}': unknown dmg_type '{}'", self.id, s))?,
        };
        for (sequence, frames) in [
            ("idle", self.idle_frames.as_slice()),
            ("fire", self.fire_frames.as_slice()),
            ("reload", self.reload_frames.as_slice()),
            ("switch", self.switch_frames.as_slice()),
        ] {
            if let Some(frame) = frames.iter().copied().find(|frame| {
                !frame.is_finite() || frame.fract() != 0.0 || !(0.0..8.0).contains(frame)
            }) {
                return Err(format!(
                    "weapon '{}': {sequence} frame '{frame}' must be an integer from 0 to 7",
                    self.id
                ));
            }
        }
        if self.idle_fps <= 0.0 || self.fire_fps <= 0.0 || self.switch_fps <= 0.0 {
            return Err(format!(
                "weapon '{}': animation FPS must be positive",
                self.id
            ));
        }
        if !self.fire_hit_ratio.is_finite() || !(0.05..=0.95).contains(&self.fire_hit_ratio) {
            return Err(format!("weapon '{}': invalid fire_hit_ratio", self.id));
        }
        if !self.reload_frames.is_empty() && self.reload_fps <= 0.0 {
            return Err(format!("weapon '{}': reload_fps must be positive", self.id));
        }
        let feedback_values = [
            self.feedback.muzzle_energy,
            self.feedback.muzzle_range,
            self.feedback.muzzle_duration,
            self.feedback.impact_scale,
            self.feedback.impact_energy,
            self.feedback.impact_duration,
            self.feedback.tracer_scale,
            self.feedback.tracer_duration,
        ];
        if feedback_values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
            || self
                .feedback
                .muzzle_color
                .iter()
                .chain(self.feedback.impact_color.iter())
                .chain(self.feedback.tracer_color.iter())
                .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        {
            return Err(format!(
                "weapon '{}': feedback colors must be 0..1 and feedback values non-negative",
                self.id
            ));
        }
        let audio = &self.audio;
        let valid_pitch = |pitch: [f32; 2]| {
            pitch[0].is_finite() && pitch[1].is_finite() && pitch[0] > 0.0 && pitch[1] >= pitch[0]
        };
        if audio.fire_sfx.is_empty()
            || audio.impact_sfx.is_empty()
            || !valid_pitch(audio.fire_pitch)
            || !valid_pitch(audio.impact_pitch)
            || !valid_pitch(audio.reload_pitch)
            || [
                audio.fire_volume_db,
                audio.impact_volume_db,
                audio.reload_volume_db,
            ]
            .iter()
            .any(|value| !value.is_finite() || !(-40.0..=6.0).contains(value))
            || audio
                .fire_sfx
                .iter()
                .chain(audio.impact_sfx.iter())
                .chain(audio.reload_sfx.iter())
                .any(|path| !path.starts_with("res://") || !path.ends_with(".wav"))
        {
            return Err(format!("weapon '{}': invalid audio profile", self.id));
        }
        if self.magazine > 0 && audio.reload_sfx.is_empty() {
            return Err(format!(
                "weapon '{}': magazine weapon requires reload_sfx",
                self.id
            ));
        }
        let accuracy_values = [
            self.accuracy.bloom_per_shot,
            self.accuracy.max_bloom,
            self.accuracy.recovery,
            self.accuracy.move_penalty,
            self.accuracy.crosshair_scale,
        ];
        if accuracy_values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
            || self.accuracy.max_bloom <= 0.0
            || self.accuracy.recovery <= 0.0
            || self.accuracy.crosshair_scale <= 0.0
        {
            return Err(format!("weapon '{}': invalid accuracy profile", self.id));
        }
        if !self.crit_chance.is_finite() || !(0.0..=1.0).contains(&self.crit_chance) {
            return Err(format!(
                "weapon '{}': crit_chance must be between 0 and 1",
                self.id
            ));
        }
        if !self.crit_mult.is_finite() || self.crit_mult < 1.0 {
            return Err(format!("weapon '{}': crit_mult must be >= 1.0", self.id));
        }
        let alt_fire = self
            .alt_fire
            .map(|alt| {
                let multipliers = [
                    alt.damage_mult,
                    alt.cooldown_mult,
                    alt.range_mult,
                    alt.recoil_mult,
                    alt.bloom_mult,
                ];
                if alt.name_ru.is_empty()
                    || alt.name_en.is_empty()
                    || alt.animation_frames.iter().any(|frame| {
                        !frame.is_finite() || frame.fract() != 0.0 || !(0.0..8.0).contains(frame)
                    })
                    || (!alt.animation_frames.is_empty() && alt.animation_fps <= 0.0)
                    || !alt.hit_ratio.is_finite()
                    || !(0.05..=0.95).contains(&alt.hit_ratio)
                    || multipliers
                        .iter()
                        .any(|value| !value.is_finite() || *value <= 0.0)
                {
                    return Err(format!("weapon '{}': invalid alt_fire profile", self.id));
                }
                Ok(AltFireDef {
                    name_ru: alt.name_ru,
                    name_en: alt.name_en,
                    kind: parse_fire(&self.id, &alt.fire)?,
                    damage_mult: alt.damage_mult,
                    cooldown_mult: alt.cooldown_mult,
                    range_mult: alt.range_mult,
                    recoil_mult: alt.recoil_mult,
                    bloom_mult: alt.bloom_mult,
                    animation_frames: alt
                        .animation_frames
                        .into_iter()
                        .map(|frame| frame as usize)
                        .collect(),
                    animation_fps: alt.animation_fps,
                    hit_ratio: alt.hit_ratio,
                })
            })
            .transpose()?;
        let idle_frames: Vec<usize> = self
            .idle_frames
            .into_iter()
            .map(|frame| frame as usize)
            .collect();
        let fire_frames: Vec<usize> = self
            .fire_frames
            .into_iter()
            .map(|frame| frame as usize)
            .collect();
        let reload_frames: Vec<usize> = self
            .reload_frames
            .into_iter()
            .map(|frame| frame as usize)
            .collect();
        if idle_frames.is_empty() {
            return Err(format!("weapon '{}': idle_frames is empty", self.id));
        }
        if fire_frames.is_empty() {
            return Err(format!("weapon '{}': fire_frames is empty", self.id));
        }
        let switch_frames: Vec<usize> = if self.switch_frames.is_empty() {
            vec![idle_frames[0]]
        } else {
            self.switch_frames
                .into_iter()
                .map(|frame| frame as usize)
                .collect()
        };
        Ok(WeaponDef {
            id,
            name_ru: self.name_ru,
            name_en: self.name_en,
            damage: self.damage,
            dmg_type,
            cooldown: self.cooldown,
            range: self.range,
            kind,
            ammo,
            auto: self.auto,
            sheet: self.sheet,
            frame_h: self.frame_h,
            idle_frames,
            fire_frames,
            fire_fps: self.fire_fps,
            fire_hit_ratio: self.fire_hit_ratio,
            reload_frames,
            reload_fps: self.reload_fps.max(0.1),
            switch_frames,
            switch_fps: self.switch_fps.max(0.1),
            magazine: self.magazine,
            reload_time: self.reload_time.max(0.1),
            idle_fps: self.idle_fps.max(0.1),
            view_scale: self.view_scale.max(0.5),
            view_offset: self.view_offset,
            bob_amount: self.bob_amount.max(0.0),
            bob_speed: self.bob_speed.max(0.1),
            recoil: self.recoil.max(0.0),
            switch_time: self.switch_time.max(0.05),
            feedback: WeaponFeedback {
                muzzle_color: self.feedback.muzzle_color,
                muzzle_energy: self.feedback.muzzle_energy,
                muzzle_range: self.feedback.muzzle_range,
                muzzle_duration: self.feedback.muzzle_duration,
                impact_color: self.feedback.impact_color,
                impact_scale: self.feedback.impact_scale,
                impact_energy: self.feedback.impact_energy,
                impact_duration: self.feedback.impact_duration,
                tracer_color: self.feedback.tracer_color,
                tracer_scale: self.feedback.tracer_scale,
                tracer_duration: self.feedback.tracer_duration,
            },
            audio: WeaponAudio {
                fire_sfx: self.audio.fire_sfx,
                impact_sfx: self.audio.impact_sfx,
                reload_sfx: self.audio.reload_sfx,
                fire_pitch: self.audio.fire_pitch,
                impact_pitch: self.audio.impact_pitch,
                reload_pitch: self.audio.reload_pitch,
                fire_volume_db: self.audio.fire_volume_db,
                impact_volume_db: self.audio.impact_volume_db,
                reload_volume_db: self.audio.reload_volume_db,
            },
            accuracy: WeaponAccuracy {
                bloom_per_shot: self.accuracy.bloom_per_shot,
                max_bloom: self.accuracy.max_bloom,
                recovery: self.accuracy.recovery,
                move_penalty: self.accuracy.move_penalty,
                crosshair_scale: self.accuracy.crosshair_scale,
            },
            alt_fire,
            status: self.status.map(|s| (s.id, s.chance)),
            crit_chance: self.crit_chance,
            crit_mult: self.crit_mult,
        })
    }
}

pub(crate) fn parse(json: &str) -> Result<Vec<WeaponDef>, String> {
    let raws: Vec<WeaponRaw> = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut slots: Vec<Option<WeaponDef>> = (0..8).map(|_| None).collect();
    for r in raws {
        let id = WeaponId::from_id(&r.id).ok_or_else(|| format!("unknown weapon id '{}'", r.id))?;
        slots[id.slot()] = Some(r.into_def(id)?);
    }
    let mut out = Vec::with_capacity(8);
    for (i, s) in slots.into_iter().enumerate() {
        out.push(s.ok_or_else(|| format!("missing weapon for slot {i}"))?);
    }
    Ok(out)
}

/// Встроенная копия конфига — гарантированный фолбэк.
const EMBEDDED: &str = include_str!("../../../game/presets/core/weapons.json");
const EMBEDDED_MODS: &str = include_str!("../../../game/presets/core/weapon_mods.json");

static WEAPONS: RwLock<Option<&'static [WeaponDef]>> = RwLock::new(None);
static WEAPON_MODS: RwLock<Option<&'static [WeaponModDef]>> = RwLock::new(None);

fn embedded() -> Vec<WeaponDef> {
    parse(EMBEDDED).expect("встроенный weapons.json должен быть валиден")
}

/// Загрузить (или перезагрузить при смене пресета) таблицу оружия.
/// `runtime_json` — содержимое `weapons.json` пресета (или None → встроенная копия).
pub(crate) fn parse_mods(json: &str) -> Result<Vec<WeaponModDef>, String> {
    let mods: Vec<WeaponModDef> = serde_json::from_str(json).map_err(|error| error.to_string())?;
    for weapon in WeaponId::ALL {
        for branch in 1..=2 {
            let matching: Vec<_> = mods
                .iter()
                .filter(|entry| entry.weapon == weapon.id() && entry.branch == branch)
                .collect();
            if matching.len() != 1 {
                return Err(format!(
                    "weapon_mods: '{}' branch {} must have exactly one entry",
                    weapon.id(),
                    branch
                ));
            }
        }
    }
    for entry in &mods {
        if WeaponId::from_id(&entry.weapon).is_none()
            || !(1..=2).contains(&entry.branch)
            || entry.name_ru.is_empty()
            || entry.name_en.is_empty()
            || entry.desc_ru.is_empty()
            || entry.desc_en.is_empty()
            || [
                entry.damage_mult,
                entry.cooldown_mult,
                entry.range_mult,
                entry.recoil_mult,
                entry.bloom_mult,
            ]
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
            || entry
                .feedback
                .tint
                .iter()
                .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
            || !entry.feedback.color_mix.is_finite()
            || !(0.0..=1.0).contains(&entry.feedback.color_mix)
            || [
                entry.feedback.muzzle_mult,
                entry.feedback.tracer_mult,
                entry.feedback.impact_mult,
                entry.feedback.pitch_mult,
            ]
            .iter()
            .any(|value| !value.is_finite() || !(0.5..=1.75).contains(value))
        {
            return Err(format!("weapon_mods: invalid entry '{}'", entry.id));
        }
    }
    Ok(mods)
}

pub fn load(runtime_json: Option<&str>, runtime_mods: Option<&str>) {
    let defs = match runtime_json {
        Some(j) => match parse(j) {
            Ok(d) => d,
            Err(e) => {
                crate::warn!("weapons.json: {e}; using embedded");
                embedded()
            }
        },
        None => embedded(),
    };
    *WEAPONS.write().unwrap() = Some(Box::leak(defs.into_boxed_slice()));
    let mods = runtime_mods
        .and_then(|json| parse_mods(json).ok())
        .unwrap_or_else(|| parse_mods(EMBEDDED_MODS).expect("embedded weapon_mods.json"));
    *WEAPON_MODS.write().unwrap() = Some(Box::leak(mods.into_boxed_slice()));
}

fn store() -> &'static [WeaponDef] {
    if let Some(w) = *WEAPONS.read().unwrap() {
        return w;
    }
    load(None, None);
    WEAPONS.read().unwrap().expect("weapons after load(None)")
}

pub fn weapons() -> &'static [WeaponDef] {
    store()
}

pub fn weapon_def(id: WeaponId) -> &'static WeaponDef {
    &store()[id.slot()]
}

pub fn weapon_mods() -> &'static [WeaponModDef] {
    if WEAPON_MODS.read().unwrap().is_none() {
        load(None, None);
    }
    WEAPON_MODS.read().unwrap().expect("weapon mods after load")
}

pub fn weapon_mod_for(id: WeaponId, branch: u8) -> Option<&'static WeaponModDef> {
    weapon_mods()
        .iter()
        .find(|entry| entry.weapon == id.id() && entry.branch == branch)
}

// ── Состояние арсенала игрока ─────────────────────────────────────────────────

pub struct Arsenal {
    pub owned: [bool; 8],
    pub ammo: [u32; 4],
    pub clips: [u32; 8],
    pub current: WeaponId,
}

impl Default for Arsenal {
    fn default() -> Self {
        Self::new()
    }
}

impl Arsenal {
    pub fn new() -> Self {
        Self {
            owned: [false; 8],
            ammo: [0; 4],
            clips: [0; 8],
            current: WeaponId::Pistol,
        }
    }

    pub fn give_weapon(&mut self, id: WeaponId) -> bool {
        let had = self.owned[id.slot()];
        self.owned[id.slot()] = true;
        if !had {
            self.clips[id.slot()] = weapon_def(id).magazine;
        }
        !had
    }

    pub fn has(&self, id: WeaponId) -> bool {
        self.owned[id.slot()]
    }

    pub fn add_ammo(&mut self, t: AmmoType, n: u32, max_mult: f32) -> u32 {
        let cap = (t.max() as f32 * max_mult) as u32;
        let cur = self.ammo[t.idx()];
        let add = n.min(cap.saturating_sub(cur));
        self.ammo[t.idx()] = cur + add;
        add
    }

    pub fn ammo_of(&self, t: AmmoType) -> u32 {
        self.ammo[t.idx()]
    }

    /// Достаточно ли боеприпасов для выстрела из оружия.
    pub fn can_fire(&self, id: WeaponId) -> bool {
        let def = weapon_def(id);
        if def.magazine > 0 {
            return self.clips[id.slot()] > 0;
        }
        match def.ammo {
            None => true,
            Some((t, cost)) => self.ammo[t.idx()] >= cost,
        }
    }

    pub fn consume(&mut self, id: WeaponId) {
        let def = weapon_def(id);
        if def.magazine > 0 {
            self.clips[id.slot()] = self.clips[id.slot()].saturating_sub(1);
        } else if let Some((t, cost)) = def.ammo {
            let a = &mut self.ammo[t.idx()];
            *a = a.saturating_sub(cost);
        }
    }

    pub fn reload(&mut self, id: WeaponId) -> u32 {
        let def = weapon_def(id);
        let Some((ammo_type, _)) = def.ammo else {
            return 0;
        };
        if def.magazine == 0 {
            return 0;
        }
        let missing = def.magazine.saturating_sub(self.clips[id.slot()]);
        let moved = missing.min(self.ammo[ammo_type.idx()]);
        self.ammo[ammo_type.idx()] -= moved;
        self.clips[id.slot()] += moved;
        moved
    }

    /// Следующее/предыдущее доступное оружие (колесо мыши).
    pub fn cycle(&self, dir: i32) -> WeaponId {
        let mut s = self.current.slot() as i32;
        for _ in 0..8 {
            s = (s + dir).rem_euclid(8);
            if self.owned[s as usize] {
                return WeaponId::from_slot(s as usize);
            }
        }
        self.current
    }
}

#[cfg(test)]
mod arsenal_tests {
    use super::{AmmoType, Arsenal, WeaponId};

    #[test]
    fn magazine_consumes_and_reloads_from_reserve() {
        super::load(None, None);
        let mut arsenal = Arsenal::new();
        arsenal.give_weapon(WeaponId::Pistol);
        arsenal.ammo[AmmoType::Bullets.idx()] = 20;
        for _ in 0..12 {
            assert!(arsenal.can_fire(WeaponId::Pistol));
            arsenal.consume(WeaponId::Pistol);
        }
        assert!(!arsenal.can_fire(WeaponId::Pistol));
        assert_eq!(arsenal.reload(WeaponId::Pistol), 12);
        assert_eq!(arsenal.clips[WeaponId::Pistol.slot()], 12);
        assert_eq!(arsenal.ammo[AmmoType::Bullets.idx()], 8);
    }
}
