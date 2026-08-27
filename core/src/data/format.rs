//! Форматы файлов пресета: RON / JSON / TOML.
//!
//! Разбор общий для клиента и сервера; сам файл читает хост через
//! [`crate::content::read_file`].

use serde::de::DeserializeOwned;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataFormat {
    Ron,
    Json,
    Toml,
}

pub struct DataFile {
    pub path: String,
    pub text: String,
    pub format: DataFormat,
}

fn read(path: &str) -> Option<String> {
    crate::content::read_file(path)
}

pub fn find(base: &str, stem: &str) -> Option<DataFile> {
    for (extension, format) in [
        ("ron", DataFormat::Ron),
        ("json", DataFormat::Json),
        ("toml", DataFormat::Toml),
    ] {
        let path = format!("{base}/{stem}.{extension}");
        if let Some(text) = read(&path) {
            return Some(DataFile { path, text, format });
        }
    }
    None
}

pub fn parse<T: DeserializeOwned>(file: &DataFile) -> Result<T, String> {
    let result = match file.format {
        DataFormat::Ron => ron::from_str(&file.text).map_err(|error| error.to_string()),
        DataFormat::Json => serde_json::from_str(&file.text).map_err(|error| error.to_string()),
        DataFormat::Toml => toml::from_str(&file.text).map_err(|error| error.to_string()),
    };
    result.map_err(|error| format!("{}: {error}", file.path))
}

pub fn normalized_json(base: &str, stem: &str) -> Option<Result<String, String>> {
    find(base, stem).map(|file| match file.format {
        DataFormat::Json => Ok(file.text),
        _ => parse::<serde_json::Value>(&file)
            .and_then(|value| serde_json::to_string(&value).map_err(|error| error.to_string())),
    })
}

#[cfg(test)]
mod tests {
    use super::{parse, DataFile, DataFormat};
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Sample {
        id: String,
        damage: f32,
    }

    #[test]
    fn parses_supported_formats() {
        let cases = [
            (DataFormat::Json, r#"{"id":"blade","damage":12.0}"#),
            (DataFormat::Ron, r#"(id:"blade",damage:12.0)"#),
            (DataFormat::Toml, "id = \"blade\"\ndamage = 12.0"),
        ];
        for (format, text) in cases {
            let file = DataFile {
                path: "memory".into(),
                text: text.into(),
                format,
            };
            assert_eq!(
                parse::<Sample>(&file).unwrap(),
                Sample {
                    id: "blade".into(),
                    damage: 12.0
                }
            );
        }
    }

    #[test]
    fn migrated_core_weapon_content_round_trips() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../game/presets/core/weapons.ron");
        let text = std::fs::read_to_string(&path).unwrap();
        let file = DataFile {
            path: path.display().to_string(),
            text,
            format: DataFormat::Ron,
        };
        let value = parse::<serde_json::Value>(&file).unwrap();
        let json = serde_json::to_string(&value).unwrap();
        let weapons = crate::weapon::parse(&json).unwrap();
        assert_eq!(weapons.len(), 8);
    }
}
