//! Мир комнаты: геометрия, по которой ходят враги, и точки появления.
//!
//! Для делва мир строится планом данжа из ядра — тем же, по которому клиент
//! рисует меши. Поэтому серверные враги ходят ровно там, где игрок видит пол.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::config::GameConfig;
use crate::math::Vec3;
use crate::nav::NavGrid;
use crate::worldgen::dungeon::{self, Spawns};
use crate::worldgen::GRID;

/// Смещение данжа относительно мира: клиент строит его в стороне от хаба,
/// чтобы координаты не пересекались. Сервер обязан знать то же смещение.
pub const DUNGEON_OFFSET: Vec3 = Vec3::new(500.0, 0.0, 500.0);

/// Виды комнат.
pub const KIND_HUB: &str = "hub";
pub const KIND_DELVE: &str = "delve";

/// Мир комнаты: где пол, куда можно идти, где появляются игроки и враги.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct World {
    pub kind: String,
    /// Смещение локальных координат сетки в мировые.
    pub offset: Vec3,
    /// GRID×GRID: проходимый пол (пусто для хаба — там сетки нет).
    pub floor: Vec<bool>,
    pub floor_heights: Vec<f32>,
    pub player_spawn: Vec3,
    /// Что должно появиться в комнате; враги забираются отсюда при старте.
    #[serde(skip)]
    pub spawns: Spawns,
    /// Навигационная сетка: восстанавливается из `floor`/`floor_heights`.
    #[serde(skip)]
    pub nav: Option<Arc<NavGrid>>,
}

impl World {
    /// Хаб: открытая площадка, сетки навигации нет — враги ходят напрямик.
    pub fn hub() -> Self {
        Self {
            kind: KIND_HUB.to_string(),
            offset: Vec3::ZERO,
            player_spawn: Vec3::new(0.0, 1.1, 10.0),
            ..Default::default()
        }
    }

    /// Делв: план данжа из ядра + навигационная сетка по нему.
    pub fn delve(depth: u32, seed: u64, cfg: &GameConfig) -> Self {
        let layout = dungeon::plan(depth.max(1), seed, cfg);
        let nav = NavGrid::new(layout.floor.clone(), layout.floor_heights.clone());
        Self {
            kind: KIND_DELVE.to_string(),
            offset: DUNGEON_OFFSET,
            player_spawn: layout.spawns.player_spawn + DUNGEON_OFFSET,
            floor: layout.floor,
            floor_heights: layout.floor_heights,
            spawns: layout.spawns,
            nav: Some(nav),
        }
    }

    /// Восстановить навигацию после загрузки сохранённого состояния.
    pub fn rebuild_nav(&mut self) {
        if self.nav.is_none() && self.floor.len() == GRID * GRID {
            self.nav = Some(NavGrid::new(self.floor.clone(), self.floor_heights.clone()));
        }
    }

    /// Высота пола под точкой (мировые координаты). Вне сетки — уровень мира.
    pub fn floor_at(&self, world_pos: Vec3) -> f32 {
        if self.floor_heights.len() != GRID * GRID {
            return 0.0;
        }
        let (i, j) = NavGrid::cell_of(world_pos - self.offset);
        if i < 0 || j < 0 || i as usize >= GRID || j as usize >= GRID {
            return 0.0;
        }
        self.floor_heights[j as usize * GRID + i as usize]
    }

    /// Проходима ли клетка под точкой.
    pub fn walkable_at(&self, world_pos: Vec3) -> bool {
        if self.floor.len() != GRID * GRID {
            return true; // в хабе сетки нет — считаем, что идти можно
        }
        let (i, j) = NavGrid::cell_of(world_pos - self.offset);
        if i < 0 || j < 0 || i as usize >= GRID || j as usize >= GRID {
            return false;
        }
        self.floor[j as usize * GRID + i as usize]
    }
}
