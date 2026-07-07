//! Диалоговая система: реплики, ветвления, эффекты.
//!
//! Сцены существуют в двух видах: встроенные (story.rs, код) и data-driven
//! (`dialogues.json` пресета — raw-структуры ниже). JSON-сцены имеют приоритет:
//! пресет может добавлять новые сцены и переопределять встроенные по id.

use serde::Deserialize;

use crate::character::StatKind;

#[derive(Clone, Debug)]
pub enum Effect {
    Stat(StatKind, i32),
    Rel(String, i32),          // id NPC, delta
    Flag(String),
    UnFlag(String),
    Gold(i32),
    Xp(u32),                   // опыт (может дать уровень — обрабатывается после apply)
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

// ── dialogues.json: raw-структуры и конвертация ───────────────────────────────
// Формат описан в docs/DATA_FORMATS.md#dialoguesjson. Редактируется категорией
// «Диалоги» в редакторе OpenHeart.

#[derive(Deserialize)]
pub struct SceneRaw {
    pub id: String,
    // Option: редактор может записать null — считаем это «пусто», не ошибкой
    #[serde(default)] pub lines:   Option<Vec<LineRaw>>,
    #[serde(default)] pub choices: Option<Vec<ChoiceRaw>>,
}

#[derive(Deserialize)]
pub struct LineRaw {
    #[serde(default)] pub speaker:  String,   // "" = нарратор
    #[serde(default)] pub portrait: String,
    pub text: String,
}

#[derive(Deserialize)]
pub struct ChoiceRaw {
    pub text: String,
    #[serde(default)] pub requires: Option<ReqRaw>,
    #[serde(default)] pub effects:  Option<Vec<EffectRaw>>,
    #[serde(default)] pub next:     Option<String>,
}

#[derive(Deserialize)]
pub struct ReqRaw {
    pub stat: String,   // int | chr | fit | rep | wil
    pub min:  i32,
}

/// Эффект выбора в JSON: `{"kind": "...", ...}`.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EffectRaw {
    Stat      { stat: String, value: i32 },
    Rel       { npc: String, value: i32 },
    Flag      { flag: String },
    Unflag    { flag: String },
    Gold      { value: i32 },
    Xp        { value: u32 },
    Quest     { id: String, title: String, #[serde(default)] desc: String },
    QuestDone { id: String },
    Flash     { text: String },
}

impl EffectRaw {
    fn into_effect(self) -> Result<Effect, String> {
        Ok(match self {
            Self::Stat { stat, value } => Effect::Stat(
                StatKind::from_id(&stat).ok_or_else(|| format!("неизвестный стат '{stat}'"))?,
                value),
            Self::Rel { npc, value }        => Effect::Rel(npc, value),
            Self::Flag { flag }             => Effect::Flag(flag),
            Self::Unflag { flag }           => Effect::UnFlag(flag),
            Self::Gold { value }            => Effect::Gold(value),
            Self::Xp { value }              => Effect::Xp(value),
            Self::Quest { id, title, desc } => Effect::Quest { id, title, desc },
            Self::QuestDone { id }          => Effect::QuestDone(id),
            Self::Flash { text }            => Effect::Flash(text),
        })
    }
}

impl SceneRaw {
    /// Конвертация в рантайм-сцену; ошибка содержит id сцены и причину.
    pub fn into_scene(self) -> Result<Scene, String> {
        let id = self.id;
        let lines = self.lines.unwrap_or_default().into_iter()
            .map(|l| Line { speaker: l.speaker, portrait: l.portrait, text: l.text })
            .collect();
        let raw_choices = self.choices.unwrap_or_default();
        let mut choices = Vec::with_capacity(raw_choices.len());
        for c in raw_choices {
            let requires = match c.requires {
                None => None,
                Some(r) => Some((
                    StatKind::from_id(&r.stat)
                        .ok_or_else(|| format!("сцена '{id}': неизвестный стат '{}'", r.stat))?,
                    r.min,
                )),
            };
            let raw_effects = c.effects.unwrap_or_default();
            let mut effects = Vec::with_capacity(raw_effects.len());
            for e in raw_effects {
                effects.push(e.into_effect().map_err(|e| format!("сцена '{id}': {e}"))?);
            }
            choices.push(Choice { text: c.text, requires, effects, next: c.next });
        }
        Ok(Scene { id, lines, choices })
    }
}

/// Распарсить dialogues.json (массив сцен). Битые сцены пропускаются с ошибкой
/// в out-параметре, остальные живут — одна опечатка не валит весь файл.
/// Каждая сцена парсится отдельно (через Value), поэтому и структурная ошибка
/// в одной сцене не роняет остальные.
pub fn parse_scenes(json: &str) -> Result<(Vec<Scene>, Vec<String>), String> {
    let raws: Vec<serde_json::Value> = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut scenes = Vec::with_capacity(raws.len());
    let mut errors = Vec::new();
    for (i, v) in raws.into_iter().enumerate() {
        let label = v.get("id").and_then(|x| x.as_str()).unwrap_or("?").to_string();
        match serde_json::from_value::<SceneRaw>(v) {
            Ok(raw) => match raw.into_scene() {
                Ok(s) => scenes.push(s),
                Err(e) => errors.push(e),
            },
            Err(e) => errors.push(format!("сцена #{i} ('{label}'): {e}")),
        }
    }
    Ok((scenes, errors))
}
