//! Минимальная математика ядра: то, что в клиенте даёт `godot::builtin`.
//!
//! Конвертация в типы движка живёт на стороне gdext (`From`/`Into` в клиентском слое),
//! чтобы ядро оставалось свободным от зависимости на движок.

use serde::{Deserialize, Serialize};

/// Трёхмерный вектор. В JSON — массив `[x, y, z]`, как в protocol/schema.md.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(from = "[f32; 3]", into = "[f32; 3]")]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn distance(self, other: Self) -> f32 {
        (self - other).length()
    }

    /// Расстояние по горизонтали: высота в геймплейных проверках чаще мешает.
    pub fn distance_flat(self, other: Self) -> f32 {
        let dx = self.x - other.x;
        let dz = self.z - other.z;
        (dx * dx + dz * dz).sqrt()
    }

    pub fn normalized(self) -> Self {
        let len = self.length();
        if len <= f32::EPSILON {
            Self::ZERO
        } else {
            Self::new(self.x / len, self.y / len, self.z / len)
        }
    }

    /// Тот же вектор без вертикальной составляющей: враги и игроки ходят по полу,
    /// и высота в проверках дистанции только мешает.
    pub fn flat(self) -> Self {
        Self::new(self.x, 0.0, self.z)
    }

    /// Длина по горизонтали.
    pub fn length_flat(self) -> f32 {
        (self.x * self.x + self.z * self.z).sqrt()
    }

    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self::new(
            self.x + (other.x - self.x) * t,
            self.y + (other.y - self.y) * t,
            self.z + (other.z - self.z) * t,
        )
    }
}

impl From<[f32; 3]> for Vec3 {
    fn from(v: [f32; 3]) -> Self {
        Self::new(v[0], v[1], v[2])
    }
}

impl From<Vec3> for [f32; 3] {
    fn from(v: Vec3) -> Self {
        [v.x, v.y, v.z]
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, k: f32) -> Self {
        Self::new(self.x * k, self.y * k, self.z * k)
    }
}

/// Двумерный вектор: направление движения из ввода.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(from = "[f32; 2]", into = "[f32; 2]")]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

impl From<[f32; 2]> for Vec2 {
    fn from(v: [f32; 2]) -> Self {
        Self::new(v[0], v[1])
    }
}

impl From<Vec2> for [f32; 2] {
    fn from(v: Vec2) -> Self {
        [v.x, v.y]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec3_json_is_array() {
        let v = Vec3::new(1.0, 2.0, 3.5);
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "[1.0,2.0,3.5]");
        assert_eq!(serde_json::from_str::<Vec3>(&json).unwrap(), v);
    }

    #[test]
    fn distances() {
        let a = Vec3::new(0.0, 5.0, 0.0);
        let b = Vec3::new(3.0, 5.0, 4.0);
        assert_eq!(a.distance(b), 5.0);
        assert_eq!(a.distance_flat(b), 5.0);
    }
}
