//! Крошечная нейросеть прямого распространения.
//!
//! Зачем своя, а не библиотека: ядро исполняется и в клиенте, и в `core.wasm`
//! на сервере, а решение врага обязано совпадать до бита — иначе игрок увидит
//! одно, а сервер посчитает другое. Здесь нет ни потоков, ни аллокаций сверх
//! необходимого, ни зависимостей: тридцать строк арифметики.
//!
//! Веса — данные пресета (`brains.json`), а не код: обучение идёт офлайн
//! (docs/GAMEPLAY_SYSTEMS.md §3), в игру приезжают уже готовые числа.

use serde::{Deserialize, Serialize};

/// Функция активации слоя.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Activation {
    /// Для скрытых слоёв: дёшево и не насыщается.
    #[default]
    Relu,
    /// Когда нужен выход в [-1, 1] (повороты, «сила» действия).
    Tanh,
    /// Последний слой, если выход трактуется как оценки действий.
    Linear,
}

impl Activation {
    #[inline]
    fn apply(self, x: f32) -> f32 {
        match self {
            Self::Relu => x.max(0.0),
            Self::Tanh => x.tanh(),
            Self::Linear => x,
        }
    }
}

/// Полносвязный слой: `out = act(W · in + b)`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Layer {
    /// Матрица весов построчно: `w[o]` — веса o-го выхода по всем входам.
    pub w: Vec<Vec<f32>>,
    pub b: Vec<f32>,
    #[serde(default)]
    pub act: Activation,
}

impl Layer {
    pub fn inputs(&self) -> usize {
        self.w.first().map(|row| row.len()).unwrap_or(0)
    }

    pub fn outputs(&self) -> usize {
        self.w.len()
    }

    fn forward(&self, input: &[f32], out: &mut Vec<f32>) {
        out.clear();
        for (row, bias) in self.w.iter().zip(self.b.iter()) {
            let mut sum = *bias;
            for (weight, value) in row.iter().zip(input.iter()) {
                sum += weight * value;
            }
            out.push(self.act.apply(sum));
        }
    }
}

/// Сеть: последовательность слоёв.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mlp {
    pub layers: Vec<Layer>,
}

/// Что не так с весами из пресета.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetError {
    Empty,
    /// Размерности соседних слоёв не сходятся.
    ShapeMismatch { layer: usize, expected: usize, got: usize },
    /// Число весов не совпадает с числом смещений.
    BiasMismatch { layer: usize },
}

impl Mlp {
    /// Проверить форму сети. Битые веса должны отваливаться при загрузке,
    /// а не сюрпризом в середине боя.
    pub fn validate(&self, inputs: usize, outputs: usize) -> Result<(), NetError> {
        let Some(first) = self.layers.first() else {
            return Err(NetError::Empty);
        };
        if first.inputs() != inputs {
            return Err(NetError::ShapeMismatch {
                layer: 0,
                expected: inputs,
                got: first.inputs(),
            });
        }
        for (index, layer) in self.layers.iter().enumerate() {
            if layer.w.len() != layer.b.len() {
                return Err(NetError::BiasMismatch { layer: index });
            }
            if index > 0 {
                let previous = self.layers[index - 1].outputs();
                if layer.inputs() != previous {
                    return Err(NetError::ShapeMismatch {
                        layer: index,
                        expected: previous,
                        got: layer.inputs(),
                    });
                }
            }
        }
        let last = self.layers.last().map(|l| l.outputs()).unwrap_or(0);
        if last != outputs {
            return Err(NetError::ShapeMismatch {
                layer: self.layers.len() - 1,
                expected: outputs,
                got: last,
            });
        }
        Ok(())
    }

    /// Посчитать выход сети.
    pub fn forward(&self, input: &[f32]) -> Vec<f32> {
        let mut current: Vec<f32> = input.to_vec();
        let mut next: Vec<f32> = Vec::new();
        for layer in &self.layers {
            layer.forward(&current, &mut next);
            std::mem::swap(&mut current, &mut next);
        }
        current
    }

    /// Индекс наибольшего выхода — выбранное действие.
    pub fn argmax(values: &[f32]) -> usize {
        let mut best = 0;
        let mut best_value = f32::NEG_INFINITY;
        for (index, value) in values.iter().enumerate() {
            if *value > best_value {
                best_value = *value;
                best = index;
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(size: usize, act: Activation) -> Layer {
        let mut w = vec![vec![0.0; size]; size];
        for (index, row) in w.iter_mut().enumerate() {
            row[index] = 1.0;
        }
        Layer { w, b: vec![0.0; size], act }
    }

    #[test]
    fn forward_computes_expected_values() {
        let net = Mlp {
            layers: vec![Layer {
                w: vec![vec![1.0, 2.0], vec![-1.0, 0.5]],
                b: vec![0.0, 1.0],
                act: Activation::Linear,
            }],
        };
        // 1*3 + 2*4 + 0 = 11;  -1*3 + 0.5*4 + 1 = 0
        let out = net.forward(&[3.0, 4.0]);
        assert_eq!(out, vec![11.0, 0.0]);
    }

    #[test]
    fn relu_clips_negatives() {
        let net = Mlp {
            layers: vec![Layer {
                w: vec![vec![1.0], vec![-1.0]],
                b: vec![0.0, 0.0],
                act: Activation::Relu,
            }],
        };
        assert_eq!(net.forward(&[2.0]), vec![2.0, 0.0]);
    }

    #[test]
    fn same_input_gives_same_output() {
        let net = Mlp { layers: vec![identity(4, Activation::Tanh)] };
        let input = [0.3, -0.7, 1.0, 0.0];
        assert_eq!(net.forward(&input), net.forward(&input));
    }

    #[test]
    fn validate_catches_broken_shapes() {
        let good = Mlp {
            layers: vec![identity(3, Activation::Relu), identity(3, Activation::Linear)],
        };
        assert!(good.validate(3, 3).is_ok());

        let mismatched = Mlp {
            layers: vec![identity(3, Activation::Relu), identity(2, Activation::Linear)],
        };
        assert_eq!(
            mismatched.validate(3, 2),
            Err(NetError::ShapeMismatch { layer: 1, expected: 3, got: 2 })
        );

        assert_eq!(Mlp { layers: vec![] }.validate(1, 1), Err(NetError::Empty));
    }

    #[test]
    fn weights_survive_json_round_trip() {
        let net = Mlp { layers: vec![identity(2, Activation::Tanh)] };
        let json = serde_json::to_string(&net).unwrap();
        let back: Mlp = serde_json::from_str(&json).unwrap();
        assert_eq!(back.forward(&[0.5, -0.5]), net.forward(&[0.5, -0.5]));
    }
}
