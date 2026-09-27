//! Классы персонажа: 3 класса × 3 специализации. Data-driven (`data/classes.json`).
//!
//! Мили («Берсерк»)         — пила/клинок, толстый, урон в упор.
//! Клоус-рэнж («Штурмовик») — пистолет/дробовик, ближняя перестрелка.
//! Мид-рэнж («Оператор»)    — автомат/плазма, держит дистанцию.

use crate::weapon::{AmmoType, WeaponId};
use serde::Deserialize;
use std::sync::RwLock;

pub struct SpecDef {
    pub id: String,
    pub name_ru: String,
    pub desc_ru: String,
    pub name_en: String,
    pub desc_en: String,
    pub hp_bonus: f32,
    pub speed_mult: f32,
    pub dmg_mult: f32,
    pub cd_mult: f32,
    pub lifesteal: f32,
    pub ammo_mult: f32,
    pub extra_weapon: Option<WeaponId>,
    /// Слот умения (0/1), которому этот спек даёт второй заряд (план §5).
    pub charge_slot: Option<usize>,
}

pub struct ClassDef {
    pub id: String,
    pub name_ru: String,
    pub role_ru: String,
    pub desc_ru: String,
    pub name_en: String,
    pub role_en: String,
    pub desc_en: String,
    pub base_hp: f32,
    pub speed: f32,
    pub dmg_mult: f32,
    pub start_weapons: Vec<WeaponId>,
    pub start_ammo: Vec<(AmmoType, u32)>,
    pub specs: Vec<SpecDef>,
}

impl SpecDef {
    pub fn name(&self, lang: &str) -> &str {
        if lang == "en" && !self.name_en.is_empty() {
            &self.name_en
        } else {
            &self.name_ru
        }
    }
    pub fn description(&self, lang: &str) -> &str {
        if lang == "en" && !self.desc_en.is_empty() {
            &self.desc_en
        } else {
            &self.desc_ru
        }
    }
}

impl ClassDef {
    pub fn name(&self, lang: &str) -> &str {
        if lang == "en" && !self.name_en.is_empty() {
            &self.name_en
        } else {
            &self.name_ru
        }
    }
    pub fn role(&self, lang: &str) -> &str {
        if lang == "en" && !self.role_en.is_empty() {
            &self.role_en
        } else {
            &self.role_ru
        }
    }
    pub fn description(&self, lang: &str) -> &str {
        if lang == "en" && !self.desc_en.is_empty() {
            &self.desc_en
        } else {
            &self.desc_ru
        }
    }
}

// ── Загрузка из JSON ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct SpecRaw {
    id: String,
    name_ru: String,
    desc_ru: String,
    #[serde(default)]
    name_en: String,
    #[serde(default)]
    desc_en: String,
    hp_bonus: f32,
    speed_mult: f32,
    dmg_mult: f32,
    cd_mult: f32,
    lifesteal: f32,
    ammo_mult: f32,
    #[serde(default)]
    extra_weapon: Option<String>,
    #[serde(default)]
    charge_slot: Option<usize>,
}

#[derive(Deserialize)]
struct AmmoStartRaw {
    #[serde(rename = "type")]
    ty: String,
    amount: f64,
}

#[derive(Deserialize)]
struct ClassRaw {
    id: String,
    name_ru: String,
    role_ru: String,
    desc_ru: String,
    #[serde(default)]
    name_en: String,
    #[serde(default)]
    role_en: String,
    #[serde(default)]
    desc_en: String,
    base_hp: f32,
    speed: f32,
    dmg_mult: f32,
    start_weapons: Vec<String>,
    start_ammo: Vec<AmmoStartRaw>,
    specs: Vec<SpecRaw>,
}

impl SpecRaw {
    fn into_def(self) -> Result<SpecDef, String> {
        let extra = match self.extra_weapon {
            None => None,
            Some(ref s) => Some(
                WeaponId::from_id(s)
                    .ok_or_else(|| format!("spec '{}': unknown weapon '{}'", self.id, s))?,
            ),
        };
        Ok(SpecDef {
            id: self.id,
            name_ru: self.name_ru,
            desc_ru: self.desc_ru,
            name_en: self.name_en,
            desc_en: self.desc_en,
            hp_bonus: self.hp_bonus,
            speed_mult: self.speed_mult,
            dmg_mult: self.dmg_mult,
            cd_mult: self.cd_mult,
            lifesteal: self.lifesteal,
            ammo_mult: self.ammo_mult,
            extra_weapon: extra,
            charge_slot: self.charge_slot.filter(|slot| *slot < 2),
        })
    }
}

impl ClassRaw {
    fn into_def(self) -> Result<ClassDef, String> {
        let mut weapons = Vec::new();
        for w in &self.start_weapons {
            weapons.push(
                WeaponId::from_id(w)
                    .ok_or_else(|| format!("class '{}': unknown weapon '{}'", self.id, w))?,
            );
        }
        let mut ammo = Vec::new();
        for a in &self.start_ammo {
            let t = AmmoType::from_id(&a.ty)
                .ok_or_else(|| format!("class '{}': unknown ammo '{}'", self.id, a.ty))?;
            ammo.push((t, a.amount as u32));
        }
        let mut specs = Vec::new();
        for s in self.specs {
            specs.push(s.into_def()?);
        }
        if specs.len() < 3 {
            return Err(format!(
                "class '{}': needs 3 specs, got {}",
                self.id,
                specs.len()
            ));
        }
        Ok(ClassDef {
            id: self.id,
            name_ru: self.name_ru,
            role_ru: self.role_ru,
            desc_ru: self.desc_ru,
            name_en: self.name_en,
            role_en: self.role_en,
            desc_en: self.desc_en,
            base_hp: self.base_hp,
            speed: self.speed,
            dmg_mult: self.dmg_mult,
            start_weapons: weapons,
            start_ammo: ammo,
            specs,
        })
    }
}

pub(crate) fn parse(json: &str) -> Result<Vec<ClassDef>, String> {
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

const EMBEDDED: &str = include_str!("../../../game/presets/core/classes.json");

static CLASSES: RwLock<Option<&'static [ClassDef]>> = RwLock::new(None);

fn embedded() -> Vec<ClassDef> {
    parse(EMBEDDED).expect("встроенный classes.json должен быть валиден")
}

/// Загрузить (или перезагрузить при смене пресета) таблицу классов.
pub fn load(runtime_json: Option<&str>) {
    let defs = match runtime_json {
        Some(j) => match parse(j) {
            Ok(d) => d,
            Err(e) => {
                crate::warn!("classes.json: {e}; using embedded");
                embedded()
            }
        },
        None => embedded(),
    };
    *CLASSES.write().unwrap() = Some(Box::leak(defs.into_boxed_slice()));
}

pub fn classes() -> &'static [ClassDef] {
    if let Some(c) = *CLASSES.read().unwrap() {
        return c;
    }
    load(None);
    CLASSES.read().unwrap().expect("classes after load(None)")
}

pub fn class_by_id(id: &str) -> Option<&'static ClassDef> {
    classes().iter().find(|c| c.id == id)
}

// ── Расчёт лоадаута ───────────────────────────────────────────────────────────

/// Итоговые боевые параметры: класс + спек + уровень.
pub struct Loadout {
    pub max_hp: f32,
    pub speed: f32,
    pub dmg_mult: f32,
    pub cd_mult: f32,
    pub lifesteal: f32,
    pub ammo_mult: f32,
    /// Слот умения со вторым зарядом от специализации (None — у всех по одному).
    pub charge_slot: Option<usize>,
    /// Прибавка к шансу крита оружия от перков (0..1). Спеки его пока не дают.
    pub crit_bonus: f32,
}

pub fn compute_loadout(class_idx: usize, spec_idx: usize, level: u32) -> Loadout {
    let list = classes();
    let c = &list[class_idx.min(list.len() - 1)];
    let s = &c.specs[spec_idx.min(c.specs.len() - 1)];
    let lvl_bonus = (level.saturating_sub(1)) as f32;
    Loadout {
        max_hp: (c.base_hp + s.hp_bonus + lvl_bonus * 8.0).max(40.0),
        speed: c.speed * s.speed_mult,
        dmg_mult: c.dmg_mult * s.dmg_mult * (1.0 + lvl_bonus * 0.03),
        cd_mult: s.cd_mult,
        lifesteal: s.lifesteal,
        ammo_mult: s.ammo_mult,
        charge_slot: s.charge_slot,
        crit_bonus: 0.0,
    }
}

/// Опыт для перехода с уровня level на следующий.
pub fn xp_to_next(level: u32) -> u32 {
    80 + level * 50
}
