//! OpenHeart — DOOM-style 3D квест. GDExtension на Rust (gdext).

use godot::prelude::*;

pub mod character;
pub mod config;
pub mod dialogue;
pub mod enemy;
pub mod game;
pub mod game_state;
pub mod item;
pub mod locale;
pub mod main_menu;
pub mod npc;
pub mod player;
pub mod quest;
pub mod save;
pub mod settings;
pub mod story;

struct OpenHeart;

#[gdextension]
unsafe impl ExtensionLibrary for OpenHeart {}
