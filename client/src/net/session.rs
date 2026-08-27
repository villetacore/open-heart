//! Сессия: разбор сообщений сервера и очередь исходящих.
//!
//! Здесь нет ни движка, ни сокета — только состояние разговора с сервером.
//! Благодаря этому логика проверяется обычными тестами, а транспорт
//! (`WebSocketPeer`) остаётся тонкой обёрткой в [`super::NetClient`].

use openheart_core::protocol::{
    self, ClientMessage, Hello, PlayerInfo, Reject, ServerMessage, Welcome, World,
};
use openheart_core::sim::{Event, Fire, HitClaim, Input, Snapshot};

/// Стадия разговора.
#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    /// Отправили `hello`, ждём `welcome`.
    Handshake,
    /// Сервер принял: можно играть.
    Ready,
    /// Сервер отказал — с причиной из protocol/schema.md.
    Rejected(Reject),
    /// Соединение закрыто.
    Closed,
}

/// То, что сессия отдаёт игровому коду.
#[derive(Clone, Debug)]
pub enum NetEvent {
    /// Нас пустили: наш peer, мир комнаты и кто уже внутри.
    Welcome {
        peer: u16,
        world: World,
        players: Vec<PlayerInfo>,
    },
    /// Состояние мира на тик сервера.
    Snapshot(Snapshot),
    /// Событие мира: урон, смерть, вход и выход игроков, чат.
    Game(Event),
    /// Отказ во входе.
    Rejected(Reject),
    /// Ответ на ping — по нему считаем задержку.
    Pong(i64),
}

/// Разговор с сервером.
pub struct Session {
    phase: Phase,
    local_peer: u16,
    world: World,
    outbox: Vec<String>,
    /// Номер следующего пакета ввода: сервер подтверждает его в снапшоте.
    input_tick: u32,
    /// Последний подтверждённый сервером ввод — по нему сверяется предсказание.
    acked_tick: u32,
    /// Сколько сообщений не удалось разобрать: важно видеть в логе, а не молчать.
    parse_errors: u32,
}

impl Session {
    /// Начать разговор: `hello` сразу уходит в очередь отправки.
    pub fn new(hello: Hello) -> Self {
        let mut session = Self {
            phase: Phase::Handshake,
            local_peer: 0,
            world: World::default(),
            outbox: Vec::new(),
            input_tick: 0,
            acked_tick: 0,
            parse_errors: 0,
        };
        session.queue(ClientMessage::Hello(hello));
        session
    }

    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    pub fn is_ready(&self) -> bool {
        self.phase == Phase::Ready
    }

    /// Наш идентификатор в комнате (0 — ещё не представились).
    pub fn local_peer(&self) -> u16 {
        self.local_peer
    }

    /// Мир комнаты из `welcome`: вид, сид, глубина.
    #[allow(dead_code)] // пригодится, когда клиент начнёт строить мир по сиду сервера
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Последний подтверждённый сервером номер ввода — основа перемотки (N2).
    #[allow(dead_code)]
    pub fn acked_tick(&self) -> u32 {
        self.acked_tick
    }

    /// Счётчик неразобранных сообщений: полезен в диагностике и тестах.
    #[allow(dead_code)]
    pub fn parse_errors(&self) -> u32 {
        self.parse_errors
    }

    /// Обработать текстовый кадр от сервера.
    pub fn on_text(&mut self, raw: &str) -> Option<NetEvent> {
        let message = match protocol::parse_server(raw) {
            Ok(message) => message,
            Err(_) => {
                self.parse_errors += 1;
                return None;
            }
        };

        match message {
            ServerMessage::Welcome(welcome) => Some(self.on_welcome(welcome)),
            ServerMessage::Snapshot(snapshot) => {
                self.acked_tick = snapshot.ack;
                Some(NetEvent::Snapshot(snapshot))
            }
            ServerMessage::Event(event) => Some(NetEvent::Game(event)),
            ServerMessage::Reject(reject) => {
                self.phase = Phase::Rejected(reject.clone());
                Some(NetEvent::Rejected(reject))
            }
            ServerMessage::Pong(t) => Some(NetEvent::Pong(t)),
            // Незнакомый тег — не повод рвать соединение: сервер может быть новее.
            ServerMessage::Unknown(_) => None,
        }
    }

    fn on_welcome(&mut self, welcome: Welcome) -> NetEvent {
        self.phase = Phase::Ready;
        self.local_peer = welcome.peer;
        self.world = welcome.world.clone();
        NetEvent::Welcome {
            peer: welcome.peer,
            world: welcome.world,
            players: welcome.players,
        }
    }

    /// Соединение закрылось (сокет или сервер).
    pub fn on_closed(&mut self) {
        if !matches!(self.phase, Phase::Rejected(_)) {
            self.phase = Phase::Closed;
        }
    }

    // ── исходящие ────────────────────────────────────────────────────────────

    /// Отправить ввод. Возвращает номер пакета — его же сервер подтвердит.
    pub fn send_input(&mut self, mut input: Input) -> u32 {
        if !self.is_ready() {
            return self.input_tick;
        }
        self.input_tick = self.input_tick.wrapping_add(1);
        input.tick = self.input_tick;
        self.queue(ClientMessage::Input(input));
        self.input_tick
    }

    pub fn send_fire(&mut self, fire: Fire) {
        if self.is_ready() {
            self.queue(ClientMessage::Fire(fire));
        }
    }

    /// Заявить о попадании. Урон посчитает сервер — он же может отказать.
    pub fn send_hit(&mut self, hit: HitClaim) {
        if self.is_ready() {
            self.queue(ClientMessage::Hit(hit));
        }
    }

    /// Измерение задержки; ответ придёт как [`NetEvent::Pong`].
    ///
    /// Он же держит соединение живым, пока игроку нечего отправлять
    /// (см. `NetClient::send_keepalive`).
    pub fn send_ping(&mut self, now_ms: i64) {
        self.queue(ClientMessage::Ping(now_ms));
    }

    fn queue(&mut self, message: ClientMessage) {
        match message.encode() {
            Ok(raw) => self.outbox.push(raw),
            Err(error) => godot::global::godot_warn!("[net] не сериализовать сообщение: {error}"),
        }
    }

    /// Забрать накопленные сообщения для отправки в сокет.
    pub fn take_outbox(&mut self) -> Vec<String> {
        std::mem::take(&mut self.outbox)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello() -> Hello {
        Hello {
            protocol: openheart_core::protocol::VERSION,
            game_version: "test".into(),
            preset_id: "core".into(),
            content_hash: "hash".into(),
            auth: openheart_core::protocol::Auth {
                mode: "open".into(),
                nickname: "vasya".into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn ready_session() -> Session {
        let mut session = Session::new(hello());
        session.take_outbox();
        session.on_text(
            r#"{"t":"welcome","d":{"peer":4,"world":{"kind":"delve","seed":7,"depth":2}}}"#,
        );
        session
    }

    #[test]
    fn hello_goes_out_first() {
        let mut session = Session::new(hello());
        let outbox = session.take_outbox();
        assert_eq!(outbox.len(), 1);
        assert!(outbox[0].contains(r#""t":"hello""#), "{}", outbox[0]);
        assert_eq!(*session.phase(), Phase::Handshake);
        assert!(!session.is_ready());
    }

    #[test]
    fn welcome_opens_the_session() {
        let session = ready_session();
        assert!(session.is_ready());
        assert_eq!(session.local_peer(), 4);
        assert_eq!(session.world().kind, "delve");
        assert_eq!(session.world().seed, 7);
    }

    #[test]
    fn reject_stops_the_session() {
        let mut session = Session::new(hello());
        let event = session
            .on_text(r#"{"t":"reject","d":{"reason":"content","detail":"другой пресет"}}"#)
            .expect("отказ разобран");
        assert!(matches!(event, NetEvent::Rejected(ref r) if r.reason == "content"));
        assert!(!session.is_ready());
        // После отказа ввод не шлём — сервер нас не ждёт.
        session.take_outbox();
        session.send_input(Input::default());
        assert!(session.take_outbox().is_empty());
    }

    #[test]
    fn input_is_numbered_and_acked() {
        let mut session = ready_session();
        session.take_outbox();

        assert_eq!(session.send_input(Input::default()), 1);
        assert_eq!(session.send_input(Input::default()), 2);
        let outbox = session.take_outbox();
        assert_eq!(outbox.len(), 2);
        assert!(outbox[1].contains(r#""tick":2"#), "{}", outbox[1]);

        session.on_text(r#"{"t":"snap","d":{"tick":9,"ack":2}}"#);
        assert_eq!(session.acked_tick(), 2);
    }

    #[test]
    fn snapshot_reaches_the_game() {
        let mut session = ready_session();
        let event = session
            .on_text(
                r#"{"t":"snap","d":{"tick":3,"ack":1,"players":[{"id":4,"kind":0,"pos":[1.0,0.0,2.0],"yaw":0.0,"hp":100,"flags":0}],"enemies":[{"id":1000,"kind":1,"pos":[9.0,0.0,9.0],"yaw":0.0,"hp":50,"flags":0}]}}"#,
            )
            .expect("снапшот разобран");
        match event {
            NetEvent::Snapshot(snapshot) => {
                assert_eq!(snapshot.tick, 3);
                assert_eq!(snapshot.players.len(), 1);
                assert_eq!(snapshot.enemies[0].id, 1000);
            }
            other => panic!("ожидался снапшот: {other:?}"),
        }
    }

    #[test]
    fn broken_and_unknown_messages_do_not_break_us() {
        let mut session = ready_session();
        assert!(session.on_text("{это не json").is_none());
        assert_eq!(session.parse_errors(), 1);
        assert!(session.on_text(r#"{"t":"quest_sync","d":{}}"#).is_none());
        assert!(session.is_ready(), "сессия должна пережить мусор");
    }

    // Пинг — не только замер задержки: им клиент держит соединение, пока
    // отправлять нечего (выбор класса, диалог).
    #[test]
    fn ping_keeps_the_line_busy() {
        let mut session = ready_session();
        session.take_outbox();
        session.send_ping(1234);
        let outbox = session.take_outbox();
        assert_eq!(outbox.len(), 1);
        assert!(outbox[0].contains(r#""t":"ping""#), "{}", outbox[0]);
    }

    #[test]
    fn hit_claims_go_only_when_ready() {
        let mut session = Session::new(hello());
        session.take_outbox();
        session.send_hit(HitClaim { target: 1000, weapon: "pistol".into(), ..Default::default() });
        assert!(session.take_outbox().is_empty(), "до welcome стрелять некуда");

        session.on_text(r#"{"t":"welcome","d":{"peer":1}}"#);
        session.take_outbox();
        session.send_hit(HitClaim { target: 1000, weapon: "pistol".into(), ..Default::default() });
        let outbox = session.take_outbox();
        assert_eq!(outbox.len(), 1);
        assert!(outbox[0].contains(r#""t":"hit""#), "{}", outbox[0]);
    }
}
