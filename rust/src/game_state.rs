//! Чистое игровое состояние — без зависимостей Godot.
//! Вся логика переходов и доступных действий здесь.

use std::collections::{HashMap, HashSet};
use crate::character::{StatKind, Stats};
use crate::dialogue::Effect;
use crate::quest::QuestLog;
use crate::item::Inventory;

#[derive(Clone, PartialEq, Debug)]
pub enum Period { Morning, Afternoon, Evening, Night }

impl Period {
    pub fn label(&self) -> &'static str {
        match self { Self::Morning => "Утро", Self::Afternoon => "День", Self::Evening => "Вечер", Self::Night => "Ночь" }
    }
    pub fn icon(&self) -> &'static str {
        match self { Self::Morning => "🌅", Self::Afternoon => "☀", Self::Evening => "🌆", Self::Night => "🌙" }
    }
    pub fn next(&self) -> Self {
        match self { Self::Morning => Self::Afternoon, Self::Afternoon => Self::Evening, Self::Evening => Self::Night, Self::Night => Self::Morning }
    }
}

#[derive(Clone, PartialEq, Debug, Hash, Eq)]
pub enum Location { Dorm, Hallway, Classroom, Library, Gym, Cafeteria, Park, Office }

impl Location {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Dorm => "Общежитие",
            Self::Hallway => "Коридор",
            Self::Classroom => "Учебный класс",
            Self::Library => "Библиотека",
            Self::Gym => "Спортзал",
            Self::Cafeteria => "Столовая",
            Self::Park => "Парк",
            Self::Office => "Кабинет Ms. Вейл",
        }
    }
    pub fn bg(&self) -> &'static str {
        match self {
            Self::Dorm => "dorm",
            Self::Hallway => "hallway",
            Self::Classroom => "classroom",
            Self::Library => "library",
            Self::Gym => "gym",
            Self::Cafeteria => "cafeteria",
            Self::Park => "park",
            Self::Office => "office",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Action {
    pub key: &'static str,
    pub label: String,
    pub icon: &'static str,
    pub time_cost: bool,
    pub effects: Vec<Effect>,
    pub scene: Option<&'static str>,
    pub goto: Option<Location>,
}

impl Action {
    fn go(key: &'static str, label: &str, loc: Location) -> Self {
        Self { key, label: label.to_string(), icon: "🚶", time_cost: false, effects: vec![], scene: None, goto: Some(loc) }
    }
    fn act(key: &'static str, icon: &'static str, label: &str, effects: Vec<Effect>) -> Self {
        Self { key, label: label.to_string(), icon, time_cost: true, effects, scene: None, goto: None }
    }
    fn scene_act(key: &'static str, icon: &'static str, label: &str, scene: &'static str) -> Self {
        Self { key, label: label.to_string(), icon, time_cost: false, effects: vec![], scene: Some(scene), goto: None }
    }
    fn scene_time(key: &'static str, icon: &'static str, label: &str, scene: &'static str, effects: Vec<Effect>) -> Self {
        Self { key, label: label.to_string(), icon, time_cost: true, effects, scene: Some(scene), goto: None }
    }
    fn end_day(key: &'static str) -> Self {
        Self { key, label: "Закончить день (спать)".to_string(), icon: "💤", time_cost: true, effects: vec![], scene: None, goto: None }
    }
}

pub struct GameState {
    pub day: u32,
    pub period: Period,
    pub location: Location,
    pub stats: Stats,
    pub gold: i32,
    pub relations: HashMap<String, i32>,
    pub flags: HashSet<String>,
    pub quests: QuestLog,
    pub inventory: Inventory,
}

impl GameState {
    pub fn new(name: &str) -> Self {
        let mut relations = HashMap::new();
        relations.insert("vale".into(), 10);
        relations.insert("elena".into(), 0);
        relations.insert("victor".into(), 20);
        relations.insert("sofia".into(), 0);

        Self {
            day: 1,
            period: Period::Morning,
            location: Location::Dorm,
            stats: Stats::new(name),
            gold: 30,
            relations,
            flags: HashSet::new(),
            quests: QuestLog::default(),
            inventory: Inventory::default(),
        }
    }

    pub fn rel(&self, npc: &str) -> i32 { *self.relations.get(npc).unwrap_or(&0) }
    pub fn has(&self, flag: &str) -> bool { self.flags.contains(flag) }
    pub fn stat(&self, k: &StatKind) -> i32 { self.stats.get(k) }

    /// Применить список эффектов, вернуть строки для флэш-сообщений.
    pub fn apply(&mut self, effects: &[Effect]) -> Vec<String> {
        let mut msgs = Vec::new();
        for e in effects {
            match e {
                Effect::Stat(k, v) => {
                    self.stats.modify(k, *v);
                    if *v != 0 { msgs.push(format!("{:+} {}", v, k.short())); }
                }
                Effect::Rel(id, v) => {
                    let r = self.relations.entry(id.clone()).or_insert(0);
                    *r = (*r + v).clamp(0, 100);
                    if *v != 0 { msgs.push(format!("{:+} к отношениям ({})", v, id)); }
                }
                Effect::Flag(f) => { self.flags.insert(f.clone()); }
                Effect::UnFlag(f) => { self.flags.remove(f); }
                Effect::Gold(v) => { self.gold += v; msgs.push(format!("{:+} зол.", v)); }
                Effect::Flash(m) => msgs.push(m.clone()),
                Effect::Quest { id, title, desc } => {
                    self.quests.add(id, title, desc);
                    msgs.push(format!("Новый квест: «{}»", title));
                }
                Effect::QuestDone(id) => {
                    self.quests.complete(id);
                    msgs.push("Квест выполнен!".into());
                }
            }
        }
        msgs
    }

    /// Перейти к следующему периоду. Вернуть true если наступил новый день.
    pub fn tick(&mut self) -> bool {
        self.period = self.period.next();
        if self.period == Period::Morning {
            self.day += 1;
            self.location = Location::Dorm;
            true
        } else { false }
    }

    /// Список доступных действий для текущей локации + периода + флагов.
    pub fn available_actions(&self) -> Vec<Action> {
        use Effect::*; use StatKind::*;
        let mut a: Vec<Action> = Vec::new();

        match &self.location {
            Location::Dorm => {
                // Первый разговор с Виктором
                if !self.has("met_victor") {
                    a.push(Action::scene_act("talk_victor", "💬", "Поговорить с Виктором", "intro_victor"));
                } else {
                    a.push(Action::go("go_hallway", "Выйти в коридор", Location::Hallway));
                }
                if matches!(self.period, Period::Evening | Period::Night) {
                    a.push(Action::act("rest_early", "📖", "Почитать перед сном (+INT)", vec![Stat(Intelligence, 1)]));
                    a.push(Action::end_day("sleep"));
                }
                if self.has("met_victor") {
                    a.push(Action::go("go_hallway", "Выйти в коридор", Location::Hallway));
                }
            }
            Location::Hallway => {
                a.push(Action::go("go_class", "→ Учебный класс", Location::Classroom));
                a.push(Action::go("go_lib", "→ Библиотека", Location::Library));
                a.push(Action::go("go_gym", "→ Спортзал", Location::Gym));
                a.push(Action::go("go_caf", "→ Столовая", Location::Cafeteria));
                a.push(Action::go("go_park", "→ Парк (выход)", Location::Park));
                a.push(Action::go("go_dorm", "← Вернуться в общежитие", Location::Dorm));
                // Кабинет Vale доступен если познакомились
                if self.has("met_vale") {
                    a.push(Action::go("go_office", "→ Кабинет Ms. Вейл", Location::Office));
                }
            }
            Location::Classroom => {
                a.push(Action::act("study", "📚", "Учиться (+2 INT)", vec![Stat(Intelligence, 2)]));
                if !self.has("met_vale") {
                    a.push(Action::scene_act("meet_vale", "✨", "Подойти к Ms. Вейл", "meet_vale"));
                } else if self.rel("vale") >= 15 && !self.has("vale_chat_1_done") {
                    a.push(Action::scene_time("chat_vale_class", "💬", "Поговорить с Ms. Вейл после урока", "vale_class_chat", vec![Rel("vale".into(), 5)]));
                }
                if !self.has("met_elena") && self.period == Period::Afternoon {
                    a.push(Action::scene_act("notice_elena", "👁", "Заметить ту девушку у окна", "first_elena"));
                }
                a.push(Action::go("back_hall", "← Коридор", Location::Hallway));
            }
            Location::Library => {
                a.push(Action::act("study_hard", "📖", "Усиленно учиться (+3 INT)", vec![Stat(Intelligence, 3)]));
                if self.has("met_elena") && !self.has("elena_lib_1") {
                    a.push(Action::scene_time("elena_lib", "💬", "Подойти к Елене (она снова здесь)", "elena_library_1", vec![Rel("elena".into(), 8)]));
                }
                a.push(Action::go("back_hall", "← Коридор", Location::Hallway));
            }
            Location::Gym => {
                a.push(Action::act("train", "💪", "Тренироваться (+2 FIT)", vec![Stat(Fitness, 2)]));
                a.push(Action::act("train_hard", "🏋", "Серьёзная тренировка (+3 FIT, -1 WIL)", vec![Stat(Fitness, 3), Stat(Willpower, -1)]));
                a.push(Action::go("back_hall", "← Коридор", Location::Hallway));
            }
            Location::Cafeteria => {
                a.push(Action::act("socialize", "🗣", "Общаться (+2 CHR, +1 REP)", vec![Stat(Charm, 2), Stat(Reputation, 1)]));
                if !self.has("met_sofia") {
                    a.push(Action::scene_act("meet_sofia", "👑", "Подойти к компании Sofii", "meet_sofia"));
                } else if self.rel("sofia") >= 10 {
                    a.push(Action::scene_time("chat_sofia", "💬", "Поговорить с Sofiej", "sofia_chat", vec![Rel("sofia".into(), 5), Stat(Reputation, 1)]));
                }
                a.push(Action::go("back_hall", "← Коридор", Location::Hallway));
            }
            Location::Park => {
                a.push(Action::act("walk", "🌿", "Прогуляться (+1 WIL, +1 REP)", vec![Stat(Willpower, 1), Stat(Reputation, 1)]));
                a.push(Action::act("reflect", "🌙", "Поразмышлять (+1 INT, +1 WIL)", vec![Stat(Intelligence, 1), Stat(Willpower, 1)]));
                a.push(Action::go("back_hall", "← К школе", Location::Hallway));
            }
            Location::Office => {
                let _session_key = format!("vale_session_{}", self.rel("vale") / 15);
                let scene_id = if self.rel("vale") < 25 { "vale_office_1" }
                    else if self.rel("vale") < 45 { "vale_office_2" }
                    else { "vale_office_deep" };
                a.push(Action::scene_time("session_vale", "🛋", "Консультация у Ms. Вейл", scene_id, vec![Rel("vale".into(), 8)]));
                a.push(Action::go("back_hall", "← Коридор", Location::Hallway));
            }
        }
        a
    }

    /// NPC для отображения портрета в текущей локации/периоде.
    pub fn present_npc(&self) -> Option<&'static str> {
        match (&self.location, &self.period) {
            (Location::Dorm, _) if !self.has("met_victor") => Some("victor"),
            (Location::Dorm, Period::Evening) => Some("victor"),
            (Location::Classroom, _) if !self.has("met_vale") => Some("vale"),
            (Location::Classroom, Period::Morning) => Some("vale"),
            (Location::Library, _) => Some("elena"),
            (Location::Cafeteria, _) => Some("sofia"),
            (Location::Office, _) => Some("vale"),
            _ => None,
        }
    }

    pub fn rel_label(rel: i32) -> &'static str {
        match rel {
            0..=9 => "Незнакомец",
            10..=24 => "Знакомый",
            25..=44 => "Приятель",
            45..=64 => "Друг",
            65..=84 => "Близкий друг",
            _ => "Особый",
        }
    }
}
