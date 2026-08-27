//! Сетевой режим Game3D: подключение к серверу и жизнь по его снапшотам.
//!
//! В сетевой игре мир авторитетно считает сервер: он двигает врагов, решает урон
//! и подтверждает наши позиции. Клиент шлёт ввод, рисует то, что пришло, и просит
//! засчитать попадания (docs/MULTIPLAYER.md §5).
//!
//! Подключение — аргументом запуска: `godot --path game -- --server=ws://host:7777/ws`.

use godot::classes::{CharacterBody3D, Os};
use godot::prelude::*;

use openheart_core::protocol::World;
use openheart_core::sim::{Ent, Event as SimEvent, Fire, HitClaim, Input as SimInput};

use crate::convert;
use crate::enemy::Enemy;
use crate::net::{NetClient, NetEvent};
use crate::player::Player;

use super::{Game3D, Loc, PeerId};

/// Насколько быстро сетевые сущности подтягиваются к позиции из снапшота.
/// Снапшоты приходят 20 раз в секунду, между ними положение интерполируется.
const INTERP_SPEED: f32 = 12.0;

/// Расхождение, ниже которого коррекция не нужна: сеть всегда чуть врёт.
const DRIFT_IGNORE: f32 = 0.35;
/// Расхождение, при котором мягко подтягивать уже поздно — телепорт
/// (портал, респавн, серьёзная рассинхронизация).
const DRIFT_TELEPORT: f32 = 5.0;
/// Скорость мягкой коррекции, м/с: заметно исправляет, но не выдёргивает из-под ног.
const CORRECTION_SPEED: f32 = 6.0;

/// Что клиент знает о сетевой сущности между снапшотами.
pub(super) struct NetEntity {
    pub node: Gd<CharacterBody3D>,
    pub target: Vector3,
    pub yaw: f32,
    pub hp: u8,
}

impl Game3D {
    /// Адрес сервера из аргументов запуска (`-- --server=ws://…`) или из окружения.
    pub(super) fn server_from_cmdline() -> Option<String> {
        let args = Os::singleton().get_cmdline_user_args();
        for index in 0..args.len() {
            let arg = args.get(index)?.to_string();
            if let Some(url) = arg.strip_prefix("--server=") {
                return Some(url.to_string());
            }
        }
        match std::env::var("OH_SERVER") {
            Ok(url) if !url.is_empty() => Some(url),
            _ => None,
        }
    }

    /// Подключиться к серверу. Одиночная игра при этом не ломается: пока сервер
    /// не прислал `welcome`, всё работает как раньше.
    pub(super) fn net_connect(&mut self, url: &str) {
        // Имени персонажа в состоянии нет — берём имя пользователя системы,
        // а на публичных серверах ник придёт из аккаунта (этап N5).
        let nickname = match std::env::var("OH_NICK") {
            Ok(nick) if !nick.is_empty() => nick,
            _ => "player".to_string(),
        };
        // Отпечаток данных: сервер не пустит с другим набором JSON.
        let hash = crate::content::preset_content_hash(&self.preset);

        match NetClient::connect(url, &nickname, &self.preset, &hash) {
            Some(client) => {
                godot_print!("[net] подключаюсь к {url} как {nickname}");
                self.net = Some(client);
            }
            None => godot_warn!("[net] не удалось начать подключение к {url}"),
        }
    }

    /// Подключены ли мы к серверу и он уже пустил нас в комнату.
    pub(super) fn net_ready(&self) -> bool {
        self.net.as_ref().map(|net| net.is_ready()).unwrap_or(false)
    }

    /// Шаг сетевого режима: разобрать входящее, отправить ввод.
    pub(super) fn tick_net(&mut self, dt: f32) {
        let Some(mut net) = self.net.take() else {
            return;
        };

        for event in net.poll(dt) {
            match event {
                NetEvent::Welcome { peer, world, players } => {
                    self.on_welcome(peer, &world, players.len());
                }
                NetEvent::Snapshot(snapshot) => {
                    self.apply_snapshot(&snapshot.players, &snapshot.enemies);
                    self.apply_loot_snapshot(&snapshot.items);
                }
                NetEvent::Game(event) => self.on_net_event(&event),
                NetEvent::Rejected(reject) => {
                    godot_warn!("[net] сервер отказал: {} ({})", reject.reason, reject.detail);
                }
                NetEvent::Pong(_) => {}
            }
        }

        if net.is_ready() && net.input_due() {
            match self.local_input_packet() {
                Some(input) => net.send_input(input),
                // Игрока ещё нет: выбор класса, диалог, экран смерти. Молчать
                // нельзя — сервер отключит по таймауту чтения.
                None => net.send_keepalive(),
            }
        }

        self.interpolate_net_entities(dt);
        self.apply_correction(dt);
        self.net = Some(net);
    }

    fn on_welcome(&mut self, peer: PeerId, world: &World, players: usize) {
        self.local_peer = peer;
        godot_print!(
            "[net] в комнате: peer={peer} мир={} сид={} игроков={}",
            world.kind,
            world.seed,
            players
        );
        self.follow_server_world(world);
        // Врагов и лут теперь раздаёт сервер — локальные чистим, чтобы не двоились.
        self.clear_local_enemies();
    }

    /// Перестроить мир под тот, что назвал сервер.
    ///
    /// Welcome приходит и при входе, и при переходе между комнатами. Без этого
    /// шага можно оказаться в хабе, пока сервер считает забег: игрок ходит по
    /// своей геометрии, а сервер поправляет его позицию телепортом.
    fn follow_server_world(&mut self, world: &World) {
        const KIND_DELVE: &str = "delve";
        if world.kind == KIND_DELVE {
            let depth = world.depth.max(1) as u32;
            // Тот же забег заново не строим: снапшоты и так идут потоком.
            let same_run = self.loc == Loc::Dungeon
                && self.dungeon_depth == depth
                && self.state.as_ref().is_some_and(|state| state.dungeon_seed == world.seed);
            if same_run {
                return;
            }
            self.enter_dungeon_seeded(depth, world.seed);
        } else if self.loc == Loc::Dungeon {
            // Сервер держит нас в хабе: выходим без сохранения — прогресс
            // забега всё равно живёт на сервере.
            self.exit_dungeon_impl(false);
        }
    }

    fn on_net_event(&mut self, event: &SimEvent) {
        use openheart_core::sim::event;
        match event.kind.as_str() {
            event::DAMAGE if event.target == self.local_peer => {
                self.damage_player(event.amount);
            }
            event::ENEMY_DIED => {
                self.remove_net_enemy(event.target);
            }
            event::DOWNED if event.target == self.local_peer => {
                // Лежачий не бегает и не стреляет — движением займётся ревайв.
                self.freeze_player(true);
                let lang = self.settings.lang.clone();
                self.show_flash(crate::locale::t("net_downed", &lang));
            }
            event::REVIVED if event.target == self.local_peer => {
                self.freeze_player(false);
                let lang = self.settings.lang.clone();
                self.show_flash(crate::locale::t("net_revived", &lang));
            }
            event::PLAYER_OUT if event.target == self.local_peer => {
                let lang = self.settings.lang.clone();
                self.show_flash(crate::locale::t("net_out", &lang));
            }
            event::LOOT if event.actor == self.local_peer => {
                // Запоминаем, что именно выпало: снапшот несёт только id.
                self.net_item_kinds.insert(event.target, event.text.clone());
            }
            event::PICKUP if event.actor == self.local_peer => {
                self.take_net_item(event.target);
                let lang = self.settings.lang.clone();
                self.show_flash(&format!("{} {}", crate::locale::t("loot_taken", &lang), event.text));
            }
            event::DESPAWN => {
                self.take_net_item(event.target);
            }
            event::WIPE => {
                let lang = self.settings.lang.clone();
                self.show_flash(crate::locale::t("net_wipe", &lang));
            }
            event::XP => {
                if let Some(state) = self.state.as_mut() {
                    state.xp += event.amount as u32;
                }
            }
            _ => {}
        }
    }

    /// Собрать пакет ввода локального игрока.
    fn local_input_packet(&self) -> Option<SimInput> {
        let player = self.player()?;
        let position = player.get_global_position();
        let velocity = player.get_velocity();
        let on_floor = player.is_on_floor();
        let node = player.try_cast::<Player>().ok()?;
        let bound = node.bind();
        Some(
            bound
                .current_input()
                .to_core(0, position, velocity, on_floor),
        )
    }

    /// Заявить серверу о попадании: урон считает он.
    pub(super) fn net_report_hit(&mut self, enemy: &Gd<Enemy>, weapon: &str, at: Vector3) {
        let Some(net) = self.net.as_mut() else {
            return;
        };
        let target = enemy.bind().entity_id() as u16;
        net.send_hit(HitClaim {
            tick: 0,
            weapon: weapon.to_string(),
            target,
            pos: convert::to_core(at),
            part: 0,
        });
    }

    /// Сообщить о выстреле — остальным клиентам нужны звук и вспышка.
    pub(super) fn net_report_fire(&mut self, weapon: &str, origin: Vector3, dir: Vector3) {
        let Some(net) = self.net.as_mut() else {
            return;
        };
        net.send_fire(Fire {
            tick: 0,
            weapon: weapon.to_string(),
            origin: convert::to_core(origin),
            dir: convert::to_core(dir),
            secondary: false,
        });
    }

    // ── сущности из снапшота ─────────────────────────────────────────────────

    fn apply_snapshot(&mut self, players: &[Ent], enemies: &[Ent]) {
        for ent in players {
            if ent.id == self.local_peer {
                self.reconcile_local_player(ent);
            } else {
                self.update_remote_player(ent);
            }
        }
        let seen: Vec<PeerId> = players.iter().map(|ent| ent.id).collect();
        self.remote_players
            .retain(|peer, entity| {
                let keep = seen.contains(peer);
                if !keep {
                    entity.node.clone().queue_free();
                }
                keep
            });

        for ent in enemies {
            self.update_net_enemy(ent);
        }
        let alive: Vec<u16> = enemies.iter().map(|ent| ent.id).collect();
        self.net_enemies.retain(|id, entity| {
            let keep = alive.contains(id);
            if !keep {
                entity.node.clone().queue_free();
            }
            keep
        });
    }

    /// Сверка предсказания: сервер прислал нашу позицию.
    ///
    /// Полноценной перемотки ввода нет и не будет в этом виде: движение считает
    /// `move_and_slide` движка, переиграть его назад нельзя. Поэтому мелкое
    /// расхождение мы подтягиваем плавно, а крупное — телепортом.
    fn reconcile_local_player(&mut self, ent: &Ent) {
        let Some(mut player) = self.player() else {
            return;
        };
        let server_pos = convert::to_godot(ent.pos);
        let drift = player.get_global_position().distance_to(server_pos);

        if drift > DRIFT_TELEPORT {
            godot_warn!("[net] рассинхронизация {drift:.1} м — возвращаю к серверной позиции");
            if let Ok(mut node) = player.clone().try_cast::<Player>() {
                node.bind_mut().teleport(server_pos);
            } else {
                player.set_global_position(server_pos);
            }
            self.net_correction = None;
        } else if drift > DRIFT_IGNORE {
            self.net_correction = Some(server_pos);
        } else {
            self.net_correction = None;
        }
    }

    /// Плавно свести локальную позицию с серверной.
    fn apply_correction(&mut self, dt: f32) {
        let Some(target) = self.net_correction else {
            return;
        };
        let Some(mut player) = self.player() else {
            return;
        };
        let current = player.get_global_position();
        let offset = target - current;
        let distance = offset.length();
        if distance < 0.05 {
            self.net_correction = None;
            return;
        }
        let step = (CORRECTION_SPEED * dt).min(distance);
        player.set_global_position(current + offset.normalized() * step);
    }

    fn update_remote_player(&mut self, ent: &Ent) {
        let target = convert::to_godot(ent.pos);
        if let Some(entity) = self.remote_players.get_mut(&ent.id) {
            entity.target = target;
            entity.yaw = ent.yaw;
            entity.hp = ent.hp;
            return;
        }
        // Чужой игрок пока рисуется спрайтом-биллбордом: полноценная модель
        // с анимациями появится на этапе N2.
        let Some(node) = self.spawn_remote_avatar(target) else {
            return;
        };
        self.remote_players.insert(
            ent.id,
            NetEntity { node, target, yaw: ent.yaw, hp: ent.hp },
        );
    }

    /// Враг из снапшота: тот же узел `Enemy`, что и в одиночной игре, но с
    /// выключенным ИИ — двигает его сервер, клиент только рисует.
    fn update_net_enemy(&mut self, ent: &Ent) {
        let target = convert::to_godot(ent.pos);
        if let Some(entity) = self.net_enemies.get_mut(&ent.id) {
            entity.target = target;
            entity.yaw = ent.yaw;
            entity.hp = ent.hp;
            return;
        }
        let Some(node) = self.spawn_net_enemy(ent, target) else {
            return;
        };
        self.net_enemies.insert(
            ent.id,
            NetEntity { node, target, yaw: ent.yaw, hp: ent.hp },
        );
    }

    /// Создать узел врага по индексу вида из снапшота.
    fn spawn_net_enemy(&mut self, ent: &Ent, pos: Vector3) -> Option<Gd<CharacterBody3D>> {
        let cfg = self.cfg.take()?;
        let kind = ent
            .type_id
            .checked_sub(1)
            .and_then(|index| cfg.enemies.get(index as usize))
            .map(|enemy| enemy.id.clone());
        let in_dungeon = matches!(self.loc, super::Loc::Dungeon);
        if let Some(kind) = kind.as_deref() {
            self.spawn_enemy(&cfg, kind, pos, 1.0, false, in_dungeon, &[]);
        } else {
            godot_warn!("[net] неизвестный вид врага t={}", ent.type_id);
        }
        self.cfg = Some(cfg);

        // spawn_enemy кладёт врага в локальный список — забираем его оттуда:
        // этим врагом управляет сервер, локальный ИИ и урон ему не нужны.
        let mut enemy = self.enemies.pop()?;
        {
            let mut bound = enemy.bind_mut();
            bound.frozen = true;
            bound.set_entity_id(ent.id as u32);
        }
        enemy.set_global_position(pos);
        Some(enemy.upcast())
    }

    /// Простой аватар чужого игрока: тело с биллбордом.
    fn spawn_remote_avatar(&mut self, pos: Vector3) -> Option<Gd<CharacterBody3D>> {
        let mut body = CharacterBody3D::new_alloc();
        body.set_global_position(pos);
        let sprite = crate::gfx::make_billboard(
            &mut self.cache,
            "res://assets/sprites/characters/npc_vale.png",
            Vector3::new(0.0, 0.9, 0.0),
            0.03,
        )?;
        body.add_child(&sprite);
        self.base_mut().add_child(&body);
        Some(body)
    }

    /// Убрать локальных врагов: в сетевой игре их раздаёт сервер.
    fn clear_local_enemies(&mut self) {
        for enemy in self.enemies.drain(..) {
            enemy.clone().queue_free();
        }
    }

    /// Лут из снапшота: сервер шлёт нам только наш, но проверяем и здесь —
    /// чужие предметы рисовать нельзя ни при каких обстоятельствах.
    fn apply_loot_snapshot(&mut self, items: &[Ent]) {
        let local = self.local_peer;
        for ent in items.iter().filter(|ent| ent.owner == local) {
            let target = convert::to_godot(ent.pos);
            if let Some(entity) = self.net_items.get_mut(&ent.id) {
                entity.target = target;
                continue;
            }
            let kind = self
                .net_item_kinds
                .get(&ent.id)
                .cloned()
                .unwrap_or_else(|| "medkit".to_string());
            let texture = super::item_sprite_tex(&kind);
            if let Some(node) = self.spawn_pickup_avatar(texture, target) {
                self.net_items.insert(
                    ent.id,
                    NetEntity { node, target, yaw: 0.0, hp: 100 },
                );
            }
        }

        let alive: Vec<u16> = items
            .iter()
            .filter(|ent| ent.owner == local)
            .map(|ent| ent.id)
            .collect();
        self.net_items.retain(|id, entity| {
            let keep = alive.contains(id);
            if !keep {
                entity.node.clone().queue_free();
            }
            keep
        });
    }

    /// Убрать предмет: подобрали или истёк срок.
    fn take_net_item(&mut self, id: u16) {
        if let Some(entity) = self.net_items.remove(&id) {
            entity.node.clone().queue_free();
        }
        self.net_item_kinds.remove(&id);
    }

    /// Биллборд предмета на земле.
    fn spawn_pickup_avatar(&mut self, texture: &str, pos: Vector3) -> Option<Gd<CharacterBody3D>> {
        let mut body = CharacterBody3D::new_alloc();
        body.set_global_position(pos);
        let sprite = crate::gfx::make_billboard(
            &mut self.cache,
            texture,
            Vector3::new(0.0, 0.4, 0.0),
            0.02,
        )?;
        body.add_child(&sprite);
        self.base_mut().add_child(&body);
        Some(body)
    }

    fn remove_net_enemy(&mut self, id: u16) {
        if let Some(entity) = self.net_enemies.remove(&id) {
            entity.node.clone().queue_free();
        }
    }

    /// Плавно подтянуть сетевые сущности к последним известным позициям.
    fn interpolate_net_entities(&mut self, dt: f32) {
        let factor = (dt * INTERP_SPEED).min(1.0);
        for entity in self
            .remote_players
            .values_mut()
            .chain(self.net_enemies.values_mut())
            .chain(self.net_items.values_mut())
        {
            let current = entity.node.get_global_position();
            entity
                .node
                .set_global_position(current.lerp(entity.target, factor));
            entity
                .node
                .set_rotation(Vector3::new(0.0, entity.yaw, 0.0));
        }
    }
}
