//! Диалоговая система: реплики, ветвления, эффекты.

use crate::character::StatKind;

#[derive(Clone, Debug)]
pub enum Effect {
    Stat(StatKind, i32),
    Rel(String, i32),          // id NPC, delta
    Flag(String),
    UnFlag(String),
    Gold(i32),
    Quest { id: String, title: String, desc: String },
    QuestDone(String),
    Flash(String),              // просто текст на экране
}

#[derive(Clone, Debug)]
pub struct Choice {
    pub text: String,
    pub requires: Option<(StatKind, i32)>,   // требование к стату
    pub effects: Vec<Effect>,
    pub next: Option<String>,               // id следующей сцены (None = закрыть диалог)
}

impl Choice {
    pub fn simple(text: &str, effects: Vec<Effect>) -> Self {
        Self { text: text.to_string(), requires: None, effects, next: None }
    }
    pub fn req(text: &str, stat: StatKind, min: i32, effects: Vec<Effect>) -> Self {
        Self { text: text.to_string(), requires: Some((stat, min)), effects, next: None }
    }
}

#[derive(Clone, Debug)]
pub struct Line {
    pub speaker: String,   // "" = нарратор (курсив)
    pub portrait: String,  // ключ портрета; "" = без портрета
    pub text: String,
}

impl Line {
    pub fn new(speaker: &str, portrait: &str, text: &str) -> Self {
        Self { speaker: speaker.to_string(), portrait: portrait.to_string(), text: text.to_string() }
    }
    pub fn narr(text: &str) -> Self {
        Self { speaker: String::new(), portrait: String::new(), text: text.to_string() }
    }
}

#[derive(Clone, Debug)]
pub struct Scene {
    pub id: String,
    pub lines: Vec<Line>,
    pub choices: Vec<Choice>,  // пусто → просто «Далее»
}
