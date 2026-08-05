use std::fs;
use std::path::{Path, PathBuf};

const CONTENT_FILES: &[&str] = &[
    "abilities",
    "affixes",
    "classes",
    "dialogues",
    "dungeon",
    "enemies",
    "items",
    "level",
    "loot",
    "npcs",
    "perks",
    "quests",
    "statuses",
    "synergies",
    "weapons",
    "weapon_mods",
];

fn migrate_file(preset: &Path, stem: &str) -> Result<bool, String> {
    let source = preset.join(format!("{stem}.json"));
    if !source.exists() {
        return Ok(false);
    }
    let target = preset.join(format!("{stem}.ron"));
    let text = fs::read_to_string(&source).map_err(|error| error.to_string())?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", source.display()))?;
    let pretty = ron::ser::PrettyConfig::new()
        .depth_limit(8)
        .separate_tuple_members(true)
        .enumerate_arrays(true);
    let output = ron::ser::to_string_pretty(&value, pretty)
        .map_err(|error| format!("{}: {error}", target.display()))?;
    fs::write(&target, format!("{output}\n")).map_err(|error| error.to_string())?;
    Ok(true)
}

fn main() -> Result<(), String> {
    let preset = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: cargo run --bin preset_migrate -- <preset-directory>")?;
    let mut migrated = 0;
    for stem in CONTENT_FILES {
        if migrate_file(&preset, stem)? {
            migrated += 1;
            println!("migrated {stem}.json -> {stem}.ron");
        }
    }
    println!("done: {migrated} files migrated");
    Ok(())
}
