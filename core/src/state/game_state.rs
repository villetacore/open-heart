//! Чистое игровое состояние — без зависимостей Godot.
//! Вся логика переходов и доступных действий здесь.

use crate::character::{StatKind, Stats};
use crate::dialogue::Effect;
use crate::item::Inventory;
use crate::quest::QuestLog;
use std::collections::{HashMap, HashSet};

#[derive(Clone, PartialEq, Debug)]
pub enum Period {
    Morning,
    Afternoon,
    Evening,
    Night,
}

impl Period {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Morning => "Утро",
            Self::Afternoon => "День",
            Self::Evening => "Вечер",
            Self::Night => "Ночь",
        }
    }
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Morning => "🌅",
            Self::Afternoon => "☀",
            Self::Evening => "🌆",
            Self::Night => "🌙",
        }
    }
    pub fn next(&self) -> Self {
        match self {
            Self::Morning => Self::Afternoon,
            Self::Afternoon => Self::Evening,
            Self::Evening => Self::Night,
            Self::Night => Self::Morning,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Hash, Eq)]
pub enum Location {
    Dorm,
    Hallway,
    Classroom,
    Library,
    Gym,
    Cafeteria,
    Park,
    Office,
}

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
        Self {
            key,
            label: label.to_string(),
            icon: "🚶",
            time_cost: false,
            effects: vec![],
            scene: None,
            goto: Some(loc),
        }
    }
    fn act(key: &'static str, icon: &'static str, label: &str, effects: Vec<Effect>) -> Self {
        Self {
            key,
            label: label.to_string(),
            icon,
            time_cost: true,
            effects,
            scene: None,
            goto: None,
        }
    }
    fn scene_act(key: &'static str, icon: &'static str, label: &str, scene: &'static str) -> Self {
        Self {
            key,
            label: label.to_string(),
            icon,
            time_cost: false,
            effects: vec![],
            scene: Some(scene),
            goto: None,
        }
    }
    fn scene_time(
        key: &'static str,
        icon: &'static str,
        label: &str,
        scene: &'static str,
        effects: Vec<Effect>,
    ) -> Self {
        Self {
            key,
            label: label.to_string(),
            icon,
            time_cost: true,
            effects,
            scene: Some(scene),
            goto: None,
        }
    }
    fn end_day(key: &'static str) -> Self {
        Self {
            key,
            label: "Закончить день (спать)".to_string(),
            icon: "💤",
            time_cost: true,
            effects: vec![],
            scene: None,
            goto: None,
        }
    }
}

pub struct GameState {
    pub abilities: crate::combat::ability::AbilityState,
    pub day: u32,
    pub period: Period,
    pub location: Location,
    pub stats: Stats,
    pub gold: i32,
    pub relations: HashMap<String, i32>,
    pub flags: HashSet<String>,
    pub quests: QuestLog,
    pub inventory: Inventory,

    // ── RPG-прогрессия ──
    pub class_idx: Option<usize>, // индекс в classes::CLASSES
    pub spec_idx: usize,          // специализация внутри класса
    pub level: u32,
    pub xp: u32,
    pub dungeon_seed: u64, // счётчик сидов для процедурных данжей
    pub dungeons_cleared: u32,
    pub hearts: u32,                 // собранные «сердца жизни» (+15 макс. HP каждое)
    pub perks: HashMap<String, u32>, // id перка → купленный ранг
    pub perk_points: u32,            // очки на покупку перков
    /// Прогресс data-driven квестов (quest_id → счётчик убийств/подборов).
    pub quest_kills: HashMap<String, u32>,
    /// Пресет, с которым начата эта игра.
    pub preset: String,
    pub weapon_mods: [u8; 8],
    pub weapon_mod_cores: u32,
    /// Был ли уже использован сброс перков: первый сброс бесплатный (план §5).
    pub respec_used: bool,
}

/// Стоимость золота за повторный сброс перков (первый — бесплатный).
pub const RESPEC_COST: i32 = 100;

/// Крафт очка перка из материалов: расходуемый предмет, его количество и золото.
pub const PERK_CRAFT_ITEM: &str = "neon_shard";
pub const PERK_CRAFT_ITEM_QTY: u32 = 3;
pub const PERK_CRAFT_GOLD: i32 = 150;

impl GameState {
    pub fn new(name: &str) -> Self {
        let mut relations = HashMap::new();
        relations.insert("vale".into(), 10);
        relations.insert("elena".into(), 0);
        relations.insert("victor".into(), 20);
        relations.insert("sofia".into(), 0);

        Self {
            abilities: Default::default(),
            day: 1,
            period: Period::Morning,
            location: Location::Dorm,
            stats: Stats::new(name),
            gold: 30,
            relations,
            flags: HashSet::new(),
            quests: QuestLog::default(),
            inventory: Inventory::default(),
            class_idx: None,
            spec_idx: 0,
            level: 1,
            xp: 0,
            dungeon_seed: {
                // Мешаем время и константу → уникальный сид для каждой новой игры.
                let t = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(0x5EED_0001);
                t ^ 0xDEAD_BEEF_CAFE_0001
            },
            dungeons_cleared: 0,
            hearts: 0,
            perks: HashMap::new(),
            perk_points: 0,
            quest_kills: HashMap::new(),
            preset: "core".into(),
            weapon_mods: [0; 8],
            weapon_mod_cores: 0,
            respec_used: false,
        }
    }

    /// Списать золото, если хватает. Возвращает `true` при успешной покупке.
    /// Единая проверка для магазина Торговца, сброса перков и модов оружия.
    pub fn spend_gold(&mut self, cost: i32) -> bool {
        if cost < 0 || self.gold < cost {
            return false;
        }
        self.gold -= cost;
        true
    }

    /// Хватает ли материалов и золота, чтобы скрафтить очко перка.
    pub fn can_craft_perk_point(&self) -> bool {
        self.gold >= PERK_CRAFT_GOLD
            && self.inventory.count(PERK_CRAFT_ITEM) >= PERK_CRAFT_ITEM_QTY
    }

    /// Скрафтить очко перка: списать материалы и золото, добавить `perk_point`.
    /// Возвращает `false`, если ресурсов не хватило. Альтернатива прокачке через
    /// бой — превращает собранные осколки в развитие билда (план §5).
    pub fn craft_perk_point(&mut self) -> bool {
        if !self.can_craft_perk_point() {
            return false;
        }
        self.inventory.remove(PERK_CRAFT_ITEM, PERK_CRAFT_ITEM_QTY);
        self.gold -= PERK_CRAFT_GOLD;
        self.perk_points += 1;
        true
    }

    /// Стоимость следующего сброса перков: первый бесплатный, далее [`RESPEC_COST`].
    pub fn respec_cost(&self) -> i32 {
        if self.respec_used { RESPEC_COST } else { 0 }
    }

    /// Сбросить перки: снять золото по [`Self::respec_cost`], вернуть все вложенные
    /// очки (ранг × стоимость перка) в `perk_points` и очистить набор перков.
    /// Возвращает число возвращённых очков или `None`, если не хватает золота.
    /// Вызывающая сторона обязана пересчитать лоадаут после сброса.
    pub fn respec_perks(&mut self) -> Option<u32> {
        let cost = self.respec_cost();
        if self.gold < cost {
            return None;
        }
        let refunded: u32 = self
            .perks
            .iter()
            .map(|(id, rank)| crate::perk::perk_cost(id) * rank)
            .sum();
        self.gold -= cost;
        self.perk_points += refunded;
        self.perks.clear();
        self.respec_used = true;
        Some(refunded)
    }

    pub fn add_heart(&mut self) {
        self.hearts += 1;
    }
    pub fn stat_hearts(&self) -> u32 {
        self.hearts
    }

    /// Начислить опыт; вернуть количество полученных уровней. Каждый уровень даёт очко перка.
    pub fn add_xp(&mut self, amount: u32) -> u32 {
        self.xp += amount;
        let mut gained = 0;
        while self.xp >= crate::classes::xp_to_next(self.level) {
            self.xp -= crate::classes::xp_to_next(self.level);
            self.level += 1;
            gained += 1;
        }
        self.perk_points += gained;
        gained
    }

    pub fn rel(&self, npc: &str) -> i32 {
        *self.relations.get(npc).unwrap_or(&0)
    }
    pub fn has(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }
    pub fn stat(&self, k: &StatKind) -> i32 {
        self.stats.get(k)
    }

    /// Применить список эффектов, вернуть строки для флэш-сообщений.
    pub fn apply(&mut self, effects: &[Effect], lang: &str) -> Vec<String> {
        let mut msgs = Vec::new();
        for e in effects {
            match e {
                Effect::Stat(k, v) => {
                    self.stats.modify(k, *v);
                    if *v != 0 {
                        msgs.push(format!("{:+} {}", v, k.short()));
                    }
                }
                Effect::Rel(id, v) => {
                    let r = self.relations.entry(id.clone()).or_insert(0);
                    *r = (*r + v).clamp(0, 100);
                    if *v != 0 {
                        msgs.push(if lang == "en" {
                            format!("{:+} relationship ({})", v, id)
                        } else {
                            format!("{:+} к отношениям ({})", v, id)
                        });
                    }
                }
                Effect::Flag(f) => {
                    self.flags.insert(f.clone());
                }
                Effect::UnFlag(f) => {
                    self.flags.remove(f);
                }
                Effect::Gold(v) => {
                    self.gold += v;
                    msgs.push(format!(
                        "{:+} {}",
                        v,
                        if lang == "en" { "gold" } else { "зол." }
                    ));
                }
                Effect::Xp(v) => {
                    let gained = self.add_xp(*v);
                    msgs.push(format!("+{} XP", v));
                    if gained > 0 {
                        msgs.push(format!(
                            "{} {}!",
                            if lang == "en" {
                                "LEVEL"
                            } else {
                                "УРОВЕНЬ"
                            },
                            self.level
                        ));
                    }
                }
                Effect::Item { id, name, qty } => {
                    let name = crate::dialogue::localized(name, lang);
                    self.inventory
                        .add(crate::item::Item::new(id, &name, "", *qty));
                    msgs.push(format!("+{} {}", qty, name));
                }
                Effect::Flash(m) => msgs.push(crate::dialogue::localized(m, lang)),
                Effect::Quest { id, title, desc } => {
                    let title = crate::dialogue::localized(title, lang);
                    let desc = crate::dialogue::localized(desc, lang);
                    self.quests.add(id, &title, &desc);
                    msgs.push(if lang == "en" {
                        format!("New quest: \"{title}\"")
                    } else {
                        format!("Новый квест: «{title}»")
                    });
                }
                Effect::QuestDone(id) => {
                    self.quests.complete(id);
                    msgs.push(if lang == "en" {
                        "Quest completed!".into()
                    } else {
                        "Квест выполнен!".into()
                    });
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
        } else {
            false
        }
    }

    /// Список доступных действий для текущей локации + периода + флагов.
    pub fn available_actions(&self) -> Vec<Action> {
        use Effect::*;
        use StatKind::*;
        let mut a: Vec<Action> = Vec::new();

        match &self.location {
            Location::Dorm => {
                // Первый разговор с Виктором
                if !self.has("met_victor") {
                    a.push(Action::scene_act(
                        "talk_victor",
                        "💬",
                        "Поговорить с Виктором",
                        "intro_victor",
                    ));
                } else {
                    a.push(Action::go(
                        "go_hallway",
                        "Выйти в коридор",
                        Location::Hallway,
                    ));
                }
                if matches!(self.period, Period::Evening | Period::Night) {
                    a.push(Action::act(
                        "rest_early",
                        "📖",
                        "Почитать перед сном (+INT)",
                        vec![Stat(Intelligence, 1)],
                    ));
                    a.push(Action::end_day("sleep"));
                }
                if self.has("met_victor") {
                    a.push(Action::go(
                        "go_hallway",
                        "Выйти в коридор",
                        Location::Hallway,
                    ));
                }
            }
            Location::Hallway => {
                a.push(Action::go(
                    "go_class",
                    "→ Учебный класс",
                    Location::Classroom,
                ));
                a.push(Action::go("go_lib", "→ Библиотека", Location::Library));
                a.push(Action::go("go_gym", "→ Спортзал", Location::Gym));
                a.push(Action::go("go_caf", "→ Столовая", Location::Cafeteria));
                a.push(Action::go("go_park", "→ Парк (выход)", Location::Park));
                a.push(Action::go(
                    "go_dorm",
                    "← Вернуться в общежитие",
                    Location::Dorm,
                ));
                // Кабинет Vale доступен если познакомились
                if self.has("met_vale") {
                    a.push(Action::go(
                        "go_office",
                        "→ Кабинет Ms. Вейл",
                        Location::Office,
                    ));
                }
            }
            Location::Classroom => {
                a.push(Action::act(
                    "study",
                    "📚",
                    "Учиться (+2 INT)",
                    vec![Stat(Intelligence, 2)],
                ));
                if !self.has("met_vale") {
                    a.push(Action::scene_act(
                        "meet_vale",
                        "✨",
                        "Подойти к Ms. Вейл",
                        "meet_vale",
                    ));
                } else if self.rel("vale") >= 15 && !self.has("vale_chat_1_done") {
                    a.push(Action::scene_time(
                        "chat_vale_class",
                        "💬",
                        "Поговорить с Ms. Вейл после урока",
                        "vale_class_chat",
                        vec![Rel("vale".into(), 5)],
                    ));
                }
                if !self.has("met_elena") && self.period == Period::Afternoon {
                    a.push(Action::scene_act(
                        "notice_elena",
                        "👁",
                        "Заметить ту девушку у окна",
                        "first_elena",
                    ));
                }
                a.push(Action::go("back_hall", "← Коридор", Location::Hallway));
            }
            Location::Library => {
                a.push(Action::act(
                    "study_hard",
                    "📖",
                    "Усиленно учиться (+3 INT)",
                    vec![Stat(Intelligence, 3)],
                ));
                if self.has("met_elena") && !self.has("elena_lib_1") {
                    a.push(Action::scene_time(
                        "elena_lib",
                        "💬",
                        "Подойти к Елене (она снова здесь)",
                        "elena_library_1",
                        vec![Rel("elena".into(), 8)],
                    ));
                }
                a.push(Action::go("back_hall", "← Коридор", Location::Hallway));
            }
            Location::Gym => {
                a.push(Action::act(
                    "train",
                    "💪",
                    "Тренироваться (+2 FIT)",
                    vec![Stat(Fitness, 2)],
                ));
                a.push(Action::act(
                    "train_hard",
                    "🏋",
                    "Серьёзная тренировка (+3 FIT, -1 WIL)",
                    vec![Stat(Fitness, 3), Stat(Willpower, -1)],
                ));
                a.push(Action::go("back_hall", "← Коридор", Location::Hallway));
            }
            Location::Cafeteria => {
                a.push(Action::act(
                    "socialize",
                    "🗣",
                    "Общаться (+2 CHR, +1 REP)",
                    vec![Stat(Charm, 2), Stat(Reputation, 1)],
                ));
                if !self.has("met_sofia") {
                    a.push(Action::scene_act(
                        "meet_sofia",
                        "👑",
                        "Подойти к компании Sofii",
                        "meet_sofia",
                    ));
                } else if self.rel("sofia") >= 10 {
                    a.push(Action::scene_time(
                        "chat_sofia",
                        "💬",
                        "Поговорить с Sofiej",
                        "sofia_chat",
                        vec![Rel("sofia".into(), 5), Stat(Reputation, 1)],
                    ));
                }
                a.push(Action::go("back_hall", "← Коридор", Location::Hallway));
            }
            Location::Park => {
                a.push(Action::act(
                    "walk",
                    "🌿",
                    "Прогуляться (+1 WIL, +1 REP)",
                    vec![Stat(Willpower, 1), Stat(Reputation, 1)],
                ));
                a.push(Action::act(
                    "reflect",
                    "🌙",
                    "Поразмышлять (+1 INT, +1 WIL)",
                    vec![Stat(Intelligence, 1), Stat(Willpower, 1)],
                ));
                a.push(Action::go("back_hall", "← К школе", Location::Hallway));
            }
            Location::Office => {
                let _session_key = format!("vale_session_{}", self.rel("vale") / 15);
                let scene_id = if self.rel("vale") < 25 {
                    "vale_office_1"
                } else if self.rel("vale") < 45 {
                    "vale_office_2"
                } else {
                    "vale_office_deep"
                };
                a.push(Action::scene_time(
                    "session_vale",
                    "🛋",
                    "Консультация у Ms. Вейл",
                    scene_id,
                    vec![Rel("vale".into(), 8)],
                ));
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

#[cfg(test)]
mod localization_tests {
    use super::*;

    #[test]
    fn dialogue_effect_messages_follow_selected_language() {
        let mut state = GameState::new("Tester");
        let effects = [Effect::Rel("vale".into(), 5), Effect::QuestDone("q".into())];
        let english = state.apply(&effects, "en").join(" ");
        let russian = state.apply(&effects, "ru").join(" ");

        assert!(english.contains("relationship"));
        assert!(english.contains("Quest completed"));
        assert!(!english.contains("отнош"));
        assert!(russian.contains("отношениям"));
        assert!(russian.contains("Квест выполнен"));
    }
}

#[cfg(test)]
mod respec_tests {
    use super::*;

    #[test]
    fn first_respec_is_free_then_costs_gold_and_refunds_points() {
        let perk = &crate::perk::perks()[0];
        let cost = perk.cost.max(1);

        let mut state = GameState::new("Tester");
        state.gold = 150;
        // Куплен один перк на два ранга (вложено cost*2 очков).
        state.perks.insert(perk.id.clone(), 2);

        // Первый сброс — бесплатный, возвращает вложенные очки, чистит перки.
        assert_eq!(state.respec_cost(), 0);
        let refunded = state.respec_perks().expect("free respec must succeed");
        assert_eq!(refunded, cost * 2, "должны вернуться все вложенные очки");
        assert_eq!(state.perk_points, cost * 2);
        assert!(state.perks.is_empty());
        assert_eq!(state.gold, 150, "первый сброс не тратит золото");
        assert!(state.respec_used);

        // Второй сброс — платный.
        assert_eq!(state.respec_cost(), RESPEC_COST);
        state.perks.insert(perk.id.clone(), 1);
        let before = state.perk_points;
        let refunded2 = state.respec_perks().expect("paid respec with enough gold");
        assert_eq!(refunded2, cost);
        assert_eq!(state.perk_points, before + cost);
        assert_eq!(state.gold, 150 - RESPEC_COST);
    }

    #[test]
    fn craft_perk_point_consumes_materials_and_gold() {
        use crate::item::Item;
        let mut state = GameState::new("Tester");
        state.gold = PERK_CRAFT_GOLD;
        assert!(!state.can_craft_perk_point(), "без материалов крафт невозможен");
        state.inventory.add(Item::new(PERK_CRAFT_ITEM, "Neon shard", "", PERK_CRAFT_ITEM_QTY));
        assert!(state.can_craft_perk_point());
        assert!(state.craft_perk_point());
        assert_eq!(state.perk_points, 1);
        assert_eq!(state.gold, 0);
        assert_eq!(state.inventory.count(PERK_CRAFT_ITEM), 0);
        assert!(!state.craft_perk_point(), "повторно без ресурсов — отказ");
        assert_eq!(state.perk_points, 1);
    }

    #[test]
    fn spend_gold_checks_balance_and_rejects_negative() {
        let mut state = GameState::new("Tester");
        state.gold = 50;
        assert!(!state.spend_gold(60), "нельзя купить дороже баланса");
        assert_eq!(state.gold, 50);
        assert!(state.spend_gold(30));
        assert_eq!(state.gold, 20);
        assert!(!state.spend_gold(-5), "отрицательная цена недопустима");
        assert_eq!(state.gold, 20);
    }

    #[test]
    fn respec_refused_without_enough_gold() {
        let mut state = GameState::new("Tester");
        state.respec_used = true; // повторный сброс — платный
        state.gold = RESPEC_COST - 1;
        state.perk_points = 3;
        assert!(state.respec_perks().is_none(), "нет золота — сброс не проходит");
        assert_eq!(state.perk_points, 3, "очки не должны меняться при отказе");
    }
}
