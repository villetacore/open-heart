//! Сообщения сетевого протокола.
//!
//! Единственный источник правды по формату — protocol/schema.md. Здесь его
//! реализация для Rust (клиент и ядро), в Go ей соответствует `internal/proto`.
//! Игровые структуры (`Input`, `Snapshot`, `Event`, `Fire`, `HitClaim`) не
//! дублируются — они живут в [`crate::sim`] и ходят по проводу как есть.

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use crate::math::Vec3;
use crate::sim::{Event, Fire, HitClaim, Input, Snapshot};

/// Версия протокола: инкремент при любом несовместимом изменении схемы.
pub const VERSION: i32 = crate::PROTOCOL_VERSION;

/// Теги сообщений (поле `t` конверта).
pub mod tag {
    // клиент → сервер
    pub const HELLO: &str = "hello";
    pub const INPUT: &str = "input";
    pub const FIRE: &str = "fire";
    pub const HIT: &str = "hit";
    pub const INTERACT: &str = "interact";
    pub const CMD: &str = "cmd";
    pub const CHAT: &str = "chat";
    pub const PING: &str = "ping";
    pub const SAVE: &str = "save";

    // сервер → клиент
    pub const WELCOME: &str = "welcome";
    pub const SNAPSHOT: &str = "snap";
    pub const EVENT: &str = "event";
    pub const REJECT: &str = "reject";
    pub const PONG: &str = "pong";
}

/// Причины отказа во входе (`Reject::reason`).
pub mod reject {
    pub const PROTOCOL: &str = "protocol";
    pub const VERSION: &str = "version";
    pub const CONTENT: &str = "content";
    pub const AUTH: &str = "auth";
    pub const BANNED: &str = "banned";
    pub const FULL: &str = "full";
    pub const BAD_REQUEST: &str = "bad_request";
    pub const TOO_BIG: &str = "too_big";
    pub const RATE: &str = "rate";
}

/// Конверт: тег плюс тело. Тело держим сырым, чтобы разбирать его только тогда,
/// когда тег уже известен.
#[derive(Debug, Serialize, Deserialize)]
pub struct Envelope<'a> {
    pub t: String,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    pub d: Option<&'a RawValue>,
}

impl<'a> Envelope<'a> {
    /// Разобрать входящее сообщение.
    pub fn parse(raw: &'a str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(raw)
    }

    /// Тело конверта в нужном типе.
    pub fn body<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        match self.d {
            Some(raw) => serde_json::from_str(raw.get()),
            None => serde_json::from_str("null"),
        }
    }
}

/// Собрать сообщение для отправки.
pub fn encode<T: Serialize>(tag: &str, body: &T) -> Result<String, serde_json::Error> {
    let body = serde_json::to_string(body)?;
    Ok(format!(r#"{{"t":"{tag}","d":{body}}}"#))
}

// ── клиент → сервер ──────────────────────────────────────────────────────────

/// Как игрок входит на сервер. Пароль от мастер-аккаунта здесь не появляется
/// никогда — только подписанный мастером тикет (docs/MULTIPLAYER.md §10).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Auth {
    pub mode: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ticket: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub login: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub password: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub nickname: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub register: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Hello {
    pub protocol: i32,
    pub game_version: String,
    pub preset_id: String,
    pub content_hash: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub password: String,
    pub auth: Auth,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Chat {
    pub text: String,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Ping {
    pub t: i64,
}

/// Состояние персонажа целиком.
///
/// Сервер его не разбирает: прогресс пока считает клиент, а сервер хранит блоб
/// и отдаёт при следующем входе. Так персонаж переживает и рестарт сервера, и
/// переустановку игры — но локальный сейв одиночной игры он не трогает.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Save {
    #[serde(default)]
    pub ver: i32,
    #[serde(default)]
    pub data: String,
}

/// Предел на состояние персонажа — столько же, сколько у сервера.
pub const MAX_SAVE_BYTES: usize = 64 * 1024;

// ── сервер → клиент ──────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct World {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub seed: u64,
    #[serde(default)]
    pub depth: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub peer: u16,
    #[serde(default)]
    pub nickname: String,
    #[serde(default)]
    pub class: String,
    #[serde(default)]
    pub level: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Welcome {
    pub peer: u16,
    #[serde(default)]
    pub tick: u32,
    #[serde(default)]
    pub preset_id: String,
    #[serde(default)]
    pub content_hash: String,
    #[serde(default)]
    pub world: World,
    #[serde(default)]
    pub players: Vec<PlayerInfo>,
    /// Персонаж, которого сервер помнит за этим игроком.
    #[serde(default)]
    pub character: Option<Save>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reject {
    pub reason: String,
    #[serde(default)]
    pub detail: String,
}

/// Разобранное сообщение сервера — то, с чем работает игровой код.
#[derive(Clone, Debug)]
pub enum ServerMessage {
    Welcome(Welcome),
    Snapshot(Snapshot),
    Event(Event),
    Reject(Reject),
    Pong(i64),
    /// Тег, которого мы не знаем: старый клиент не должен падать от нового сервера.
    Unknown(String),
}

/// Разобрать сообщение сервера целиком.
pub fn parse_server(raw: &str) -> Result<ServerMessage, serde_json::Error> {
    let envelope = Envelope::parse(raw)?;
    Ok(match envelope.t.as_str() {
        tag::WELCOME => ServerMessage::Welcome(envelope.body()?),
        tag::SNAPSHOT => ServerMessage::Snapshot(envelope.body()?),
        tag::EVENT => ServerMessage::Event(envelope.body()?),
        tag::REJECT => ServerMessage::Reject(envelope.body()?),
        tag::PONG => ServerMessage::Pong(envelope.body::<Ping>()?.t),
        other => ServerMessage::Unknown(other.to_string()),
    })
}

/// Сообщение клиента.
#[derive(Clone, Debug)]
pub enum ClientMessage {
    Command(serde_json::Value),
    Hello(Hello),
    Input(Input),
    Fire(Fire),
    Hit(HitClaim),
    Chat(Chat),
    Ping(i64),
    Save(Save),
}

impl ClientMessage {
    pub fn encode(&self) -> Result<String, serde_json::Error> {
        match self {
            Self::Command(msg) => encode(tag::CMD, msg),
            Self::Hello(msg) => encode(tag::HELLO, msg),
            Self::Input(msg) => encode(tag::INPUT, msg),
            Self::Fire(msg) => encode(tag::FIRE, msg),
            Self::Hit(msg) => encode(tag::HIT, msg),
            Self::Chat(msg) => encode(tag::CHAT, msg),
            Self::Ping(t) => encode(tag::PING, &Ping { t: *t }),
            Self::Save(msg) => encode(tag::SAVE, msg),
        }
    }
}

/// Позиция сущности из снапшота — удобный доступ для клиента.
pub fn ent_pos(ent: &crate::sim::Ent) -> Vec3 {
    ent.pos
}

fn is_false(v: &bool) -> bool {
    !*v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_round_trip() {
        let raw = encode(tag::HELLO, &Hello {
            protocol: VERSION,
            game_version: "test".into(),
            preset_id: "core".into(),
            content_hash: "abc".into(),
            auth: Auth { mode: "open".into(), nickname: "vasya".into(), ..Default::default() },
            ..Default::default()
        })
        .unwrap();

        assert!(raw.starts_with(r#"{"t":"hello","d":{"#), "{raw}");
        let envelope = Envelope::parse(&raw).unwrap();
        assert_eq!(envelope.t, tag::HELLO);
        let hello: Hello = envelope.body().unwrap();
        assert_eq!(hello.auth.nickname, "vasya");
        assert_eq!(hello.protocol, VERSION);
    }

    #[test]
    fn parses_server_messages() {
        let welcome = r#"{"t":"welcome","d":{"peer":3,"preset_id":"core","world":{"kind":"delve","seed":42,"depth":2},"players":[{"peer":3,"nickname":"vasya"}]}}"#;
        match parse_server(welcome).unwrap() {
            ServerMessage::Welcome(w) => {
                assert_eq!(w.peer, 3);
                assert_eq!(w.world.kind, "delve");
                assert_eq!(w.players.len(), 1);
            }
            other => panic!("не welcome: {other:?}"),
        }

        let snap = r#"{"t":"snap","d":{"tick":7,"ack":5,"players":[{"id":3,"kind":0,"pos":[1.0,0.0,2.0],"yaw":0.5,"hp":100,"flags":0}],"enemies":[{"id":1000,"kind":1,"pos":[5.0,0.0,5.0],"yaw":0.0,"hp":80,"flags":8}]}}"#;
        match parse_server(snap).unwrap() {
            ServerMessage::Snapshot(s) => {
                assert_eq!(s.tick, 7);
                assert_eq!(s.players.len(), 1);
                assert_eq!(s.enemies[0].id, 1000);
            }
            other => panic!("не снапшот: {other:?}"),
        }

        let reject = r#"{"t":"reject","d":{"reason":"content","detail":"другой пресет"}}"#;
        match parse_server(reject).unwrap() {
            ServerMessage::Reject(r) => assert_eq!(r.reason, reject::CONTENT),
            other => panic!("не отказ: {other:?}"),
        }
    }

    /// Незнакомый тег не должен ронять клиент: сервер может быть новее.
    #[test]
    fn unknown_tag_is_tolerated() {
        let raw = r#"{"t":"quest_sync","d":{"whatever":1}}"#;
        match parse_server(raw).unwrap() {
            ServerMessage::Unknown(tag) => assert_eq!(tag, "quest_sync"),
            other => panic!("ожидался Unknown: {other:?}"),
        }
    }

    #[test]
    fn client_messages_encode_with_expected_tags() {
        let input = ClientMessage::Input(Input { tick: 4, ..Default::default() }).encode().unwrap();
        assert!(input.contains(r#""t":"input""#), "{input}");
        assert!(input.contains(r#""tick":4"#), "{input}");

        let hit = ClientMessage::Hit(HitClaim {
            tick: 4,
            weapon: "pistol".into(),
            target: 1000,
            ..Default::default()
        })
        .encode()
        .unwrap();
        assert!(hit.contains(r#""t":"hit""#), "{hit}");
        assert!(hit.contains(r#""target":1000"#), "{hit}");
    }
}
