//! Доступ к данным пресета.
//!
//! В клиенте файлы читает `FileAccess` движка (`res://`, `user://`), на сервере —
//! обычная файловая система, в wasm — заранее переданный набор строк. Ядру всё равно:
//! оно ходит через [`ContentSource`].

use std::collections::BTreeMap;
use std::sync::RwLock;

/// Источник данных пресета. Пути — относительные, со слэшами (`weapons/weapons.json`).
pub trait ContentSource {
    /// Прочитать файл целиком; None — файла нет.
    fn read(&self, rel_path: &str) -> Option<String>;

    /// Перечислить файлы каталога (без рекурсии), пути — относительно корня пресета.
    fn list(&self, rel_dir: &str) -> Vec<String>;
}

/// Источник поверх обычной файловой системы (сервер, тесты, инструменты).
pub struct FsContent {
    root: std::path::PathBuf,
}

impl FsContent {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl ContentSource for FsContent {
    fn read(&self, rel_path: &str) -> Option<String> {
        std::fs::read_to_string(self.root.join(rel_path)).ok()
    }

    fn list(&self, rel_dir: &str) -> Vec<String> {
        let dir = self.root.join(rel_dir);
        let mut out = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    out.push(if rel_dir.is_empty() {
                        name
                    } else {
                        format!("{rel_dir}/{name}")
                    });
                }
            }
        }
        out.sort();
        out
    }
}

/// Источник в памяти: набор данных, переданный хостом (wasm) или собранный в тесте.
#[derive(Default)]
pub struct MemContent {
    files: BTreeMap<String, String>,
}

impl MemContent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, rel_path: impl Into<String>, body: impl Into<String>) {
        self.files.insert(rel_path.into(), body.into());
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

impl ContentSource for MemContent {
    fn read(&self, rel_path: &str) -> Option<String> {
        self.files.get(rel_path).cloned()
    }

    fn list(&self, rel_dir: &str) -> Vec<String> {
        let prefix = if rel_dir.is_empty() {
            String::new()
        } else {
            format!("{rel_dir}/")
        };
        self.files
            .keys()
            .filter(|key| {
                key.starts_with(&prefix) && !key[prefix.len()..].contains('/')
            })
            .cloned()
            .collect()
    }
}
// ── Глобальный читатель файлов ───────────────────────────────────────────────
//
// Загрузчики контента ходят по путям вида `res://presets/core` (движок) или
// `./game/presets/core` (сервер). Кто именно читает файл — решает хост: он ставит
// читатель один раз при старте, как и приёмник предупреждений в [`crate::log`].

type Reader = fn(&str) -> Option<String>;

static READER: RwLock<Option<Reader>> = RwLock::new(None);

/// Поставить читателя файлов. Клиент передаёт чтение движку (`FileAccess`),
/// сервер — обычной файловой системе.
pub fn set_reader(reader: Reader) {
    if let Ok(mut slot) = READER.write() {
        *slot = Some(reader);
    }
}

/// Прочитать файл целиком. Без установленного читателя ядро падает на `std::fs` —
/// этого достаточно для тестов и инструментов.
pub fn read_file(path: &str) -> Option<String> {
    let reader = READER.read().ok().and_then(|slot| *slot);
    match reader {
        Some(reader) => reader(path),
        None => std::fs::read_to_string(path).ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mem_lists_only_own_level() {
        let mut mem = MemContent::new();
        mem.insert("weapons/weapons.json", "[]");
        mem.insert("weapons/mods/mods.json", "[]");
        mem.insert("classes.json", "[]");

        assert_eq!(mem.list("weapons"), vec!["weapons/weapons.json"]);
        assert_eq!(mem.list(""), vec!["classes.json"]);
        assert_eq!(mem.read("classes.json").as_deref(), Some("[]"));
        assert!(mem.read("нет.json").is_none());
    }
}
