//! Отпечаток пресета обязан совпасть с тем, что считает сервер на Go.
//!
//! Здесь Rust читает тот же каталог с диска и сравнивает результат с эталоном,
//! записанным Go-реализацией (`server/internal/content`). Разойдутся алгоритмы —
//! клиент перестанет пускаться на сервер, и лучше узнать об этом тестом.

use std::path::{Path, PathBuf};

use crate::data::hash::{is_content_file, preset_hash};

/// Значение, посчитанное `server/internal/content.Hash` для `game/presets/core`.
/// Меняется вместе с данными пресета — тогда обнови и его.
const CORE_PRESET_HASH: &str = "463c712d3715404ac2e4c873050c8bd08c4b80ab1898a8452439a18ec349bdd9";

fn collect(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, out);
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !is_content_file(&name) {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if let Ok(body) = std::fs::read_to_string(&path) {
            out.push((rel, body));
        }
    }
}

#[test]
fn core_preset_hash_matches_go() {
    let root: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "game", "presets", "core"]
        .iter()
        .collect();
    let mut files = Vec::new();
    collect(&root, &root, &mut files);

    assert!(!files.is_empty(), "не нашлись файлы пресета в {root:?}");
    assert_eq!(
        preset_hash(&files),
        CORE_PRESET_HASH,
        "хеш разошёлся с Go-реализацией: клиент не сможет войти на сервер"
    );
}
