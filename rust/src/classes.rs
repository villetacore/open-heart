//! Классы персонажа: 3 класса × 3 специализации. Data-driven (`data/classes.json`).
//!
//! Мили («Берсерк»)         — пила/клинок, толстый, урон в упор.
//! Клоус-рэнж («Штурмовик») — пистолет/дробовик, ближняя перестрелка.
//! Мид-рэнж («Оператор»)    — автомат/плазма, держит дистанцию.

use serde::Deserialize;
use std::sync::OnceLock;
use crate::weapon::{AmmoType, WeaponId};

pub struct SpecDef {
    pub id:            String,
    pub name_ru:       String,
    pub desc_ru:       String,
    pub hp_bonus:      f32,
    pub speed_mult:    f32,
    pub dmg_mult:      f32,
    pub cd_mult:       f32,
    pub lifesteal:     f32,
    pub ammo_mult:     f32,
    pub extra_weapon:  Option<WeaponId>,
}

pub struct ClassDef {
    pub id:            String,
    pub name_ru:       String,
    pub role_ru:       String,
    pub desc_ru:       String,
    pub base_hp:       f32,
    pub speed:         f32,
    pub dmg_mult:      f32,
    pub start_weapons: Vec<WeaponId>,
    pub start_ammo:    Vec<(AmmoType, u32)>,
    pub specs:         Vec<SpecDef>,
}

// ── Загрузка из JSON ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct SpecRaw {
    id: String, name_ru: String, desc_ru: String,
    hp_bonus: f32, speed_mult: f32, dmg_mult: f32,
    cd_mult: f32, lifesteal: f32, ammo_mult: f32,
    #[serde(default)] extra_weapon: Option<String>,
}

#[derive(Deserialize)]
struct AmmoStartRaw { #[serde(rename = "type")] ty: String, amount: u32 }

#[derive(Deserialize)]
struct ClassRaw {
    id: String, name_ru: String, role_ru: String, desc_ru: String,
    base_hp: f32, speed: f32, dmg_mult: f32,
    start_weapons: Vec<String>,
    start_ammo: Vec<AmmoStartRaw>,
    specs: Vec<SpecRaw>,
}

impl SpecRaw {
    fn into_def(self) -> Result<SpecDef, String> {
        let extra = match self.extra_weapon {
            None => None,
            Some(ref s) => Some(WeaponId::from_id(s)
                .ok_or_else(|| format!("spec '{}': unknown weapon '{}'", self.id, s))?),
        };
        Ok(SpecDef {
            id: self.id, name_ru: self.name_ru, desc_ru: self.desc_ru,
            hp_bonus: self.hp_bonus, speed_mult: self.speed_mult, dmg_mult: self.dmg_mult,
            cd_mult: self.cd_mult, lifesteal: self.lifesteal, ammo_mult: self.ammo_mult,
            extra_weapon: extra,
        })
    }
}

impl ClassRaw {
    fn into_def(self) -> Result<ClassDef, String> {
        let mut weapons = Vec::new();
        for w in &self.start_weapons {
            weapons.push(WeaponId::from_id(w)
                .ok_or_else(|| format!("class '{}': unknown weapon '{}'", self.id, w))?);
        }
        let mut ammo = Vec::new();
        for a in &self.start_ammo {
            let t = AmmoType::from_id(&a.ty)
                .ok_or_else(|| format!("class '{}': unknown ammo '{}'", self.id, a.ty))?;
            ammo.push((t, a.amount));
        }
        let mut specs = Vec::new();
        for s in self.specs {
            specs.push(s.into_def()?);
        }
        if specs.len() < 3 {
            return Err(format!("class '{}': needs 3 specs, got {}", self.id, specs.len()));
        }
        Ok(ClassDef {
            id: self.id, name_ru: self.name_ru, role_ru: self.role_ru, desc_ru: self.desc_ru,
            base_hp: self.base_hp, speed: self.speed, dmg_mult: self.dmg_mult,
            start_weapons: weapons, start_ammo: ammo, specs,
        })
    }
}

fn parse(json: &str) -> Result<Vec<ClassDef>, String> {
    let raws: Vec<ClassRaw> = serde_json::from_str(json).map_err(|e| e.to_string())?;
    if raws.len() < 3 {
        return Err(format!("classes.json: needs 3 classes, got {}", raws.len()));
    }
    let mut out = Vec::with_capacity(raws.len());
    for r in raws {
        out.push(r.into_def()?);
    }
    Ok(out)
}

const EMBEDDED: &str = include_str!("../../godot/data/classes.json");

static CLASSES: OnceLock<Vec<ClassDef>> = OnceLock::new();

fn embedded() -> Vec<ClassDef> {
    parse(EMBEDDED).expect("встроенный classes.json должен быть валиден")
}

/// Инициализировать таблицу классов из `data/classes.json` (или встроенной копии).
pub fn init(runtime_json: Option<&str>) {
    if CLASSES.get().is_some() { return; }
    let defs = match runtime_json {
        Some(j) => match parse(j) {
            Ok(d) => d,
            Err(e) => { godot::global::godot_warn!("classes.json: {e}; using embedded"); embedded() }
        },
        None => embedded(),
    };
    let _ = CLASSES.set(defs);
}

pub fn classes() -> &'static [ClassDef] {
    CLASSES.get_or_init(embedded).as_slice()
}

pub fn class_by_id(id: &str) -> Option<&'static ClassDef> {
    classes().iter().find(|c| c.id == id)
}

// ── Расчёт лоадаута ───────────────────────────────────────────────────────────

/// Итоговые боевые параметры: класс + спек + уровень.
pub struct Loadout {
    pub max_hp:     f32,
    pub speed:      f32,
    pub dmg_mult:   f32,
    pub cd_mult:    f32,
    pub lifesteal:  f32,
    pub ammo_mult:  f32,
}

pub fn compute_loadout(class_idx: usize, spec_idx: usize, level: u32) -> Loadout {
    let list = classes();
    let c = &list[class_idx.min(list.len() - 1)];
    let s = &c.specs[spec_idx.min(c.specs.len() - 1)];
    let lvl_bonus = (level.saturating_sub(1)) as f32;
    Loadout {
        max_hp:    (c.base_hp + s.hp_bonus + lvl_bonus * 8.0).max(40.0),
        speed:     c.speed * s.speed_mult,
        dmg_mult:  c.dmg_mult * s.dmg_mult * (1.0 + lvl_bonus * 0.03),
        cd_mult:   s.cd_mult,
        lifesteal: s.lifesteal,
        ammo_mult: s.ammo_mult,
    }
}

/// Опыт для перехода с уровня level на следующий.
pub fn xp_to_next(level: u32) -> u32 {
    80 + level * 50
}
