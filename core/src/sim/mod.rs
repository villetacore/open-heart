//! Авторитетная симуляция комнаты.
//!
//! Здесь живёт состояние мира, которое считает сервер (и предсказывает клиент):
//! игроки, враги, урон. Мир комнаты строится тем же кодом, которым клиент рисует
//! данж, поэтому серверные враги ходят там же, где игрок видит пол.
//!
//! Формат структур обязан совпадать с protocol/schema.md и Go-пакетом `internal/proto`.

pub mod damage;
mod abilities;
pub mod enemy;
pub mod loot;
pub mod rescue;
pub mod world;

#[cfg(test)]
mod brain_tests;
#[cfg(test)]
mod coop_tests;
#[cfg(test)]
mod party_tests;
#[cfg(test)]
mod rescue_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod threat_tests;
#[cfg(test)]
mod tests_support;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::GameConfig;
use crate::math::{Vec2, Vec3};
use crate::rng::Rng;

use enemy::Enemy;
use world::World;

/// Идентификаторы врагов начинаются отсюда: peer-ы игроков живут ниже, и по
/// одному числу всегда понятно, о ком речь.
const ENEMY_ID_BASE: u16 = 1000;

/// Насколько больше врагов на каждого игрока сверх первого.
const PARTY_COUNT_STEP: f32 = 0.6;
/// Насколько крепче каждый враг на каждого игрока сверх первого.
const PARTY_HP_STEP: f32 = 0.35;

/// Множители сложности для пати из `party` человек.
///
/// Урон врагов **не** масштабируем: вчетвером и так больше поводов получить по
/// голове, а растущий урон превращает кооператив в лотерею.
fn party_scaling(party: i32) -> (f32, f32) {
    let extra = (party.max(1) - 1) as f32;
    (1.0 + PARTY_COUNT_STEP * extra, 1.0 + PARTY_HP_STEP * extra)
}

/// Предел правдоподобной скорости игрока, м/с: база ~5, спринт ×1.42, запас на склоны.
pub const MAX_SPEED: f32 = 12.0;
/// Допуск на сетевой джиттер при проверке скорости.
pub const SPEED_TOLERANCE: f32 = 1.25;

/// Типы сущностей в снапшоте (`Ent::kind`).
pub mod kind {
    pub const PLAYER: u8 = 0;
    pub const ENEMY: u8 = 1;
    pub const PROJECTILE: u8 = 2;
    pub const ITEM: u8 = 3;
    pub const NPC: u8 = 4;
}

/// Биты поля `Input::buttons`. Общие для клиента, сервера и protocol/schema.md —
/// менять только вместе с версией протокола.
pub mod buttons {
    pub const JUMP: u16 = 1 << 0;
    pub const SPRINT: u16 = 1 << 1;
    pub const FIRE: u16 = 1 << 2;
    pub const ALT_FIRE: u16 = 1 << 3;
    pub const RELOAD: u16 = 1 << 4;
    pub const USE: u16 = 1 << 5;
    pub const CROUCH: u16 = 1 << 6;
}

/// Виды событий (`Event::kind`) — зеркало protocol/schema.md.
pub mod event {
    pub const DAMAGE: &str = "damage";
    pub const ENEMY_DIED: &str = "enemy_died";
    pub const XP: &str = "xp";
    pub const LOOT: &str = "loot";
    pub const PICKUP: &str = "pickup";
    pub const DOWNED: &str = "downed";
    pub const REVIVED: &str = "revived";
    /// Игрок истёк кровью и выбыл до конца забега.
    pub const PLAYER_OUT: &str = "player_out";
    /// Легли все — забег провален.
    pub const WIPE: &str = "wipe";
    /// Служебное уведомление: сменился состав пати, изменилась сложность.
    pub const NOTICE: &str = "notice";
    pub const DESPAWN: &str = "despawn";
    pub const WORLD_LOAD: &str = "world_load";
}

/// Биты `Ent::flags`.
pub mod flags {
    pub const DOWNED: u8 = 1 << 0;
    pub const STUNNED: u8 = 1 << 1;
    pub const BURNING: u8 = 1 << 2;
    pub const ELITE: u8 = 1 << 3;
    pub const FIRING: u8 = 1 << 4;
    pub const HIDDEN: u8 = 1 << 5;
}

/// Параметры комнаты, приходят от хоста при `sim_new`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub preset: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub seed: u64,
    #[serde(default)]
    pub depth: i32,
    #[serde(default)]
    pub party_size: i32,
    /// Каталог пресета на стороне хоста (`../game/presets/core`, `res://presets/core`).
    /// Пусто — играем на встроенных копиях данных.
    #[serde(default)]
    pub preset_base: String,
}

/// Ввод игрока за тик.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Input {
    #[serde(default)]
    pub tick: u32,
    #[serde(rename = "move", default)]
    pub move_dir: Vec2,
    #[serde(default)]
    pub yaw: f32,
    #[serde(default)]
    pub pitch: f32,
    #[serde(default)]
    pub buttons: u16,
    #[serde(default)]
    pub pos: Vec3,
    #[serde(default)]
    pub vel: Vec3,
    #[serde(default)]
    pub on_floor: bool,
}

/// Ввод с пометкой, чей он (батч за тик).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct TaggedInput {
    pub peer: u16,
    #[serde(rename = "in")]
    pub input: Input,
}

/// Запись сущности в снапшоте.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Ent {
    pub id: u16,
    pub kind: u8,
    /// Индекс вида в `enemies.json` пресета (+1; 0 — не задан). Пресет у клиента
    /// и сервера один (сверяется `content_hash`), поэтому индекса достаточно.
    #[serde(default, rename = "t", skip_serializing_if = "is_zero_u16")]
    pub type_id: u16,
    /// Кому принадлежит сущность (лут). 0 — общая для всех.
    #[serde(default, skip_serializing_if = "is_zero_u16")]
    pub owner: u16,
    pub pos: Vec3,
    pub yaw: f32,
    pub hp: u8,
    pub flags: u8,
}

/// Состояние мира, уходящее клиентам.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Snapshot {
    pub tick: u32,
    #[serde(default)]
    pub ack: u32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub full: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub players: Vec<Ent>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enemies: Vec<Ent>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub projectiles: Vec<Ent>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<Ent>,
}

/// Событие мира (урон, смерть, лут, вход/выход…).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Event {
    pub kind: String,
    #[serde(default, skip_serializing_if = "is_zero_u16")]
    pub actor: u16,
    #[serde(default, skip_serializing_if = "is_zero_u16")]
    pub target: u16,
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub amount: f32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<Vec3>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<Value>,
}

impl Event {
    pub fn new(kind: &str) -> Self {
        Self { kind: kind.to_string(), ..Default::default() }
    }
}

/// Выстрел клиента: нужен для звука и вспышки у остальных.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Fire {
    #[serde(default)]
    pub tick: u32,
    #[serde(default)]
    pub weapon: String,
    #[serde(default)]
    pub origin: Vec3,
    #[serde(default)]
    pub dir: Vec3,
    #[serde(default)]
    pub secondary: bool,
}

/// Заявка клиента о попадании. Сервер её проверяет (см. [`damage::apply_hit`]).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HitClaim {
    #[serde(default)]
    pub tick: u32,
    #[serde(default)]
    pub weapon: String,
    #[serde(default)]
    pub target: u16,
    #[serde(default)]
    pub pos: Vec3,
    #[serde(default)]
    pub part: u8,
}

/// Команда от хоста: вход/выход игрока, выстрел, попадание, действие.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cmd {
    pub kind: String,
    #[serde(default)]
    pub peer: u16,
    #[serde(default)]
    pub data: Option<Value>,
}

/// Результат тика.
#[derive(Clone, Debug, Default, Serialize)]
pub struct TickResult {
    pub snapshot: Snapshot,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<Event>,
}

/// Игрок глазами сервера.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    #[serde(default)]
    pub class: Option<usize>,
    #[serde(default)]
    pub abilities: crate::combat::ability::AbilityState,
    pub peer: u16,
    pub nickname: String,
    pub pos: Vec3,
    pub yaw: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub flags: u8,
    pub ack: u32,
    /// Сколько раз клиент заявлял невозможное перемещение или выстрел.
    pub violations: u32,
    /// Время последнего засчитанного выстрела — по нему проверяется темп стрельбы.
    #[serde(default)]
    pub last_fire: f32,
    /// Кнопки последнего ввода: по ним видно, тянется ли игрок поднимать товарища.
    #[serde(default)]
    pub buttons: u16,
    /// Сколько секунд лежит.
    #[serde(default)]
    pub downed_time: f32,
    /// Накопленный прогресс подъёма.
    #[serde(default)]
    pub revive_progress: f32,
    /// Выбыл до конца забега: истёк кровью, поднимать больше нельзя.
    #[serde(default)]
    pub out: bool,
}

impl Player {
    fn new(peer: u16, nickname: String) -> Self {
        Self {
            peer,
            nickname,
            class: None,
            abilities: Default::default(),
            pos: Vec3::ZERO,
            yaw: 0.0,
            hp: 100.0,
            max_hp: 100.0,
            flags: 0,
            ack: 0,
            violations: 0,
            last_fire: -100.0,
            buttons: 0,
            downed_time: 0.0,
            revive_progress: 0.0,
            out: false,
        }
    }

    /// Игрок выведен из строя: не цель для врагов и не может стрелять.
    pub fn downed(&self) -> bool {
        self.hp <= 0.0
    }

    fn hp_percent(&self) -> u8 {
        if self.max_hp <= 0.0 {
            return 0;
        }
        ((self.hp / self.max_hp) * 100.0).clamp(0.0, 100.0) as u8
    }
}

/// Состояние комнаты.
#[derive(Clone, Serialize, Deserialize)]
pub struct State {
    pub cfg: Config,
    pub tick: u32,
    /// Модельное время комнаты в секундах: по нему считаются кулдауны.
    pub time: f32,
    pub rng: Rng,
    pub players: HashMap<u16, Player>,
    pub enemies: Vec<Enemy>,
    /// Лут на земле: у каждого предмета есть владелец.
    pub items: Vec<loot::WorldItem>,
    next_item_id: u16,
    pub world: World,
    /// Забег уже объявлен проваленным — не спамим событием каждый тик.
    #[serde(default)]
    pub wiped: bool,
    next_enemy_id: u16,
    /// Данные пресета: общие для всех копий состояния, поэтому за `Arc`.
    /// Не сериализуются — при загрузке читаются заново.
    #[serde(skip)]
    game: Option<std::sync::Arc<GameConfig>>,
}

impl State {
    pub fn new(cfg: Config) -> Self {
        let rng = Rng::new(cfg.seed);
        let game = std::sync::Arc::new(load_preset(&cfg.preset_base));
        let world = match cfg.kind.as_str() {
            world::KIND_DELVE => World::delve(cfg.depth.max(1) as u32, cfg.seed, &game),
            _ => World::hub(),
        };
        let mut state = Self {
            cfg,
            tick: 0,
            time: 0.0,
            rng,
            players: HashMap::new(),
            enemies: Vec::new(),
            items: Vec::new(),
            next_item_id: 1,
            world,
            wiped: false,
            next_enemy_id: ENEMY_ID_BASE,
            game: Some(game),
        };
        state.spawn_world_enemies();
        state
    }

    /// Данные пресета; после загрузки состояния читаются заново.
    fn game(&mut self) -> std::sync::Arc<GameConfig> {
        if self.game.is_none() {
            self.game = Some(std::sync::Arc::new(load_preset(&self.cfg.preset_base)));
        }
        self.game.clone().expect("конфиг пресета загружен")
    }

    /// Заселить комнату врагами из плана мира.
    fn spawn_world_enemies(&mut self) {
        let spawns = std::mem::take(&mut self.world.spawns);
        let offset = self.world.offset;
        let mut created = Vec::new();
        let (count_mult, hp_mult) = party_scaling(self.cfg.party_size);
        let mut extra_budget = 0.0f32;
        let game = self.game();
        for spawn in &spawns.enemies {
            let Some(index) = game.enemies.iter().position(|e| e.id == spawn.kind) else {
                crate::warn!("[sim] враг '{}' не найден в enemies.json — пропускаю", spawn.kind);
                continue;
            };
            let Some(cfg) = game.enemies.get(index) else {
                crate::warn!("[sim] враг '{}' не найден в enemies.json — пропускаю", spawn.kind);
                continue;
            };
            let id = self.next_enemy_id;
            self.next_enemy_id = self.next_enemy_id.wrapping_add(1).max(ENEMY_ID_BASE);
            created.push(Enemy::from_cfg(
                id,
                index as u16 + 1,
                cfg,
                spawn.pos + offset,
                spawn.mult,
                hp_mult,
                spawn.is_boss,
                spawn.affixes.clone(),
            ));

            // Лишние враги на большую пати. Копим дробный остаток и ставим
            // копию, как только он дорос до целого: так добавка размазывается
            // по всему данжу, а не сваливается в первую комнату.
            extra_budget += count_mult - 1.0;
            let mut extra_index = 0;
            while extra_budget >= 1.0 {
                extra_budget -= 1.0;
                extra_index += 1;
                let id = self.next_enemy_id;
                self.next_enemy_id = self.next_enemy_id.wrapping_add(1).max(ENEMY_ID_BASE);
                let angle = extra_index as f32 * 2.1;
                let shifted = spawn.pos
                    + offset
                    + crate::math::Vec3::new(angle.cos() * 1.5, 0.0, angle.sin() * 1.5);
                created.push(Enemy::from_cfg(
                    id,
                    index as u16 + 1,
                    cfg,
                    shifted,
                    spawn.mult,
                    hp_mult,
                    false,
                    Vec::new(),
                ));
            }
        }
        self.enemies = created;
        self.world.spawns = spawns;
    }

    /// Игрок вошёл в комнату.
    pub fn join(&mut self, peer: u16, nickname: String) -> Vec<Event> {
        let mut player = Player::new(peer, nickname);
        player.pos = self.world.player_spawn;
        self.players.insert(peer, player);

        let mut event = Event::new(event::WORLD_LOAD);
        event.actor = peer;
        event.text = self.world.kind.clone();
        event.pos = Some(self.world.player_spawn);
        vec![event]
    }

    /// Игрок вышел.
    pub fn leave(&mut self, peer: u16) -> Vec<Event> {
        self.players.remove(&peer);
        Vec::new()
    }

    /// Приём ввода: сервер не пересчитывает шаг, а проверяет, что игрок не
    /// переместился дальше, чем мог (docs/MULTIPLAYER.md §5).
    pub fn apply_input(&mut self, peer: u16, input: Input, dt: f32) {
        let Some(player) = self.players.get_mut(&peer) else {
            return;
        };
        player.ack = input.tick;
        player.yaw = input.yaw;
        player.buttons = input.buttons;
        if player.downed() {
            // Лежачий не бегает: позицию от него больше не принимаем.
            return;
        }

        let limit = MAX_SPEED * player.abilities.speed_scale() * SPEED_TOLERANCE * dt.max(0.001);
        if player.pos != Vec3::ZERO && player.pos.distance(input.pos) > limit {
            player.violations += 1;
            return; // позицию не принимаем, клиент откатится по снапшоту
        }
        player.pos = input.pos;
    }

    /// Команда, не связанная с движением. Пока обрабатываются только вход и выход.
    pub fn command(&mut self, cmd: &Cmd) -> Vec<Event> {
        match cmd.kind.as_str() {
            "join" => {
                let nickname = cmd
                    .data
                    .as_ref()
                    .and_then(|d| d.get("nickname"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("player")
                    .to_string();
                self.join(cmd.peer, nickname)
            }
            "leave" => self.leave(cmd.peer),
            // Игровая команда клиента: хост заворачивает её целиком, поэтому
            // разбираем вложенный вид (`enter_delve`, `party`, `craft`…).
            "cmd" => {
                let inner = cmd.data.as_ref();
                let kind = inner
                    .and_then(|data| data.get("kind"))
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string();
                let args = inner.and_then(|data| data.get("args")).cloned();
                self.game_command(cmd.peer, &kind, args.as_ref())
            }
            "party" => {
                let size = cmd
                    .data
                    .as_ref()
                    .and_then(|data| data.get("size"))
                    .and_then(|value| value.as_i64())
                    .unwrap_or(1) as i32;
                self.set_party(size)
            }
            "hit" => match cmd
                .data
                .clone()
                .and_then(|d| serde_json::from_value::<HitClaim>(d).ok())
            {
                Some(claim) => damage::apply_hit(self, cmd.peer, &claim),
                None => Vec::new(),
            },
            // Выстрел сам по себе ничего не меняет: урон приходит заявкой о попадании.
            // Событие нужно клиентам, чтобы проиграть звук и вспышку чужого оружия.
            "fire" => Vec::new(),
            _ => Vec::new(),
        }
    }

    /// Шаг симуляции.
    pub fn tick(&mut self, dt: f32, inputs: &[TaggedInput]) -> TickResult {
        for tagged in inputs {
            self.apply_input(tagged.peer, tagged.input, dt);
        }
        self.tick = self.tick.wrapping_add(1);
        self.time += dt;
        for player in self.players.values_mut() { player.abilities.tick(dt); }

        let mut events = self.tick_enemies(dt);
        let hits = self.apply_enemy_damage(&events);
        events.extend(hits);
        let rescued = rescue::tick(self, dt);
        events.extend(rescued);
        let picked = self.tick_loot(dt);
        events.extend(picked);

        let mut players: Vec<Ent> = self
            .players
            .values()
            .map(|p| Ent {
                id: p.peer,
                kind: kind::PLAYER,
                type_id: 0,
                owner: 0,
                pos: p.pos,
                yaw: p.yaw,
                hp: p.hp_percent(),
                flags: p.flags,
            })
            .collect();
        // Порядок обхода HashMap не определён — сортируем, чтобы снапшоты
        // были воспроизводимы и дельта-кодирование не шумело.
        players.sort_by_key(|e| e.id);

        let enemies: Vec<Ent> = self
            .enemies
            .iter()
            .filter(|e| e.alive())
            .map(|e| Ent {
                id: e.id,
                kind: kind::ENEMY,
                type_id: e.type_id,
                owner: 0,
                pos: e.pos,
                yaw: e.yaw,
                hp: e.hp_percent(),
                flags: e.flags(),
            })
            .collect();

        // Мёртвые враги живут ещё тик — чтобы событие смерти ушло раньше, чем
        // сущность исчезнет из снапшота, и клиент успел проиграть анимацию.
        self.enemies.retain(|e| e.alive());

        let items: Vec<Ent> = self
            .items
            .iter()
            .map(|item| Ent {
                id: item.id,
                kind: kind::ITEM,
                type_id: item.type_id(),
                owner: item.owner,
                pos: item.pos,
                yaw: 0.0,
                hp: 100,
                flags: 0,
            })
            .collect();

        TickResult {
            snapshot: Snapshot {
                tick: self.tick,
                full: true,
                players,
                enemies,
                items,
                ..Default::default()
            },
            events,
        }
    }

    /// Лут: истечение срока и подбор владельцем.
    fn tick_loot(&mut self, dt: f32) -> Vec<Event> {
        if self.items.is_empty() {
            return Vec::new();
        }
        let mut events = Vec::new();
        let positions: Vec<(u16, crate::math::Vec3)> = self
            .players
            .values()
            .filter(|player| !player.downed() && !player.out)
            .map(|player| (player.peer, player.pos))
            .collect();

        let mut taken: Vec<u16> = Vec::new();
        for item in &mut self.items {
            item.ttl -= dt;
            if item.ttl <= 0.0 {
                taken.push(item.id);
                let mut gone = Event::new(event::DESPAWN);
                gone.target = item.id;
                events.push(gone);
                continue;
            }
            for (peer, pos) in &positions {
                if loot::can_pick_up(item, *peer, *pos) {
                    taken.push(item.id);
                    let mut picked = Event::new(event::PICKUP);
                    picked.actor = *peer;
                    picked.target = item.id;
                    picked.text = item.label();
                    picked.pos = Some(item.pos);
                    events.push(picked);
                    break;
                }
            }
        }
        self.items.retain(|item| !taken.contains(&item.id));
        events
    }

    /// Разложить дроп с убитого врага — по броску на каждого живого игрока.
    fn drop_loot(&mut self, at: crate::math::Vec3) -> Vec<Event> {
        // Порядок обхода HashMap не определён, а от него зависит, кому достанутся
        // первые броски. Сортируем, чтобы раздача была воспроизводимой.
        let mut owners: Vec<u16> = self
            .players
            .values()
            .filter(|player| !player.out)
            .map(|player| player.peer)
            .collect();
        owners.sort_unstable();
        if owners.is_empty() {
            return Vec::new();
        }
        let game = self.game();
        let mut rng = self.rng;
        let mut next_id = self.next_item_id;
        let dropped = loot::roll_kill_drops(&game, &mut rng, at, &owners, &mut next_id);
        self.rng = rng;
        self.next_item_id = next_id;

        let events = dropped
            .iter()
            .map(|item| {
                let mut event = Event::new(event::LOOT);
                event.actor = item.owner;
                event.target = item.id;
                event.text = item.label();
                event.pos = Some(item.pos);
                event
            })
            .collect();
        self.items.extend(dropped);
        events
    }

    /// Ход врагов: аггро, движение, атаки.
    fn tick_enemies(&mut self, dt: f32) -> Vec<Event> {
        if self.enemies.is_empty() {
            return Vec::new();
        }
        let mut events = Vec::new();
        // Врагов двигаем по копии списка игроков: заимствовать `self` целиком
        // внутри цикла нельзя, а игроков в этот момент никто не меняет.
        let players: Vec<Player> = self.players.values().cloned().collect();
        let refs: Vec<&Player> = players.iter().collect();
        let game = self.game();
        let world = &self.world;
        for enemy in &mut self.enemies {
            let brain = enemy
                .brain
                .as_ref()
                .and_then(|id| game.brains.iter().find(|brain| brain.id == *id));
            enemy.tick(dt, world, &refs, brain, &mut events);
        }
        events
    }

    /// Тестовый доступ к применению вражеского урона.
    #[cfg(test)]
    pub(crate) fn apply_enemy_damage_for_test(&mut self, events: &[Event]) -> Vec<Event> {
        self.apply_enemy_damage(events)
    }

    /// Применить урон, который враги нанесли за этот тик: события об атаках
    /// они уже сложили, здесь эти удары превращаются в потерянное здоровье.
    fn apply_enemy_damage(&mut self, events: &[Event]) -> Vec<Event> {
        let mut extra = Vec::new();
        let damage: Vec<(u16, f32)> = events
            .iter()
            .filter(|e| e.kind == event::DAMAGE && e.actor >= ENEMY_ID_BASE)
            .map(|e| (e.target, e.amount))
            .collect();
        for (peer, amount) in damage {
            let Some(player) = self.players.get_mut(&peer) else {
                continue;
            };
            if player.downed() {
                continue;
            }
            player.hp = (player.hp - amount * player.abilities.damage_scale()).max(0.0);
            if player.downed() {
                rescue::knock_down(player);
                let mut event = Event::new(event::DOWNED);
                event.target = peer;
                event.pos = Some(player.pos);
                extra.push(event);
            }
        }
        extra
    }

    /// Игровые команды, пришедшие от клиента через хост.
    ///
    /// Пока их немного: состав пати сообщает сама комната, остальное
    /// (крафт, постройки) появится вместе с инвентарём на сервере.
    fn game_command(&mut self, peer: u16, kind: &str, args: Option<&Value>) -> Vec<Event> {
        match kind {
            "choose_class" => {
                if let Some(class) = args.and_then(|v| v.get("class")).and_then(Value::as_u64).filter(|c| *c < 3) {
                    if let Some(player) = self.players.get_mut(&peer) {
                        if player.class.is_none() { player.class = Some(class as usize); }
                    }
                }
                Vec::new()
            }
            "ability" => self.cast_ability(peer, args),
            "party" => {
                let size = args
                    .and_then(|value| value.get("size"))
                    .and_then(|value| value.as_i64())
                    .unwrap_or(1) as i32;
                self.set_party(size)
            }
            _ => Vec::new(),
        }
    }

    /// Пересчитать сложность под новый состав пати.
    ///
    /// Живым врагам поднимается запас здоровья, а на свободных точках спавна
    /// появляется добавка. Уменьшение состава здоровье не срезает: отнимать
    /// у врага здоровье посреди боя — это подарок, который игрок не заметит,
    /// зато заметит рассинхронизацию полоски.
    pub fn set_party(&mut self, party: i32) -> Vec<Event> {
        let party = party.max(1);
        let previous = self.cfg.party_size.max(1);
        if party == previous {
            return Vec::new();
        }
        self.cfg.party_size = party;

        let (old_count, old_hp) = party_scaling(previous);
        let (new_count, new_hp) = party_scaling(party);

        if party > previous {
            let ratio = new_hp / old_hp.max(0.01);
            for enemy in self.enemies.iter_mut().filter(|enemy| enemy.alive()) {
                enemy.max_hp *= ratio;
                enemy.hp *= ratio;
            }
            self.reinforce((new_count - old_count).max(0.0));
        }

        let mut event = Event::new(event::NOTICE);
        event.amount = party as f32;
        event.text = "party".to_string();
        vec![event]
    }

    /// Досыпать врагов на точки спавна подальше от игроков.
    ///
    /// Появляться в лицо тому, кто только что зашёл, — плохой способ
    /// поздороваться, поэтому близкие точки пропускаем.
    fn reinforce(&mut self, per_point: f32) {
        if per_point <= 0.0 || self.world.spawns.enemies.is_empty() {
            return;
        }
        const KEEP_AWAY: f32 = 25.0;

        let players: Vec<crate::math::Vec3> =
            self.players.values().map(|player| player.pos).collect();
        let spawns = std::mem::take(&mut self.world.spawns);
        let offset = self.world.offset;
        let game = self.game();

        let mut budget = 0.0f32;
        let mut created = Vec::new();
        for spawn in &spawns.enemies {
            let at = spawn.pos + offset;
            if players
                .iter()
                .any(|player| (*player - at).length() < KEEP_AWAY)
            {
                continue;
            }
            budget += per_point;
            while budget >= 1.0 {
                budget -= 1.0;
                let Some(index) = game.enemies.iter().position(|e| e.id == spawn.kind) else {
                    continue;
                };
                let Some(cfg) = game.enemies.get(index) else {
                    continue;
                };
                let id = self.next_enemy_id;
                self.next_enemy_id = self.next_enemy_id.wrapping_add(1).max(ENEMY_ID_BASE);
                let (_, hp_mult) = party_scaling(self.cfg.party_size);
                created.push(Enemy::from_cfg(
                    id,
                    index as u16 + 1,
                    cfg,
                    at,
                    spawn.mult,
                    hp_mult,
                    false,
                    Vec::new(),
                ));
            }
        }
        self.world.spawns = spawns;
        self.enemies.extend(created);
    }

    /// Последний учтённый `Input::tick` игрока.
    pub fn ack_of(&self, peer: u16) -> u32 {
        self.players.get(&peer).map(|p| p.ack).unwrap_or(0)
    }
}

/// Прочитать данные пресета. Пустой путь — встроенные копии core.
fn load_preset(base: &str) -> GameConfig {
    GameConfig::load_from(base)
}

fn is_false(v: &bool) -> bool {
    !*v
}

fn is_zero_u16(v: &u16) -> bool {
    *v == 0
}

fn is_zero_f32(v: &f32) -> bool {
    *v == 0.0
}
