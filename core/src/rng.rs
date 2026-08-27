//! Детерминированный генератор случайных чисел (xorshift64*).
//!
//! Перенесён из `support/gfx.rs` без изменения алгоритма: от него зависит
//! генерация данжей, а сервер и клиент обязаны получать одинаковый мир из
//! одинакового сида (docs/MULTIPLAYER.md §5).

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    /// [0, n)
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        (self.next() >> 33) as u32 % n
    }

    /// [lo, hi] включительно
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + self.below((hi - lo + 1) as u32) as i32
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u32) as usize]
    }

    pub fn f32(&mut self) -> f32 {
        ((self.next() >> 40) as f32) / ((1u64 << 24) as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Один сид — одна последовательность. Если этот тест упал, все существующие
    /// данжи сгенерируются иначе, а клиент и сервер разойдутся.
    #[test]
    fn deterministic() {
        let mut a = Rng::new(0xDEAD_BEEF);
        let mut b = Rng::new(0xDEAD_BEEF);
        for _ in 0..1000 {
            assert_eq!(a.next(), b.next());
        }
    }

    #[test]
    fn ranges_are_bounded() {
        let mut rng = Rng::new(7);
        for _ in 0..10_000 {
            let v = rng.below(10);
            assert!(v < 10);
            let r = rng.range(-3, 3);
            assert!((-3..=3).contains(&r));
            let f = rng.f32();
            assert!((0.0..1.0).contains(&f));
        }
    }

    #[test]
    fn zero_seed_is_not_stuck() {
        let mut rng = Rng::new(0);
        assert_ne!(rng.next(), 0);
    }
}
