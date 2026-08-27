//! Общие помощники тестов симуляции.

use std::path::Path;

use crate::sim::{Config, State};

/// Путь к core-пресету от манифеста крейта.
pub(crate) fn preset_base() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../game/presets/core")
        .to_string_lossy()
        .into_owned()
}

/// Комната-делв на реальных данных пресета.
pub(crate) fn delve_state(seed: u64) -> State {
    State::new(Config {
        preset: "core".into(),
        kind: "delve".into(),
        seed,
        depth: 2,
        party_size: 4,
        preset_base: preset_base(),
    })
}
