//! Манифест пресета: `preset.json` / `preset.toml`.
//!
//! Модель общая: клиент показывает пресеты в меню, сервер сверяет, какой пресет
//! у него активен, и отдаёт его id в анонсе.

use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct PresetInfo {
    pub id: String,
    #[serde(default = "schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub name_ru: String,
    #[serde(default)]
    pub name_en: String,
    #[serde(default)]
    pub desc_ru: String,
    #[serde(default)]
    pub desc_en: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub version: u32,
    #[serde(default = "preset_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub base: Option<String>,
    #[serde(default)]
    pub entry_map: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn preset_enabled() -> bool {
    true
}
fn schema_version() -> u32 {
    1
}
