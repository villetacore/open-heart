//! Многомерная система статов игрока.

#[derive(Clone, Debug, PartialEq)]
pub enum StatKind { Intelligence, Charm, Fitness, Reputation, Willpower }

impl StatKind {
    pub fn short(&self) -> &'static str {
        match self {
            Self::Intelligence => "INT",
            Self::Charm => "CHR",
            Self::Fitness => "FIT",
            Self::Reputation => "REP",
            Self::Willpower => "WIL",
        }
    }

    /// Стат по id из данных (dialogues.json): int/chr/fit/rep/wil (без регистра).
    pub fn from_id(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "int" | "intelligence" => Self::Intelligence,
            "chr" | "charm"        => Self::Charm,
            "fit" | "fitness"      => Self::Fitness,
            "rep" | "reputation"   => Self::Reputation,
            "wil" | "willpower"    => Self::Willpower,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Stats {
    pub name: String,
    pub intelligence: i32,
    pub charm: i32,
    pub fitness: i32,
    pub reputation: i32,
    pub willpower: i32,
}

impl Stats {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), intelligence: 5, charm: 5, fitness: 5, reputation: 5, willpower: 5 }
    }

    pub fn get(&self, kind: &StatKind) -> i32 {
        match kind {
            StatKind::Intelligence => self.intelligence,
            StatKind::Charm => self.charm,
            StatKind::Fitness => self.fitness,
            StatKind::Reputation => self.reputation,
            StatKind::Willpower => self.willpower,
        }
    }

    pub fn modify(&mut self, kind: &StatKind, delta: i32) {
        let v = match kind {
            StatKind::Intelligence => &mut self.intelligence,
            StatKind::Charm => &mut self.charm,
            StatKind::Fitness => &mut self.fitness,
            StatKind::Reputation => &mut self.reputation,
            StatKind::Willpower => &mut self.willpower,
        };
        *v = (*v + delta).clamp(0, 99);
    }
}
