use crate::{content::Content, network::Network, options::Options};
use anyhow::{anyhow, Result};
use openheart_core::{
    classes,
    game_state::GameState,
    math::{Vec2, Vec3},
    protocol::{self, Auth, Hello, ServerMessage},
    save::SaveData,
    sim::{self, Input, Snapshot},
    weapon::{self, Arsenal},
};
use serde_json::{json, Value};
use std::{
    collections::{HashSet, VecDeque},
    fs,
    path::PathBuf,
};

pub struct App {
    pub content: Content,
    pub state: GameState,
    pub arsenal: Arsenal,
    pub world: sim::world::World,
    pub snapshot: Snapshot,
    pub peer: u16,
    pub pos: Vec3,
    pub yaw: f32,
    pub hp: u8,
    pub downed: bool,
    pub ready: bool,
    pub target: Option<u16>,
    pub messages: VecDeque<String>,
    pub panel: String,
    pub scroll: u16,
    pub command_line: Option<String>,
    pub scene: Option<openheart_core::dialogue::Scene>,
    pub scene_line: usize,
    pub status: String,
    pub depth: u32,
    pub next_depth: Option<u32>,
    pub seed: u64,
    pub zoom: f32,
    pub quit: bool,
    pub snapshots_received: u64,
    pub pickups: Vec<crate::world::Pickup>,
    pub points: Vec<crate::world::Point>,
    pub collected: HashSet<String>,
    pub network: Option<Network>,
    pub local: Option<sim::State>,
    pub preset_base: String,
    pub save_path: PathBuf,
    local_cache: bool,
    tick: u32,
    cooldown: f32,
    reload: f32,
    reload_weapon: weapon::WeaponId,
    pub move_dir: Vec2,
    pub move_left: f32,
    pub use_left: f32,
    autosave: f32,
    loaded_character: bool,
}

impl App {
    pub fn new(options: &Options, url: Option<String>) -> Result<Self> {
        let content = Content::load(&options.preset_base())?;
        let mut state = GameState::new(&options.nick);
        state.preset = options.preset.clone();
        let mut arsenal = Arsenal::new();
        let class_idx = options.class as usize - 1;
        let class = &classes::classes()[class_idx];
        state.class_idx = Some(class_idx);
        for id in &class.start_weapons {
            arsenal.give_weapon(*id);
        }
        for (kind, count) in &class.start_ammo {
            arsenal.add_ammo(*kind, *count, 1.0);
        }
        if let Some(id) = class.start_weapons.first() {
            arsenal.current = *id;
        }
        fs::create_dir_all(&options.data_dir)?;
        let local_cache = url.is_none()
            || options.login.is_none() && std::env::var("OH_TICKET").unwrap_or_default().is_empty();
        let save_path = if let Some(url) = &url {
            let key = openheart_core::data::hash::preset_hash(&[(
                url.clone(),
                format!("{}:{}", options.preset, options.nick),
            )]);
            options.data_dir.join(format!("guest-{key}.json"))
        } else {
            options
                .data_dir
                .join(format!("solo-{}.json", options.preset))
        };
        let mut saved_hp = 100.0;
        if local_cache && save_path.exists() {
            let save = SaveData::from_json(&fs::read_to_string(&save_path)?)?;
            anyhow::ensure!(
                save.preset == options.preset,
                "Сохранение относится к другому пресету"
            );
            (state, saved_hp, arsenal) = save.into_game();
        }
        let preset_base = options.preset_base().to_string_lossy().into_owned();
        let network = if let Some(url) = &url {
            let ticket = std::env::var("OH_TICKET").unwrap_or_default();
            let auth = Auth {
                mode: if !ticket.is_empty() {
                    "master"
                } else if options.login.is_some() {
                    "local"
                } else {
                    "open"
                }
                .into(),
                nickname: options.nick.clone(),
                login: options.login.clone().unwrap_or_default(),
                password: std::env::var("OH_PASSWORD").unwrap_or_default(),
                ticket,
                register: options.register,
            };
            Some(Network::connect(
                url.clone(),
                Hello {
                    protocol: protocol::VERSION,
                    game_version: "0.1.0-dev".into(),
                    preset_id: options.preset.clone(),
                    content_hash: content.hash.clone(),
                    auth,
                    password: std::env::var("OH_SERVER_PASSWORD").unwrap_or_default(),
                },
            )?)
        } else {
            None
        };
        let mut app = Self {
            content,
            state,
            arsenal,
            world: sim::world::World::hub(),
            snapshot: Snapshot::default(),
            peer: 1,
            pos: Vec3::ZERO,
            yaw: 0.0,
            hp: 100,
            downed: false,
            ready: false,
            target: None,
            messages: VecDeque::new(),
            panel: "help".into(),
            scroll: 0,
            command_line: None,
            scene: None,
            scene_line: 0,
            status: url.unwrap_or_else(|| "Локальная симуляция Rust".into()),
            depth: options.depth,
            next_depth: None,
            seed: options.seed,
            zoom: 1.5,
            quit: false,
            snapshots_received: 0,
            pickups: vec![],
            points: vec![],
            collected: HashSet::new(),
            network,
            local: None,
            preset_base,
            save_path,
            local_cache,
            tick: 0,
            cooldown: 0.0,
            reload: 0.0,
            move_dir: Vec2::ZERO,
            move_left: 0.0,
            use_left: 0.0,
            autosave: 0.0,
            loaded_character: false,
            reload_weapon: weapon::WeaponId::Pistol,
        };
        if app.network.is_none() {
            app.local_world(&options.world);
            if let Some(player) = app.local.as_mut().and_then(|s| s.players.get_mut(&1)) {
                player.hp = saved_hp.clamp(1.0, player.max_hp);
            }
        }
        app.log("Добро пожаловать! F1 — помощь, Tab — цель, : — команды.");
        Ok(app)
    }

    pub fn log(&mut self, text: impl Into<String>) {
        // Server/chat/preset strings must never inject terminal control sequences.
        self.messages
            .push_back(text.into().chars().filter(|c| !c.is_control()).collect());
        while self.messages.len() > 100 {
            self.messages.pop_front();
        }
    }

    fn local_world(&mut self, kind: &str) {
        let mut local = sim::State::new(sim::Config {
            preset: self.state.preset.clone(),
            kind: kind.into(),
            seed: self.seed,
            depth: self.depth as i32,
            party_size: 1,
            preset_base: self.preset_base.clone(),
        });
        local.join(1, "player".into());
        self.world = local.world.clone();
        self.populate_points();
        self.pos = self.world.player_spawn;
        self.local = Some(local);
        self.snapshot = Snapshot::default();
        self.peer = 1;
        self.ready = true;
        self.hp = 100;
        self.downed = false;
        self.move_left = 0.0;
        self.target = None;
        let _ = self.game_command(
            "choose_class",
            json!({"class": self.state.class_idx.unwrap_or(1)}),
        );
    }

    pub fn game_command(&mut self, kind: &str, args: Value) -> Result<()> {
        if let Some(net) = &self.network {
            net.raw(protocol::encode(
                "cmd",
                &json!({"kind": kind, "args": args}),
            )?)?;
        } else if kind == "enter_delve" {
            self.depth = args["depth"].as_u64().unwrap_or(1).clamp(1, 100) as u32;
            self.seed = self.seed.wrapping_add(1);
            self.local_world("delve");
        } else if kind == "leave_delve" {
            self.local_world("hub");
        } else if let Some(local) = &mut self.local {
            let events = local.command(&sim::Cmd {
                kind: "cmd".into(),
                peer: self.peer,
                data: Some(json!({"kind": kind, "args": args})),
            });
            for event in events {
                self.on_event(event);
            }
        }
        Ok(())
    }

    pub fn update(&mut self, dt: f32) -> Result<()> {
        if let Some(net) = &self.network {
            let received: Vec<_> = net.rx.try_iter().take(256).collect();
            for message in received {
                self.receive(message.map_err(|e| anyhow!(e))?)?;
            }
        }
        if !self.ready {
            return Ok(());
        }
        if self.world.kind == "hub" {
            if let Some(depth) = self.next_depth.take() {
                self.execute(&format!("delve {depth}"))?;
            }
        }
        self.cooldown = (self.cooldown - dt).max(0.0);
        if self.reload > 0.0 {
            self.reload -= dt;
            if self.reload <= 0.0 {
                self.arsenal.reload(self.reload_weapon);
            }
        }
        let old_pos = self.pos;
        if self.move_left > 0.0 && !self.downed {
            let delta = Vec3::new(self.move_dir.x, 0.0, self.move_dir.y) * (5.0 * dt);
            // Axis sliding makes the same floor grid comfortable to navigate from keys.
            let x = self.pos + Vec3::new(delta.x, 0.0, 0.0);
            if self.walkable(x) {
                self.pos = x;
            }
            let z = self.pos + Vec3::new(0.0, 0.0, delta.z);
            if self.walkable(z) {
                self.pos = z;
            }
            self.pos.y = self.world.floor_at(self.pos) + 1.1;
            self.yaw = (-self.move_dir.x).atan2(-self.move_dir.y);
        }
        self.move_left = (self.move_left - dt).max(0.0);
        self.tick = self.tick.wrapping_add(1);
        let input = Input {
            tick: self.tick,
            pos: self.pos,
            yaw: self.yaw,
            move_dir: if self.move_left > 0.0 {
                self.move_dir
            } else {
                Vec2::ZERO
            },
            vel: (self.pos - old_pos) * (1.0 / dt),
            on_floor: true,
            buttons: if self.use_left > 0.0 {
                sim::buttons::USE
            } else {
                0
            },
            ..Default::default()
        };
        self.use_left = (self.use_left - dt).max(0.0);
        if let Some(net) = &self.network {
            net.raw(protocol::encode("input", &input)?)?;
        }
        if let Some(local) = &mut self.local {
            let result = local.tick(
                dt,
                &[sim::TaggedInput {
                    peer: self.peer,
                    input,
                }],
            );
            for event in result.events {
                self.on_event(event);
            }
            self.on_snapshot(result.snapshot);
        }
        self.pickup_nearby();
        self.autosave += dt;
        if self.autosave >= 30.0 {
            self.save()?;
            self.autosave = 0.0;
        }
        Ok(())
    }

    fn receive(&mut self, message: ServerMessage) -> Result<()> {
        match message {
            ServerMessage::Welcome(w) => {
                anyhow::ensure!(
                    w.preset_id == self.state.preset && w.content_hash == self.content.hash,
                    "Сервер прислал несовместимый пресет"
                );
                self.peer = w.peer;
                self.depth = w.world.depth.max(1) as u32;
                self.seed = w.world.seed;
                self.world = if w.world.kind == "delve" {
                    sim::world::World::delve(self.depth, self.seed, &self.content.cfg)
                } else {
                    sim::world::World::hub()
                };
                self.pos = self.world.player_spawn;
                self.populate_points();
                self.snapshot = Snapshot::default();
                self.target = None;
                self.move_left = 0.0;
                self.downed = false;
                if !self.loaded_character {
                    if let Some(save) = w.character {
                        let parsed = SaveData::from_json(&save.data)?;
                        (self.state, _, self.arsenal) = parsed.into_game();
                    }
                    self.loaded_character = true;
                }
                self.ready = true;
                self.game_command(
                    "choose_class",
                    json!({"class": self.state.class_idx.unwrap_or(1)}),
                )?;
                self.log(format!(
                    "Вход: {} · peer {} · игроков {}",
                    self.world.kind,
                    w.peer,
                    w.players.len()
                ));
            }
            ServerMessage::Snapshot(snapshot) => self.on_snapshot(snapshot),
            ServerMessage::Event(event) => self.on_event(event),
            ServerMessage::Reject(reject) => {
                return Err(anyhow!(
                    "Сервер отказал: {} — {}",
                    reject.reason,
                    reject.detail
                ))
            }
            ServerMessage::Pong(_) | ServerMessage::Unknown(_) => {}
        }
        Ok(())
    }

    fn on_snapshot(&mut self, snapshot: Snapshot) {
        if let Some(player) = snapshot.players.iter().find(|e| e.id == self.peer) {
            self.hp = player.hp;
            self.downed = player.flags & sim::flags::DOWNED != 0;
            if self.local.is_some() || (player.pos - self.pos).length() > 1.5 {
                self.pos = player.pos;
            }
        }
        self.snapshots_received += 1;
        self.snapshot = snapshot;
    }

    fn on_event(&mut self, event: sim::Event) {
        if event.kind == "xp" && event.actor == self.peer {
            self.state.add_xp(event.amount.max(0.0) as u32);
        }
        if event.kind == "enemy_died" {
            if let Some(enemy) = self.snapshot.enemies.iter().find(|e| e.id == event.target) {
                if let Some(cfg) = self
                    .content
                    .cfg
                    .enemies
                    .get(enemy.type_id.saturating_sub(1) as usize)
                {
                    let id = cfg.id.clone();
                    self.bump("kill", &id);
                    if self
                        .world
                        .spawns
                        .enemies
                        .iter()
                        .any(|s| s.is_boss && s.kind == id)
                    {
                        self.bump("boss", &id);
                        self.state.dungeons_cleared = self.state.dungeons_cleared.max(self.depth);
                        self.state.gold += 50;
                        self.state.weapon_mod_cores += 1;
                        self.state.add_xp(120);
                        if self.state.flags.insert(format!("boss_defeated_{id}")) {
                            let trophy = match id.as_str() {
                                "crypt_warden" => "trophy_warden_seal",
                                "blood_oracle" => "trophy_oracle_chalice",
                                "archive_sentinel" => "trophy_archive_lens",
                                _ => "trophy_tyrant_heart",
                            };
                            self.give_item(trophy, 1);
                        }
                        if self.state.quests.is_active("dungeon_heart") {
                            self.state.quests.complete("dungeon_heart");
                        }
                        self.log("Страж повержен: +50 золота, +1 ядро, +120 опыта. Портал следующего этажа открыт.");
                    }
                }
            }
        }
        if event.kind == "pickup" && event.actor == self.peer {
            if let Some((kind, id)) = event.text.split_once(':') {
                match kind {
                    "item" => self.take_item(id),
                    "ammo" => {
                        if let Ok(index @ 0..=3) = id.parse::<usize>() {
                            let ammo = weapon::AmmoType::from_idx(index);
                            self.arsenal.add_ammo(ammo, ammo.pack_size(), 1.0);
                        }
                    }
                    "weapon" => {
                        if let Some(weapon) = weapon::WeaponId::from_id(id) {
                            self.arsenal.give_weapon(weapon);
                        }
                    }
                    _ => {}
                }
            }
        }
        if event.kind == "wipe" {
            self.log("Забег провален. :hub — вернуться в хаб.");
        }
        if event.kind == "player_out" && event.target == self.peer {
            self.downed = true;
            self.hp = 0;
        }
        if event.kind != "despawn" {
            self.log(format!(
                "{} #{}→#{} {} {}",
                event.kind,
                event.actor,
                event.target,
                if event.amount != 0.0 {
                    format!("{:.0}", event.amount)
                } else {
                    String::new()
                },
                event.text
            ));
        }
    }

    pub fn walkable(&self, pos: Vec3) -> bool {
        if !self.world.walkable_at(pos) {
            return false;
        }
        if self.world.kind == "hub" {
            if let Some(map) = &self.content.map {
                if let Some(ground) = &map.ground {
                    if pos.x.abs() > ground.size * 0.5 || pos.z.abs() > ground.size * 0.5 {
                        return false;
                    }
                }
                for b in &map.buildings {
                    if (pos.x - b.pos[0]).abs() < b.size[0] * 0.5 + 0.2
                        && (pos.z - b.pos[1]).abs() < b.size[2] * 0.5 + 0.2
                    {
                        return false;
                    }
                }
                for b in &map.blocks {
                    if b.shape == "box" {
                        if let (Some(center), Some(size)) = (b.pos, b.size) {
                            if center[1] + size[1] * 0.5 < 0.5 || center[1] - size[1] * 0.5 > 1.8 {
                                continue;
                            }
                            let angle = b.rot.to_radians();
                            let dx = pos.x - center[0];
                            let dz = pos.z - center[2];
                            let x = dx * angle.cos() - dz * angle.sin();
                            let z = dx * angle.sin() + dz * angle.cos();
                            if x.abs() < size[0] * 0.5 + 0.2 && z.abs() < size[2] * 0.5 + 0.2 {
                                return false;
                            }
                        }
                    }
                }
            }
        }
        true
    }

    pub fn cycle_target(&mut self) {
        // HP is quantized to percent: a living enemy can legitimately report 0%.
        let mut enemies: Vec<_> = self.snapshot.enemies.iter().collect();
        enemies.sort_by(|a, b| {
            (a.pos - self.pos)
                .length()
                .total_cmp(&(b.pos - self.pos).length())
        });
        let next = enemies
            .iter()
            .position(|e| Some(e.id) == self.target)
            .map_or(0, |i| (i + 1) % enemies.len().max(1));
        self.target = enemies.get(next).map(|e| e.id);
    }

    pub fn attack(&mut self) -> Result<()> {
        if !self.ready || self.downed || self.cooldown > 0.0 || self.reload > 0.0 {
            return Ok(());
        }
        if !self.arsenal.can_fire(self.arsenal.current) {
            self.log("Нет патронов. R — перезарядить.");
            return Ok(());
        }
        if self.target.is_none() {
            self.cycle_target();
        }
        let Some(target) = self
            .snapshot
            .enemies
            .iter()
            .find(|e| Some(e.id) == self.target)
            .copied()
        else {
            self.target = None;
            return Ok(());
        };
        let def = weapon::weapon_def(self.arsenal.current);
        if (target.pos - self.pos).length() > def.range {
            self.log("Цель вне дальности оружия");
            return Ok(());
        }
        let ray = target.pos - self.pos;
        let steps = (ray.length() / 0.2).ceil().max(1.0) as usize;
        if (1..steps).any(|i| !self.walkable(self.pos + ray * (i as f32 / steps as f32))) {
            self.log("Цель за стеной");
            return Ok(());
        }
        self.cooldown = def.cooldown;
        self.arsenal.consume(def.id);
        let fire = sim::Fire {
            tick: self.tick,
            weapon: def.id.id().into(),
            origin: self.pos,
            dir: (target.pos - self.pos).normalized(),
            secondary: false,
        };
        let hit = sim::HitClaim {
            tick: self.tick,
            weapon: def.id.id().into(),
            target: target.id,
            pos: target.pos,
            part: 0,
        };
        if let Some(net) = &self.network {
            net.raw(protocol::encode("fire", &fire)?)?;
            net.raw(protocol::encode("hit", &hit)?)?;
        } else if let Some(local) = &mut self.local {
            let events = local.command(&sim::Cmd {
                kind: "hit".into(),
                peer: self.peer,
                data: Some(serde_json::to_value(hit)?),
            });
            for event in events {
                self.on_event(event);
            }
        }
        Ok(())
    }

    pub fn reload(&mut self) {
        if self.reload <= 0.0 {
            self.reload_weapon = self.arsenal.current;
            self.reload = weapon::weapon_def(self.reload_weapon).reload_time;
            self.log("Перезарядка…");
        }
    }

    pub fn save(&mut self) -> Result<()> {
        if !self.ready {
            return Ok(());
        }
        let data = SaveData::from_game(&self.state, self.hp as f32, &self.arsenal).to_json()?;
        if self.local_cache {
            let temporary = self.save_path.with_extension("tmp");
            fs::write(&temporary, &data)?;
            fs::rename(temporary, &self.save_path)?;
        }
        if let Some(net) = &self.network {
            anyhow::ensure!(
                data.len() <= protocol::MAX_SAVE_BYTES,
                "Персонаж превышает лимит сохранения"
            );
            net.raw(protocol::encode(
                "save",
                &protocol::Save {
                    ver: openheart_core::save::SAVE_VERSION as i32,
                    data,
                },
            )?)?;
        }
        Ok(())
    }

    pub fn give_item(&mut self, id: &str, count: u32) {
        let name = self
            .content
            .cfg
            .item(id)
            .map(|i| i.name("ru"))
            .unwrap_or(id);
        self.state
            .inventory
            .add(openheart_core::item::Item::new(id, name, "", count));
    }

    pub fn bump(&mut self, kind: &str, target: &str) {
        for q in &self.content.cfg.quests {
            if q.kind == kind
                && (q.target == target || q.target.is_empty())
                && self.state.quests.is_active(&q.id)
            {
                *self.state.quest_kills.entry(q.id.clone()).or_default() += 1;
            }
        }
    }
}
