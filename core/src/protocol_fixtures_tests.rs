//! Совместимость с Go: обе стороны разбирают одни и те же образцы сообщений.
//!
//! Файлы лежат в `protocol/fixtures/` и читаются ещё и Go-тестом
//! (`server/internal/proto/fixtures_test.go`). Если формат разъедется, упадёт
//! одна из сторон — а не игрок на живом сервере.

use std::path::PathBuf;

use crate::protocol::{parse_server, Envelope, Hello, ServerMessage};
use crate::sim::{HitClaim, Input};

fn fixture(name: &str) -> String {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "protocol", "fixtures", name]
        .iter()
        .collect();
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("не прочитать {}: {e}", path.display()))
}

#[test]
fn client_messages_parse() {
    let raw = fixture("hello.json");
    let envelope = Envelope::parse(raw.trim()).unwrap();
    let hello: Hello = envelope.body().unwrap();
    assert_eq!(hello.protocol, crate::protocol::VERSION);
    assert_eq!(hello.preset_id, "core");
    assert_eq!(hello.auth.mode, "open");
    assert_eq!(hello.auth.nickname, "vasya");

    let raw = fixture("input.json");
    let input: Input = Envelope::parse(raw.trim()).unwrap().body().unwrap();
    assert_eq!(input.tick, 42);
    assert!(input.on_floor);
    assert_eq!(input.buttons, 6);
    assert_eq!(input.pos.y, 1.1);

    let raw = fixture("hit.json");
    let hit: HitClaim = Envelope::parse(raw.trim()).unwrap().body().unwrap();
    assert_eq!(hit.target, 1000);
    assert_eq!(hit.weapon, "pistol");
    assert_eq!(hit.part, 1);
}

#[test]
fn server_messages_parse() {
    match parse_server(fixture("welcome.json").trim()).unwrap() {
        ServerMessage::Welcome(welcome) => {
            assert_eq!(welcome.peer, 3);
            assert_eq!(welcome.world.kind, "delve");
            assert_eq!(welcome.world.seed, 11951638873787587581);
            assert_eq!(welcome.players.len(), 2);
        }
        other => panic!("ожидался welcome: {other:?}"),
    }

    match parse_server(fixture("snap.json").trim()).unwrap() {
        ServerMessage::Snapshot(snapshot) => {
            assert_eq!(snapshot.tick, 181);
            assert_eq!(snapshot.ack, 42);
            assert_eq!(snapshot.players[0].hp, 87);
            let enemy = snapshot.enemies[0];
            assert_eq!(enemy.id, 1000);
            assert_eq!(enemy.type_id, 3, "вид врага должен доезжать до клиента");
            assert_eq!(enemy.flags, crate::sim::flags::ELITE);
        }
        other => panic!("ожидался снапшот: {other:?}"),
    }

    match parse_server(fixture("event.json").trim()).unwrap() {
        ServerMessage::Event(event) => {
            assert_eq!(event.kind, crate::sim::event::ENEMY_DIED);
            assert_eq!(event.target, 1000);
        }
        other => panic!("ожидалось событие: {other:?}"),
    }

    match parse_server(fixture("reject.json").trim()).unwrap() {
        ServerMessage::Reject(reject) => {
            assert_eq!(reject.reason, crate::protocol::reject::CONTENT);
            assert!(!reject.detail.is_empty());
        }
        other => panic!("ожидался отказ: {other:?}"),
    }
}
