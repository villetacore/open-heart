//! Отпечаток набора данных пресета.
//!
//! Пресет и есть игра: если у клиента и сервера разные файлы, у одного дробовик
//! на 30 урона, у другого на 45. Поэтому обе стороны считают один и тот же хеш
//! и сверяют его при входе (docs/MULTIPLAYER.md §8.1).
//!
//! Алгоритм обязан совпадать с `server/internal/content` на Go:
//! файлы `*.json`, `*.toml`, `*.ron` берутся с относительными путями через `/`,
//! сортируются, и в sha256 уходит `путь + "\n" + содержимое без \r`.

use sha2::{Digest, Sha256};

/// Посчитать хеш по парам (относительный путь, содержимое файла).
pub fn preset_hash(files: &[(String, String)]) -> String {
    let mut sorted: Vec<&(String, String)> = files.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));

    let mut digest = Sha256::new();
    for (path, body) in sorted {
        digest.update(path.as_bytes());
        digest.update(b"\n");
        // Перевод строки нормализуем: git на Windows не должен менять хеш.
        for chunk in body.as_bytes().split(|byte| *byte == b'\r') {
            digest.update(chunk);
        }
    }
    hex(&digest.finalize())
}

/// Учитывается ли файл в отпечатке пресета.
pub fn is_content_file(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".json") || lower.ends_with(".toml") || lower.ends_with(".ron")
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_does_not_matter_but_content_does() {
        let a = preset_hash(&[
            ("weapons.json".into(), "[1]".into()),
            ("classes.json".into(), "[2]".into()),
        ]);
        let b = preset_hash(&[
            ("classes.json".into(), "[2]".into()),
            ("weapons.json".into(), "[1]".into()),
        ]);
        assert_eq!(a, b, "порядок файлов не должен влиять");

        let c = preset_hash(&[
            ("weapons.json".into(), "[1]".into()),
            ("classes.json".into(), "[3]".into()),
        ]);
        assert_ne!(a, c, "изменение данных обязано менять хеш");
    }

    #[test]
    fn line_endings_are_normalized() {
        let unix = preset_hash(&[("a.json".into(), "{\n  \"x\": 1\n}".into())]);
        let windows = preset_hash(&[("a.json".into(), "{\r\n  \"x\": 1\r\n}".into())]);
        assert_eq!(unix, windows, "перевод строки не должен менять хеш");
    }

    #[test]
    fn extensions_are_filtered() {
        assert!(is_content_file("weapons.json"));
        assert!(is_content_file("preset.TOML"));
        assert!(is_content_file("maps/hub.ron"));
        assert!(!is_content_file("sprite.png"));
    }
}
