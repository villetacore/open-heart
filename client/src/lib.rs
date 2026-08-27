//! OpenHeart — клиент: расширение движка (GDExtension на gdext).
//!
//! Здесь только то, что знает про движок: узлы, рендер, ввод, HUD, файловый
//! ввод-вывод. Правила игры — в крейте `openheart-core` (`core/`), они же
//! исполняются сервером; см. docs/MULTIPLAYER.md §4.
//!
//! Модули ядра ре-экспортируются в корень крейта, поэтому `crate::weapon`,
//! `crate::config`, `crate::game_state` работают одинаково независимо от того,
//! в каком крейте лежит файл.

use godot::prelude::*;

// ── Клиентские папки ──────────────────────────────────────────────────────────
mod data;
mod net;
mod nodes;
mod state;
mod support;
mod worldgen;

// ── Правила игры из ядра ──────────────────────────────────────────────────────
pub use openheart_core::{
    character, classes, config, dialogue, format, game_state, item, nav, perk, quest, status,
    story, weapon,
};

// ── Клиентские модули ─────────────────────────────────────────────────────────
pub use data::content;
pub use net::NetClient;
pub use nodes::{enemy, game, main_menu, npc, player};
pub use state::{save, settings};
pub use support::{convert, gfx, locale};
pub use worldgen::{dungeon, map, world};

struct OpenHeart;

#[gdextension]
unsafe impl ExtensionLibrary for OpenHeart {
    fn on_stage_init(stage: InitStage) {
        if stage == InitStage::Scene {
            // Ядро не знает про движок: даём ему консоль и файлы движка.
            openheart_core::log::set_hook(|message| godot_warn!("{message}"));
            openheart_core::content::set_reader(|path| {
                use godot::classes::{file_access::ModeFlags, FileAccess};
                FileAccess::open(path, ModeFlags::READ).map(|file| file.get_as_text().to_string())
            });
        }
    }
}
