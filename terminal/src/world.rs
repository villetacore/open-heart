use crate::app::App;
use openheart_core::{
    math::Vec3,
    weapon::{self, AmmoType, WeaponId},
    worldgen::dungeon::DungeonEventSpawn,
};

#[derive(Clone)]
pub enum PickupKind {
    Item(String),
    Ammo(AmmoType, u32),
    Weapon(WeaponId),
}
pub struct Pickup {
    pub pos: Vec3,
    pub kind: PickupKind,
    pub key: String,
}
pub struct Point {
    pub event: DungeonEventSpawn,
    pub used: bool,
}

impl App {
    /// Static preset pickups, as in the graphical client; dropped loot remains server-owned.
    pub fn populate_points(&mut self) {
        self.pickups.clear();
        self.points.clear();
        let mut pickups = Vec::new();
        if self.world.kind == "delve" {
            let spawns = &self.world.spawns;
            let offset = self.world.offset;
            for (id, pos) in &spawns.items {
                pickups.push((*pos + offset, PickupKind::Item(id.clone())));
            }
            for (kind, n, pos) in &spawns.ammo {
                pickups.push((*pos + offset, PickupKind::Ammo(*kind, *n)));
            }
            for (id, pos) in &spawns.weapons {
                pickups.push((*pos + offset, PickupKind::Weapon(*id)));
            }
            self.points = spawns
                .events
                .iter()
                .cloned()
                .map(|mut event| {
                    event.pos = event.pos + offset;
                    Point { event, used: false }
                })
                .collect();
        } else {
            let spawns = self
                .content
                .map
                .as_ref()
                .map(|m| &m.spawns)
                .unwrap_or(&self.content.cfg.level);
            for s in &spawns.spawn_items {
                pickups.push((Vec3::new(s.x, 0.0, s.z), PickupKind::Item(s.kind.clone())));
            }
            for s in &spawns.spawn_ammo {
                if let Some(kind) = AmmoType::from_id(&s.kind) {
                    pickups.push((Vec3::new(s.x, 0.0, s.z), PickupKind::Ammo(kind, s.amount)));
                }
            }
            for s in &spawns.spawn_weapons {
                if let Some(id) = WeaponId::from_id(&s.kind) {
                    pickups.push((Vec3::new(s.x, 0.0, s.z), PickupKind::Weapon(id)));
                }
            }
        }
        for (i, (pos, kind)) in pickups.into_iter().enumerate() {
            let key = format!(
                "terminal_pickup_{}_{}_{}",
                self.world.kind,
                if self.world.kind == "hub" {
                    0
                } else {
                    self.seed
                },
                i
            );
            if !self.collected.contains(&key) && !self.state.flags.contains(&key) {
                self.pickups.push(Pickup { pos, kind, key });
            }
        }
    }

    pub fn pickup_nearby(&mut self) {
        if self.downed {
            return;
        }
        let mut taken = Vec::new();
        self.pickups.retain(|p| {
            if (p.pos - self.pos).length_flat() < 1.6 {
                taken.push((p.kind.clone(), p.key.clone()));
                false
            } else {
                true
            }
        });
        for (kind, key) in taken {
            if self.world.kind == "hub" {
                self.state.flags.insert(key.clone());
            }
            self.collected.insert(key);
            match kind {
                PickupKind::Item(id) => {
                    self.take_item(&id);
                }
                PickupKind::Ammo(kind, count) => {
                    self.arsenal.add_ammo(kind, count, 1.0);
                    self.log(format!("{} +{}", kind.name("ru"), count));
                }
                PickupKind::Weapon(id) => {
                    self.arsenal.give_weapon(id);
                    self.bump("discover_weapon", id.id());
                    if let Some((ammo, _)) = weapon::weapon_def(id).ammo {
                        self.arsenal.add_ammo(ammo, ammo.pack_size(), 1.0);
                    }
                    self.log(format!("Подобрано: {}", weapon::weapon_def(id).name("ru")));
                }
            }
        }
    }

    pub fn take_item(&mut self, id: &str) {
        self.bump("collect", id);
        if let Some(item) = self.content.cfg.item(id) {
            let category = item.category.clone();
            let value = item.value as i32;
            let name = item.name("ru").to_string();
            self.bump("collect_category", &category);
            if category == "currency" {
                self.state.gold += value;
            } else {
                self.give_item(id, 1);
            }
            self.log(format!("Подобрано: {name}"));
        } else if id == "heart_1up" {
            self.state.add_heart();
            self.log("Сердце жизни найдено");
        }
    }

    pub fn use_point(&mut self, index: usize) {
        let point = &self.points[index];
        let group = point.event.group;
        let step = point.event.step;
        if point.used {
            return;
        }
        if point.event.kind == "story_echo" {
            self.points[index].used = true;
            self.state.add_xp(35 + self.depth * 5);
            self.bump("discover_lore", &self.depth.to_string());
            let lines = [
                "Память: первые смотрители запечатали сердце внизу.",
                "Память: архив стёр все карты нижнего хора.",
                "Память: машины научились подражать пульсу.",
                "Память: тиран — лишь замок, а не то, что спит за ним.",
            ];
            self.log(lines[self.depth.saturating_sub(1) as usize % lines.len()]);
            return;
        }
        let expected = self
            .points
            .iter()
            .filter(|p| p.event.group == group && p.used)
            .count() as u32;
        if step != expected {
            for p in self.points.iter_mut().filter(|p| p.event.group == group) {
                p.used = false;
            }
            self.log("Неверный порядок реле — последовательность сброшена");
            return;
        }
        self.points[index].used = true;
        if self
            .points
            .iter()
            .filter(|p| p.event.group == group)
            .all(|p| p.used)
        {
            self.state.gold += 30 + self.depth as i32 * 12;
            self.state.add_xp(45 + self.depth * 8);
            self.bump("solve_puzzle", &self.depth.to_string());
            self.log("Реле запущено! Получены золото и опыт.");
        } else {
            self.log(format!("Реле {} синхронизировано", step + 1));
        }
    }
}
