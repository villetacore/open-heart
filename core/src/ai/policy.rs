//! Политика поведения врага: признаки → намерение.
//!
//! Сеть **не** водит врага по миру напрямую — она выбирает намерение
//! («догонять», «отойти», «обойти», «бить»), а исполняет его обычный код
//! движения. Так поведение остаётся отлаживаемым: видно, что именно выбрала
//! сеть, и всегда есть предсказуемый запасной вариант.
//!
//! Признаки нормализованы в [-1, 1] или [0, 1]: обучение на сырых метрах
//! разъезжается, стоит поменять масштаб карты.

use serde::{Deserialize, Serialize};

use crate::ai::net::Mlp;

/// Сколько признаков видит сеть. Менять только вместе с весами в пресете.
pub const FEATURES: usize = 8;

/// Намерения, между которыми выбирает сеть.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    /// Сближаться с целью.
    #[default]
    Chase,
    /// Держать дистанцию, отходя назад.
    Retreat,
    /// Двигаться боком, не сокращая дистанцию.
    Strafe,
    /// Атаковать (если позволяет дистанция и кулдаун).
    Attack,
    /// Стоять на месте: выжидать, восстанавливаться.
    Wait,
}

/// Порядок выходов сети совпадает с этим списком.
pub const INTENTS: [Intent; 5] = [
    Intent::Chase,
    Intent::Retreat,
    Intent::Strafe,
    Intent::Attack,
    Intent::Wait,
];

/// Что враг знает о ситуации в момент решения.
#[derive(Clone, Copy, Debug, Default)]
pub struct Situation {
    /// Дистанция до цели в метрах.
    pub distance: f32,
    /// Радиус, с которого враг может ударить.
    pub attack_range: f32,
    /// Радиус, дальше которого он теряет интерес.
    pub chase_range: f32,
    /// Своё здоровье, доля от максимума.
    pub hp_frac: f32,
    /// Здоровье цели, доля от максимума.
    pub target_hp_frac: f32,
    /// Готовность атаки: 1.0 — можно бить сейчас, 0.0 — только что ударил.
    pub attack_ready: f32,
    /// Сколько союзников рядом (нормируется внутри).
    pub allies_near: u32,
    /// Есть ли путь до цели по навигации.
    pub has_path: bool,
}

impl Situation {
    /// Признаки для сети. Порядок фиксирован: это часть формата весов.
    pub fn features(&self) -> [f32; FEATURES] {
        let chase = self.chase_range.max(1.0);
        [
            (self.distance / chase).clamp(0.0, 2.0) - 1.0,
            (self.distance / self.attack_range.max(0.5)).clamp(0.0, 4.0) / 4.0,
            self.hp_frac.clamp(0.0, 1.0) * 2.0 - 1.0,
            self.target_hp_frac.clamp(0.0, 1.0) * 2.0 - 1.0,
            self.attack_ready.clamp(0.0, 1.0),
            (self.allies_near as f32 / 4.0).clamp(0.0, 1.0),
            if self.has_path { 1.0 } else { -1.0 },
            // Смещение: даёт сети постоянный «фон», обучение с ним стабильнее.
            1.0,
        ]
    }
}

/// Обученное поведение одного вида врагов.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Brain {
    pub id: String,
    pub net: Mlp,
}

impl Brain {
    /// Проверить, что веса подходят под текущий формат признаков и намерений.
    pub fn validate(&self) -> Result<(), crate::ai::net::NetError> {
        self.net.validate(FEATURES, INTENTS.len())
    }

    /// Выбрать намерение. Детерминированно: одинаковая ситуация — одинаковый ответ.
    pub fn decide(&self, situation: &Situation) -> Intent {
        let output = self.net.forward(&situation.features());
        INTENTS[Mlp::argmax(&output).min(INTENTS.len() - 1)]
    }
}

/// Запасное поведение без сети — то же, что враги делали всегда.
///
/// Работает, когда у вида нет обученных весов: сначала подойти, потом бить.
pub fn fallback_intent(situation: &Situation) -> Intent {
    if situation.distance <= situation.attack_range {
        if situation.attack_ready >= 1.0 {
            Intent::Attack
        } else {
            // Кулдаун не прошёл — не липнуть к игроку вплотную.
            Intent::Strafe
        }
    } else if situation.distance > situation.chase_range {
        Intent::Wait
    } else {
        Intent::Chase
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::net::{Activation, Layer};

    fn situation(distance: f32, ready: f32) -> Situation {
        Situation {
            distance,
            attack_range: 2.0,
            chase_range: 14.0,
            hp_frac: 1.0,
            target_hp_frac: 1.0,
            attack_ready: ready,
            allies_near: 0,
            has_path: true,
        }
    }

    /// Сеть, которая всегда выбирает одно намерение: удобно для проверки проводки.
    fn constant_brain(intent: usize) -> Brain {
        let mut b = vec![0.0; INTENTS.len()];
        b[intent] = 1.0;
        Brain {
            id: "test".into(),
            net: Mlp {
                layers: vec![Layer {
                    w: vec![vec![0.0; FEATURES]; INTENTS.len()],
                    b,
                    act: Activation::Linear,
                }],
            },
        }
    }

    #[test]
    fn features_are_normalized() {
        let features = situation(7.0, 1.0).features();
        assert_eq!(features.len(), FEATURES);
        for (index, value) in features.iter().enumerate() {
            assert!(
                (-1.01..=1.01).contains(value),
                "признак {index} вне диапазона: {value}"
            );
        }
    }

    #[test]
    fn brain_decides_deterministically() {
        let brain = constant_brain(3); // Attack
        assert!(brain.validate().is_ok());
        let s = situation(10.0, 0.0);
        assert_eq!(brain.decide(&s), Intent::Attack);
        assert_eq!(brain.decide(&s), Intent::Attack, "решение обязано повторяться");
    }

    #[test]
    fn fallback_behaves_like_the_old_ai() {
        assert_eq!(fallback_intent(&situation(1.0, 1.0)), Intent::Attack);
        assert_eq!(fallback_intent(&situation(1.0, 0.2)), Intent::Strafe);
        assert_eq!(fallback_intent(&situation(6.0, 1.0)), Intent::Chase);
        assert_eq!(fallback_intent(&situation(40.0, 1.0)), Intent::Wait);
    }

    #[test]
    fn broken_weights_are_rejected() {
        let mut brain = constant_brain(0);
        brain.net.layers[0].w = vec![vec![0.0; FEATURES - 1]; INTENTS.len()];
        assert!(brain.validate().is_err(), "сеть с чужой формой не должна пройти");
    }
}
