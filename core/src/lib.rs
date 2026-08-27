//! openheart-core — правила игры без движка.
//!
//! Здесь всё, что считает игру: боевые формулы, модели контента, прогрессия,
//! навигация, симуляция комнаты. Крейт собирается двумя способами:
//!
//! * как обычная зависимость клиента (`openheart`, gdext) — тогда клиент и сервер
//!   считают по одному и тому же коду;
//! * как `core.wasm` (`--target wasm32-unknown-unknown --features abi`) — его грузит
//!   сервер через wazero и крутит авторитетный тик.
//!
//! Отсюда правило: **ничего из `godot::` здесь быть не может**. Математика своя
//! ([`math`]), файлы приходят строками, предупреждения уходят в хук ([`log`]).
//!
//! Модули ре-экспортированы в корень плоскими именами, поэтому и внутри ядра,
//! и в клиенте работают короткие пути: `openheart_core::weapon`, `crate::config`.

pub mod ai;
pub mod combat;
pub mod content;
pub mod craft;
pub mod data;
pub mod log;
pub mod math;
pub mod protocol;

#[cfg(test)]
mod protocol_fixtures_tests;
pub mod rng;
pub mod sim;
pub mod state;
pub mod worldgen;

#[cfg(feature = "abi")]
pub mod abi;

// ── Плоские имена модулей ────────────────────────────────────────────────────
pub use combat::{perk, status, weapon};
pub use data::{character, classes, config, dialogue, format, item, quest, story};
pub use state::{game_state, save};
pub use worldgen::{map_def as map, nav};

/// Версия сетевого протокола; должна совпадать с `proto.Version` в Go
/// и с `protocol/schema.md`.
pub const PROTOCOL_VERSION: i32 = 1;
