//! Статусы урона: контейнер активных эффектов, общий для врага и игрока.
//!
//! Виды (StatusCfg::kind): dot (периодический урон — горение/кровь),
//! slow (замедление), stun (оглушение), vulnerable (уязвимость: +входящий урон).
//! Определения — в statuses.json пресета; применяются оружием и способностями.

use godot::builtin::Color;

use crate::config::StatusCfg;
use crate::weapon::DmgType;

#[derive(Clone, Copy, PartialEq)]
pub enum StatusKind { Dot, Slow, Stun, Vulnerable }

impl StatusKind {
    pub fn from_id(s: &str) -> Option<Self> {
        Some(match s {
            "dot"        => Self::Dot,
            "slow"       => Self::Slow,
            "stun"       => Self::Stun,
            "vulnerable" => Self::Vulnerable,
            _ => return None,
        })
    }
}

struct Active {
    id:          String,
    name_ru:     String,
    kind:        StatusKind,
    remaining:   f32,
    // dot
    damage:      f32,
    dmg_type:    DmgType,
    tick_period: f32,
    tick_timer:  f32,
    // slow / vulnerable
    amount:      f32,
    tint:        Option<Color>,
    icon:        String,
}

#[derive(Default)]
pub struct StatusSet {
    active: Vec<Active>,
}

impl StatusSet {
    pub fn new() -> Self { Self { active: Vec::new() } }
    pub fn is_empty(&self) -> bool { self.active.is_empty() }
    pub fn clear(&mut self) { self.active.clear(); }

    /// Применить статус: у существующего того же вида обновляет длительность
    /// (refresh, не стакается бесконечно), иначе добавляет новый.
    pub fn apply(&mut self, cfg: &StatusCfg) {
        let kind = StatusKind::from_id(&cfg.kind).unwrap_or(StatusKind::Dot);
        if let Some(a) = self.active.iter_mut().find(|a| a.id == cfg.id) {
            a.remaining = a.remaining.max(cfg.duration);
            return;
        }
        let dmg_type = cfg.dmg_type.as_deref()
            .and_then(DmgType::from_id)
            .unwrap_or(DmgType::Physical);
        let tick_period = cfg.tick.max(0.1);
        self.active.push(Active {
            id: cfg.id.clone(),
            name_ru: cfg.name_ru.clone(),
            kind,
            remaining: cfg.duration.max(0.1),
            damage: cfg.damage,
            dmg_type,
            tick_period,
            tick_timer: tick_period,
            amount: cfg.amount,
            tint: cfg.tint.map(|t| Color::from_rgba(t[0], t[1], t[2], 1.0)),
            icon: cfg.icon.clone().unwrap_or_default(),
        });
    }

    /// Тик кадра: истекает длительности, копит DoT-урон.
    /// Возвращает (урон, тип) для каждого сработавшего DoT-тика.
    pub fn tick(&mut self, dt: f32) -> Vec<(f32, DmgType)> {
        let mut dots = Vec::new();
        for a in self.active.iter_mut() {
            a.remaining -= dt;
            if a.kind == StatusKind::Dot {
                a.tick_timer -= dt;
                if a.tick_timer <= 0.0 {
                    a.tick_timer += a.tick_period;
                    dots.push((a.damage, a.dmg_type));
                }
            }
        }
        self.active.retain(|a| a.remaining > 0.0);
        dots
    }

    /// Множитель скорости (сильнейшее замедление, не складываем; пол — 0.1).
    pub fn slow_mult(&self) -> f32 {
        let s = self.active.iter()
            .filter(|a| a.kind == StatusKind::Slow)
            .map(|a| a.amount)
            .fold(0.0f32, f32::max);
        (1.0 - s).clamp(0.1, 1.0)
    }

    /// Множитель входящего урона (уязвимости складываются).
    pub fn vuln_mult(&self) -> f32 {
        1.0 + self.active.iter()
            .filter(|a| a.kind == StatusKind::Vulnerable)
            .map(|a| a.amount)
            .sum::<f32>()
    }

    pub fn stunned(&self) -> bool {
        self.active.iter().any(|a| a.kind == StatusKind::Stun)
    }

    /// Цвет-подсветка от первого активного статуса (для тинта спрайта).
    pub fn tint(&self) -> Option<Color> {
        self.active.iter().find_map(|a| a.tint)
    }

    /// Строка для HUD: иконки или названия через пробел.
    pub fn summary(&self) -> String {
        self.active.iter()
            .map(|a| if a.icon.is_empty() { a.name_ru.clone() } else { a.icon.clone() })
            .collect::<Vec<_>>()
            .join(" ")
    }
}
