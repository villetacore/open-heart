//! Перевод между типами ядра и типами движка.
//!
//! Ядро считает на своей математике (`openheart_core::math`), движок — на своей.
//! Все переходы через эту границу собраны здесь, чтобы не расползались по коду.

use godot::builtin::{Color, Vector2, Vector3};
use openheart_core::math::{Vec2, Vec3};

/// Позиция движка → позиция ядра.
#[inline]
pub fn to_core(v: Vector3) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

/// Позиция ядра → позиция движка.
#[inline]
pub fn to_godot(v: Vec3) -> Vector3 {
    Vector3::new(v.x, v.y, v.z)
}

#[inline]
pub fn vec2_to_core(v: Vector2) -> Vec2 {
    Vec2::new(v.x, v.y)
}

#[inline]
pub fn vec2_to_godot(v: Vec2) -> Vector2 {
    Vector2::new(v.x, v.y)
}

/// Цвет из данных пресета (ядро отдаёт RGB) → цвет движка.
#[inline]
pub fn color(rgb: [f32; 3]) -> Color {
    Color::from_rgba(rgb[0], rgb[1], rgb[2], 1.0)
}
