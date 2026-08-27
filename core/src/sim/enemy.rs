//! Враги на сервере: аггро, путь по нав-сетке, атака.
//!
//! Это не копия клиентского `Enemy` (там анимации, спрайты и звук), а его
//! авторитетная половина: где враг стоит, кого бьёт и сколько у него здоровья.
//! Клиент рисует то, что пришло в снапшоте.

use serde::{Deserialize, Serialize};

use crate::ai::policy::{fallback_intent, Brain, Intent, Situation};
use crate::config::EnemyCfg;
use crate::math::Vec3;
use crate::nav::NavGrid;
use crate::sim::world::World;
use crate::sim::{flags, Event, Player};

/// Как часто перестраивается путь: A* каждый тик не нужен и дорог.
const PATH_PERIOD: f32 = 0.7;
/// Дистанция, на которой враг считает вейпоинт достигнутым.
const WAYPOINT_REACHED: f32 = 1.2;
/// Запас к радиусу погони, пока цель уже выбрана (чтобы враг не «моргал» целью).
const CHASE_HYSTERESIS: f32 = 1.35;

/// Как быстро забывается нанесённый урон, доля в секунду. Полминуты — и враг
/// снова смотрит на того, кто просто ближе.
const THREAT_DECAY: f32 = 0.08;
/// Насколько сильно на выбор влияет близость: угроза важнее, но стоящего
/// вплотную игрока враг не должен игнорировать.
const PROXIMITY_WEIGHT: f32 = 12.0;
/// Насколько убедительнее должен быть новый кандидат, чтобы враг сменил цель.
/// Без этого запаса двое с равным уроном перекидывали бы врага туда-сюда.
const SWITCH_MARGIN: f32 = 1.25;

/// Сколько «внимания» враг должен конкретному игроку.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Threat {
    pub peer: u16,
    pub value: f32,
}

/// Враг глазами сервера.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Enemy {
    pub id: u16,
    /// id из enemies.json.
    pub kind: String,
    /// Индекс вида в пресете (+1) — по нему клиент берёт спрайт и размеры.
    pub type_id: u16,
    /// id обученного поведения; None — обычный автомат.
    pub brain: Option<String>,
    pub pos: Vec3,
    pub yaw: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub speed: f32,
    pub damage: f32,
    pub attack_range: f32,
    pub attack_cooldown: f32,
    pub chase_range: f32,
    /// [physical, fire, energy, void] — в порядке `DmgType::idx`.
    pub resist: [f32; 4],
    pub xp: f32,
    pub is_boss: bool,
    pub affixes: Vec<String>,

    /// peer игрока, которого враг преследует.
    pub target: Option<u16>,
    /// Кто и сколько урона нанёс: по этому враг и выбирает, кого бить.
    /// Список короткий — в пати редко больше четырёх человек.
    #[serde(default)]
    pub threat: Vec<Threat>,
    pub attack_timer: f32,
    pub path: Vec<(i32, i32)>,
    pub path_timer: f32,
}

impl Enemy {
    /// Собрать врага из конфига пресета и точки спавна.
    ///
    /// `mult` — множитель глубины и «босс-тира»: он усиливает врага целиком.
    /// `hp_mult` — надбавка за размер пати: она трогает **только** здоровье,
    /// иначе вчетвером игроки получали бы и больше врагов, и больнее удары.
    #[allow(clippy::too_many_arguments)]
    pub fn from_cfg(id: u16, type_id: u16, cfg: &EnemyCfg, pos: Vec3, mult: f32, hp_mult: f32,
                    is_boss: bool, affixes: Vec<String>) -> Self {
        let hp = cfg.hp * mult.max(0.1) * hp_mult.max(0.1);
        Self {
            id,
            kind: cfg.id.clone(),
            type_id,
            brain: cfg.brain.clone(),
            pos,
            yaw: 0.0,
            hp,
            max_hp: hp,
            speed: cfg.speed,
            damage: cfg.attack_damage * mult.max(0.1),
            attack_range: cfg.attack_range,
            attack_cooldown: cfg.attack_cooldown.max(0.2),
            chase_range: cfg.chase_range,
            resist: cfg.resist.arr(),
            xp: cfg.xp * mult.max(0.1),
            is_boss,
            affixes,
            target: None,
            threat: Vec::new(),
            attack_timer: 0.0,
            path: Vec::new(),
            path_timer: 0.0,
        }
    }

    pub fn alive(&self) -> bool {
        self.hp > 0.0
    }

    /// Биты состояния для снапшота.
    pub fn flags(&self) -> u8 {
        let mut bits = 0;
        if self.is_boss || !self.affixes.is_empty() {
            bits |= flags::ELITE;
        }
        bits
    }

    /// Здоровье в процентах — снапшот шлёт именно его.
    pub fn hp_percent(&self) -> u8 {
        if self.max_hp <= 0.0 {
            return 0;
        }
        ((self.hp / self.max_hp) * 100.0).clamp(0.0, 100.0) as u8
    }

    /// Шаг врага: выбрать цель, решить намерение, исполнить его.
    ///
    /// `brain` — обученное поведение вида, если пресет его дал. Сеть решает
    /// «что делать», а как именно двигаться — обычный код ниже.
    pub fn tick(
        &mut self,
        dt: f32,
        world: &World,
        players: &[&Player],
        brain: Option<&Brain>,
        events: &mut Vec<Event>,
    ) {
        if !self.alive() {
            return;
        }
        self.attack_timer = (self.attack_timer - dt).max(0.0);
        self.path_timer = (self.path_timer - dt).max(0.0);
        self.decay_threat(dt);

        self.pick_target(players);
        let Some(target_peer) = self.target else {
            return;
        };
        let Some(target) = players.iter().find(|p| p.peer == target_peer) else {
            self.target = None;
            return;
        };

        let to_target = target.pos - self.pos;
        let distance = to_target.length_flat();
        if distance > 0.05 {
            self.yaw = (-to_target.x).atan2(-to_target.z);
        }

        let situation = Situation {
            distance,
            attack_range: self.attack_range,
            chase_range: self.chase_range,
            hp_frac: if self.max_hp > 0.0 { self.hp / self.max_hp } else { 0.0 },
            target_hp_frac: if target.max_hp > 0.0 { target.hp / target.max_hp } else { 0.0 },
            attack_ready: if self.attack_timer <= 0.0 { 1.0 } else { 0.0 },
            allies_near: 0,
            has_path: world.nav.is_none() || !self.path.is_empty(),
        };
        let intent = match brain {
            Some(brain) => brain.decide(&situation),
            None => fallback_intent(&situation),
        };

        match intent {
            Intent::Attack => {
                // Бить можно только в пределах досягаемости — сеть не отменяет физику.
                if distance <= self.attack_range {
                    self.attack(target, events);
                } else {
                    self.step_towards(dt, world, target.pos);
                }
            }
            Intent::Chase => self.step_towards(dt, world, target.pos),
            Intent::Retreat => {
                let away = self.pos + (self.pos - target.pos).flat().normalized() * 4.0;
                self.step_towards(dt, world, away);
            }
            Intent::Strafe => {
                let side = Vec3::new(-(target.pos.z - self.pos.z), 0.0, target.pos.x - self.pos.x);
                if side.length() > 0.01 {
                    self.step_towards(dt, world, self.pos + side.normalized() * 3.0);
                }
            }
            Intent::Wait => {}
        }
    }

    /// Записать урон от игрока: чем больше бьёт, тем настойчивее враг за ним идёт.
    pub fn add_threat(&mut self, peer: u16, amount: f32) {
        if amount <= 0.0 {
            return;
        }
        match self.threat.iter_mut().find(|entry| entry.peer == peer) {
            Some(entry) => entry.value += amount,
            None => self.threat.push(Threat { peer, value: amount }),
        }
    }

    /// Сколько внимания враг должен этому игроку.
    pub fn threat_of(&self, peer: u16) -> f32 {
        self.threat
            .iter()
            .find(|entry| entry.peer == peer)
            .map(|entry| entry.value)
            .unwrap_or(0.0)
    }

    /// Урон забывается: иначе враг до конца забега помнил бы первый выстрел.
    fn decay_threat(&mut self, dt: f32) {
        let keep = 1.0 - THREAT_DECAY * dt;
        for entry in &mut self.threat {
            entry.value *= keep.max(0.0);
        }
        self.threat.retain(|entry| entry.value > 0.5);
    }

    /// Выбрать цель: сильнее всего тянет тот, кто наносит урон, но стоящего
    /// вплотную враг тоже не игнорирует.
    ///
    /// Смена цели требует запаса: без него двое с похожим уроном раскачивали бы
    /// врага между собой, и он не дошёл бы ни до кого.
    fn pick_target(&mut self, players: &[&Player]) {
        let mut best: Option<(u16, f32)> = None;
        let mut current: Option<f32> = None;

        for player in players {
            if player.downed() {
                continue;
            }
            let distance = (player.pos - self.pos).length_flat();
            let limit = if self.target == Some(player.peer) {
                self.chase_range * CHASE_HYSTERESIS
            } else {
                self.chase_range
            };
            if distance > limit {
                continue;
            }

            // Близость даёт небольшой вклад: у самого носа — максимум, на краю
            // радиуса погони — ноль.
            let closeness = 1.0 - (distance / self.chase_range.max(0.1)).clamp(0.0, 1.0);
            let score = self.threat_of(player.peer) + closeness * PROXIMITY_WEIGHT;

            if self.target == Some(player.peer) {
                current = Some(score);
            }
            if best.map(|(_, value)| score > value).unwrap_or(true) {
                best = Some((player.peer, score));
            }
        }

        let Some((candidate, candidate_score)) = best else {
            self.target = None;
            return;
        };
        match (self.target, current) {
            // Текущая цель ещё в игре: меняем только на заметно более наглого.
            (Some(target), Some(score)) if candidate != target => {
                if candidate_score > score * SWITCH_MARGIN {
                    self.target = Some(candidate);
                }
            }
            _ => self.target = Some(candidate),
        }
    }

    fn attack(&mut self, target: &Player, events: &mut Vec<Event>) {
        if self.attack_timer > 0.0 {
            return;
        }
        self.attack_timer = self.attack_cooldown;
        let mut event = Event::new(crate::sim::event::DAMAGE);
        event.actor = self.id;
        event.target = target.peer;
        event.amount = self.damage;
        event.pos = Some(target.pos);
        events.push(event);
    }

    /// Движение к цели: по A* в данже, напрямик там, где сетки нет.
    fn step_towards(&mut self, dt: f32, world: &World, target_pos: Vec3) {
        let direction = match world.nav.as_ref() {
            Some(nav) => self.path_dir(nav, world, target_pos),
            None => Some((target_pos - self.pos).flat().normalized()),
        };
        let Some(direction) = direction else {
            return;
        };
        let step = direction * (self.speed * dt);
        let next = self.pos + step;
        if world.walkable_at(next) {
            self.pos = Vec3::new(next.x, world.floor_at(next), next.z);
        }
    }

    /// Направление на следующий вейпоинт пути; None — идти некуда.
    fn path_dir(&mut self, nav: &NavGrid, world: &World, target_pos: Vec3) -> Option<Vec3> {
        let local = self.pos - world.offset;
        let target_local = target_pos - world.offset;

        if self.path_timer <= 0.0 {
            self.path = nav
                .astar(NavGrid::cell_of(local), NavGrid::cell_of(target_local))
                .unwrap_or_default();
            // Пустой путь тоже кэшируем, но ненадолго: искать каждый кадр дорого.
            self.path_timer = if self.path.is_empty() { 0.35 } else { PATH_PERIOD };
        }

        while let Some(&(i, j)) = self.path.first() {
            let waypoint = NavGrid::center_of(i, j);
            if (waypoint - local).length_flat() < WAYPOINT_REACHED {
                self.path.remove(0);
            } else {
                break;
            }
        }

        let &(i, j) = self.path.first()?;
        let waypoint = NavGrid::center_of(i, j);
        let direction = (waypoint - local).flat();
        (direction.length() > 0.01).then(|| direction.normalized())
    }
}
