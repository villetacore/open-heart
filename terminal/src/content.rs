use anyhow::{Context, Result};
use openheart_core::{
    config::GameConfig,
    data::{format, hash},
    map::MapDef,
};
use std::{fs, path::Path};

pub struct Content {
    pub cfg: GameConfig,
    pub map: Option<MapDef>,
    pub hash: String,
}

impl Content {
    pub fn load(root: &Path) -> Result<Self> {
        anyhow::ensure!(
            root.is_dir(),
            "Каталог пресета не найден: {}",
            root.display()
        );
        let mut files = Vec::new();
        collect(root, root, &mut files)?;
        anyhow::ensure!(!files.is_empty(), "Пресет пустой");
        let base = root.to_string_lossy();
        let read = |stem| {
            format::normalized_json(&base, stem)
                .transpose()
                .map_err(anyhow::Error::msg)
        };
        let weapons = read("weapons")?;
        let mods = read("weapon_mods")?;
        openheart_core::weapon::load(weapons.as_deref(), mods.as_deref());
        let classes = read("classes")?;
        openheart_core::classes::load(classes.as_deref());
        let perks = read("perks")?;
        let synergies = read("synergies")?;
        openheart_core::perk::load(perks.as_deref(), synergies.as_deref());
        let map = match fs::read_to_string(root.join("maps/hub.json")) {
            Ok(text) => Some(serde_json::from_str(&text).context("maps/hub.json")?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        Ok(Self {
            cfg: GameConfig::load_from(&base),
            map,
            hash: hash::preset_hash(&files),
        })
    }
}

fn collect(root: &Path, directory: &Path, files: &mut Vec<(String, String)>) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_dir() {
            collect(root, &path, files)?;
        } else if kind.is_file() && hash::is_content_file(&entry.file_name().to_string_lossy()) {
            files.push((
                path.strip_prefix(root)?
                    .to_string_lossy()
                    .replace('\\', "/"),
                fs::read_to_string(&path)?,
            ));
        }
    }
    Ok(())
}
