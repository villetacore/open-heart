//! Арсенал: типы боеприпасов, описания оружия, состояние оружия игрока.

// ── Боеприпасы ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum AmmoType { Bullets, Shells, Rockets, Cells }

impl AmmoType {
    pub const ALL: [AmmoType; 4] = [AmmoType::Bullets, AmmoType::Shells, AmmoType::Rockets, AmmoType::Cells];

    pub fn idx(&self) -> usize {
        match self { Self::Bullets => 0, Self::Shells => 1, Self::Rockets => 2, Self::Cells => 3 }
    }
    pub fn from_idx(i: usize) -> Self { Self::ALL[i % 4] }

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
}

/// Тип выстрела.
#[derive(Clone, Copy, PartialEq)]
pub enum FireKind {
    Melee,                       // ближний удар (дуга перед собой)
    Hitscan { pellets: u32, spread: f32 }, // мгновенные лучи
    Projectile { speed: f32, splash: f32 }, // летящий снаряд (сплэш > 0 — взрыв)
}

pub struct WeaponDef {
    pub id:          WeaponId,
    pub name_ru:     &'static str,
    pub damage:      f32,          // урон за луч/удар/снаряд
    pub cooldown:    f32,
    pub range:       f32,
    pub kind:        FireKind,
    pub ammo:        Option<(AmmoType, u32)>, // тип и расход за выстрел
    pub auto:        bool,         // стреляет при удержании
    // FP-спрайт
    pub sheet:       &'static str,
    pub frame_h:     f32,
    pub idle_frames: &'static [usize],
    pub fire_frames: &'static [usize],
    pub fire_fps:    f32,
}

pub const FRAME_W: f32 = 84.0;

pub const WEAPONS: [WeaponDef; 8] = [
    WeaponDef {
        id: WeaponId::Sword, name_ru: "Энергоклинок",
        damage: 34.0, cooldown: 0.5, range: 2.8,
        kind: FireKind::Melee, ammo: None, auto: false,
        sheet: "res://assets/sprites/weapons_fp/wf_sword.png",
        frame_h: 59.0, idle_frames: &[0], fire_frames: &[1, 2, 3, 4, 5, 6], fire_fps: 16.0,
    },
    WeaponDef {
        id: WeaponId::Chainsaw, name_ru: "Цепная пила",
        damage: 9.0, cooldown: 0.12, range: 2.5,
        kind: FireKind::Melee, ammo: None, auto: true,
        sheet: "res://assets/sprites/weapons_fp/wf_chainsaw.png",
        frame_h: 70.0, idle_frames: &[0, 1], fire_frames: &[4, 5, 6, 7], fire_fps: 14.0,
    },
    WeaponDef {
        id: WeaponId::Pistol, name_ru: "Пистолет",
        damage: 16.0, cooldown: 0.38, range: 32.0,
        kind: FireKind::Hitscan { pellets: 1, spread: 0.012 },
        ammo: Some((AmmoType::Bullets, 1)), auto: false,
        sheet: "res://assets/sprites/weapons_fp/wf_pistol.png",
        frame_h: 86.0, idle_frames: &[0], fire_frames: &[6, 7, 2], fire_fps: 14.0,
    },
    WeaponDef {
        id: WeaponId::Shotgun, name_ru: "Дробовик",
        damage: 9.0, cooldown: 0.95, range: 16.0,
        kind: FireKind::Hitscan { pellets: 7, spread: 0.09 },
        ammo: Some((AmmoType::Shells, 1)), auto: false,
        sheet: "res://assets/sprites/weapons_fp/wf_shotgun.png",
        frame_h: 95.0, idle_frames: &[0], fire_frames: &[6, 7, 3, 4], fire_fps: 10.0,
    },
    WeaponDef {
        id: WeaponId::Rifle, name_ru: "Автомат",
        damage: 12.0, cooldown: 0.12, range: 38.0,
        kind: FireKind::Hitscan { pellets: 1, spread: 0.03 },
        ammo: Some((AmmoType::Bullets, 1)), auto: true,
        sheet: "res://assets/sprites/weapons_fp/wf_rifle.png",
        frame_h: 91.0, idle_frames: &[0], fire_frames: &[6, 7], fire_fps: 18.0,
    },
    WeaponDef {
        id: WeaponId::Nailgun, name_ru: "Гвоздемёт",
        damage: 30.0, cooldown: 0.3, range: 30.0,
        kind: FireKind::Hitscan { pellets: 1, spread: 0.006 },
        ammo: Some((AmmoType::Bullets, 2)), auto: true,
        sheet: "res://assets/sprites/weapons_fp/wf_nailgun.png",
        frame_h: 80.0, idle_frames: &[0], fire_frames: &[6, 7], fire_fps: 12.0,
    },
    WeaponDef {
        id: WeaponId::Plasma, name_ru: "Плазмаган",
        damage: 26.0, cooldown: 0.22, range: 45.0,
        kind: FireKind::Projectile { speed: 26.0, splash: 0.0 },
        ammo: Some((AmmoType::Cells, 1)), auto: true,
        sheet: "res://assets/sprites/weapons_fp/wf_plasma.png",
        frame_h: 65.0, idle_frames: &[0], fire_frames: &[5, 6, 7], fire_fps: 16.0,
    },
    WeaponDef {
        id: WeaponId::Rocket, name_ru: "Ракетница",
        damage: 95.0, cooldown: 1.15, range: 60.0,
        kind: FireKind::Projectile { speed: 19.0, splash: 4.2 },
        ammo: Some((AmmoType::Rockets, 1)), auto: false,
        sheet: "res://assets/sprites/weapons_fp/wf_rocket.png",
        frame_h: 82.0, idle_frames: &[0], fire_frames: &[6, 7], fire_fps: 10.0,
    },
];

pub fn weapon_def(id: WeaponId) -> &'static WeaponDef {
    &WEAPONS[id.slot()]
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
