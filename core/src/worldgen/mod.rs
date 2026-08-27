//! Пространство мира: сетка данжа и навигация по ней.

pub mod dungeon;
pub mod map_def;
pub mod nav;

/// Размер клетки сетки данжа в метрах.
pub const CELL: f32 = 3.0;
/// Сторона сетки данжа в клетках.
pub const GRID: usize = 44;

#[cfg(test)]
mod dungeon_tests;
