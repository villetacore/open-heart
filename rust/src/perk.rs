//! Перки, синергии и агрегированные модификаторы. Data-driven (`data/perks.json`,
//! `data/synergies.json`). Тот же паттерн загрузки, что у оружия/классов:
//! OnceLock + встроенный фолбэк `include_str!`.
//!
//! Перк даёт эффекты по рангам; синергия — именованный бонус за комбинацию перков.
//! Эффекты складываются в [`PerkMods`], который [`crate::game`] вливает в `Loadout`.

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::RwLock;

fn f_one() -> f32 { 1.0 }
fn u_one() -> u32 { 1 }
fn de_u32<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    f64::deserialize(d).map(|v| v as u32)
}

/// Один эффект: либо аддитивный (`add`), либо мультипликативный (`mult`).
#[derive(Deserialize, Clone)]
pub struct PerkEffect {
    pub stat: String,
    #[serde(default)] pub add: f32,
    #[serde(default = "f_one")] pub mult: f32,
}

#[derive(Deserialize, Clone)]
pub struct PerkDef {
    pub id:        String,
    pub branch:    String,
    #[serde(deserialize_with = "de_u32")] pub tier:      u32,
    #[serde(deserialize_with = "de_u32")] pub max_ranks: u32,
    #[serde(default = "u_one", deserialize_with = "de_u32")] pub cost: u32,
    #[serde(default)] pub requires: Vec<String>, // "perk_id:rank"
    pub name_ru:   String,
    pub desc_ru:   String,
    pub effects:   Vec<PerkEffect>,
}

#[derive(Deserialize, Clone)]
pub struct SynergyDef {
    pub id:      String,
    pub needs:   Vec<String>, // "perk_id:rank"
    pub name_ru: String,
    pub desc_ru: String,
    pub effects: Vec<PerkEffect>,
}

// ── Модификаторы, вливаемые в Loadout ─────────────────────────────────────────

pub struct PerkMods {
    pub max_hp_add:    f32,
    pub speed_mult:    f32,
    pub dmg_add:       f32,   // добавка к dmg_mult (доля)
    pub cd_mult:       f32,   // множитель кулдауна (меньше = быстрее)
    pub lifesteal_add: f32,
    pub ammo_add:      f32,   // добавка к ammo_mult
}

impl Default for PerkMods {
    fn default() -> Self {
        Self { max_hp_add: 0.0, speed_mult: 1.0, dmg_add: 0.0,
               cd_mult: 1.0, lifesteal_add: 0.0, ammo_add: 0.0 }
    }
}

impl PerkMods {
    fn apply(&mut self, e: &PerkEffect, rank: u32) {
        let r = rank as i32;
        match e.stat.as_str() {
            "max_hp"    => self.max_hp_add    += e.add * rank as f32,
            "speed"     => self.speed_mult    *= e.mult.powi(r),
            "dmg"       => self.dmg_add       += e.add * rank as f32,
            "cd"        => self.cd_mult       *= e.mult.powi(r),
            "lifesteal" => self.lifesteal_add += e.add * rank as f32,
            "ammo"      => self.ammo_add      += e.add * rank as f32,
            other       => godot::global::godot_warn!("perk effect: unknown stat '{}'", other),
        }
    }
}

// ── Загрузка ──────────────────────────────────────────────────────────────────

pub(crate) fn parse_perks(json: &str) -> Result<Vec<PerkDef>, String> {
    serde_json::from_str(json).map_err(|e| e.to_string())
}
pub(crate) fn parse_syn(json: &str) -> Result<Vec<SynergyDef>, String> {
    serde_json::from_str(json).map_err(|e| e.to_string())
}

const EMBEDDED_PERKS: &str = include_str!("../../godot/presets/core/perks.json");
const EMBEDDED_SYN:   &str = include_str!("../../godot/presets/core/synergies.json");

static PERKS: RwLock<Option<&'static [PerkDef]>> = RwLock::new(None);
static SYNERGIES: RwLock<Option<&'static [SynergyDef]>> = RwLock::new(None);

fn embedded_perks() -> Vec<PerkDef> { parse_perks(EMBEDDED_PERKS).expect("встроенный perks.json") }
fn embedded_syn() -> Vec<SynergyDef> { parse_syn(EMBEDDED_SYN).expect("встроенный synergies.json") }

/// Загрузить (или перезагрузить при смене пресета) перки и синергии.
pub fn load(perks_json: Option<&str>, syn_json: Option<&str>) {
    let d = match perks_json {
        Some(j) => parse_perks(j).unwrap_or_else(|e| {
            godot::global::godot_warn!("perks.json: {e}; using embedded"); embedded_perks() }),
        None => embedded_perks(),
    };
    *PERKS.write().unwrap() = Some(Box::leak(d.into_boxed_slice()));

    let d = match syn_json {
        Some(j) => parse_syn(j).unwrap_or_else(|e| {
            godot::global::godot_warn!("synergies.json: {e}; using embedded"); embedded_syn() }),
        None => embedded_syn(),
    };
    *SYNERGIES.write().unwrap() = Some(Box::leak(d.into_boxed_slice()));
}

pub fn perks() -> &'static [PerkDef] {
    if let Some(p) = *PERKS.read().unwrap() { return p; }
    load(None, None);
    PERKS.read().unwrap().expect("perks after load")
}
pub fn synergies() -> &'static [SynergyDef] {
    if let Some(s) = *SYNERGIES.read().unwrap() { return s; }
    load(None, None);
    SYNERGIES.read().unwrap().expect("synergies after load")
}

pub fn perk_by_id(id: &str) -> Option<&'static PerkDef> {
    perks().iter().find(|p| p.id == id)
}

// ── Логика ────────────────────────────────────────────────────────────────────

/// Разобрать "perk_id:rank".
fn parse_req(s: &str) -> (&str, u32) {
    match s.split_once(':') {
        Some((id, n)) => (id, n.parse().unwrap_or(1)),
        None => (s, 1),
    }
}

pub fn reqs_met(reqs: &[String], owned: &HashMap<String, u32>) -> bool {
    reqs.iter().all(|r| {
        let (id, need) = parse_req(r);
        owned.get(id).copied().unwrap_or(0) >= need
    })
}

pub fn synergy_active(s: &SynergyDef, owned: &HashMap<String, u32>) -> bool {
    reqs_met(&s.needs, owned)
}

/// Итоговые модификаторы от всех купленных перков и активных синергий.
pub fn mods_for(owned: &HashMap<String, u32>) -> PerkMods {
    let mut m = PerkMods::default();
    for p in perks() {
        let rank = owned.get(&p.id).copied().unwrap_or(0);
        if rank == 0 { continue; }
        for e in &p.effects { m.apply(e, rank); }
    }
    for s in synergies() {
        if synergy_active(s, owned) {
            for e in &s.effects { m.apply(e, 1); }
        }
    }
    m
}

/// Перки, доступные к покупке прямо сейчас (требования выполнены, ранг не максимален,
/// хватает очков). До 8 штук — под номера 1..8 в экране перков.
pub fn available(owned: &HashMap<String, u32>, points: u32) -> Vec<&'static PerkDef> {
    perks().iter().filter(|p| {
        let rank = owned.get(&p.id).copied().unwrap_or(0);
        rank < p.max_ranks && reqs_met(&p.requires, owned) && points >= p.cost
    }).take(8).collect()
}
