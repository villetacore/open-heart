//! OpenHeart — DOOM-style 3D квест. GDExtension на Rust (gdext).
//!
//! Исходники разложены по доменным папкам (nodes / worldgen / combat / data /
//! state / support). Чтобы не тащить длинные пути через весь код, модули
//! ре-экспортируются в корень крейта — поэтому `crate::game`, `crate::enemy`,
//! `crate::config` и т.д. работают как раньше, независимо от того, в какой
//! папке лежит файл.

use godot::prelude::*;

// ── Доменные папки ────────────────────────────────────────────────────────────
mod nodes;
mod worldgen;
mod combat;
mod data;
mod state;
mod support;

// ── Ре-экспорт модулей в корень (сохраняет пути crate::<module>) ───────────────
pub use nodes::{enemy, game, main_menu, player};
pub use worldgen::{dungeon, map, nav, world};
pub use combat::{perk, status, weapon};
pub use data::{character, classes, config, content, dialogue, item, npc, quest, story};
pub use state::{game_state, save, settings};
pub use support::{gfx, locale};

struct OpenHeart;

#[gdextension]
unsafe impl ExtensionLibrary for OpenHeart {}
