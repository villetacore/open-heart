use crate::{app::App, options::Options};
use clap::Parser;
use openheart_core::{
    math::{Vec2, Vec3},
    protocol::{Auth, Hello, ServerMessage},
    sim,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

struct TestData(PathBuf);
impl TestData {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "oh-terminal-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn options(&self) -> Options {
        let mut options = Options::parse_from(["oh-terminal", "--offline", "--headless"]);
        options.presets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../game/presets");
        options.data_dir = self.0.clone();
        options
    }
}
impl Drop for TestData {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn offline_input_moves_and_cannot_walk_out_of_dungeon() {
    let data = TestData::new();
    let mut options = data.options();
    options.world = "delve".into();
    let mut app = App::new(&options, None).unwrap();
    let before = app.pos;
    let direction = [
        Vec2::new(1.0, 0.0),
        Vec2::new(-1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(0.0, -1.0),
    ]
    .into_iter()
    .find(|d| app.walkable(before + Vec3::new(d.x, 0.0, d.y) * 0.25))
    .unwrap();
    app.move_dir = direction;
    app.move_left = 0.15;
    app.update(0.05).unwrap();
    assert!((app.pos - before).length() > 0.1);
    assert!(!app.walkable(Vec3::new(-10000.0, 0.0, -10000.0)));
    assert_eq!(app.snapshot.players.len(), 1);
}

#[test]
fn character_roundtrip_preserves_quests_inventory_and_weapons() {
    let data = TestData::new();
    let options = data.options();
    let mut app = App::new(&options, None).unwrap();
    app.state.gold = 321;
    app.give_item("medkit", 3);
    app.state.quests.add("test", "Test", "test");
    app.state.quest_kills.insert("test".into(), 5);
    let weapon = app.arsenal.current;
    app.save().unwrap();
    let loaded = App::new(&options, None).unwrap();
    assert_eq!(loaded.state.gold, 321);
    assert_eq!(loaded.state.inventory.count("medkit"), 3);
    assert!(loaded.state.quests.is_active("test"));
    assert_eq!(loaded.state.quest_kills["test"], 5);
    assert_eq!(loaded.arsenal.current, weapon);
}

#[test]
fn dialogue_requirements_block_effects_and_apply_once() {
    use openheart_core::dialogue::{Choice, ChoiceRequirement, Effect, Line, Scene};
    let data = TestData::new();
    let mut app = App::new(&data.options(), None).unwrap();
    let gold = app.state.gold;
    app.scene = Some(Scene {
        id: "test".into(),
        lines: vec![Line::narr("Test")],
        choices: vec![Choice {
            text: "Choice".into(),
            requires: Some(ChoiceRequirement {
                flag: Some("key".into()),
                ..Default::default()
            }),
            effects: vec![Effect::Gold(7)],
            next: None,
        }],
    });
    assert!(app.choose(1).is_err());
    assert_eq!(app.state.gold, gold);
    app.state.flags.insert("key".into());
    app.choose(1).unwrap();
    assert_eq!(app.state.gold, gold + 7);
    assert!(app.choose(1).is_err());
    assert_eq!(app.state.gold, gold + 7);
}

#[test]
fn invalid_actions_do_not_spend_resources() {
    let data = TestData::new();
    let mut app = App::new(&data.options(), None).unwrap();
    let gold = app.state.gold;
    assert!(app.execute("buy medkit").is_err());
    assert_eq!(app.state.gold, gold);
    assert!(app.execute("craft nonexistent").is_err());
    assert!(app.execute("perk nonexistent").is_err());
    assert!(app.execute("delve 0").is_err());
    assert_eq!(app.world.kind, "hub");
}

#[test]
fn layout_handles_small_and_large_terminals_and_cyrillic() {
    let data = TestData::new();
    let app = App::new(&data.options(), None).unwrap();
    for (width, height) in [(20, 8), (45, 15), (100, 35), (180, 50)] {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| crate::ui::draw(frame, &app)).unwrap();
        let rendered = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(rendered.contains(if width < 45 {
            "Увеличьте"
        } else {
            "OPENHEART"
        }));
    }
}

#[test]
fn content_hash_changes_when_nested_content_changes() {
    let data = TestData::new();
    let root = data.0.join("preset");
    std::fs::create_dir_all(root.join("maps")).unwrap();
    std::fs::write(root.join("preset.json"), "{}").unwrap();
    let first = crate::content::Content::load(&root).unwrap().hash;
    std::fs::write(root.join("maps/extra.json"), "{}").unwrap();
    let second = crate::content::Content::load(&root).unwrap().hash;
    assert_ne!(first, second);
}

fn pump_until(app: &mut App, other: &mut App, predicate: impl Fn(&App, &App) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !predicate(app, other) {
        app.update(0.05).unwrap();
        other.update(0.05).unwrap();
        assert!(
            Instant::now() < deadline,
            "Сетевое условие не выполнено: {} / {}",
            app.status,
            other.status
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Real Go + core.wasm; opt in with prebuilt artifact paths, see terminal/README.md.
#[test]
#[ignore = "requires OH_TEST_SERVER and OH_TEST_CORE"]
fn go_host_two_clients_room_transfer_chat_and_cleanup() {
    let data = TestData::new();
    let mut options = data.options();
    options.server_bin = Some(std::env::var("OH_TEST_SERVER").unwrap().into());
    options.core_wasm = Some(std::env::var("OH_TEST_CORE").unwrap().into());
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    options.listen = addr.to_string();
    let (host, url, _) = crate::host::Host::start(&options).unwrap();
    let mut a = App::new(&options, Some(url.clone())).unwrap();
    options.nick = "friend".into();
    let mut b = App::new(&options, Some(url.clone())).unwrap();
    pump_until(&mut a, &mut b, |a, b| {
        a.snapshot.players.len() == 2 && b.snapshot.players.len() == 2
    });
    a.execute("say привет из терминала").unwrap();
    pump_until(&mut a, &mut b, |_, b| {
        b.messages.iter().any(|m| m.contains("привет из терминала"))
    });
    let owner = a.peer;
    a.execute("delve 2").unwrap();
    pump_until(&mut a, &mut b, |a, _| {
        a.world.kind == "delve" && !a.snapshot.enemies.is_empty()
    });
    b.execute(&format!("delve 2 {owner}")).unwrap();
    pump_until(&mut a, &mut b, |a, b| {
        a.snapshot.players.len() == 2 && b.snapshot.players.len() == 2 && b.world.kind == "delve"
    });
    assert_eq!(a.seed, b.seed);
    assert_eq!(a.world.floor, b.world.floor);
    a.execute("hub").unwrap();
    b.execute("hub").unwrap();
    pump_until(&mut a, &mut b, |a, b| {
        a.world.kind == "hub" && b.world.kind == "hub" && a.snapshot.players.len() == 2
    });
    let bad = crate::network::Network::connect(
        url,
        Hello {
            protocol: 1,
            game_version: "0.1.0-dev".into(),
            preset_id: "core".into(),
            content_hash: "wrong".into(),
            auth: Auth {
                mode: "open".into(),
                nickname: "bad".into(),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    let rejection = bad.rx.recv_timeout(Duration::from_secs(5)).unwrap();
    // Go can close before its queued rejection frame is flushed. Either way no welcome.
    assert!(
        matches!(rejection,Ok(ServerMessage::Reject(ref r)) if r.reason=="content")
            || rejection.is_err()
    );
    drop(bad);
    drop(a);
    drop(b);
    drop(host);
    assert!(std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_err());
}

#[test]
fn simulation_wire_fixture_remains_compatible() {
    let raw = include_str!("../../protocol/fixtures/welcome.json");
    assert!(matches!(
        openheart_core::protocol::parse_server(raw).unwrap(),
        ServerMessage::Welcome(_)
    ));
    let input = sim::Input {
        pos: Vec3::new(1.0, 1.1, 2.0),
        ..Default::default()
    };
    let text = openheart_core::protocol::encode("input", &input).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["d"]["pos"], serde_json::json!([1.0, 1.1, 2.0]));
}

#[test]
fn combat_uses_simulation_awards_xp_and_obeys_cooldown() {
    let data = TestData::new();
    let mut options = data.options();
    options.world = "delve".into();
    let mut app = App::new(&options, None).unwrap();
    let sim = app.local.as_mut().unwrap();
    sim.time = 10.0;
    let enemy = &mut sim.enemies[0];
    enemy.hp = 0.1;
    let id = enemy.id;
    let at = enemy.pos;
    sim.players.get_mut(&1).unwrap().pos = at;
    app.pos = at;
    app.update(0.05).unwrap();
    app.target = Some(id);
    let xp = app.state.xp;
    let level = app.state.level;
    app.attack().unwrap();
    assert!(app.state.xp > xp || app.state.level > level);
    let clip = app.arsenal.clips[app.arsenal.current.slot()];
    app.attack().unwrap();
    assert_eq!(app.arsenal.clips[app.arsenal.current.slot()], clip);
    app.update(0.05).unwrap();
    assert!(!app.snapshot.enemies.iter().any(|e| e.id == id));
}

#[test]
fn static_pickup_cannot_be_taken_twice_and_lore_rewards_once() {
    let data = TestData::new();
    let mut options = data.options();
    options.world = "delve".into();
    let mut app = App::new(&options, None).unwrap();
    let pickup = app.pickups.first().unwrap();
    app.pos = pickup.pos;
    let key = pickup.key.clone();
    app.pickup_nearby();
    assert!(app.collected.contains(&key));
    app.populate_points();
    assert!(!app.pickups.iter().any(|p| p.key == key));
    app.points = vec![crate::world::Point {
        event: openheart_core::worldgen::dungeon::DungeonEventSpawn {
            kind: "story_echo".into(),
            group: 0,
            step: 0,
            pos: app.pos,
        },
        used: false,
    }];
    app.use_point(0);
    let xp = app.state.xp;
    app.use_point(0);
    assert_eq!(app.state.xp, xp);
}

#[test]
fn restore_isolates_guest_profiles_from_solo() {
    let data = TestData::new();
    let options = data.options();
    let mut solo = App::new(&options, None).unwrap();
    solo.state.gold = 987;
    solo.save().unwrap();
    // No socket server is needed: loading a guest must not consume the solo save.
    let guest = App::new(&options, Some("ws://127.0.0.1:1/ws".into())).unwrap();
    assert_ne!(guest.state.gold, 987);
    assert_ne!(solo.save_path, guest.save_path);
}

#[test]
#[ignore = "requires OH_TEST_SERVER and OH_TEST_CORE"]
fn go_local_account_restores_character_after_reconnect() {
    let data = TestData::new();
    let mut options = data.options();
    options.login = Some("terminal-test".into());
    options.server_bin = Some(std::env::var("OH_TEST_SERVER").unwrap().into());
    options.core_wasm = Some(std::env::var("OH_TEST_CORE").unwrap().into());
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    options.listen = listener.local_addr().unwrap().to_string();
    drop(listener);
    let (host, url, _) = crate::host::Host::start(&options).unwrap();
    let mut character = App::new(&options, None).unwrap();
    character.state.gold = 456;
    let mut hello = Hello {
        protocol: 1,
        game_version: "0.1.0-dev".into(),
        preset_id: "core".into(),
        content_hash: character.content.hash.clone(),
        auth: Auth {
            mode: "local".into(),
            login: "terminal-test".into(),
            nickname: "Terminal".into(),
            password: "terminal-test-password".into(),
            register: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let connection = crate::network::Network::connect(url.clone(), hello.clone()).unwrap();
    wait_welcome(&connection);
    let save = openheart_core::protocol::Save {
        ver: openheart_core::save::SAVE_VERSION as i32,
        data: openheart_core::save::SaveData::from_game(
            &character.state,
            100.0,
            &character.arsenal,
        )
        .to_json()
        .unwrap(),
    };
    connection
        .raw(openheart_core::protocol::encode("save", &save).unwrap())
        .unwrap();
    connection
        .raw(
            openheart_core::protocol::encode("ping", &openheart_core::protocol::Ping { t: 123 })
                .unwrap(),
        )
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if matches!(
            connection
                .rx
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .unwrap(),
            ServerMessage::Pong(123)
        ) {
            break;
        }
        assert!(Instant::now() < deadline);
    }
    drop(connection);
    hello.auth.register = false;
    let reconnected = crate::network::Network::connect(url, hello).unwrap();
    let welcome = wait_welcome(&reconnected);
    let save = welcome.character.expect("server character missing");
    assert_eq!(
        openheart_core::save::SaveData::from_json(&save.data)
            .unwrap()
            .gold,
        456
    );
    drop(reconnected);
    drop(host);
}

fn wait_welcome(connection: &crate::network::Network) -> openheart_core::protocol::Welcome {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match connection
            .rx
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap()
        {
            ServerMessage::Welcome(w) => return w,
            ServerMessage::Reject(r) => panic!("Rejected: {r:?}"),
            _ => {}
        }
        assert!(Instant::now() < deadline, "No welcome");
    }
}
