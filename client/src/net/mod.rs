//! Сетевой слой клиента: сокет к игровому серверу.
//!
//! Транспорт — `WebSocketPeer` движка (он же работает в веб-сборке). Вся логика
//! разговора лежит в [`session::Session`] и проверяется тестами без движка;
//! здесь только качание пакетов и состояние соединения.

pub mod host;
pub mod master;
pub mod session;

use godot::classes::web_socket_peer::State;
use godot::classes::WebSocketPeer;
use godot::prelude::*;

use openheart_core::protocol::{Auth, Hello, VERSION};
use openheart_core::sim::{Fire, HitClaim, Input};

pub use session::{NetEvent, Phase, Session};

/// Как часто клиент шлёт ввод: плотнее серверного тика, сервер берёт последний.
pub const INPUT_HZ: f32 = 30.0;

/// Соединение с игровым сервером.
pub struct NetClient {
    socket: Gd<WebSocketPeer>,
    session: Session,
    url: String,
    /// Накопитель времени до следующей отправки ввода.
    input_timer: f32,
    /// Сокет уже открывался: по переходу в CLOSED сообщаем об обрыве один раз.
    was_open: bool,
    closed_reported: bool,
    /// Когда последний раз уходил пинг-заполнитель (мс движка).
    last_keepalive: i64,
}

impl NetClient {
    /// Подключиться к серверу. Ошибка — если движок не смог даже начать коннект.
    pub fn connect(url: &str, nickname: &str, preset: &str, content_hash: &str) -> Option<Self> {
        let mut socket = WebSocketPeer::new_gd();
        let error = socket.connect_to_url(url);
        if error != godot::global::Error::OK {
            godot_warn!("[net] не подключиться к {url}: {error:?}");
            return None;
        }

        let hello = Hello {
            protocol: VERSION,
            game_version: env!("CARGO_PKG_VERSION").to_string(),
            preset_id: preset.to_string(),
            content_hash: content_hash.to_string(),
            auth: Auth {
                mode: "open".to_string(),
                nickname: nickname.to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        Some(Self {
            socket,
            session: Session::new(hello),
            url: url.to_string(),
            input_timer: 0.0,
            was_open: false,
            closed_reported: false,
            last_keepalive: 0,
        })
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn is_ready(&self) -> bool {
        self.session.is_ready()
    }

    pub fn local_peer(&self) -> u16 {
        self.session.local_peer()
    }

    pub fn phase(&self) -> &Phase {
        self.session.phase()
    }

    /// Прокачать сокет: отправить накопленное, разобрать входящее.
    /// Возвращает события для игрового кода.
    pub fn poll(&mut self, dt: f32) -> Vec<NetEvent> {
        self.socket.poll();
        let mut events = Vec::new();

        match self.socket.get_ready_state() {
            State::OPEN => {
                self.was_open = true;
                self.flush();
                while self.socket.get_available_packet_count() > 0 {
                    let packet = self.socket.get_packet();
                    let text = String::from_utf8_lossy(packet.as_slice()).into_owned();
                    if let Some(event) = self.session.on_text(&text) {
                        events.push(event);
                    }
                }
                self.flush(); // ответы на входящее уходят в том же кадре
            }
            State::CLOSED => {
                self.session.on_closed();
                if self.was_open && !self.closed_reported {
                    self.closed_reported = true;
                    let reason = self.socket.get_close_reason();
                    godot_warn!("[net] соединение закрыто: {reason}");
                }
            }
            // CONNECTING / CLOSING — просто ждём следующего кадра.
            _ => {}
        }

        self.input_timer += dt;
        events
    }

    /// Пора ли отправлять очередной пакет ввода.
    pub fn input_due(&mut self) -> bool {
        let period = 1.0 / INPUT_HZ;
        if self.input_timer >= period {
            self.input_timer -= period;
            return true;
        }
        false
    }

    pub fn send_input(&mut self, input: Input) {
        self.session.send_input(input);
    }

    /// Держать соединение живым, когда отправлять нечего.
    ///
    /// Пока игрок выбирает класс или читает диалог, узла игрока ещё нет и ввод
    /// не с чего собрать. Сервер же рвёт соединение, из которого 15 с ничего не
    /// пришло, — поэтому вместо ввода шлём пинг, не чаще раза в секунду.
    pub fn send_keepalive(&mut self) {
        const PERIOD_MS: i64 = 1000;
        let now = godot::classes::Time::singleton().get_ticks_msec() as i64;
        if now - self.last_keepalive < PERIOD_MS {
            return;
        }
        self.last_keepalive = now;
        self.session.send_ping(now);
    }

    pub fn send_fire(&mut self, fire: Fire) {
        self.session.send_fire(fire);
    }

    pub fn send_hit(&mut self, hit: HitClaim) {
        self.session.send_hit(hit);
    }

    /// Закрыть соединение (выход в меню, смена сервера).
    pub fn close(&mut self) {
        self.flush();
        self.socket.close();
        self.session.on_closed();
    }

    fn flush(&mut self) {
        for message in self.session.take_outbox() {
            let error = self.socket.send_text(&message);
            if error != godot::global::Error::OK {
                godot_warn!("[net] пакет не ушёл: {error:?}");
                break;
            }
        }
    }
}
