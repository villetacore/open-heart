//! Convert selected JSON content files into the RON files preferred by runtime.
//! cargo run -p openheart-core --example sync_preset -- game/presets/core quests dialogues
use std::{error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let directory = PathBuf::from(args.next().ok_or("expected preset directory")?);
    let stems: Vec<_> = args.collect();
    if stems.is_empty() {
        return Err("expected at least one content stem".into());
    }
    let mut outputs = Vec::new();
    for stem in stems {
        if stem.is_empty() || !stem.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return Err(format!("invalid content stem: {stem}").into());
        }
        let json = std::fs::read_to_string(directory.join(format!("{stem}.json")))?;
        let value: serde_json::Value = serde_json::from_str(&json)?;
        let ron = ron::ser::to_string_pretty(
            &value,
            ron::ser::PrettyConfig::default().enumerate_arrays(true),
        )?;
        let restored: serde_json::Value = ron::from_str(&ron)?;
        if value != restored {
            return Err(format!("round-trip changed {stem}").into());
        }
        outputs.push((directory.join(format!("{stem}.ron")), ron));
    }
    for (path, ron) in outputs {
        std::fs::write(&path, format!("{ron}\n"))?;
        println!("synced {}", path.display());
    }
    Ok(())
}
