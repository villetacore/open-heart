//! Game3D — главный узел игры.
//!
//! Открытый мир (хаб + пустоши) и процедурные данжи, RPG-классы (3×3 спека),
//! DOOM-боёвка: hitscan / мили / снаряды, FP-спрайт оружия на HUD.

use godot::classes::audio_stream_wav::Format as WavFormat;
use godot::classes::base_material_3d::TextureFilter;
use godot::classes::environment::{AmbientSource, BgMode, ToneMapper};
use godot::classes::file_access::ModeFlags;
use godot::classes::image::Format;
use godot::classes::{
    AtlasTexture, AudioStream, AudioStreamPlayer, AudioStreamPlayer3D, AudioStreamWav, CanvasLayer,
    CharacterBody3D, DirectionalLight3D, Environment, FileAccess, INode3D, Image, ImageTexture,
    Input, InputEvent, InputEventKey, Label, Node3D, OmniLight3D, Panel, PanoramaSkyMaterial,
    PhysicsRayQueryParameters3D, ScrollContainer, Sky, Sprite3D, StyleBoxFlat, Texture2D,
    TextureRect, VBoxContainer, WorldEnvironment,
};
use godot::global::HorizontalAlignment;
use godot::prelude::*;

use std::collections::HashMap;

use crate::classes::{classes, compute_loadout, xp_to_next, ClassDef, Loadout};
use crate::config::GameConfig;
use crate::dialogue::{Choice, Effect, Line, Scene};
use crate::dungeon::{self, DungeonPlan};
use crate::enemy::Enemy;
use crate::game_state::GameState;
use crate::gfx::{make_billboard, make_glow_slab, make_light, Rng, TexCache};
use crate::locale::t;
use crate::convert;
use crate::nav::NavGrid;
use crate::player::Player;
use crate::save;
use crate::settings::Settings;
use crate::story::get_scene;
#[cfg(debug_assertions)]
use crate::weapon::weapon_mods;
use crate::weapon::{
    weapon_def, weapon_mod_for, AmmoType, Arsenal, DmgType, FireKind, WeaponFeedback, WeaponId,
    FRAME_W,
};
use crate::world;

/// Идентификатор игрока в комнате. Совпадает с `peer` сетевого протокола
/// (proto/schema.md); 0 зарезервирован под «нет игрока».
pub type PeerId = u16;

/// Peer локального игрока в одиночной игре.
pub const LOCAL_PEER: PeerId = 1;


// ── Режимы и полезная нагрузка предметов ─────────────────────────────────────

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    ClassSelect,
    SpecSelect,
    Explore,
    Dialogue,
    Dead,
    Inventory,
    Craft,
    Journal,
    Perks,
    Paused,
    Epilogue,
    Shop,
    Creative,
}

#[derive(PartialEq, Clone, Copy)]
enum Loc {
    World,
    Dungeon,
}

#[allow(dead_code)]
enum Payload {
    Consumable { heal: f32 },
    Gold(i32),
    Ammo(AmmoType, u32),
    Weapon(WeaponId),
    Heart,
    KeyItem,
}

struct WorldItemNode {
    node: Gd<Node3D>,
    item_id: String,
    name: String,
    payload: Payload,
    in_dungeon: bool,
    /// Высота спрайта над полом. Пикапы хаба приземляются лучом на реальную
    /// поверхность (рампы/платформы), сохраняя этот зазор — иначе торчали бы
    /// под приподнятыми блоками (Y спавнов карты жёстко 0).
    ground_offset: f32,
}

struct DungeonEventNode {
    kind: String,
    group: u32,
    step: u32,
    pos: Vector3,
    marker: Option<Gd<Sprite3D>>,
    used: bool,
}

/// Рантайм-описание NPC (из npcs.json пресета или legacy NPC_DATA).
struct NpcRt {
    id: String,
    name: String,
    scene: Option<String>, // "story" → динамика story.rs; иначе конкретный id сцены
    quest: Option<String>, // id квеста из quests.json (гивер)
}

struct SpriteFx {
    node: Gd<Sprite3D>,
    ttl: f32,
    total: f32,
    color: Color,
}
struct LightFx {
    node: Gd<OmniLight3D>,
    ttl: f32,
    total: f32,
    energy: f32,
}

struct Projectile {
    node: Gd<Node3D>,
    pos: Vector3,
    vel: Vector3,
    dmg: f32,
    dmg_type: DmgType,
    splash: f32,
    ttl: f32,
    feedback: WeaponFeedback,
    weapon: WeaponId,
    status: Option<(String, f32)>, // статус оружия при попадании
}

/// Прямое попадание снаряда: (враг, урон, тип, точка, статус оружия).
type DirectHit = (
    Gd<Enemy>,
    f32,
    DmgType,
    Vector3,
    Option<(String, f32)>,
    WeaponFeedback,
    WeaponId,
);
/// Убитый враг для пост-обработки: (позиция, XP, босс?, id, посмертный взрыв).
type KillInfo = (Vector3, f32, bool, String, Option<(f32, f32)>);

/// Снаряд врага (abilities.json: projectile_burst) — летит в игрока, можно увернуться.
struct EnemyProjectile {
    node: Gd<Node3D>,
    pos: Vector3,
    vel: Vector3,
    dmg: f32,
    ttl: f32,
    status: Option<String>, // статус на игрока при попадании (id)
}

#[derive(PartialEq, Clone, Copy)]
#[allow(clippy::enum_variant_names)]
enum PortalKind {
    EnterDungeon,
    ExitDungeon,
    DeeperDungeon,
}

// ── Анимация FP-оружия ────────────────────────────────────────────────────────

#[derive(PartialEq, Clone, Copy)]
enum WeaponAnim {
    Idle,
    Fire { frame: usize, secondary: bool },
    Reload(f32),
    Switch(f32),
}

// ── Главная структура ─────────────────────────────────────────────────────────

#[derive(GodotClass)]
#[class(base = Node3D)]
pub struct Game3D {
    base: Base<Node3D>,

    cache: TexCache,
    rng: Rng,
    cfg: Option<GameConfig>,

    preset: String,
    gate_pos: Vector3,
    world_name: String,
    world_districts: Vec<crate::map::MapDistrict>,
    /// Зоны станций крафта в текущем мире: (kind, центр, радиус).
    station_zones: Vec<(String, Vector3, f32)>,
    map_ambient: Vec<crate::map::MapAmbient>,
    active_district: Option<usize>,

    /// Игроки комнаты по peer-идентификатору. В одиночной игре здесь ровно один
    /// локальный игрок; сетевые копии добавятся на этапе N1 (docs/MULTIPLAYER.md §12).
    players: HashMap<PeerId, Gd<CharacterBody3D>>,
    /// Чей игрок «наш»: к нему привязаны камера, HUD и режимы.
    local_peer: PeerId,
    /// Счётчик стабильных id сущностей: по ним сервер и клиент говорят об одном
    /// и том же враге. Индексы в `enemies` для этого не годятся — они плывут.
    next_entity_id: u32,
    /// Соединение с игровым сервером; None — одиночная игра.
    net: Option<crate::net::NetClient>,
    /// Чужие игроки из снапшотов.
    remote_players: HashMap<PeerId, net::NetEntity>,
    /// Враги, которых ведёт сервер (в одиночной игре пусто).
    net_enemies: HashMap<u16, net::NetEntity>,
    /// Наш лут на земле: сервер шлёт его только нам.
    net_items: HashMap<u16, net::NetEntity>,
    /// Какой предмет за каким id: узнаём из события `loot` при выпадении.
    net_item_kinds: HashMap<u16, String>,
    /// Куда сервер просит сместить локального игрока: подтягиваем плавно.
    net_correction: Option<Vector3>,
    /// Экран крафта: панель, список и рецепты в порядке показа.
    craft_panel: Option<Gd<Panel>>,
    craft_list: Option<Gd<Label>>,
    craft_shown: Vec<String>,
    /// Станции крафта рядом с игроком (пока пусто — появятся со строительством).
    craft_stations: Vec<String>,
    /// Сколько кадров ещё приземлять пикапы хаба на пол (физика геометрии
    /// готова не сразу после сборки мира — снап повторяется несколько кадров).
    ground_snap_ticks: i32,
    npc_sprites: Vec<Gd<Sprite3D>>,
    npcs: Vec<NpcRt>,
    enemies: Vec<Gd<Enemy>>,
    world_items: Vec<WorldItemNode>,
    projectiles: Vec<Projectile>,
    enemy_projectiles: Vec<EnemyProjectile>,
    sprite_fx: Vec<SpriteFx>,
    light_fx: Vec<LightFx>,
    sfx_2d: Vec<Gd<AudioStreamPlayer>>,
    sfx_3d: Vec<Gd<AudioStreamPlayer3D>>,

    // Реактивный пост-процесс: хэндл материала + затухающие импульсы эффектов
    post_mat: Option<Gd<godot::classes::ShaderMaterial>>,
    fx_hit: f32,  // получен урон  → красная пульсация/аберрация
    fx_kill: f32, // убийство      → короткий яркий «панч»
    fx_pick: f32, // подбор предмета→ тёплое золотистое свечение

    // Анимация HUD: плавные бары + пульс при низком HP
    hp_shown: f32, // отображаемая доля HP (лерп к цели)
    xp_shown: f32,
    hp_target: f32,
    xp_target: f32,
    hud_time: f32, // время для пульсаций

    state: Option<GameState>,
    settings: Settings,
    arsenal: Arsenal,
    loadout: Loadout,
    mode: Mode,
    loc: Loc,

    // данж
    dungeon_root: Option<Gd<Node3D>>,
    dungeon_depth: u32,
    dungeon_name: String,
    dungeon_room_roles: Vec<String>,
    dungeon_room_markers: Vec<(String, Vector3)>,
    dungeon_dressing_nodes: usize,
    dungeon_dressing_variants: Vec<u8>,
    dungeon_nav: Option<std::sync::Arc<NavGrid>>,
    exit_portal: Vector3,
    next_portal: Vector3,
    boss_alive: bool,
    dungeon_hazards: Vec<crate::dungeon::HazardZone>,
    dungeon_hazard_tick: f32,
    dungeon_hazard_time: f32,
    dungeon_hazard_active: bool,

    // диалог
    scene: Option<Scene>,
    line_idx: usize,
    at_choices: bool,

    near_npc: Option<usize>,
    near_enemy: Option<usize>,
    near_item: Option<usize>,
    near_portal: Option<PortalKind>,
    near_dungeon_event: Option<usize>,
    dungeon_events: Vec<DungeonEventNode>,

    shoot_cd: f32,
    weapon_anim: WeaponAnim,
    melee_impact_pending: Option<(f32, f32, DmgType, usize)>,
    weapon_impact_sfx_pending: bool,
    weapon_bloom: f32,
    hit_confirm_timer: f32,
    critical_confirm_timer: f32,
    kill_confirm_timer: f32,
    anim_timer: f32,
    idle_frame: usize,

    npc_anim_timer: f32,
    npc_anim_frame: usize,

    class_pick: usize, // выбранный класс на этапе выбора спека

    // статусы игрока (горение/кровь/замедление/уязвимость от врагов)
    player_statuses: crate::status::StatusSet,

    // HUD
    hint_label: Option<Gd<Label>>,
    status_label: Option<Gd<Label>>,
    hp_bar_fg: Option<Gd<Panel>>,
    hp_label: Option<Gd<Label>>,
    xp_bar_fg: Option<Gd<Panel>>,
    xp_label: Option<Gd<Label>>,
    ammo_label: Option<Gd<Label>>,
    weapon_label: Option<Gd<Label>>,
    loc_label: Option<Gd<Label>>,
    dlg_panel: Option<Gd<Panel>>,
    dlg_speaker: Option<Gd<Label>>,
    dlg_text: Option<Gd<Label>>,
    choice_box: Option<Gd<VBoxContainer>>,
    cl0: Option<Gd<Label>>,
    cl1: Option<Gd<Label>>,
    cl2: Option<Gd<Label>>,
    cl3: Option<Gd<Label>>,
    flash_label: Option<Gd<Label>>,
    flash_timer: f32,
    inv_label: Option<Gd<Label>>,
    quest_label: Option<Gd<Label>>,
    quest_objective_label: Option<Gd<Label>>,
    quest_objective_marker: Option<Gd<Sprite3D>>,
    quest_objective_target: Option<Vector3>,
    tracked_quest_index: usize,
    inv_panel: Option<Gd<Panel>>,
    inv_list: Option<Gd<Label>>,
    ability_labels: Vec<Gd<Label>>,
    ability_icons: Vec<Gd<AtlasTexture>>,
    inventory_grid: Option<Gd<godot::classes::GridContainer>>,
    inventory_detail: Option<Gd<Label>>,
    inventory_use: Option<Gd<godot::classes::Button>>,
    selected_item: String,
    inventory_filter: i64,
    perk_grid: Option<Gd<godot::classes::GridContainer>>,
    journal_buttons: Option<Gd<VBoxContainer>>,
    journal_panel: Option<Gd<Panel>>,
    journal_list: Option<Gd<Label>>,
    perk_panel: Option<Gd<Panel>>,
    perk_list: Option<Gd<Label>>,
    perk_respec: Option<Gd<godot::classes::Button>>,
    perk_craft: Option<Gd<godot::classes::Button>>,
    /// Слой магазина Торговца: создаётся при открытии, удаляется при закрытии.
    shop_layer: Option<Gd<CanvasLayer>>,
    crosshair: Option<Gd<Label>>,
    dead_panel: Option<Gd<Panel>>,
    pause_panel: Option<Gd<Panel>>,
    epilogue_panel: Option<Gd<Panel>>,
    enemies_frozen: bool,
    compass_label: Option<Gd<Label>>,
    targeting_label: Option<Gd<Label>>,
    damage_flash: Option<Gd<Panel>>,
    damage_flash_timer: f32,

    weapon_rect: Option<Gd<TextureRect>>,
    weapon_atlas: Option<Gd<AtlasTexture>>,
    muzzle_light: Option<Gd<OmniLight3D>>,
    muzzle_timer: f32,

    // миникарта данжа (floor_map генератора → текстура + точка игрока)
    minimap_bg: Option<Gd<Panel>>,
    minimap_rect: Option<Gd<TextureRect>>,
    minimap_dot: Option<Gd<Panel>>,
    minimap_legend: Option<Gd<Label>>,
    minimap_floor: Vec<bool>,

    // выбор класса
    select_panel: Option<Gd<Panel>>,
    select_title: Option<Gd<Label>>,
    card_titles: Vec<Gd<Label>>,
    card_bodies: Vec<Gd<Label>>,
    class_portraits: Vec<Gd<AtlasTexture>>,

    game_time: f32,
    autosave_enabled: bool,
    creative: bool,
    creative_panel: Option<Gd<Panel>>,
    dialogue_actions: Option<Gd<VBoxContainer>>,
    fashion_icon: Option<Gd<TextureRect>>,
    weapon_icon: Option<Gd<TextureRect>>,
    dialogue_portrait: Option<Gd<TextureRect>>,
    /// Сколько прошло с последней отправки персонажа на сервер.
    net_save_timer: f32,
}

// ── Константы ─────────────────────────────────────────────────────────────────

const INTERACT_R: f32 = 2.8;
const PICKUP_R: f32 = 1.6;
const PORTAL_R: f32 = 2.4;
const PIXEL_SZ: f32 = 0.010;
const HUD_W: f32 = 1920.0;
const HUD_H: f32 = 1080.0;

const DUNGEON_OFFSET: Vector3 = Vector3::new(500.0, 0.0, 500.0);

// ── Звуковые эффекты (варианты — выбираем случайный для разнообразия) ──────────
const SFX_DEATH: [&str; 2] = [
    "res://assets/sounds/The Evil Robot Dies.wav",
    "res://assets/sounds/The Evil Robot Dies1.wav",
];
const SFX_WALK: [&str; 2] = [
    "res://assets/sounds/An Evil Robot Is Walking.wav",
    "res://assets/sounds/An Evil Robot Is Walking1.wav",
];

const NPC_IDLE_FRAMES: [(f32, f32, f32, f32); 4] = [
    (0.0, 0.0, 128.0, 256.0),
    (128.0, 0.0, 128.0, 256.0),
    (256.0, 0.0, 128.0, 256.0),
    (384.0, 0.0, 128.0, 256.0),
];
const IDLE_FPS: f32 = 5.0;

const C_UI_BG: Color = Color::from_rgba(0.04, 0.03, 0.07, 0.94);
const C_BORDER: Color = Color::from_rgba(0.65, 0.30, 0.52, 1.0);
const C_MAIN: Color = Color::from_rgba(0.95, 0.92, 0.98, 1.0);
const C_DIM: Color = Color::from_rgba(0.58, 0.52, 0.66, 1.0);
const C_PINK: Color = Color::from_rgba(1.00, 0.55, 0.80, 1.0);
const C_GOLD: Color = Color::from_rgba(1.00, 0.84, 0.30, 1.0);
const C_RED: Color = Color::from_rgba(0.90, 0.15, 0.15, 1.0);
const C_CYAN: Color = Color::from_rgba(0.40, 0.90, 1.00, 1.0);
const C_XP: Color = Color::from_rgba(0.55, 0.35, 0.95, 1.0);

impl Game3D {
    // ── игроки комнаты ────────────────────────────────────────────────────────
    //
    // Аксессоры отдают клон Gd-хендла, а не ссылку: вокруг слишком много методов
    // по &mut self, и удержанное заимствование поля ломало бы их. Клон Gd —
    // это копия указателя, не узла.

    /// Локальный игрок — тот, за кого играет этот клиент.
    pub(super) fn player(&self) -> Option<Gd<CharacterBody3D>> {
        self.players.get(&self.local_peer).cloned()
    }

    /// Игрок по peer: в кооперативе так адресуются чужие игроки.
    #[allow(dead_code)] // используется с этапа N1 (сетевые игроки)
    pub(super) fn player_of(&self, peer: PeerId) -> Option<Gd<CharacterBody3D>> {
        self.players.get(&peer).cloned()
    }

    /// Все игроки комнаты — для аггро врагов и HUD-меток пати.
    #[allow(dead_code)] // используется с этапа N2 (таблица угрозы)
    pub(super) fn players_iter(&self) -> impl Iterator<Item = (PeerId, Gd<CharacterBody3D>)> + '_ {
        self.players.iter().map(|(peer, node)| (*peer, node.clone()))
    }

    /// Назначить локального игрока (вызывается при сборке сцены).
    pub(super) fn set_local_player(&mut self, node: Gd<CharacterBody3D>) {
        if let Ok(mut player) = node.clone().try_cast::<Player>() {
            player.bind_mut().set_local(true);
        }
        let peer = self.local_peer;
        self.players.insert(peer, node);
    }

    /// Добавить сетевую копию чужого игрока (этап N1).
    #[allow(dead_code)]
    pub(super) fn add_remote_player(&mut self, peer: PeerId, node: Gd<CharacterBody3D>) {
        if peer == self.local_peer {
            godot_warn!("[net] попытка добавить чужого игрока с локальным peer {peer}");
            return;
        }
        if let Ok(mut player) = node.clone().try_cast::<Player>() {
            player.bind_mut().set_local(false);
        }
        self.players.insert(peer, node);
    }

    /// Убрать игрока из комнаты; локального убрать нельзя.
    #[allow(dead_code)]
    pub(super) fn remove_player(&mut self, peer: PeerId) -> Option<Gd<CharacterBody3D>> {
        if peer == self.local_peer {
            return None;
        }
        self.players.remove(&peer)
    }

    // ── стабильные id сущностей ───────────────────────────────────────────────

    /// Выдать следующий id сущности (враг, снаряд): по нему сервер и клиент
    /// говорят об одном объекте.
    pub(super) fn take_entity_id(&mut self) -> u32 {
        let id = self.next_entity_id;
        self.next_entity_id = self.next_entity_id.wrapping_add(1).max(1);
        id
    }

    /// Найти врага по стабильному id.
    #[allow(dead_code)]
    pub(super) fn enemy_by_id(&self, id: u32) -> Option<Gd<Enemy>> {
        self.enemies
            .iter()
            .find(|enemy| enemy.bind().entity_id() == id)
            .cloned()
    }

}

#[godot_api]
impl Game3D {
    #[func]
    fn ui_creative_action(&mut self, action: i64) { self.creative_action(action); }
    #[func]
    fn ui_dialogue_action(&mut self, action: i64) {
        if self.mode != Mode::Dialogue { return; }
        if action == -2 { self.end_dialogue(); }
        else if action == -1 { self.advance_dialogue(); }
        else if self.at_choices { self.select_choice(action as usize); }
    }
    #[func]
    fn ui_select_item(&mut self, id: GString) { self.ui_select_item_impl(id); }
    #[func]
    fn ui_inventory_filter(&mut self, filter: i64) { self.ui_inventory_filter_impl(filter); }
    #[func]
    fn ui_weapon_mod(&mut self, branch: i64) { self.ui_weapon_mod_impl(branch); }
    #[func]
    pub(super) fn ui_use_item(&mut self) { self.ui_use_item_impl(); }
    #[func]
    fn ui_buy_perk(&mut self, id: GString) { self.ui_buy_perk_impl(id); }
    #[func]
    fn ui_respec_perks(&mut self) { self.ui_respec_perks_impl(); }
    #[func]
    fn ui_craft_perk(&mut self) { self.ui_craft_perk_impl(); }
    #[func]
    fn ui_shop_buy(&mut self, id: GString) { self.ui_shop_buy_impl(id); }
    #[func]
    fn ui_close_shop(&mut self) { self.close_shop(); }
    #[func]
    fn ui_close_panel(&mut self) { self.ui_close_panel_impl(); }
    #[func]
    fn ui_track_quest(&mut self, index: i64) { self.ui_track_quest_impl(index); }
    #[func]
    fn ui_finish_campaign(&mut self) { self.finish_epilogue(); }
}

fn make_style(bg: Color, border: Color, width: i32) -> Gd<StyleBoxFlat> {
    let mut s = StyleBoxFlat::new_gd();
    s.set_bg_color(bg);
    s.set_border_color(border);
    s.set_border_width_all(width);
    s.set_corner_radius_all(4);
    s.set_content_margin_all(8.0);
    s
}

/// Привязать Control к экрану: якорь (ax, ay ∈ {0.0, 0.5, 1.0} — лево/центр/право
/// и верх/центр/низ) + смещения из дизайн-координат (HUD_W×HUD_H). На 16:9 позиция
/// идентична исходной, на других соотношениях элемент липнет к своему краю/углу
/// (работает вместе с display stretch aspect=expand).
fn place<T>(c: &Gd<T>, ax: f32, ay: f32, x: f32, y: f32, w: f32, h: f32)
where
    T: godot::obj::Inherits<godot::classes::Control>,
{
    use godot::builtin::Side;
    let mut c: Gd<godot::classes::Control> = c.clone().upcast();
    c.set_anchor(Side::LEFT, ax);
    c.set_anchor(Side::RIGHT, ax);
    c.set_anchor(Side::TOP, ay);
    c.set_anchor(Side::BOTTOM, ay);
    c.set_offset(Side::LEFT, x - ax * HUD_W);
    c.set_offset(Side::RIGHT, x + w - ax * HUD_W);
    c.set_offset(Side::TOP, y - ay * HUD_H);
    c.set_offset(Side::BOTTOM, y + h - ay * HUD_H);
}


// ── Подмодули: реализация Game3D разложена по концернам ───────────────────────
mod tables;
use tables::*;

mod lifecycle;
mod class_select;
mod abilities;
mod campaign;
mod interface;
pub(crate) mod creative;
mod presentation;
#[cfg(debug_assertions)]
mod content_smoke;
mod combat;
mod conversation;
mod delve;
mod environment;
mod gameplay;
mod hud;
mod hud_update;
mod crafting;
mod items;
mod net;
mod shop;
#[cfg(debug_assertions)]
mod runtime_smoke;
