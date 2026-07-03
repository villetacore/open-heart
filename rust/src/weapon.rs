//! Арсенал: типы боеприпасов, описания оружия, состояние оружия игрока.
//!
//! Оружие — data-driven: параметры грузятся из `data/weapons.json` через [`init`].
//! Если файл сломан/отсутствует — используется встроенная копия (`include_str!`),
//! так что игра никогда не падает из-за опечатки в конфиге.

use serde::Deserialize;
use std::sync::OnceLock;

// ── Боеприпасы ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum AmmoType { Bullets, Shells, Rockets, Cells }

impl AmmoType {
    pub const ALL: [AmmoType; 4] = [AmmoType::Bullets, AmmoType::Shells, AmmoType::Rockets, AmmoType::Cells];

    pub fn idx(&self) -> usize {
        match self { Self::Bullets => 0, Self::Shells => 1, Self::Rockets => 2, Self::Cells => 3 }
    }
    pub fn from_idx(i: usize) -> Self { Self::ALL[i % 4] }

    /// Разбор строкового id из конфигов.
    pub fn from_id(s: &str) -> Option<Self> {
        Some(match s {
            "bullets" => Self::Bullets,
            "shells"  => Self::Shells,
            "rockets" => Self::Rockets,
            "cells"   => Self::Cells,
            _         => return None,
        })
    }

    pub fn name_ru(&self) -> &'static str {
        match self {
            Self::Bullets => "Патроны",
            Self::Shells  => "Дробь",
            Self::Rockets => "Ракеты",
            Self::Cells   => "Энергия",
        }
    }
    pub fn max(&self) -> u32 {
        match self { Self::Bullets => 240, Self::Shells => 60, Self::Rockets => 24, Self::Cells => 180 }
    }
    /// Спрайт пикапа в мире.
    pub fn pickup_tex(&self) -> &'static str {
        match self {
            Self::Bullets => "res://assets/sprites/pickups/ammo_bullets.png",
            Self::Shells  => "res://assets/sprites/pickups/ammo_shells.png",
            Self::Rockets => "res://assets/sprites/pickups/ammo_rockets.png",
            Self::Cells   => "res://assets/sprites/pickups/ammo_cells.png",
        }
    }
    /// Стандартная пачка при подборе.
    pub fn pack_size(&self) -> u32 {
        match self { Self::Bullets => 30, Self::Shells => 8, Self::Rockets => 4, Self::Cells => 30 }
    }
}

// ── Типы урона ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum DmgType { Physical, Fire, Energy, Void }

impl DmgType {
    pub const ALL: [DmgType; 4] = [DmgType::Physical, DmgType::Fire, DmgType::Energy, DmgType::Void];

    pub fn idx(&self) -> usize {
        match self { Self::Physical => 0, Self::Fire => 1, Self::Energy => 2, Self::Void => 3 }
    }
    pub fn from_id(s: &str) -> Option<Self> {
        Some(match s {
            "physical" => Self::Physical,
            "fire"     => Self::Fire,
            "energy"   => Self::Energy,
            "void"     => Self::Void,
            _          => return None,
        })
    }
    pub fn name_ru(&self) -> &'static str {
        match self {
            Self::Physical => "Физический",
            Self::Fire     => "Огонь",
            Self::Energy   => "Энергия",
            Self::Void     => "Пустота",
        }
    }
}

// ── Оружие ────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum WeaponId { Sword, Chainsaw, Pistol, Shotgun, Rifle, Nailgun, Plasma, Rocket }

impl WeaponId {
    pub const ALL: [WeaponId; 8] = [
        WeaponId::Sword, WeaponId::Chainsaw, WeaponId::Pistol, WeaponId::Shotgun,
        WeaponId::Rifle, WeaponId::Nailgun, WeaponId::Plasma, WeaponId::Rocket,
    ];
    pub fn slot(&self) -> usize {
        match self {
            Self::Sword => 0, Self::Chainsaw => 1, Self::Pistol => 2, Self::Shotgun => 3,
            Self::Rifle => 4, Self::Nailgun => 5, Self::Plasma => 6, Self::Rocket => 7,
        }
    }
    pub fn from_slot(i: usize) -> Self { Self::ALL[i % 8] }

    /// Разбор строкового id из конфигов.
    pub fn from_id(s: &str) -> Option<Self> {
        Some(match s {
            "sword"    => Self::Sword,
            "chainsaw" => Self::Chainsaw,
            "pistol"   => Self::Pistol,
            "shotgun"  => Self::Shotgun,
            "rifle"    => Self::Rifle,
            "nailgun"  => Self::Nailgun,
            "plasma"   => Self::Plasma,
            "rocket"   => Self::Rocket,
            _          => return None,
        })
    }
}

/// Тип выстрела.
#[derive(Clone, Copy, PartialEq)]
pub enum FireKind {
    Melee,                                  // ближний удар (дуга перед собой)
    Hitscan { pellets: u32, spread: f32 },  // мгновенные лучи
    Projectile { speed: f32, splash: f32 }, // летящий снаряд (сплэш > 0 — взрыв)
}

/// Рантайм-описание оружия (владеет строками — грузится из JSON).
pub struct WeaponDef {
    pub id:          WeaponId,
    pub name_ru:     String,
    pub damage:      f32,
    pub dmg_type:    DmgType,
    pub cooldown:    f32,
    pub range:       f32,
    pub kind:        FireKind,
    pub ammo:        Option<(AmmoType, u32)>, // тип и расход за выстрел
    pub auto:        bool,
    // FP-спрайт
    pub sheet:       String,
    pub frame_h:     f32,
    pub idle_frames: Vec<usize>,
    pub fire_frames: Vec<usize>,
    pub fire_fps:    f32,
}

pub const FRAME_W: f32 = 84.0;

// ── Загрузка из JSON ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct FireRaw {
    kind: String,
    #[serde(default)] pellets: u32,
    #[serde(default)] spread:  f32,
    #[serde(default)] speed:   f32,
    #[serde(default)] splash:  f32,
}

#[derive(Deserialize)]
struct AmmoRaw {
    #[serde(rename = "type")] ty: String,
    per_shot: u32,
}

#[derive(Deserialize)]
struct WeaponRaw {
    id:          String,
    name_ru:     String,
    damage:      f32,
    #[serde(default)] dmg_type: Option<String>,
    cooldown:    f32,
    range:       f32,
    fire:        FireRaw,
    #[serde(default)] ammo: Option<AmmoRaw>,
    auto:        bool,
    sheet:       String,
    frame_h:     f32,
    idle_frames: Vec<usize>,
    fire_frames: Vec<usize>,
    fire_fps:    f32,
}

impl WeaponRaw {
    fn into_def(self, id: WeaponId) -> Result<WeaponDef, String> {
        let kind = match self.fire.kind.as_str() {
            "melee"      => FireKind::Melee,
            "hitscan"    => FireKind::Hitscan { pellets: self.fire.pellets.max(1), spread: self.fire.spread },
            "projectile" => FireKind::Projectile { speed: self.fire.speed, splash: self.fire.splash },
            other        => return Err(format!("weapon '{}': unknown fire kind '{}'", self.id, other)),
        };
        let ammo = match self.ammo {
            None => None,
            Some(a) => {
                let t = AmmoType::from_id(&a.ty)
                    .ok_or_else(|| format!("weapon '{}': unknown ammo '{}'", self.id, a.ty))?;
                Some((t, a.per_shot))
            }
        };
        let dmg_type = match self.dmg_type.as_deref() {
            None | Some("") => DmgType::Physical,
            Some(s) => DmgType::from_id(s)
                .ok_or_else(|| format!("weapon '{}': unknown dmg_type '{}'", self.id, s))?,
        };
        Ok(WeaponDef {
            id, name_ru: self.name_ru, damage: self.damage, dmg_type, cooldown: self.cooldown,
            range: self.range, kind, ammo, auto: self.auto, sheet: self.sheet,
            frame_h: self.frame_h, idle_frames: self.idle_frames,
            fire_frames: self.fire_frames, fire_fps: self.fire_fps,
        })
    }
}

fn parse(json: &str) -> Result<Vec<WeaponDef>, String> {
    let raws: Vec<WeaponRaw> = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut slots: Vec<Option<WeaponDef>> = (0..8).map(|_| None).collect();
    for r in raws {
        let id = WeaponId::from_id(&r.id)
            .ok_or_else(|| format!("unknown weapon id '{}'", r.id))?;
        slots[id.slot()] = Some(r.into_def(id)?);
    }
    let mut out = Vec::with_capacity(8);
    for (i, s) in slots.into_iter().enumerate() {
        out.push(s.ok_or_else(|| format!("missing weapon for slot {i}"))?);
    }
    Ok(out)
}

/// Встроенная копия конфига — гарантированный фолбэк.
const EMBEDDED: &str = include_str!("../../godot/data/weapons.json");

static WEAPONS: OnceLock<Vec<WeaponDef>> = OnceLock::new();

fn embedded() -> Vec<WeaponDef> {
    parse(EMBEDDED).expect("встроенный weapons.json должен быть валиден")
}

/// Инициализировать таблицу оружия. `runtime_json` — содержимое `data/weapons.json`
/// (или None). При ошибке разбора — фолбэк на встроенную копию с предупреждением.
pub fn init(runtime_json: Option<&str>) {
    if WEAPONS.get().is_some() { return; }
    let defs = match runtime_json {
        Some(j) => match parse(j) {
            Ok(d) => d,
            Err(e) => { godot::global::godot_warn!("weapons.json: {e}; using embedded"); embedded() }
        },
        None => embedded(),
    };
    let _ = WEAPONS.set(defs);
}

fn store() -> &'static [WeaponDef] {
    WEAPONS.get_or_init(embedded).as_slice()
}

pub fn weapons() -> &'static [WeaponDef] { store() }

pub fn weapon_def(id: WeaponId) -> &'static WeaponDef {
    &store()[id.slot()]
}

// ── Состояние арсенала игрока ─────────────────────────────────────────────────

pub struct Arsenal {
    pub owned:   [bool; 8],
    pub ammo:    [u32; 4],
    pub current: WeaponId,
}

impl Arsenal {
    pub fn new() -> Self {
        Self { owned: [false; 8], ammo: [0; 4], current: WeaponId::Pistol }
    }

    pub fn give_weapon(&mut self, id: WeaponId) -> bool {
        let had = self.owned[id.slot()];
        self.owned[id.slot()] = true;
        !had
    }

    pub fn has(&self, id: WeaponId) -> bool { self.owned[id.slot()] }

    pub fn add_ammo(&mut self, t: AmmoType, n: u32, max_mult: f32) -> u32 {
        let cap = (t.max() as f32 * max_mult) as u32;
        let cur = self.ammo[t.idx()];
        let add = n.min(cap.saturating_sub(cur));
        self.ammo[t.idx()] = cur + add;
        add
    }

    pub fn ammo_of(&self, t: AmmoType) -> u32 { self.ammo[t.idx()] }

    /// Достаточно ли боеприпасов для выстрела из оружия.
    pub fn can_fire(&self, id: WeaponId) -> bool {
        match weapon_def(id).ammo {
            None => true,
            Some((t, cost)) => self.ammo[t.idx()] >= cost,
        }
    }

    pub fn consume(&mut self, id: WeaponId) {
        if let Some((t, cost)) = weapon_def(id).ammo {
            let a = &mut self.ammo[t.idx()];
            *a = a.saturating_sub(cost);
        }
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
