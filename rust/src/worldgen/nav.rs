//! Навигация данжа: A* по сетке проходимости (floor_map генератора).
//!
//! Сетка локальна данжу (GRID×GRID клеток по CELL метров, центр в нуле);
//! мировые координаты переводятся вычитанием DUNGEON_OFFSET (nav_offset у врага).

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::Arc;

use godot::builtin::Vector3;

use crate::dungeon::{CELL, GRID};

pub struct NavGrid {
    cells:   Vec<bool>,   // true = проходимый пол
    heights: Vec<f32>,    // высота пола клетки: враги не умеют прыгать —
                          // рёбра между разными уровнями непроходимы
}

impl NavGrid {
    pub fn new(cells: Vec<bool>, heights: Vec<f32>) -> Arc<Self> {
        debug_assert_eq!(cells.len(), GRID * GRID);
        debug_assert_eq!(heights.len(), GRID * GRID);
        Arc::new(Self { cells, heights })
    }

    #[inline]
    fn walkable(&self, i: i32, j: i32) -> bool {
        i >= 0 && j >= 0 && (i as usize) < GRID && (j as usize) < GRID
            && self.cells[j as usize * GRID + i as usize]
    }

    /// Локальная позиция (данж-координаты, без DUNGEON_OFFSET) → клетка.
    /// Обратное к dungeon::cell_at: x = (i + 0.5 − GRID/2)·CELL.
    pub fn cell_of(pos: Vector3) -> (i32, i32) {
        (
            (pos.x / CELL + GRID as f32 * 0.5).floor() as i32,
            (pos.z / CELL + GRID as f32 * 0.5).floor() as i32,
        )
    }

    /// Центр клетки в локальных координатах данжа (y — на усмотрение вызывающего).
    pub fn center_of(i: i32, j: i32) -> Vector3 {
        Vector3::new(
            (i as f32 + 0.5 - GRID as f32 * 0.5) * CELL,
            0.0,
            (j as f32 + 0.5 - GRID as f32 * 0.5) * CELL,
        )
    }

    /// Ближайшая проходимая клетка к данной (сама клетка или сосед) — на случай,
    /// когда позиция чуть за краем пола (у стены, на ступени).
    fn snap(&self, (i, j): (i32, i32)) -> Option<(i32, i32)> {
        if self.walkable(i, j) { return Some((i, j)); }
        for (di, dj) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)] {
            if self.walkable(i + di, j + dj) { return Some((i + di, j + dj)); }
        }
        None
    }

    /// A* (4 направления, манхэттенская эвристика). Возвращает путь БЕЗ стартовой
    /// клетки, от первой промежуточной до цели включительно. None — пути нет.
    pub fn astar(&self, from: (i32, i32), to: (i32, i32)) -> Option<Vec<(i32, i32)>> {
        let from = self.snap(from)?;
        let to   = self.snap(to)?;
        if from == to { return Some(Vec::new()); }

        let idx = |(i, j): (i32, i32)| j as usize * GRID + i as usize;
        let h = |(i, j): (i32, i32)| ((i - to.0).abs() + (j - to.1).abs()) as u32;

        let mut g_cost  = vec![u32::MAX; GRID * GRID];
        let mut parent  = vec![usize::MAX; GRID * GRID];
        let mut open: BinaryHeap<Reverse<(u32, (i32, i32))>> = BinaryHeap::new();

        g_cost[idx(from)] = 0;
        open.push(Reverse((h(from), from)));

        while let Some(Reverse((_, cur))) = open.pop() {
            if cur == to { break; }
            let g_here = g_cost[idx(cur)];
            for (di, dj) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let nb = (cur.0 + di, cur.1 + dj);
                if !self.walkable(nb.0, nb.1) { continue; }
                // ступени между уровнями пола (0/0.8/1.6) враг не перелезет
                if (self.heights[idx(nb)] - self.heights[idx(cur)]).abs() > 0.05 { continue; }
                let g_new = g_here + 1;
                if g_new < g_cost[idx(nb)] {
                    g_cost[idx(nb)] = g_new;
                    parent[idx(nb)] = idx(cur);
                    open.push(Reverse((g_new + h(nb), nb)));
                }
            }
        }

        if g_cost[idx(to)] == u32::MAX { return None; }

        // восстановление пути от цели к старту
        let mut path = Vec::new();
        let mut cur = idx(to);
        let start = idx(from);
        while cur != start {
            path.push(((cur % GRID) as i32, (cur / GRID) as i32));
            cur = parent[cur];
            if path.len() > GRID * GRID { return None; } // страховка от цикла
        }
        path.reverse();
        Some(path)
    }
}
