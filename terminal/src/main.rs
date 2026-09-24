mod app;
mod content;
mod gameplay;
mod host;
mod network;
mod options;
#[cfg(test)]
mod tests;
mod ui;
mod world;

use anyhow::{ensure, Result};
use clap::Parser;
use options::Options;
use std::{
    io::{self, IsTerminal, Write},
    time::{Duration, Instant},
};

fn main() {
    if let Err(error) = run() {
        eprintln!("oh-terminal: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut options = Options::parse();
    if options.presets == std::path::Path::new("game/presets") && !options.presets.is_dir() {
        if let Some(parent) = std::env::current_exe()?.parent() {
            if parent.join("presets").is_dir() {
                options.presets = parent.join("presets");
            }
        }
    }
    ensure!(
        !options.preset.is_empty()
            && options
                .preset
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'),
        "ID пресета: буквы, цифры, _ или -"
    );
    ensure!(
        options.headless || io::stdin().is_terminal() && io::stdout().is_terminal(),
        "Нужен интерактивный терминал; для проверки используйте --headless --offline"
    );
    if !options.offline && !options.host && options.server.is_none() {
        if options.headless {
            options.offline = true;
        } else {
            println!("OPENHEART / TERMINAL\n\n1. Одиночная игра\n2. Поднять Go-сервер и играть\n3. Подключиться к серверу\n4. Выход");
            print!("> ");
            io::stdout().flush()?;
            let mut choice = String::new();
            io::stdin().read_line(&mut choice)?;
            match choice.trim() {
                "1" => options.offline = true,
                "2" => options.host = true,
                "3" => {
                    print!("Адрес (ws://host:7777/ws): ");
                    io::stdout().flush()?;
                    let mut url = String::new();
                    io::stdin().read_line(&mut url)?;
                    options.server = Some(url.trim().into());
                }
                _ => return Ok(()),
            }
        }
    }
    // Validate content before starting a child process or entering alternate screen.
    let _ = content::Content::load(&options.preset_base())?;
    let mut owned_host = None;
    let mut party_code = String::new();
    if options.host {
        let (host, url, code) = host::Host::start(&options)?;
        owned_host = Some(host);
        options.server = Some(url);
        party_code = code;
    }
    let mut app = app::App::new(&options, options.server.clone())?;
    if !party_code.is_empty() {
        app.status = format!("{} · код пати: {}", app.status, party_code);
    }
    let result = if options.headless {
        headless(&mut app, options.seconds)
    } else {
        interactive(&mut app)
    };
    // Flush save and close the socket before stopping our own child.
    let saved = app.save();
    app.network.take();
    drop(owned_host);
    result?;
    saved?;
    Ok(())
}

fn interactive(app: &mut app::App) -> Result<()> {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ui::restore();
        previous(info);
    }));
    let mut screen = ui::Screen::new()?;
    let mut next = Instant::now();
    while !app.quit {
        ui::input(app)?;
        let now = Instant::now();
        if now >= next {
            app.update(0.05)?;
            screen.terminal.draw(|frame| ui::draw(frame, app))?;
            next = now + Duration::from_millis(50);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}

fn headless(app: &mut app::App, seconds: u64) -> Result<()> {
    let connect_deadline = Instant::now() + Duration::from_secs(15);
    while !app.ready {
        app.update(0.05)?;
        ensure!(
            Instant::now() < connect_deadline,
            "Нет welcome за 15 секунд"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    for _ in 0..seconds * 20 {
        app.update(0.05)?;
        std::thread::sleep(Duration::from_millis(50));
    }
    ensure!(app.snapshots_received > 0, "Сервер не прислал снапшоты");
    println!(
        "OK peer={} world={} snapshots={} tick={} players={} enemies={} hash={}",
        app.peer,
        app.world.kind,
        app.snapshots_received,
        app.snapshot.tick,
        app.snapshot.players.len(),
        app.snapshot.enemies.len(),
        app.content.hash
    );
    Ok(())
}
