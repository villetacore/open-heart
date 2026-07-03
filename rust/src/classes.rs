//! Классы персонажа: 3 класса × 3 специализации.
//!
//! Мили («Берсерк»)      — пила/клинок, толстый, быстрый, урон в упор.
//! Клоус-рэнж («Штурмовик») — пистолет/дробовик, ближняя перестрелка.
//! Мид-рэнж («Оператор»)  — автомат/плазма, держит дистанцию.

use crate::weapon::{AmmoType, WeaponId};

pub struct SpecDef {
    pub id:            &'static str,
    pub name_ru:       &'static str,
    pub desc_ru:       &'static str,
    pub hp_bonus:      f32,   // + к максимуму HP
    pub speed_mult:    f32,
    pub dmg_mult:      f32,
    pub cd_mult:       f32,   // множитель кулдауна оружия (меньше = быстрее)
    pub lifesteal:     f32,   // доля нанесённого урона, возвращаемая в HP
    pub ammo_mult:     f32,   // множитель максимального боезапаса
    pub extra_weapon:  Option<WeaponId>,
}

pub struct ClassDef {
    pub id:          &'static str,
    pub name_ru:     &'static str,
    pub role_ru:     &'static str,
    pub desc_ru:     &'static str,
    pub base_hp:     f32,
    pub speed:       f32,
    pub dmg_mult:    f32,
    pub start_weapons: &'static [WeaponId],
    pub start_ammo:  &'static [(AmmoType, u32)],
    pub specs:       [SpecDef; 3],
}

pub const CLASSES: [ClassDef; 3] = [
    ClassDef {
        id: "berserk", name_ru: "БЕРСЕРК", role_ru: "Мили",
        desc_ru: "Пила и клинок. Живёт в гуще боя, лечится чужой болью.",
        base_hp: 150.0, speed: 5.8, dmg_mult: 1.0,
        start_weapons: &[WeaponId::Sword, WeaponId::Chainsaw],
        start_ammo: &[(AmmoType::Shells, 8)],
        specs: [
            SpecDef {
                id: "bloodreaper", name_ru: "Кровожнец",
                desc_ru: "25% урона в ближнем бою возвращается здоровьем.",
                hp_bonus: 0.0, speed_mult: 1.0, dmg_mult: 1.0,
                cd_mult: 1.0, lifesteal: 0.25, ammo_mult: 1.0, extra_weapon: None,
            },
            SpecDef {
                id: "juggernaut", name_ru: "Таран",
                desc_ru: "+60 к здоровью. Медленнее, но почти неубиваем.",
                hp_bonus: 60.0, speed_mult: 0.92, dmg_mult: 1.0,
                cd_mult: 1.0, lifesteal: 0.0, ammo_mult: 1.0, extra_weapon: None,
            },
            SpecDef {
                id: "whirlwind", name_ru: "Вихрь",
                desc_ru: "+20% скорость бега, удары на 25% быстрее.",
                hp_bonus: -20.0, speed_mult: 1.2, dmg_mult: 1.0,
                cd_mult: 0.75, lifesteal: 0.0, ammo_mult: 1.0, extra_weapon: None,
            },
        ],
    },
    ClassDef {
        id: "vanguard", name_ru: "ШТУРМОВИК", role_ru: "Клоус-рэнж",
        desc_ru: "Пистолет и дробовик. Врывается, стреляет в упор, уходит.",
        base_hp: 115.0, speed: 5.3, dmg_mult: 1.0,
        start_weapons: &[WeaponId::Pistol, WeaponId::Shotgun],
        start_ammo: &[(AmmoType::Bullets, 60), (AmmoType::Shells, 20)],
        specs: [
            SpecDef {
                id: "executioner", name_ru: "Экзекутор",
                desc_ru: "+20% урона всем оружием.",
                hp_bonus: 0.0, speed_mult: 1.0, dmg_mult: 1.2,
                cd_mult: 1.0, lifesteal: 0.0, ammo_mult: 1.0, extra_weapon: None,
            },
            SpecDef {
                id: "bastion", name_ru: "Бастион",
                desc_ru: "+45 к здоровью — щит на передовой.",
                hp_bonus: 45.0, speed_mult: 0.96, dmg_mult: 1.0,
                cd_mult: 1.0, lifesteal: 0.0, ammo_mult: 1.0, extra_weapon: None,
            },
            SpecDef {
                id: "duelist", name_ru: "Дуэлянт",
                desc_ru: "Перезарядка на 20% быстрее, +10% скорость.",
                hp_bonus: 0.0, speed_mult: 1.1, dmg_mult: 1.0,
                cd_mult: 0.8, lifesteal: 0.0, ammo_mult: 1.0, extra_weapon: None,
            },
        ],
    },
    ClassDef {
        id: "operator", name_ru: "ОПЕРАТОР", role_ru: "Мид-рэнж",
        desc_ru: "Автомат и плазма. Контролирует дистанцию и толпу.",
        base_hp: 100.0, speed: 5.0, dmg_mult: 1.0,
        start_weapons: &[WeaponId::Rifle, WeaponId::Plasma],
        start_ammo: &[(AmmoType::Bullets, 120), (AmmoType::Cells, 60)],
        specs: [
            SpecDef {
                id: "sniper", name_ru: "Снайпер",
                desc_ru: "+30% урона, стартовый гвоздемёт.",
                hp_bonus: -10.0, speed_mult: 1.0, dmg_mult: 1.3,
                cd_mult: 1.0, lifesteal: 0.0, ammo_mult: 1.0,
                extra_weapon: Some(WeaponId::Nailgun),
            },
            SpecDef {
                id: "demolition", name_ru: "Подрывник",
                desc_ru: "Стартовая ракетница и 8 ракет.",
                hp_bonus: 0.0, speed_mult: 1.0, dmg_mult: 1.0,
                cd_mult: 1.0, lifesteal: 0.0, ammo_mult: 1.0,
                extra_weapon: Some(WeaponId::Rocket),
            },
            SpecDef {
                id: "technician", name_ru: "Техник",
                desc_ru: "+50% максимум боезапаса, перезарядка -10%.",
                hp_bonus: 0.0, speed_mult: 1.0, dmg_mult: 1.0,
                cd_mult: 0.9, lifesteal: 0.0, ammo_mult: 1.5, extra_weapon: None,
            },
        ],
    },
];

pub fn class_by_id(id: &str) -> Option<&'static ClassDef> {
    CLASSES.iter().find(|c| c.id == id)
}

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
    let c = &CLASSES[class_idx.min(2)];
    let s = &c.specs[spec_idx.min(2)];
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
