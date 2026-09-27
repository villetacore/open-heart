//! Модели контента и его разбор: форматы файлов, конфиг пресета, классы,
//! персонажи, предметы, квесты, сюжет, диалоги.

pub mod character;
pub mod classes;
pub mod config;
pub mod dialogue;
pub mod format;
pub mod hash;
pub mod item;
pub mod preset;
pub mod quest;
pub mod story;

#[cfg(test)]
mod hash_disk_tests;
#[cfg(test)]
mod preset_tests;
#[cfg(test)]
mod recipe_tests;
#[cfg(test)]
mod evening_tests;
