use crate::app::App;
use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use openheart_core::{
    math::{Vec2, Vec3},
    weapon,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io::{self, Stdout};

pub struct Screen {
    pub terminal: Terminal<CrosstermBackend<Stdout>>,
}
impl Screen {
    pub fn new() -> Result<Self> {
        terminal::enable_raw_mode()?;
        // Install cleanup before every fallible initialization step.
        let result = (|| {
            execute!(io::stdout(), EnterAlternateScreen, cursor::Hide)?;
            Ok(Self {
                terminal: Terminal::new(CrosstermBackend::new(io::stdout()))?,
            })
        })();
        if result.is_err() {
            restore();
        }
        result
    }
}
pub fn restore() {
    let _ = terminal::disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, cursor::Show);
}
impl Drop for Screen {
    fn drop(&mut self) {
        restore();
    }
}

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    if area.width < 45 || area.height < 15 {
        frame.render_widget(
            Paragraph::new("Увеличьте терминал до 45×15. Ctrl+C — выход."),
            area,
        );
        return;
    }
    let regions = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(5),
            Constraint::Length(5),
            Constraint::Length(2),
        ])
        .split(area);
    let weapon = weapon::weapon_def(app.arsenal.current);
    let ammo = weapon
        .ammo
        .map(|(kind, _)| {
            format!(
                "{}/{}",
                app.arsenal.clips[weapon.id.slot()],
                app.arsenal.ammo_of(kind)
            )
        })
        .unwrap_or_else(|| "∞".into());
    let header = format!(
        "OPENHEART / TERMINAL · {} · HP {}%{} · LV {} XP {}\n{} [{}]",
        app.world.kind,
        app.hp,
        if app.downed { " DOWN" } else { "" },
        app.state.level,
        app.state.xp,
        weapon.name("ru"),
        ammo
    );
    frame.render_widget(
        Paragraph::new(clean(&header))
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL).title(if app.ready {
                clean(&app.status)
            } else {
                "Подключение…".into()
            })),
        regions[0],
    );
    let cols = if app.panel.is_empty() {
        vec![regions[1]]
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(56), Constraint::Percentage(44)])
            .split(regions[1])
            .to_vec()
    };
    draw_map(frame, app, cols[0]);
    if cols.len() > 1 {
        frame.render_widget(
            Paragraph::new(clean(&app.panel_text()))
                .wrap(Wrap { trim: false })
                .scroll((app.scroll, 0))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(" {} · PgUp/PgDn ", app.panel)),
                ),
            cols[1],
        );
    }
    let log: Vec<Line> = app
        .messages
        .iter()
        .rev()
        .take(3)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|s| Line::from(s.clone()))
        .collect();
    frame.render_widget(
        Paragraph::new(log).block(Block::default().borders(Borders::ALL).title(" События ")),
        regions[2],
    );
    let bottom = if let Some(line) = &app.command_line {
        format!(":{line}▏")
    } else {
        "WASD ходить · Space бой · E действие · Tab цель · : команда · F1 помощь".into()
    };
    frame.render_widget(
        Paragraph::new(clean(&bottom)).style(Style::default().fg(Color::Yellow)),
        regions[3],
    );
}

fn clean(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || *c == '\n')
        .collect()
}

fn draw_map(frame: &mut Frame, app: &App, area: Rect) {
    let target = app
        .target
        .and_then(|id| app.snapshot.enemies.iter().find(|e| e.id == id))
        .map(|e| {
            format!(
                " · цель #{} HP {}% {:.0}м",
                e.id,
                e.hp,
                (e.pos - app.pos).length()
            )
        })
        .unwrap_or_default();
    let block = Block::default().borders(Borders::ALL).title(format!(
        " x {:.1} z {:.1} · этаж {}{} ",
        app.pos.x, app.pos.z, app.depth, target
    ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let width = inner.width as usize;
    let height = inner.height as usize;
    if width == 0 || height == 0 {
        return;
    }
    let mut cells = vec![('.', Color::DarkGray); width * height];
    for y in 0..height {
        for x in 0..width {
            let pos = Vec3::new(
                app.pos.x + (x as f32 - width as f32 / 2.0) * app.zoom * 0.5,
                app.pos.y,
                app.pos.z + (y as f32 - height as f32 / 2.0) * app.zoom,
            );
            if !app.walkable(pos) {
                cells[y * width + x] = ('#', Color::Rgb(70, 95, 120));
            }
        }
    }
    let mut put = |pos: Vec3, ch: char, color: Color| {
        let x = ((pos.x - app.pos.x) / (app.zoom * 0.5) + width as f32 / 2.0).round() as i32;
        let y = ((pos.z - app.pos.z) / app.zoom + height as f32 / 2.0).round() as i32;
        if x >= 0 && y >= 0 && x < width as i32 && y < height as i32 {
            cells[y as usize * width + x as usize] = (ch, color);
        }
    };
    if app.world.kind == "hub" {
        for n in &app.content.cfg.npcs {
            put(Vec3::new(n.pos[0], 0.0, n.pos[1]), 'N', Color::Yellow);
        }
        if let Some(gate) = app.content.map.as_ref().and_then(|m| m.gate) {
            put(gate.into(), '>', Color::Magenta);
        }
    }
    for e in &app.snapshot.items {
        put(e.pos, '$', Color::Green);
    }
    for p in &app.pickups {
        put(p.pos, '$', Color::Green);
    }
    for p in app.points.iter().filter(|p| !p.used) {
        put(
            p.event.pos,
            if p.event.kind == "story_echo" {
                '?'
            } else {
                char::from_digit(p.event.step + 1, 10).unwrap_or('!')
            },
            Color::Yellow,
        );
    }
    if app.world.kind == "delve" {
        put(
            app.world.spawns.next_portal + app.world.offset,
            '>',
            Color::Magenta,
        );
        put(
            app.world.spawns.exit_portal + app.world.offset,
            '<',
            Color::Magenta,
        );
    }
    for e in &app.snapshot.projectiles {
        put(e.pos, 'o', Color::Yellow);
    }
    for e in &app.snapshot.enemies {
        put(
            e.pos,
            if Some(e.id) == app.target {
                '*'
            } else if e.flags & openheart_core::sim::flags::ELITE != 0 {
                'E'
            } else {
                'e'
            },
            Color::Red,
        );
    }
    for p in &app.snapshot.players {
        if p.id != app.peer {
            put(p.pos, '&', Color::Blue);
        }
    }
    put(app.pos, '@', Color::Cyan);
    let lines: Vec<Line> = cells
        .chunks(width)
        .map(|row| {
            Line::from(
                row.iter()
                    .map(|(ch, color)| Span::styled(ch.to_string(), Style::default().fg(*color)))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

pub fn input(app: &mut App) -> Result<()> {
    // Bound input processing so pastes/repeats cannot starve the 20 Hz simulation.
    for _ in 0..64 {
        if !event::poll(std::time::Duration::ZERO)? {
            break;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind == KeyEventKind::Release {
            continue;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            app.quit = true;
            break;
        }
        if let Some(line) = app.command_line.as_mut() {
            match key.code {
                KeyCode::Esc => app.command_line = None,
                KeyCode::Backspace => {
                    line.pop();
                }
                KeyCode::Char(c) if line.len() < 1024 => line.push(c),
                KeyCode::Enter => {
                    let line = app.command_line.take().unwrap();
                    if let Err(e) = app.execute(&line) {
                        app.log(e.to_string());
                    }
                }
                _ => {}
            }
            continue;
        }
        let action: Result<()> = (|| {
            match key.code {
                KeyCode::Char(':') => {
                    app.command_line = Some(String::new());
                    app.move_left = 0.0;
                }
                KeyCode::Esc => {
                    app.panel.clear();
                    app.scene = None;
                    app.move_left = 0.0;
                }
                KeyCode::F(1) => {
                    app.panel = "help".into();
                    app.scroll = 0;
                }
                KeyCode::PageDown => app.scroll = app.scroll.saturating_add(8),
                KeyCode::PageUp => app.scroll = app.scroll.saturating_sub(8),
                KeyCode::Enter => app.advance_scene(),
                KeyCode::Char(c @ '1'..='9') if app.scene.is_some() => {
                    app.choose(c.to_digit(10).unwrap() as usize)?
                }
                KeyCode::Char('w') | KeyCode::Up => movement(app, 0.0, -1.0),
                KeyCode::Char('s') | KeyCode::Down => movement(app, 0.0, 1.0),
                KeyCode::Char('a') | KeyCode::Left => movement(app, -1.0, 0.0),
                KeyCode::Char('d') | KeyCode::Right => movement(app, 1.0, 0.0),
                KeyCode::Char(' ') => app.attack()?,
                KeyCode::Tab => app.cycle_target(),
                KeyCode::Char('r') => app.reload(),
                KeyCode::Char('e') => app.interact()?,
                KeyCode::Char('q') => app.game_command("ability", serde_json::json!({"slot":0}))?,
                KeyCode::Char('f') => app.game_command("ability", serde_json::json!({"slot":1}))?,
                KeyCode::Char(c @ '1'..='8') => {
                    let w = weapon::WeaponId::from_slot(c.to_digit(10).unwrap() as usize - 1);
                    if app.arsenal.has(w) {
                        app.arsenal.current = w;
                    }
                }
                KeyCode::Char('i') => {
                    app.panel = "inventory".into();
                    app.scroll = 0;
                }
                KeyCode::Char('j') => {
                    app.panel = "quests".into();
                    app.scroll = 0;
                }
                KeyCode::Char('p') => {
                    app.panel = "perks".into();
                    app.scroll = 0;
                }
                KeyCode::Char('c') => {
                    app.panel = "craft".into();
                    app.scroll = 0;
                }
                KeyCode::Char('n') => {
                    app.panel = "npcs".into();
                    app.scroll = 0;
                }
                KeyCode::Char('+') | KeyCode::Char('=') => app.zoom = (app.zoom * 0.8).max(0.3),
                KeyCode::Char('-') => app.zoom = (app.zoom * 1.25).min(8.0),
                _ => {}
            }
            Ok(())
        })();
        if let Err(error) = action {
            app.log(error.to_string());
        }
    }
    Ok(())
}
fn movement(app: &mut App, x: f32, y: f32) {
    if app.scene.is_none() {
        app.move_dir = Vec2::new(x, y);
        app.move_left = 0.16;
    }
}
