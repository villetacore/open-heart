//! Загрузка игровых конфигов пресета: presets/<id>/{enemies,items,level,npcs,quests}.json.

use serde::Deserialize;

fn default_scale() -> f32 {
    1.0
}
fn default_phase_mult() -> f32 {
    1.0
}
fn default_enemy_idle_fps() -> f32 {
    6.0
}
fn default_enemy_action_fps() -> f32 {
    10.0
}
fn default_enemy_frames() -> usize {
    8
}
fn default_attack_hit_ratio() -> f32 {
    0.42
}
fn default_weak_point_height() -> f32 {
    0.7
}
fn default_weak_point_multiplier() -> f32 {
    1.6
}

#[derive(Debug, Deserialize, Clone)]
pub struct WeakPointCfg {
    #[serde(default = "default_weak_point_height")]
    pub height: f32,
    #[serde(default = "default_weak_point_multiplier")]
    pub multiplier: f32,
}

impl Default for WeakPointCfg {
    fn default() -> Self {
        Self {
            height: default_weak_point_height(),
            multiplier: default_weak_point_multiplier(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct EnemyAnimationCfg {
    #[serde(default = "default_enemy_idle_fps")]
    pub idle_fps: f32,
    #[serde(default = "default_enemy_action_fps")]
    pub action_fps: f32,
    #[serde(default = "default_enemy_frames")]
    pub idle_frames: usize,
    #[serde(default = "default_enemy_frames")]
    pub move_frames: usize,
    #[serde(default = "default_enemy_frames")]
    pub attack_frames: usize,
    #[serde(default = "default_enemy_frames")]
    pub pain_frames: usize,
    #[serde(default = "default_enemy_frames")]
    pub alert_frames: usize,
    #[serde(default = "default_enemy_frames")]
    pub cast_frames: usize,
    #[serde(default = "default_enemy_frames")]
    pub death_frames: usize,
    #[serde(default)]
    pub attack_fps: Option<f32>,
    #[serde(default)]
    pub pain_fps: Option<f32>,
    #[serde(default)]
    pub alert_fps: Option<f32>,
    #[serde(default)]
    pub cast_fps: Option<f32>,
    #[serde(default)]
    pub death_fps: Option<f32>,
    #[serde(default = "default_attack_hit_ratio")]
    pub attack_hit_ratio: f32,
    #[serde(default)]
    pub idle_row: usize,
    #[serde(default = "default_row_move")]
    pub move_row: usize,
    #[serde(default = "default_row_left")]
    pub move_left_row: usize,
    #[serde(default = "default_row_right")]
    pub move_right_row: usize,
    #[serde(default = "default_row_attack")]
    pub attack_row: usize,
    #[serde(default = "default_row_pain")]
    pub pain_row: usize,
    #[serde(default = "default_row_alert")]
    pub alert_row: usize,
    #[serde(default = "default_row_alert")]
    pub cast_row: usize,
    #[serde(default = "default_row_death")]
    pub death_row: usize,
}

fn default_row_move() -> usize {
    1
}
fn default_row_left() -> usize {
    2
}
fn default_row_right() -> usize {
    3
}
fn default_row_attack() -> usize {
    4
}
fn default_row_pain() -> usize {
    5
}
fn default_row_alert() -> usize {
    6
}
fn default_row_death() -> usize {
    7
}

impl Default for EnemyAnimationCfg {
    fn default() -> Self {
        serde_json::from_str("{}").expect("EnemyAnimationCfg defaults")
    }
}

impl EnemyAnimationCfg {
    pub fn attack_fps(&self) -> f32 {
        self.attack_fps.unwrap_or(self.action_fps).max(0.1)
    }
    pub fn pain_fps(&self) -> f32 {
        self.pain_fps.unwrap_or(self.action_fps).max(0.1)
    }
    pub fn alert_fps(&self) -> f32 {
        self.alert_fps.unwrap_or(self.action_fps).max(0.1)
    }
    pub fn cast_fps(&self) -> f32 {
        self.cast_fps.unwrap_or(self.action_fps).max(0.1)
    }
    pub fn death_fps(&self) -> f32 {
        self.death_fps.unwrap_or(self.action_fps).max(0.1)
    }
}

// ── Serde helpers: accept both JSON integers and floats for integer fields ────
pub fn de_u32<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    f64::deserialize(d).map(|v| v as u32)
}
pub fn de_i32<'de, D: serde::Deserializer<'de>>(d: D) -> Result<i32, D::Error> {
    f64::deserialize(d).map(|v| v as i32)
}

// ── Структуры конфигов ────────────────────────────────────────────────────────

fn default_xp() -> f32 {
    15.0
}

/// Резисты урона: 0.0 = нет резиста, 1.0 = полный иммунитет, <0 = уязвимость.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct Resist {
    #[serde(default)]
    pub physical: f32,
    #[serde(default)]
    pub fire: f32,
    #[serde(default)]
    pub energy: f32,
    #[serde(default)]
    pub void: f32,
}

impl Resist {
    /// [physical, fire, energy, void] — в порядке DmgType::idx.
    pub fn arr(&self) -> [f32; 4] {
        [self.physical, self.fire, self.energy, self.void]
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct BossPhaseCfg {
    pub threshold: f32,
    #[serde(default)]
    pub abilities: Vec<String>,
    #[serde(default = "default_phase_mult")]
    pub damage_mult: f32,
    #[serde(default = "default_phase_mult")]
    pub speed_mult: f32,
    #[serde(default)]
    pub tint: Option<[f32; 3]>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EnemyCfg {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub name_en: String,
    pub hp: f32,
    pub speed: f32,
    pub attack_damage: f32,
    pub attack_range: f32,
    pub attack_cooldown: f32,
    pub chase_range: f32,
    pub patrol_radius: f32,
    pub color_r: f32,
    pub color_g: f32,
    pub color_b: f32,
    #[serde(default = "default_xp")]
    pub xp: f32,
    #[serde(default)]
    pub resist: Resist,
    /// Имя спрайт-листа (enemy_<sprite>.png); по умолчанию = id.
    #[serde(default)]
    pub sprite: Option<String>,
    /// Масштаб спрайта/коллайдера (1.0 = обычный, >1.2 = «крупный»).
    #[serde(default = "default_scale")]
    pub scale: f32,
    /// Боевое поведение: "melee" (в контакт, по умолчанию) | "ranged" (держит дистанцию).
    #[serde(default)]
    pub behavior: Option<String>,
    #[serde(default = "default_enemy_role")]
    pub role: String,
    /// id способностей из abilities.json (кастуются по кулдауну при видимости).
    #[serde(default)]
    pub abilities: Vec<String>,
    #[serde(default)]
    pub phases: Vec<BossPhaseCfg>,
    /// Шанс (0..1) прервать врага стаггером при получении урона.
    #[serde(default)]
    pub pain_chance: f32,
    /// id обученного поведения из `brains.json`; пусто — обычный автомат.
    #[serde(default)]
    pub brain: Option<String>,
    /// Статус на игрока при обычной атаке врага (мили/рывок).
    #[serde(default)]
    pub attack_status: Option<StatusHit>,
    #[serde(default)]
    pub weak_point: WeakPointCfg,
    #[serde(default)]
    pub animation: EnemyAnimationCfg,
}

fn default_enemy_role() -> String {
    "pursuer".to_string()
}

impl EnemyCfg {
    pub fn display_name(&self, lang: &str) -> &str {
        if lang == "en" && !self.name_en.is_empty() {
            &self.name_en
        } else {
            &self.name
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ItemCfg {
    pub id: String,
    pub name_ru: String,
    pub name_en: String,
    pub desc_ru: String,
    pub desc_en: String,
    pub value: f64,
    pub category: String,
    pub heal: Option<f32>,
    pub color_r: f32,
    pub color_g: f32,
    pub color_b: f32,
}

impl ItemCfg {
    pub fn name(&self, lang: &str) -> &str {
        if lang == "en" {
            &self.name_en
        } else {
            &self.name_ru
        }
    }
}

/// Ингредиент рецепта.
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct RecipeInput {
    pub item: String,
    #[serde(default = "one")]
    pub qty: u32,
}

/// Рецепт крафта из `recipes.json` пресета.
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct RecipeCfg {
    pub id: String,
    #[serde(default)]
    pub name_ru: String,
    #[serde(default)]
    pub name_en: String,
    pub inputs: Vec<RecipeInput>,
    /// id предмета-результата (из items.json).
    pub output: String,
    #[serde(default = "one")]
    pub output_qty: u32,
    /// Станция, у которой доступен рецепт: пусто — можно где угодно.
    #[serde(default)]
    pub station: String,
    /// Флаг прогресса, открывающий рецепт (чертёж, награда за квест).
    #[serde(default)]
    pub requires_flag: Option<String>,
    /// Сколько секунд занимает крафт (сервер держит каст).
    #[serde(default = "craft_time")]
    pub time: f32,
}

impl RecipeCfg {
    pub fn name(&self, lang: &str) -> &str {
        if lang == "en" && !self.name_en.is_empty() {
            &self.name_en
        } else {
            &self.name_ru
        }
    }
}

fn one() -> u32 {
    1
}

fn craft_time() -> f32 {
    1.0
}

#[derive(Debug, Deserialize, Clone)]
struct BrainsFile {
    #[serde(default)]
    brains: Vec<crate::ai::policy::Brain>,
}

#[derive(Debug, Deserialize, Clone)]
struct RecipesFile {
    #[serde(default)]
    recipes: Vec<RecipeCfg>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EnemySpawn {
    pub kind: String,
    pub x: f32,
    pub z: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ItemSpawn {
    pub kind: String,
    pub x: f32,
    pub z: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AmmoSpawn {
    pub kind: String,
    #[serde(deserialize_with = "de_u32")]
    pub amount: u32,
    pub x: f32,
    pub z: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WeaponSpawn {
    pub kind: String,
    pub x: f32,
    pub z: f32,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct LevelCfg {
    #[serde(default)]
    pub spawn_enemies: Vec<EnemySpawn>,
    #[serde(default)]
    pub spawn_items: Vec<ItemSpawn>,
    #[serde(default)]
    pub spawn_ammo: Vec<AmmoSpawn>,
    #[serde(default)]
    pub spawn_weapons: Vec<WeaponSpawn>,
}

// ── NPC ───────────────────────────────────────────────────────────────────────

/// NPC из npcs.json пресета. `scene`: "story" — динамические сцены из story.rs
/// (для 8 исходных персонажей), пусто/нет — сгенерированный квест-диалог по `quest`.
#[derive(Debug, Deserialize, Clone)]
pub struct NpcCfg {
    pub id: String,
    pub name_ru: String,
    #[serde(default)]
    pub name_en: String,
    #[serde(default)]
    pub sprite: String, // имя файла в characters/ (npc_vale...)
    pub pos: [f32; 2], // x, z на карте мира
    #[serde(default)]
    pub color: Option<[f32; 3]>,
    #[serde(default)]
    pub scene: Option<String>,
    #[serde(default)]
    pub quest: Option<String>,
}

impl NpcCfg {
    pub fn name(&self, lang: &str) -> &str {
        if lang == "en" && !self.name_en.is_empty() {
            &self.name_en
        } else {
            &self.name_ru
        }
    }
}

// ── Квесты ────────────────────────────────────────────────────────────────────

/// Квест из quests.json. kind: "kill" (target=id врага), "collect" (target=id предмета),
/// "clear_dungeon" (count = требуемая глубина).
#[derive(Debug, Deserialize, Clone)]
pub struct QuestCfg {
    pub id: String,
    pub title_ru: String,
    pub desc_ru: String,
    #[serde(default)]
    pub title_en: String,
    #[serde(default)]
    pub desc_en: String,
    pub giver: String,
    pub kind: String,
    #[serde(default)]
    pub target: String,
    #[serde(deserialize_with = "de_u32")]
    pub count: u32,
    #[serde(default, deserialize_with = "de_u32")]
    pub reward_xp: u32,
    #[serde(default, deserialize_with = "de_i32")]
    pub reward_gold: i32,
    #[serde(default)]
    pub reward_items: Vec<QuestRewardItemCfg>,
    #[serde(default)]
    pub chain_ru: String,
    #[serde(default)]
    pub chain_en: String,
    #[serde(default, deserialize_with = "de_u32")]
    pub stage: u32,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub offer_scene: Option<String>,
    #[serde(default)]
    pub progress_scene: Option<String>,
    #[serde(default)]
    pub complete_scene: Option<String>,
    #[serde(default)]
    pub world_change: Option<QuestWorldChangeCfg>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct QuestRewardItemCfg {
    pub id: String,
    #[serde(default = "d_depth_one", deserialize_with = "de_u32")]
    pub qty: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct QuestWorldChangeCfg {
    pub id: String,
    pub pattern: String,
    pub pos: [f32; 3],
    pub color: [f32; 3],
    #[serde(default = "d_world_change_scale")]
    pub scale: f32,
    #[serde(default)]
    pub sprite: String,
    #[serde(default)]
    pub activity: String,
    #[serde(default, deserialize_with = "de_u32")]
    pub activity_count: u32,
    #[serde(default = "d_world_change_radius")]
    pub activity_radius: f32,
    #[serde(default = "d_world_change_speed")]
    pub activity_speed: f32,
}

fn d_world_change_scale() -> f32 {
    1.0
}

fn d_world_change_radius() -> f32 {
    2.5
}

fn d_world_change_speed() -> f32 {
    1.0
}

impl QuestCfg {
    pub fn title(&self, lang: &str) -> &str {
        if lang == "en" && !self.title_en.is_empty() {
            &self.title_en
        } else {
            &self.title_ru
        }
    }

    pub fn chain(&self, lang: &str) -> &str {
        if lang == "en" && !self.chain_en.is_empty() {
            &self.chain_en
        } else {
            &self.chain_ru
        }
    }

    pub fn description(&self, lang: &str) -> &str {
        if lang == "en" && !self.desc_en.is_empty() {
            &self.desc_en
        } else {
            &self.desc_ru
        }
    }
}

// ── Статусы урона (statuses.json) ─────────────────────────────────────────────

fn d_status_tick() -> f32 {
    0.5
}

/// Определение статуса. kind: dot (урон по тику), slow (замедление),
/// stun (оглушение), vulnerable (+входящий урон). Применяется оружием/способностями.
#[derive(Debug, Deserialize, Clone)]
pub struct StatusCfg {
    pub id: String,
    pub name_ru: String,
    pub kind: String,
    #[serde(default)]
    pub duration: f32,
    // dot
    #[serde(default)]
    pub damage: f32, // урон за тик
    #[serde(default)]
    pub dmg_type: Option<String>,
    #[serde(default = "d_status_tick")]
    pub tick: f32, // секунд между тиками
    // slow / vulnerable
    #[serde(default)]
    pub amount: f32, // slow: 0.45=−45% скорости; vuln: 0.35=+35%
    #[serde(default)]
    pub tint: Option<[f32; 3]>,
    #[serde(default)]
    pub icon: Option<String>, // символ для HUD
}

/// Ссылка на статус при нанесении: id + шанс наложить.
#[derive(Debug, Deserialize, Clone)]
pub struct StatusHit {
    pub id: String,
    #[serde(default = "d_one")]
    pub chance: f32,
}

// ── Способности врагов (abilities.json) ──────────────────────────────────────

fn d_ab_cd() -> f32 {
    4.0
}
fn d_ab_maxr() -> f32 {
    14.0
}

/// Способность врага. kind определяет, какие поля значимы:
/// projectile_burst — count/spread/proj_speed/damage;
/// charge — speed_mult/duration/damage; summon — minion/count;
/// heal_pulse — heal/radius. Общие: cooldown, telegraph (подсветка перед
/// эффектом — окно на уворот), min_range/max_range (когда кастовать), color.
#[derive(Debug, Deserialize, Clone)]
pub struct AbilityCfg {
    pub id: String,
    pub kind: String,
    #[serde(default = "d_ab_cd")]
    pub cooldown: f32,
    #[serde(default)]
    pub telegraph: f32,
    #[serde(default)]
    pub min_range: f32,
    #[serde(default = "d_ab_maxr")]
    pub max_range: f32,
    #[serde(default)]
    pub color: Option<[f32; 3]>,
    // projectile_burst
    #[serde(default, deserialize_with = "de_u32")]
    pub count: u32,
    #[serde(default)]
    pub spread: f32,
    #[serde(default)]
    pub proj_speed: f32,
    #[serde(default)]
    pub damage: f32,
    // charge
    #[serde(default)]
    pub speed_mult: f32,
    #[serde(default)]
    pub duration: f32,
    // summon
    #[serde(default)]
    pub minion: Option<String>,
    // heal_pulse
    #[serde(default)]
    pub heal: f32,
    #[serde(default)]
    pub radius: f32,
    /// Статус, накладываемый на игрока при попадании (снаряд/рывок).
    #[serde(default)]
    pub status: Option<StatusHit>,
}

// ── Элитные аффиксы (affixes.json) ────────────────────────────────────────────

fn d_one() -> f32 {
    1.0
}

/// Аффикс элиты: элита = базовый враг + 1–2 аффикса (комбинаторика видов).
/// Мультипликаторы применяются поверх статов врага; tint подмешивается в цвет.
#[derive(Debug, Deserialize, Clone)]
pub struct AffixCfg {
    pub id: String,
    pub name_ru: String, // префикс имени: «Быстрый Грунт»
    #[serde(default)]
    pub tint: Option<[f32; 3]>,
    #[serde(default = "d_one")]
    pub hp_mult: f32,
    #[serde(default = "d_one")]
    pub dmg_mult: f32,
    #[serde(default = "d_one")]
    pub speed_mult: f32,
    #[serde(default = "d_one")]
    pub xp_mult: f32,
    /// Множитель pain_chance (<1 у «бронированных» — тяжелее прервать).
    #[serde(default = "d_one")]
    pub pain_mult: f32,
    /// Доля нанесённого игроку урона, возвращаемая элите как HP.
    #[serde(default)]
    pub lifesteal: f32,
    /// Взрыв при смерти: [урон, радиус] (урон игроку полный, врагам — половина).
    #[serde(default)]
    pub death_blast: Option<[f32; 2]>,
}

// ── Данж (dungeon.json): темы, пулы врагов, настройки ────────────────────────

/// Тема данжа. Текстуры — короткими именами (dtile_* → textures/dungeon,
/// см. map::tex_path) или полными res://-путями.
#[derive(Debug, Deserialize, Clone)]
pub struct ThemeCfg {
    pub name_ru: String,
    #[serde(default)]
    pub name_en: String,
    pub wall: String,
    pub accent: String,
    pub floor: String,
    pub ceil: String,
    pub lava: String,
    pub light: [f32; 3],
}

/// Пул врагов: действует с глубины min_depth (берётся самый глубокий из подходящих).
#[derive(Debug, Deserialize, Clone)]
pub struct PoolCfg {
    #[serde(deserialize_with = "de_u32")]
    pub min_depth: u32,
    pub enemies: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RoomArchetypeCfg {
    pub id: String,
    pub shape: String,
    #[serde(default)]
    pub role: String,
    #[serde(deserialize_with = "de_u32")]
    pub weight: u32,
    #[serde(deserialize_with = "de_u32")]
    pub min_size: u32,
    #[serde(deserialize_with = "de_u32")]
    pub max_size: u32,
    #[serde(default)]
    pub wall_height: Option<f32>,
    #[serde(default)]
    pub decor: String,
    #[serde(default)]
    pub hazard_chance: f32,
    #[serde(default)]
    pub hazard_dps: f32,
    #[serde(default)]
    pub hazard_radius: f32,
    #[serde(default)]
    pub hazard_kind: String,
    #[serde(default)]
    pub event_chance: f32,
    #[serde(default)]
    pub focal_pattern: String,
    #[serde(default)]
    pub focal_color: [f32; 3],
    #[serde(default = "d_one")]
    pub focal_scale: f32,
    #[serde(default = "d_one")]
    pub focal_height: f32,
}

fn d_boss() -> String {
    "brute".into()
}
fn d_boss_mult() -> f32 {
    1.25
}
fn d_mult_depth() -> f32 {
    0.18
}

fn d_elite_max() -> u32 {
    2
}
fn d_depth_one() -> u32 {
    1
}

#[derive(Debug, Deserialize, Clone)]
pub struct BossTierCfg {
    #[serde(default = "d_depth_one", deserialize_with = "de_u32")]
    pub min_depth: u32,
    pub boss: String,
    #[serde(default = "d_boss_mult")]
    pub mult: f32,
    #[serde(default)]
    pub guards: Vec<String>,
    #[serde(default)]
    pub items: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DungeonSettings {
    /// id босса (enemies.json)
    #[serde(default = "d_boss")]
    pub boss: String,
    #[serde(default = "d_boss_mult")]
    pub boss_mult: f32,
    /// свита босса (спавнится по бокам алтаря)
    #[serde(default)]
    pub boss_guards: Vec<String>,
    /// награда в боссовой комнате (id предметов)
    #[serde(default)]
    pub boss_items: Vec<String>,
    /// РОСТЕР боссов по глубинам; самый глубокий подходящий профиль заменяет legacy boss-поля.
    #[serde(default)]
    pub boss_roster: Vec<BossTierCfg>,
    /// прирост множителя hp/урона/XP за глубину
    #[serde(default = "d_mult_depth")]
    pub mult_per_depth: f32,
    /// пул оружейного тайника (id оружия)
    #[serde(default)]
    pub weapon_cache: Vec<String>,
    /// шанс элиты на спавн (базовый + прирост за глубину)
    #[serde(default)]
    pub elite_chance: f32,
    #[serde(default)]
    pub elite_per_depth: f32,
    /// максимум аффиксов на элиту (1..N)
    #[serde(default = "d_elite_max", deserialize_with = "de_u32")]
    pub elite_affixes_max: u32,
}

impl Default for DungeonSettings {
    fn default() -> Self {
        serde_json::from_str("{}").expect("DungeonSettings defaults")
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct DungeonCfg {
    #[serde(default)]
    pub themes: Vec<ThemeCfg>,
    #[serde(default)]
    pub pools: Vec<PoolCfg>,
    #[serde(default)]
    pub room_archetypes: Vec<RoomArchetypeCfg>,
    #[serde(default)]
    pub settings: DungeonSettings,
}

// ── Лут (loot.json) ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Clone)]
pub struct LootEntry {
    pub id: String,
    pub chance: f32,
}

/// Дроп с убитого врага. Записи проверяются по порядку ОДНИМ броском
/// (кумулятивно): сумма chance ≤ 1.0, остаток — «ничего».
#[derive(Debug, Deserialize, Clone)]
pub struct KillDrop {
    pub kind: String, // "ammo" (случайный тип) | "item"
    #[serde(default)]
    pub id: Option<String>, // id предмета для kind=item
    pub chance: f32,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct LootSettings {
    /// Шансы точек патронов в комнате данжа (каждая — своя позиция).
    #[serde(default)]
    pub room_ammo_chances: Vec<f32>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct LootCfg {
    #[serde(default)]
    pub room_items: Vec<LootEntry>,
    #[serde(default)]
    pub kill_drops: Vec<KillDrop>,
    #[serde(default)]
    pub settings: LootSettings,
}

#[derive(Deserialize, Default)]
pub(crate) struct EnemiesFile {
    pub(crate) enemies: Vec<EnemyCfg>,
}
#[derive(Deserialize, Default)]
pub(crate) struct ItemsFile {
    pub(crate) items: Vec<ItemCfg>,
}

// ── GameConfig ────────────────────────────────────────────────────────────────

pub struct GameConfig {
    pub enemies: Vec<EnemyCfg>,
    pub items: Vec<ItemCfg>,
    pub level: LevelCfg,
    pub npcs: Vec<NpcCfg>,
    pub quests: Vec<QuestCfg>,
    /// Data-driven сцены диалогов (dialogues.json); приоритетнее story.rs.
    pub dialogues: Vec<crate::dialogue::Scene>,
    /// Способности врагов (abilities.json); нет файла → встроенные core.
    pub abilities: Vec<AbilityCfg>,
    /// Элитные аффиксы (affixes.json); нет файла → встроенные core.
    pub affixes: Vec<AffixCfg>,
    /// Определения статусов (statuses.json); нет файла → встроенные core.
    pub statuses: Vec<StatusCfg>,
    /// Генерация данжей (dungeon.json); нет файла → встроенные настройки core.
    pub dungeon: DungeonCfg,
    /// Таблицы лута (loot.json); нет файла → встроенные настройки core.
    pub loot: LootCfg,
    /// npcs.json существует (пустой список ≠ отсутствие файла: пустой — это
    /// осознанное «в этом пресете NPC нет», отсутствие — legacy-фолбэк).
    pub npcs_file_present: bool,
    /// Рецепты крафта (recipes.json); пусто — в пресете крафта нет.
    pub recipes: Vec<RecipeCfg>,
    /// Обученные поведения врагов (brains.json); пусто — работает запасной автомат.
    pub brains: Vec<crate::ai::policy::Brain>,
}

fn read_data(base: &str, stem: &str) -> Option<String> {
    match crate::data::format::normalized_json(base, stem) {
        Some(Ok(text)) => Some(text),
        Some(Err(error)) => {
            crate::warn!("[preset] {base}/{stem}: {error}");
            None
        }
        None => None,
    }
}

// Встроенные копии core-пресета: битый JSON контент-мейкера не должен молча
// превращаться в пустой мир (без врагов/предметов/квестов) — падаем на них
// с предупреждением в лог (тот же принцип, что в weapon.rs/classes.rs/perk.rs).
const EMBEDDED_ENEMIES: &str = include_str!("../../../game/presets/core/enemies.json");
const EMBEDDED_ITEMS: &str = include_str!("../../../game/presets/core/items.json");
const EMBEDDED_LEVEL: &str = include_str!("../../../game/presets/core/level.json");
const EMBEDDED_NPCS: &str = include_str!("../../../game/presets/core/npcs.json");
const EMBEDDED_QUESTS: &str = include_str!("../../../game/presets/core/quests.json");
const EMBEDDED_DUNGEON: &str = include_str!("../../../game/presets/core/dungeon.json");
const EMBEDDED_LOOT: &str = include_str!("../../../game/presets/core/loot.json");
const EMBEDDED_ABILITIES: &str = include_str!("../../../game/presets/core/abilities.json");
const EMBEDDED_AFFIXES: &str = include_str!("../../../game/presets/core/affixes.json");
const EMBEDDED_STATUSES: &str = include_str!("../../../game/presets/core/statuses.json");

/// Распарсить текст конфига; при ошибке — громкое предупреждение и встроенная копия.
fn parse_loud<T: serde::de::DeserializeOwned + Default>(
    text: &str,
    file: &str,
    embedded: &str,
) -> T {
    match serde_json::from_str::<T>(text) {
        Ok(v) => v,
        Err(e) => {
            crate::warn!(
                "[preset] {file}: ошибка разбора ({e}) — использую встроенную копию core"
            );
            serde_json::from_str(embedded).unwrap_or_else(|e2| {
                crate::warn!(
                    "[preset] встроенная копия {file} тоже не парсится: {e2}"
                );
                T::default()
            })
        }
    }
}

/// Для cargo test: встроенные копии core обязаны парситься (последний рубеж фолбэков).
#[cfg(test)]
pub(crate) fn embedded_configs_parse_for_test() {
    serde_json::from_str::<EnemiesFile>(EMBEDDED_ENEMIES).expect("embedded enemies.json");
    serde_json::from_str::<ItemsFile>(EMBEDDED_ITEMS).expect("embedded items.json");
    serde_json::from_str::<LevelCfg>(EMBEDDED_LEVEL).expect("embedded level.json");
    serde_json::from_str::<Vec<NpcCfg>>(EMBEDDED_NPCS).expect("embedded npcs.json");
    serde_json::from_str::<Vec<QuestCfg>>(EMBEDDED_QUESTS).expect("embedded quests.json");
    let d = serde_json::from_str::<DungeonCfg>(EMBEDDED_DUNGEON).expect("embedded dungeon.json");
    assert!(
        !d.themes.is_empty() && !d.pools.is_empty(),
        "embedded dungeon.json: пустые themes/pools"
    );
    serde_json::from_str::<LootCfg>(EMBEDDED_LOOT).expect("embedded loot.json");
    serde_json::from_str::<Vec<AbilityCfg>>(EMBEDDED_ABILITIES).expect("embedded abilities.json");
    serde_json::from_str::<Vec<AffixCfg>>(EMBEDDED_AFFIXES).expect("embedded affixes.json");
    serde_json::from_str::<Vec<StatusCfg>>(EMBEDDED_STATUSES).expect("embedded statuses.json");
}

impl GameConfig {
    /// Загрузить конфиги из корня пресета (например "res://presets/core").
    ///
    /// Отсутствие файла и битый файл — разные состояния: отсутствие quests/level —
    /// осознанное «в этом пресете этого нет», отсутствие npcs — legacy-фолбэк на
    /// NPC_DATA; битый JSON всегда даёт предупреждение и встроенную копию core.
    pub fn load_from(base: &str) -> Self {

        let enemies = match read_data(base, "enemies") {
            Some(t) => parse_loud::<EnemiesFile>(&t, "enemies.json", EMBEDDED_ENEMIES),
            None => {
                crate::warn!(
                    "[preset] {base}/enemies.json не найден — использую встроенную копию core"
                );
                parse_loud::<EnemiesFile>(EMBEDDED_ENEMIES, "enemies.json", EMBEDDED_ENEMIES)
            }
        }
        .enemies;

        let items = match read_data(base, "items") {
            Some(t) => parse_loud::<ItemsFile>(&t, "items.json", EMBEDDED_ITEMS),
            None => {
                crate::warn!(
                    "[preset] {base}/items.json не найден — использую встроенную копию core"
                );
                parse_loud::<ItemsFile>(EMBEDDED_ITEMS, "items.json", EMBEDDED_ITEMS)
            }
        }
        .items;

        // level.json опционален: карты пресета (maps/*.json) несут свои спавны.
        let level = match read_data(base, "level") {
            Some(t) => parse_loud::<LevelCfg>(&t, "level.json", EMBEDDED_LEVEL),
            None => LevelCfg::default(),
        };

        // Рецептов может не быть вовсе — тогда в пресете просто нет крафта.
        let recipes: Vec<RecipeCfg> = match read_data(base, "recipes") {
            Some(text) => match serde_json::from_str::<RecipesFile>(&text) {
                Ok(file) => file.recipes,
                Err(error) => {
                    crate::warn!("[preset] recipes.json: {error} — крафт отключён");
                    Vec::new()
                }
            },
            None => Vec::new(),
        };

        // Веса поведения — необязательный файл: без него враги ходят по
        // обычному автомату, и это нормальное состояние пресета.
        let brains: Vec<crate::ai::policy::Brain> = match read_data(base, "brains") {
            Some(text) => match serde_json::from_str::<BrainsFile>(&text) {
                Ok(file) => file
                    .brains
                    .into_iter()
                    .filter(|brain| match brain.validate() {
                        Ok(()) => true,
                        Err(error) => {
                            crate::warn!(
                                "[preset] brains.json: поведение '{}' не подходит по форме ({:?}) — пропускаю",
                                brain.id,
                                error
                            );
                            false
                        }
                    })
                    .collect(),
                Err(error) => {
                    crate::warn!("[preset] brains.json: {error} — поведения не загружены");
                    Vec::new()
                }
            },
            None => Vec::new(),
        };

        let npcs_raw = read_data(base, "npcs");
        let npcs_file_present = npcs_raw.is_some();
        let npcs: Vec<NpcCfg> = match npcs_raw {
            Some(t) => parse_loud(&t, "npcs.json", EMBEDDED_NPCS),
            None => Vec::new(),
        };

        // квестов может осознанно не быть (например, чистая арена)
        let quests: Vec<QuestCfg> = match read_data(base, "quests") {
            Some(t) => parse_loud(&t, "quests.json", EMBEDDED_QUESTS),
            None => Vec::new(),
        };

        // генерация данжей и лут: нет файла — молча встроенные core-настройки
        // (данжи должны работать в любом пресете); битый файл — предупреждение.
        let mut dungeon = match read_data(base, "dungeon") {
            Some(t) => parse_loud::<DungeonCfg>(&t, "dungeon.json", EMBEDDED_DUNGEON),
            None => serde_json::from_str(EMBEDDED_DUNGEON).unwrap_or_default(),
        };
        // семантические минимумы: без тем/пулов генератор не сможет работать
        if dungeon.themes.is_empty() || dungeon.pools.is_empty() {
            crate::warn!("[preset] dungeon.json: пустые themes/pools — использую встроенные core");
            let emb: DungeonCfg = serde_json::from_str(EMBEDDED_DUNGEON).unwrap_or_default();
            if dungeon.themes.is_empty() {
                dungeon.themes = emb.themes;
            }
            if dungeon.pools.is_empty() {
                dungeon.pools = emb.pools;
            }
        }
        let loot = match read_data(base, "loot") {
            Some(t) => parse_loud::<LootCfg>(&t, "loot.json", EMBEDDED_LOOT),
            None => serde_json::from_str(EMBEDDED_LOOT).unwrap_or_default(),
        };

        // способности врагов: нет файла — молча встроенные core (как dungeon/loot)
        let abilities: Vec<AbilityCfg> = match read_data(base, "abilities") {
            Some(t) => parse_loud(&t, "abilities.json", EMBEDDED_ABILITIES),
            None => serde_json::from_str(EMBEDDED_ABILITIES).unwrap_or_default(),
        };

        // аффиксы элит: нет файла — молча встроенные core
        let affixes: Vec<AffixCfg> = match read_data(base, "affixes") {
            Some(t) => parse_loud(&t, "affixes.json", EMBEDDED_AFFIXES),
            None => serde_json::from_str(EMBEDDED_AFFIXES).unwrap_or_default(),
        };

        // статусы урона: нет файла — молча встроенные core
        let statuses: Vec<StatusCfg> = match read_data(base, "statuses") {
            Some(t) => parse_loud(&t, "statuses.json", EMBEDDED_STATUSES),
            None => serde_json::from_str(EMBEDDED_STATUSES).unwrap_or_default(),
        };

        // диалоги: отсутствие файла — норма (story.rs остаётся встроенным контентом)
        let dialogues = match read_data(base, "dialogues") {
            None => Vec::new(),
            Some(t) => match crate::dialogue::parse_scenes(&t) {
                Ok((scenes, errors)) => {
                    for e in errors {
                        crate::warn!("[preset] dialogues.json: {e} — сцена пропущена");
                    }
                    scenes
                }
                Err(e) => {
                    crate::warn!("[preset] dialogues.json: ошибка разбора ({e}) — диалоги пресета не загружены");
                    Vec::new()
                }
            },
        };

        Self {
            enemies,
            items,
            level,
            npcs,
            quests,
            dialogues,
            abilities,
            affixes,
            statuses,
            dungeon,
            loot,
            npcs_file_present,
            recipes,
            brains,
        }
    }

    /// Рецепт по id.
    pub fn recipe(&self, id: &str) -> Option<&RecipeCfg> {
        self.recipes.iter().find(|recipe| recipe.id == id)
    }

    /// JSON-сцена диалога по id (приоритетнее story.rs — см. game.rs).
    pub fn ability(&self, id: &str) -> Option<&AbilityCfg> {
        self.abilities.iter().find(|a| a.id == id)
    }

    pub fn dialogue(&self, id: &str) -> Option<&crate::dialogue::Scene> {
        self.dialogues.iter().find(|s| s.id == id)
    }

    pub fn enemy(&self, id: &str) -> Option<&EnemyCfg> {
        self.enemies.iter().find(|e| e.id == id)
    }

    pub fn item(&self, id: &str) -> Option<&ItemCfg> {
        self.items.iter().find(|i| i.id == id)
    }

    pub fn quest(&self, id: &str) -> Option<&QuestCfg> {
        self.quests.iter().find(|q| q.id == id)
    }
}
