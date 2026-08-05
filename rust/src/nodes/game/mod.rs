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

use crate::classes::{classes, compute_loadout, xp_to_next, ClassDef, Loadout};
use crate::config::GameConfig;
use crate::dialogue::{Choice, Effect, Line, Scene};
use crate::dungeon::{self, DungeonPlan};
use crate::enemy::Enemy;
use crate::game_state::GameState;
use crate::gfx::{make_billboard, make_glow_slab, make_light, Rng, TexCache};
use crate::locale::t;
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

// ── NPC ───────────────────────────────────────────────────────────────────────

#[allow(dead_code)]
struct NpcCfg {
    id: &'static str,
    name: &'static str,
    scene_id: &'static str,
    pos: Vector3,
    color: Color,
}

const NPC_DATA: &[NpcCfg] = &[
    NpcCfg {
        id: "vale",
        name: "Ms. Вейл",
        scene_id: "meet_vale",
        pos: Vector3::new(-6.0, 0.0, -8.0),
        color: Color::from_rgba(1.0, 0.75, 0.85, 1.0),
    },
    NpcCfg {
        id: "victor",
        name: "Виктор",
        scene_id: "intro_victor",
        pos: Vector3::new(6.0, 0.0, -8.0),
        color: Color::from_rgba(0.75, 1.0, 0.8, 1.0),
    },
    NpcCfg {
        id: "elena",
        name: "Елена",
        scene_id: "first_elena",
        pos: Vector3::new(-11.0, 0.0, 3.0),
        color: Color::from_rgba(0.75, 0.8, 1.0, 1.0),
    },
    NpcCfg {
        id: "sofia",
        name: "София",
        scene_id: "meet_sofia",
        pos: Vector3::new(11.0, 0.0, 3.0),
        color: Color::from_rgba(1.0, 0.95, 0.7, 1.0),
    },
    NpcCfg {
        id: "guard",
        name: "Охранник",
        scene_id: "meet_guard",
        pos: Vector3::new(-2.5, 0.0, -18.0),
        color: Color::from_rgba(0.85, 0.85, 0.85, 1.0),
    },
    NpcCfg {
        id: "merchant",
        name: "Торговец",
        scene_id: "meet_merchant",
        pos: Vector3::new(17.5, 0.0, -1.0),
        color: Color::from_rgba(1.0, 0.85, 0.6, 1.0),
    },
    NpcCfg {
        id: "scientist",
        name: "Учёный",
        scene_id: "meet_scientist",
        pos: Vector3::new(-18.0, 0.0, 0.0),
        color: Color::from_rgba(0.7, 1.0, 1.0, 1.0),
    },
    NpcCfg {
        id: "stranger",
        name: "Незнакомец",
        scene_id: "meet_stranger",
        pos: Vector3::new(5.0, 0.0, 16.0),
        color: Color::from_rgba(0.8, 0.65, 0.95, 1.0),
    },
];

fn npc_sprite_tex(id: &str) -> (&'static str, &'static str) {
    match id {
        "vale" => (
            "res://assets/sprites/characters/npc_vale.png",
            "res://assets/sprites/femboy_pink.png",
        ),
        "victor" => (
            "res://assets/sprites/characters/npc_victor.png",
            "res://assets/sprites/femboy_dark2.png",
        ),
        "elena" => (
            "res://assets/sprites/characters/npc_elena.png",
            "res://assets/sprites/femboy_dark1.png",
        ),
        "sofia" => (
            "res://assets/sprites/characters/npc_sofia.png",
            "res://assets/sprites/femboy_pink.png",
        ),
        "guard" => (
            "res://assets/sprites/characters/npc_guard.png",
            "res://assets/sprites/femboy_dark2.png",
        ),
        "merchant" => (
            "res://assets/sprites/characters/npc_merchant.png",
            "res://assets/sprites/femboy_pink.png",
        ),
        "scientist" => (
            "res://assets/sprites/characters/npc_scientist.png",
            "res://assets/sprites/femboy_dark1.png",
        ),
        "stranger" => (
            "res://assets/sprites/characters/npc_stranger.png",
            "res://assets/sprites/femboy_dark2.png",
        ),
        _ => (
            "res://assets/sprites/femboy_dark1.png",
            "res://assets/sprites/femboy_dark1.png",
        ),
    }
}

fn item_sprite_tex(id: &str) -> &'static str {
    match id {
        "medkit" => "res://assets/sprites/items/item_medkit.png",
        "key" => "res://assets/sprites/items/item_key.png",
        "gold_coin" | "gold_stack" => "res://assets/sprites/items/item_gold.png",
        "armor_shard" => "res://assets/sprites/items/item_armor.png",
        "energy_drink" => "res://assets/sprites/items/item_energy_drink.png",
        "potion" => "res://assets/sprites/items/item_potion.png",
        "ancient_ruby" => "res://assets/sprites/items/item_ruby.png",
        "heart_1up" => "res://assets/sprites/pickups/heart_1up.png",
        "soul" => "res://assets/sprites/pickups/soul.png",
        _ => "",
    }
}

// ── Режимы и полезная нагрузка предметов ─────────────────────────────────────

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    ClassSelect,
    SpecSelect,
    Explore,
    Dialogue,
    Dead,
    Inventory,
    Journal,
    Perks,
    Paused,
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
    map_ambient: Vec<crate::map::MapAmbient>,
    active_district: Option<usize>,

    player: Option<Gd<CharacterBody3D>>,
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
    journal_panel: Option<Gd<Panel>>,
    journal_list: Option<Gd<Label>>,
    perk_panel: Option<Gd<Panel>>,
    perk_list: Option<Gd<Label>>,
    crosshair: Option<Gd<Label>>,
    dead_panel: Option<Gd<Panel>>,
    pause_panel: Option<Gd<Panel>>,
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

    game_time: f32,
    autosave_enabled: bool,
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

// ── INode3D ───────────────────────────────────────────────────────────────────

#[godot_api]
impl INode3D for Game3D {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            cache: TexCache::new(),
            rng: Rng::new(0xBADA55),
            cfg: None,
            preset: "core".into(),
            gate_pos: world::GATE_POS,
            world_name: "ПУСТОШИ НЕОНОВОГО СЕРДЦА".into(),
            world_districts: Vec::new(),
            map_ambient: Vec::new(),
            active_district: None,
            player: None,
            npc_sprites: Vec::new(),
            npcs: Vec::new(),
            enemies: Vec::new(),
            world_items: Vec::new(),
            projectiles: Vec::new(),
            enemy_projectiles: Vec::new(),
            sprite_fx: Vec::new(),
            light_fx: Vec::new(),
            sfx_2d: Vec::new(),
            sfx_3d: Vec::new(),
            post_mat: None,
            fx_hit: 0.0,
            fx_kill: 0.0,
            fx_pick: 0.0,
            hp_shown: 1.0,
            xp_shown: 0.0,
            hp_target: 1.0,
            xp_target: 0.0,
            hud_time: 0.0,
            state: None,
            settings: Settings::default(),
            arsenal: Arsenal::new(),
            loadout: compute_loadout(0, 0, 1),
            mode: Mode::Explore,
            loc: Loc::World,
            dungeon_root: None,
            dungeon_depth: 0,
            dungeon_name: String::new(),
            dungeon_room_roles: Vec::new(),
            dungeon_room_markers: Vec::new(),
            dungeon_dressing_nodes: 0,
            dungeon_dressing_variants: Vec::new(),
            dungeon_nav: None,
            exit_portal: Vector3::ZERO,
            next_portal: Vector3::ZERO,
            boss_alive: false,
            dungeon_hazards: Vec::new(),
            dungeon_hazard_tick: 0.0,
            dungeon_hazard_time: 0.0,
            dungeon_hazard_active: false,
            scene: None,
            line_idx: 0,
            at_choices: false,
            near_npc: None,
            near_enemy: None,
            near_item: None,
            near_portal: None,
            near_dungeon_event: None,
            dungeon_events: Vec::new(),
            shoot_cd: 0.0,
            weapon_anim: WeaponAnim::Idle,
            melee_impact_pending: None,
            weapon_impact_sfx_pending: false,
            weapon_bloom: 0.0,
            hit_confirm_timer: 0.0,
            critical_confirm_timer: 0.0,
            kill_confirm_timer: 0.0,
            anim_timer: 0.0,
            idle_frame: 0,
            npc_anim_timer: 0.0,
            npc_anim_frame: 0,
            class_pick: 0,
            player_statuses: crate::status::StatusSet::new(),
            hint_label: None,
            status_label: None,
            hp_bar_fg: None,
            hp_label: None,
            xp_bar_fg: None,
            xp_label: None,
            ammo_label: None,
            weapon_label: None,
            loc_label: None,
            dlg_panel: None,
            dlg_speaker: None,
            dlg_text: None,
            choice_box: None,
            cl0: None,
            cl1: None,
            cl2: None,
            cl3: None,
            flash_label: None,
            flash_timer: 0.0,
            inv_label: None,
            quest_label: None,
            quest_objective_label: None,
            quest_objective_marker: None,
            quest_objective_target: None,
            tracked_quest_index: 0,
            inv_panel: None,
            inv_list: None,
            journal_panel: None,
            journal_list: None,
            perk_panel: None,
            perk_list: None,
            crosshair: None,
            dead_panel: None,
            pause_panel: None,
            enemies_frozen: false,
            compass_label: None,
            targeting_label: None,
            damage_flash: None,
            damage_flash_timer: 0.0,
            weapon_rect: None,
            weapon_atlas: None,
            muzzle_light: None,
            muzzle_timer: 0.0,
            minimap_bg: None,
            minimap_rect: None,
            minimap_dot: None,
            minimap_legend: None,
            minimap_floor: Vec::new(),
            select_panel: None,
            select_title: None,
            card_titles: Vec::new(),
            card_bodies: Vec::new(),
            game_time: 0.0,
            autosave_enabled: true,
        }
    }

    fn ready(&mut self) {
        self.settings = Settings::load();
        self.settings.apply_global(); // окно/vsync/громкость
        let lang = self.settings.lang.clone();

        let loaded = save::load();
        let has_class = loaded
            .as_ref()
            .map(|(s, _, _)| s.class_idx.is_some())
            .unwrap_or(false);
        // Пресет: у сейва приоритет (продолжаем ту игру, которую начали).
        let preset = loaded
            .as_ref()
            .map(|(s, _, _)| s.preset.clone())
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| self.settings.preset.clone());
        let (state, player_hp, arsenal) = match loaded {
            Some(v) => v,
            None => {
                let mut st = GameState::new("Игрок");
                st.preset = preset.clone();
                (st, 100.0, Arsenal::new())
            }
        };
        self.preset = preset.clone();
        self.state = Some(state);
        self.arsenal = arsenal;

        // ContentDb пресета: оружие/классы/перки + конфиги врагов/предметов/NPC/квестов.
        crate::content::load_preset(&preset);
        let base = crate::content::preset_base(&preset);
        self.cfg = Some(GameConfig::load_from(&base));

        // Мир: карта пресета (maps/hub.json) или legacy-мир кодом.
        let map_def = crate::map::load_map(&base, "hub");
        let spawn;
        match map_def {
            Some(def) => {
                let built = crate::map::build_map(&def, &mut self.cache);
                self.build_environment(Some(&built.env));
                spawn = built.player_spawn;
                self.gate_pos = built.gate.unwrap_or(world::GATE_POS);
                self.world_name = if lang == "en" && !built.name_en.is_empty() {
                    built.name_en.to_uppercase()
                } else {
                    built.name_ru.to_uppercase()
                };
                self.world_districts = built.districts.clone();
                self.map_ambient = built.ambient;
                self.base_mut().add_child(&built.root);
                // спавны из карты
                let map_spawns = def.spawns.clone();
                self.spawn_from_level(&map_spawns);
            }
            None => {
                self.build_environment(None);
                let plan = world::build_world(&mut self.cache);
                spawn = plan.player_spawn;
                self.gate_pos = plan.gate_portal;
                self.base_mut().add_child(&plan.root);
                self.build_world_spawns();
            }
        }
        self.build_boss_aftermath();
        self.refresh_quest_world_changes();
        self.build_npcs();
        self.build_hud(&lang);
        self.build_quest_navigation_marker();
        self.build_select_ui();

        let player_gd = self.base().get_node_as::<CharacterBody3D>("Player");
        self.player = Some(player_gd.clone());
        // FOV камеры из настроек
        if let Some(mut cam) = player_gd.try_get_node_as::<godot::classes::Camera3D>("Camera3D") {
            cam.set_fov(self.settings.fov);
        }
        if let Ok(mut p) = player_gd.try_cast::<Player>() {
            p.bind_mut().teleport(spawn);
        }
        // врагам нужна ссылка на игрока
        let pl = self.player.clone();
        if let Some(pl) = pl {
            for e in self.enemies.iter_mut() {
                e.bind_mut().set_player(pl.clone());
            }
        }

        if has_class {
            let (ci, si) = {
                let st = self.state.as_ref().unwrap();
                (st.class_idx.unwrap_or(0), st.spec_idx)
            };
            self.apply_loadout(ci, si, false);
            if let Some(ref p) = self.player {
                if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                    let max = pl.bind().max_hp;
                    // кламп снизу: сейв с hp<=0 не должен дать «живого мертвеца»
                    pl.bind_mut().hp = player_hp.min(max).max(1.0);
                }
            }
            self.set_mode_explore();
            self.refresh_weapon_sheet();
        } else {
            self.open_class_select();
        }
        self.update_loc_label();
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        // F11 — быстрый тумблер полного экрана прямо в игре
        if let Ok(k) = event.try_cast::<InputEventKey>() {
            if k.is_pressed() && !k.is_echo() && k.get_physical_keycode() == godot::global::Key::F11
            {
                self.settings.fullscreen = !self.settings.fullscreen;
                self.settings.apply_video();
                self.settings.save();
            }
        }
    }

    fn process(&mut self, delta: f64) {
        let dt = delta as f32;
        self.game_time += dt;
        self.shoot_cd = (self.shoot_cd - dt).max(0.0);
        self.tick_flash(dt);
        self.tick_damage_flash(dt);
        self.tick_npc_anim(dt);
        self.tick_map_ambient(dt);
        self.tick_fx(dt);
        self.tick_sfx();
        self.tick_post_fx(dt);
        self.tick_hud_anim(dt);
        self.tick_weapon_accuracy(dt);
        self.tick_weapon_anim(dt);
        self.tick_muzzle(dt);
        self.update_compass();

        // Бой идёт только в Explore: в меню (инвентарь/перки/диалог/пауза/смерть)
        // снаряды замирают, враги заморожены и не наносят урона.
        let in_gameplay = self.mode == Mode::Explore;
        if in_gameplay {
            self.tick_projectiles(dt);
            self.collect_enemy_requests();
            self.tick_enemy_projectiles(dt);
            self.collect_enemy_damage(dt);
            self.tick_player_statuses(dt);
            self.tick_dungeon_hazards(dt);
        }
        if self.enemies_frozen == in_gameplay {
            let frozen = !in_gameplay;
            for e in self.enemies.iter_mut() {
                let mut b = e.bind_mut();
                b.frozen = frozen;
                // Удар, успевший лечь в pending_dmg в тик перехода в меню,
                // сгорает: иначе он «прилетел бы из паузы» после закрытия.
                if frozen {
                    b.pending_dmg = 0.0;
                    // запросы способностей и статус-удар из тика перехода тоже
                    // сгорают — иначе «залп/поджог из паузы» после закрытия меню
                    let _ = b.drain_requests();
                    let _ = b.take_pending_status();
                }
            }
            self.enemies_frozen = frozen;
        }

        match self.mode {
            Mode::ClassSelect => self.process_class_select(),
            Mode::SpecSelect => self.process_spec_select(),
            Mode::Explore => self.process_explore(),
            Mode::Dialogue => self.process_dialogue(),
            Mode::Inventory => self.process_inventory(),
            Mode::Journal => self.process_journal(),
            Mode::Perks => self.process_perks(),
            Mode::Dead => self.process_dead(),
            Mode::Paused => self.process_paused(),
        }

        self.update_hp_bar();
        self.update_xp_bar();
        self.update_ammo_hud();
        self.update_targeting_hud();
        self.update_minimap();
    }
}

#[cfg(debug_assertions)]
#[godot_api]
impl Game3D {
    #[func]
    fn runtime_smoke_shutdown_audio(&mut self) {
        for mut player in self.sfx_2d.drain(..) {
            player.stop();
            player.free();
        }
        for mut player in self.sfx_3d.drain(..) {
            player.stop();
            player.free();
        }
    }

    #[func]
    fn runtime_smoke_hazard_profiles(&self) -> VarDictionary {
        let kinds = [
            dungeon::HazardKind::Blood,
            dungeon::HazardKind::Void,
            dungeon::HazardKind::Electric,
            dungeon::HazardKind::Embers,
        ];
        let mut warning_profiles = 0;
        let mut dormant_profiles = 0;
        let mut active_profiles = 0;
        let mut statuses = Vec::new();
        let mut localized = 0;
        for kind in kinds {
            let mut has_warning = false;
            let mut has_dormant = false;
            let mut has_active = false;
            for sample in 0..=120 {
                match kind.phase(sample as f32 * 0.05) {
                    dungeon::HazardPhase::Dormant => has_dormant = true,
                    dungeon::HazardPhase::Warning => has_warning = true,
                    dungeon::HazardPhase::Active => has_active = true,
                }
            }
            warning_profiles += i64::from(has_warning);
            dormant_profiles += i64::from(has_dormant);
            active_profiles += i64::from(has_active);
            if !statuses.contains(&kind.status()) {
                statuses.push(kind.status());
            }
            localized +=
                i64::from(!kind.warning("ru").is_empty() && !kind.warning("en").is_empty());
        }
        let mut snapshot = VarDictionary::new();
        snapshot.set("types", kinds.len() as i64);
        snapshot.set("warning_profiles", warning_profiles);
        snapshot.set("dormant_profiles", dormant_profiles);
        snapshot.set("active_profiles", active_profiles);
        snapshot.set("statuses", statuses.len() as i64);
        snapshot.set("localized", localized);
        snapshot
    }

    #[func]
    fn runtime_smoke_trigger_hazard(&mut self) -> VarDictionary {
        let Some(hazard) = self.dungeon_hazards.first().copied() else {
            let mut snapshot = VarDictionary::new();
            snapshot.set("found", false);
            return snapshot;
        };
        self.dungeon_hazard_time = -hazard.phase_offset;
        self.dungeon_hazard_tick = 0.0;
        if let Some(runtime_hazard) = self.dungeon_hazards.first_mut() {
            runtime_hazard.phase = dungeon::HazardPhase::Dormant;
            runtime_hazard.player_inside = false;
        }
        self.player_statuses.clear();
        let mut hp_before = 0.0;
        if let Some(player) = self.player.clone() {
            if let Ok(mut player) = player.try_cast::<Player>() {
                hp_before = player.bind().hp;
                player
                    .bind_mut()
                    .teleport(hazard.pos + Vector3::new(0.0, 1.0, 0.0));
            }
        }
        self.tick_dungeon_hazards(0.11);
        let hp_after = self
            .player
            .as_ref()
            .and_then(|player| player.clone().try_cast::<Player>().ok())
            .map(|player| player.bind().hp)
            .unwrap_or(hp_before);
        let mut snapshot = VarDictionary::new();
        snapshot.set("found", true);
        snapshot.set("kind", hazard.kind.id());
        snapshot.set("damage", hp_before - hp_after);
        snapshot.set("status", hazard.kind.status());
        snapshot.set(
            "status_applied",
            self.player_statuses.has(hazard.kind.status()),
        );
        snapshot
    }

    #[func]
    fn runtime_smoke_dungeon_events(&mut self) -> VarDictionary {
        let puzzle_group = self
            .dungeon_events
            .iter()
            .find(|event| event.kind == "puzzle_switch")
            .map(|event| event.group);
        let mut puzzle_indices = puzzle_group
            .map(|group| {
                let mut indices: Vec<_> = self
                    .dungeon_events
                    .iter()
                    .enumerate()
                    .filter_map(|(index, event)| {
                        (event.kind == "puzzle_switch" && event.group == group)
                            .then_some((event.step, index))
                    })
                    .collect();
                indices.sort_by_key(|(step, _)| *step);
                indices
            })
            .unwrap_or_default();
        let gold_before = self.state.as_ref().map(|state| state.gold).unwrap_or(0);
        let xp_before = self.state.as_ref().map(|state| state.xp).unwrap_or(0);
        let level_before = self.state.as_ref().map(|state| state.level).unwrap_or(0);
        let wrong_reset = if puzzle_indices.len() >= 2 {
            self.use_dungeon_event(puzzle_indices[1].1);
            puzzle_group.is_some_and(|group| {
                self.dungeon_events
                    .iter()
                    .filter(|event| event.group == group)
                    .all(|event| !event.used)
            })
        } else {
            false
        };
        for (_, index) in puzzle_indices.drain(..) {
            self.use_dungeon_event(index);
        }
        let puzzle_solved = puzzle_group.is_some_and(|group| {
            self.dungeon_events
                .iter()
                .filter(|event| event.group == group)
                .all(|event| event.used)
        });
        let story_index = self
            .dungeon_events
            .iter()
            .position(|event| event.kind == "story_echo" && !event.used);
        if let Some(index) = story_index {
            self.use_dungeon_event(index);
        }
        let gold_after = self.state.as_ref().map(|state| state.gold).unwrap_or(0);
        let xp_after = self.state.as_ref().map(|state| state.xp).unwrap_or(0);
        let level_after = self.state.as_ref().map(|state| state.level).unwrap_or(0);
        let mut snapshot = VarDictionary::new();
        snapshot.set("puzzle_found", puzzle_group.is_some());
        snapshot.set("wrong_reset", wrong_reset);
        snapshot.set("puzzle_solved", puzzle_solved);
        snapshot.set("story_found", story_index.is_some());
        snapshot.set(
            "story_used",
            story_index.is_some_and(|index| self.dungeon_events[index].used),
        );
        snapshot.set("gold_gained", gold_after > gold_before);
        snapshot.set(
            "xp_gained",
            xp_after != xp_before || level_after > level_before,
        );
        snapshot
    }

    #[func]
    fn runtime_smoke_enter_dungeon(&mut self, depth: i64) -> VarDictionary {
        self.enter_dungeon(depth.clamp(1, 32) as u32);

        let dungeon_enemies = self
            .enemies
            .iter()
            .filter(|enemy| enemy.get_position().x > 250.0)
            .count();
        let dungeon_items = self
            .world_items
            .iter()
            .filter(|item| item.in_dungeon)
            .count();
        let floor_cells = self.minimap_floor.iter().filter(|cell| **cell).count();
        let portal_reachable = self
            .dungeon_nav
            .as_ref()
            .and_then(|navigation| {
                let player_position = self
                    .player
                    .as_ref()
                    .map(|player| player.get_global_position() - DUNGEON_OFFSET)?;
                navigation.astar(
                    NavGrid::cell_of(player_position),
                    NavGrid::cell_of(self.next_portal - DUNGEON_OFFSET),
                )
            })
            .is_some();
        let room_focals = self
            .base()
            .get_tree()
            .get_nodes_in_group("dungeon_room_focals");
        let focal_nodes: i64 = room_focals
            .iter_shared()
            .map(|node| node.get_child_count() as i64)
            .sum();
        let mut focal_profiles = Vec::new();
        for node in room_focals.iter_shared() {
            let name = node.get_name().to_string();
            let profile = name.split('_').nth(1).unwrap_or_default().to_string();
            if !focal_profiles.contains(&profile) {
                focal_profiles.push(profile);
            }
        }

        let mut snapshot = VarDictionary::new();
        snapshot.set("depth", self.dungeon_depth as i64);
        snapshot.set("root", self.dungeon_root.is_some());
        snapshot.set("navigation", self.dungeon_nav.is_some());
        snapshot.set("floor_cells", floor_cells as i64);
        snapshot.set("enemies", dungeon_enemies as i64);
        snapshot.set("items", dungeon_items as i64);
        snapshot.set("hazards", self.dungeon_hazards.len() as i64);
        snapshot.set("room_count", self.dungeon_room_roles.len() as i64);
        snapshot.set("map_rooms", self.dungeon_room_markers.len() as i64);
        snapshot.set("map_hazards", self.dungeon_hazards.len() as i64);
        snapshot.set("dressing_nodes", self.dungeon_dressing_nodes as i64);
        snapshot.set("room_focals", room_focals.len() as i64);
        snapshot.set("focal_nodes", focal_nodes);
        snapshot.set("focal_profiles", focal_profiles.len() as i64);
        let mut dressing_variants = self.dungeon_dressing_variants.clone();
        dressing_variants.sort_unstable();
        dressing_variants.dedup();
        snapshot.set("dressing_variants", dressing_variants.len() as i64);
        snapshot.set("events", self.dungeon_events.len() as i64);
        snapshot.set(
            "puzzle_switches",
            self.dungeon_events
                .iter()
                .filter(|event| event.kind == "puzzle_switch")
                .count() as i64,
        );
        snapshot.set(
            "story_echoes",
            self.dungeon_events
                .iter()
                .filter(|event| event.kind == "story_echo")
                .count() as i64,
        );
        snapshot.set(
            "map_ready",
            self.minimap_rect
                .as_ref()
                .and_then(|rect| rect.get_texture())
                .is_some(),
        );
        snapshot.set(
            "map_legend",
            self.minimap_legend
                .as_ref()
                .is_some_and(|legend| legend.is_visible()),
        );
        for role in [
            "safe",
            "arena",
            "gallery",
            "ritual",
            "treasure",
            "ambush",
            "traversal",
            "puzzle",
            "story",
            "antechamber",
        ] {
            snapshot.set(
                format!("has_{role}"),
                self.dungeon_room_roles
                    .iter()
                    .any(|current| current == role),
            );
        }
        snapshot.set("exit_portal", self.exit_portal != Vector3::ZERO);
        snapshot.set("next_portal", self.next_portal != Vector3::ZERO);
        snapshot.set("portal_reachable", portal_reachable);
        snapshot
    }

    #[func]
    fn runtime_smoke_kill_enemy(&mut self, boss_only: bool) -> VarDictionary {
        let before = self.enemies.len();
        let cores_before = self
            .state
            .as_ref()
            .map(|state| state.weapon_mod_cores)
            .unwrap_or(0);
        let xp_before = self.state.as_ref().map(|state| state.xp).unwrap_or(0);
        let level_before = self.state.as_ref().map(|state| state.level).unwrap_or(0);
        let target = self.enemies.iter().position(|enemy| {
            enemy.get_position().x > 250.0
                && enemy.bind().alive
                && (!boss_only || enemy.bind().is_boss)
        });
        let target_kind = target.map(|index| self.enemies[index].bind().cfg_id.to_string());
        let target_trophy = target_kind.as_deref().map(|kind| match kind {
            "crypt_warden" => "trophy_warden_seal",
            "blood_oracle" => "trophy_oracle_chalice",
            "archive_sentinel" => "trophy_archive_lens",
            _ => "trophy_tyrant_heart",
        });
        if boss_only {
            if let (Some(state), Some(kind), Some(trophy)) =
                (self.state.as_mut(), target_kind.as_ref(), target_trophy)
            {
                state.flags.remove(&format!("boss_defeated_{kind}"));
                state.inventory.items.retain(|item| item.id != trophy);
            }
        }

        let mut dealt = 0.0;
        if let Some(index) = target {
            let lethal_damage = self.enemies[index].bind().max_hp * 100.0 + 1000.0;
            let mut enemy = self.enemies[index].bind_mut();
            dealt = enemy.take_damage(lethal_damage, DmgType::Physical);
            enemy.death_timer = 0.0;
        }
        self.process_kills();

        let xp_after = self.state.as_ref().map(|state| state.xp).unwrap_or(0);
        let level_after = self.state.as_ref().map(|state| state.level).unwrap_or(0);
        let mut snapshot = VarDictionary::new();
        snapshot.set("target_found", target.is_some());
        snapshot.set("damage", dealt as f64);
        snapshot.set("enemy_removed", self.enemies.len() < before);
        snapshot.set(
            "xp_gained",
            level_after > level_before || (level_after == level_before && xp_after > xp_before),
        );
        snapshot.set(
            "boss_quest_progress",
            self.state
                .as_ref()
                .and_then(|state| state.quest_kills.get("heart_execution"))
                .copied()
                .unwrap_or(0) as i64,
        );
        snapshot.set(
            "boss_core_gained",
            self.state
                .as_ref()
                .map(|state| state.weapon_mod_cores > cores_before)
                .unwrap_or(false),
        );
        snapshot.set(
            "boss_flag_set",
            target_kind
                .as_ref()
                .and_then(|kind| {
                    self.state
                        .as_ref()
                        .map(|state| state.flags.contains(&format!("boss_defeated_{kind}")))
                })
                .unwrap_or(false),
        );
        snapshot.set(
            "boss_trophy_gained",
            self.state
                .as_ref()
                .map(|state| {
                    target_trophy
                        .map(|trophy| state.inventory.items.iter().any(|item| item.id == trophy))
                        .unwrap_or(false)
                })
                .unwrap_or(false),
        );
        snapshot.set(
            "boss_memorial_spawned",
            target_kind
                .as_ref()
                .map(|kind| {
                    self.base()
                        .get_node_or_null(&NodePath::from(format!("BossMemorial_{kind}").as_str()))
                        .is_some()
                })
                .unwrap_or(false),
        );
        snapshot
    }

    #[func]
    fn runtime_smoke_world_districts(&mut self) -> VarDictionary {
        let districts = self.world_districts.clone();
        let original_position = self
            .player
            .as_ref()
            .map(|player| player.get_global_position());
        let mut recognized = 0usize;
        for (index, district) in districts.iter().enumerate() {
            if let Some(player) = self.player.as_ref() {
                if let Ok(mut player) = player.clone().try_cast::<Player>() {
                    player
                        .bind_mut()
                        .teleport(district.center + Vector3::new(0.0, 1.1, 0.0));
                }
            }
            self.active_district = None;
            self.update_world_district();
            if self.active_district == Some(index) {
                recognized += 1;
            }
        }
        if let (Some(position), Some(player)) = (original_position, self.player.as_ref()) {
            if let Ok(mut player) = player.clone().try_cast::<Player>() {
                player.bind_mut().teleport(position);
            }
        }
        self.active_district = None;
        self.update_world_district();

        let decor_clusters = self
            .base()
            .get_tree()
            .get_nodes_in_group("map_decor_clusters");
        let decor_nodes: i64 = decor_clusters
            .iter_shared()
            .map(|node| node.get_child_count() as i64)
            .sum();
        let route_layers = self
            .base()
            .get_tree()
            .get_nodes_in_group("map_route_layers");
        let route_nodes: i64 = route_layers
            .iter_shared()
            .map(|node| node.get_child_count() as i64)
            .sum();
        let skyline_beacons = self
            .base()
            .get_tree()
            .get_nodes_in_group("map_skyline_beacons");
        let beacon_nodes: i64 = skyline_beacons
            .iter_shared()
            .map(|node| node.get_child_count() as i64)
            .sum();
        let ambient_before: Vec<Vector3> = self
            .map_ambient
            .iter()
            .map(|ambient| ambient.node.get_position())
            .collect();
        self.tick_map_ambient(0.37);
        let ambient_moved = self
            .map_ambient
            .iter()
            .zip(&ambient_before)
            .filter(|(ambient, before)| ambient.node.get_position().distance_to(**before) > 0.0001)
            .count();
        let mode_before = self.mode;
        self.mode = Mode::Paused;
        let paused_before: Vec<Vector3> = self
            .map_ambient
            .iter()
            .map(|ambient| ambient.node.get_position())
            .collect();
        self.tick_map_ambient(0.5);
        let paused_static = self
            .map_ambient
            .iter()
            .zip(&paused_before)
            .all(|(ambient, before)| ambient.node.get_position().distance_to(*before) <= 0.0001);
        self.mode = mode_before;
        let mut ambient_profiles = Vec::new();
        for ambient in &self.map_ambient {
            let profile = (
                (ambient.speed * 100.0).round() as i32,
                (ambient.bob * 100.0).round() as i32,
                (ambient.spin * 100.0).round() as i32,
            );
            if !ambient_profiles.contains(&profile) {
                ambient_profiles.push(profile);
            }
        }

        let mut snapshot = VarDictionary::new();
        snapshot.set("total", districts.len() as i64);
        snapshot.set("recognized", recognized as i64);
        snapshot.set("decor_clusters", decor_clusters.len() as i64);
        snapshot.set("decor_nodes", decor_nodes);
        snapshot.set("route_layers", route_layers.len() as i64);
        snapshot.set("route_nodes", route_nodes);
        snapshot.set("skyline_beacons", skyline_beacons.len() as i64);
        snapshot.set("beacon_nodes", beacon_nodes);
        snapshot.set("ambient", self.map_ambient.len() as i64);
        snapshot.set("ambient_moved", ambient_moved as i64);
        snapshot.set("ambient_profiles", ambient_profiles.len() as i64);
        snapshot.set("ambient_paused", paused_static);
        snapshot
    }

    #[func]
    fn runtime_smoke_enemy_roster(&self) -> VarDictionary {
        let mut roles = Vec::<String>::new();
        let total = self.cfg.as_ref().map_or(0, |config| {
            for enemy in &config.enemies {
                if !roles.contains(&enemy.role) {
                    roles.push(enemy.role.clone());
                }
            }
            config.enemies.len()
        });
        let mut snapshot = VarDictionary::new();
        snapshot.set("total", total as i64);
        snapshot.set("roles", roles.len() as i64);
        snapshot
    }

    #[func]
    fn runtime_smoke_enemy_animations(&mut self) -> VarDictionary {
        let mut total = 0usize;
        let mut full_state_sets = 0usize;
        let mut cast_animated = 0usize;
        let mut pain_animated = 0usize;
        let mut one_shots = 0usize;
        let mut signatures = Vec::new();
        for enemy in self.enemies.iter_mut().filter(|enemy| enemy.bind().alive) {
            let (states, cast, pain, one_shot, signature) =
                enemy.bind_mut().runtime_smoke_animation_profile();
            total += 1;
            full_state_sets += usize::from(states == 9);
            cast_animated += usize::from(cast);
            pain_animated += usize::from(pain);
            one_shots += usize::from(one_shot);
            if !signatures.contains(&signature) {
                signatures.push(signature);
            }
        }
        let mut snapshot = VarDictionary::new();
        snapshot.set("total", total as i64);
        snapshot.set("full_state_sets", full_state_sets as i64);
        snapshot.set("cast_animated", cast_animated as i64);
        snapshot.set("pain_animated", pain_animated as i64);
        snapshot.set("one_shots", one_shots as i64);
        snapshot.set("timing_profiles", signatures.len() as i64);
        snapshot
    }

    #[func]
    fn runtime_smoke_weapon_profiles(&mut self) -> VarDictionary {
        self.autosave_enabled = false;
        let mut animated = 0usize;
        let mut feedback_complete = 0usize;
        let mut feedback_signatures = Vec::new();
        let mut feedback_profiles = Vec::new();
        let mut audio_complete = 0usize;
        let mut audio_signatures = Vec::new();
        let mut audio_assets = 0usize;
        let mut audio_assets_loaded = 0usize;
        let mut accuracy_complete = 0usize;
        let mut accuracy_signatures = Vec::new();
        let mut alt_names = Vec::new();
        let mut alt_melee = 0usize;
        let mut alt_hitscan = 0usize;
        let mut alt_projectile = 0usize;
        let mut alt_animation_complete = 0usize;
        let mut alt_animation_distinct = 0usize;
        let mut alt_animation_signatures = Vec::new();
        for slot in 0..8 {
            let weapon = WeaponId::from_slot(slot);
            let def = weapon_def(weapon);
            self.arsenal.owned[slot] = true;
            self.arsenal.current = weapon;
            self.refresh_weapon_sheet();
            self.tick_weapon_anim(def.switch_time + 0.01);
            if matches!(self.weapon_anim, WeaponAnim::Idle) && def.switch_frames.len() >= 2 {
                animated += 1;
            }
            let feedback = def.feedback;
            feedback_complete += usize::from(
                feedback.muzzle_energy > 0.0
                    && feedback.muzzle_duration > 0.0
                    && feedback.impact_scale > 0.0
                    && feedback.impact_duration > 0.0
                    && feedback.tracer_scale > 0.0
                    && feedback.tracer_duration > 0.0,
            );
            let signature = format!(
                "{:?}:{:.2}:{:?}:{:.2}:{:?}:{:.2}",
                feedback.muzzle_color,
                feedback.muzzle_energy,
                feedback.impact_color,
                feedback.impact_scale,
                feedback.tracer_color,
                feedback.tracer_scale,
            );
            if !feedback_signatures.contains(&signature) {
                feedback_signatures.push(signature);
            }
            feedback_profiles.push(feedback);
            let audio = &def.audio;
            audio_complete += usize::from(
                !audio.fire_sfx.is_empty()
                    && !audio.impact_sfx.is_empty()
                    && (def.magazine == 0 || !audio.reload_sfx.is_empty()),
            );
            let audio_signature = format!(
                "{:?}:{:?}:{:?}:{:?}:{:?}:{:.1}:{:.1}:{:.1}",
                audio.fire_sfx,
                audio.impact_sfx,
                audio.fire_pitch,
                audio.impact_pitch,
                audio.reload_pitch,
                audio.fire_volume_db,
                audio.impact_volume_db,
                audio.reload_volume_db,
            );
            if !audio_signatures.contains(&audio_signature) {
                audio_signatures.push(audio_signature);
            }
            for path in audio
                .fire_sfx
                .iter()
                .chain(audio.impact_sfx.iter())
                .chain(audio.reload_sfx.iter())
            {
                audio_assets += 1;
                audio_assets_loaded += usize::from(combat::weapon_sfx_loadable(path));
            }
            let accuracy = def.accuracy;
            accuracy_complete += usize::from(
                accuracy.bloom_per_shot >= 0.0
                    && accuracy.max_bloom > 0.0
                    && accuracy.recovery > 0.0
                    && accuracy.crosshair_scale > 0.0,
            );
            let accuracy_signature = format!(
                "{:.2}:{:.2}:{:.2}:{:.2}:{:.2}",
                accuracy.bloom_per_shot,
                accuracy.max_bloom,
                accuracy.recovery,
                accuracy.move_penalty,
                accuracy.crosshair_scale,
            );
            if !accuracy_signatures.contains(&accuracy_signature) {
                accuracy_signatures.push(accuracy_signature);
            }
            if let Some(alt) = def.alt_fire.as_ref() {
                if !alt_names.contains(&alt.name_en) {
                    alt_names.push(alt.name_en.clone());
                }
                match alt.kind {
                    FireKind::Melee => alt_melee += 1,
                    FireKind::Hitscan { .. } => alt_hitscan += 1,
                    FireKind::Projectile { .. } => alt_projectile += 1,
                }
                alt_animation_complete += usize::from(
                    !alt.animation_frames.is_empty()
                        && alt.animation_fps > 0.0
                        && (0.05..=0.95).contains(&alt.hit_ratio),
                );
                alt_animation_distinct +=
                    usize::from(def.attack_frames(true) != def.attack_frames(false));
                let signature = format!(
                    "{:?}:{:.2}:{:.2}",
                    def.attack_frames(true),
                    def.attack_fps(true),
                    def.attack_hit_ratio(true),
                );
                if !alt_animation_signatures.contains(&signature) {
                    alt_animation_signatures.push(signature);
                }
            }
        }

        let impact_before = self.sprite_fx.len();
        let impact_lights_before = self.light_fx.len();
        for (slot, feedback) in feedback_profiles.iter().copied().enumerate() {
            self.spawn_weapon_impact(
                feedback,
                Vector3::new(slot as f32 * 0.35, 1.0, -3.0),
                slot % 2 == 0,
            );
        }
        let impact_fx = self.sprite_fx.len().saturating_sub(impact_before);
        let impact_lights = self.light_fx.len().saturating_sub(impact_lights_before);

        let tracer_before = self.sprite_fx.len();
        for (slot, feedback) in feedback_profiles.iter().copied().enumerate() {
            let x = slot as f32 * 0.25;
            self.spawn_weapon_tracer(
                feedback,
                Vector3::new(x, 1.1, -1.0),
                Vector3::new(x, 1.1, -12.0),
            );
            self.flash_muzzle(feedback);
        }
        let tracer_fx = self.sprite_fx.len().saturating_sub(tracer_before);

        let impact_audio_before = self.sfx_3d.len();
        for slot in 0..8 {
            self.play_weapon_impact_sfx(
                WeaponId::from_slot(slot),
                Vector3::new(slot as f32 * 0.3, 1.0, -4.0),
            );
        }
        let impact_audio_players = self.sfx_3d.len().saturating_sub(impact_audio_before);

        self.weapon_bloom = 0.0;
        self.hit_confirm_timer = 0.0;
        self.critical_confirm_timer = 0.0;
        self.kill_confirm_timer = 0.0;
        self.update_crosshair();
        let precise_crosshair = self
            .crosshair
            .as_ref()
            .map(|crosshair| crosshair.get_text().to_string())
            .unwrap_or_default();
        let accuracy = weapon_def(self.arsenal.current).accuracy;
        self.add_weapon_bloom(accuracy.max_bloom);
        let bloom_peak = self.weapon_bloom;
        self.update_crosshair();
        let bloom_crosshair = self
            .crosshair
            .as_ref()
            .map(|crosshair| crosshair.get_text().to_string())
            .unwrap_or_default();
        self.tick_weapon_accuracy(accuracy.max_bloom / accuracy.recovery + 0.05);
        let bloom_recovered = self.weapon_bloom < 0.001;
        self.register_weapon_hit(false, false);
        self.update_crosshair();
        let hit_crosshair = self
            .crosshair
            .as_ref()
            .map(|crosshair| crosshair.get_text().to_string())
            .unwrap_or_default();
        self.register_weapon_hit(true, false);
        self.update_crosshair();
        let kill_crosshair = self
            .crosshair
            .as_ref()
            .map(|crosshair| crosshair.get_text().to_string())
            .unwrap_or_default();

        self.weapon_anim = WeaponAnim::Idle;
        self.shoot_cd = 0.0;
        self.weapon_bloom = 0.0;
        let alt_weapon = self.arsenal.current;
        let alt_def = weapon_def(alt_weapon);
        self.arsenal.clips[alt_weapon.slot()] = alt_def.magazine.max(1);
        let alt_projectiles_before = self.projectiles.len();
        self.try_alt_fire();
        let alt_animation_started = matches!(
            self.weapon_anim,
            WeaponAnim::Fire {
                frame: 0,
                secondary: true
            }
        );
        let alt_spawned_projectile = self.projectiles.len() > alt_projectiles_before;
        let alt_extended_cooldown = self.shoot_cd > alt_def.cooldown * self.loadout.cd_mult;
        let alt_added_bloom = self.weapon_bloom > alt_def.accuracy.bloom_per_shot;

        let weapon = self.arsenal.current;
        let def = weapon_def(weapon);
        self.weapon_anim = WeaponAnim::Idle;
        self.shoot_cd = 0.0;
        self.arsenal.clips[weapon.slot()] = def.magazine.max(1);
        let clip_before = self.arsenal.clips[weapon.slot()];
        let aim_before = self
            .player
            .as_ref()
            .and_then(|player| player.clone().try_cast::<Player>().ok())
            .map(|player| player.bind().aim_dir().y)
            .unwrap_or(0.0);
        let fire_audio_before = self.sfx_2d.len();
        self.try_fire();
        let aim_after = self
            .player
            .as_ref()
            .and_then(|player| player.clone().try_cast::<Player>().ok())
            .map(|player| player.bind().aim_dir().y)
            .unwrap_or(0.0);

        let mut snapshot = VarDictionary::new();
        snapshot.set("total", 8i64);
        snapshot.set("animated", animated as i64);
        snapshot.set("feedback_complete", feedback_complete as i64);
        snapshot.set("feedback_profiles", feedback_signatures.len() as i64);
        snapshot.set("audio_complete", audio_complete as i64);
        snapshot.set("audio_profiles", audio_signatures.len() as i64);
        snapshot.set("audio_assets", audio_assets as i64);
        snapshot.set("audio_assets_loaded", audio_assets_loaded as i64);
        snapshot.set("accuracy_complete", accuracy_complete as i64);
        snapshot.set("accuracy_profiles", accuracy_signatures.len() as i64);
        snapshot.set("alt_profiles", alt_names.len() as i64);
        snapshot.set("alt_melee", alt_melee as i64);
        snapshot.set("alt_hitscan", alt_hitscan as i64);
        snapshot.set("alt_projectile", alt_projectile as i64);
        snapshot.set("alt_animation_complete", alt_animation_complete as i64);
        snapshot.set("alt_animation_distinct", alt_animation_distinct as i64);
        snapshot.set(
            "alt_animation_profiles",
            alt_animation_signatures.len() as i64,
        );
        snapshot.set("alt_animation_started", alt_animation_started);
        snapshot.set("alt_spawned_projectile", alt_spawned_projectile);
        snapshot.set("alt_extended_cooldown", alt_extended_cooldown);
        snapshot.set("alt_added_bloom", alt_added_bloom);
        snapshot.set("bloom_peak", bloom_peak as f64);
        snapshot.set("bloom_recovered", bloom_recovered);
        snapshot.set("crosshair_expanded", precise_crosshair != bloom_crosshair);
        snapshot.set("hit_confirmed", hit_crosshair.contains('×'));
        snapshot.set("kill_confirmed", kill_crosshair.contains('X'));
        snapshot.set("impact_audio_players", impact_audio_players as i64);
        snapshot.set("fire_audio_played", self.sfx_2d.len() > fire_audio_before);
        snapshot.set("impact_fx", impact_fx as i64);
        snapshot.set("impact_lights", impact_lights as i64);
        snapshot.set("tracer_fx", tracer_fx as i64);
        snapshot.set(
            "muzzle_active",
            self.muzzle_timer > 0.0 && self.muzzle_light.is_some(),
        );
        snapshot.set(
            "ammo_consumed",
            self.arsenal.clips[weapon.slot()] < clip_before,
        );
        snapshot.set("recoil_applied", aim_after > aim_before);
        snapshot
    }

    #[func]
    fn runtime_smoke_weapon_mods(&mut self) -> VarDictionary {
        self.autosave_enabled = false;
        let mods = weapon_mods();
        let localized = mods
            .iter()
            .filter(|entry| {
                !entry.name_ru.is_empty()
                    && !entry.name_en.is_empty()
                    && !entry.desc_ru.is_empty()
                    && !entry.desc_en.is_empty()
            })
            .count();
        let visual_complete = mods
            .iter()
            .filter(|entry| {
                entry.feedback.color_mix > 0.0
                    && entry.feedback.muzzle_mult > 0.0
                    && entry.feedback.tracer_mult > 0.0
                    && entry.feedback.impact_mult > 0.0
                    && entry.feedback.pitch_mult > 0.0
            })
            .count();
        let mut visual_signatures = Vec::new();
        for entry in mods {
            let signature = format!(
                "{:?}:{:.2}:{:.2}:{:.2}:{:.2}:{:.2}",
                entry.feedback.tint,
                entry.feedback.color_mix,
                entry.feedback.muzzle_mult,
                entry.feedback.tracer_mult,
                entry.feedback.impact_mult,
                entry.feedback.pitch_mult,
            );
            if !visual_signatures.contains(&signature) {
                visual_signatures.push(signature);
            }
        }
        let mut branch_pairs_distinct = 0usize;
        let mut effective_signatures = Vec::new();
        let impact_before = self.sprite_fx.len();
        let light_before = self.light_fx.len();
        for weapon in WeaponId::ALL {
            let mut pair = Vec::new();
            for branch in 1..=2 {
                if let Some(state) = self.state.as_mut() {
                    state.weapon_mods[weapon.slot()] = branch;
                }
                let feedback = self.effective_weapon_feedback(weapon);
                let signature = format!(
                    "{:?}:{:.2}:{:?}:{:.2}:{:?}:{:.2}:{:.2}",
                    feedback.muzzle_color,
                    feedback.muzzle_energy,
                    feedback.impact_color,
                    feedback.impact_scale,
                    feedback.tracer_color,
                    feedback.tracer_scale,
                    self.effective_weapon_pitch(weapon),
                );
                pair.push(signature.clone());
                if !effective_signatures.contains(&signature) {
                    effective_signatures.push(signature);
                }
                self.spawn_weapon_impact(
                    feedback,
                    Vector3::new(weapon.slot() as f32, 1.0, -(branch as f32)),
                    false,
                );
            }
            branch_pairs_distinct += usize::from(pair[0] != pair[1]);
        }
        let branch_impact_fx = self.sprite_fx.len().saturating_sub(impact_before);
        let branch_impact_lights = self.light_fx.len().saturating_sub(light_before);
        let weapon = WeaponId::Pistol;
        self.arsenal.current = weapon;
        if let Some(state) = self.state.as_mut() {
            state.weapon_mods = [0; 8];
            state.weapon_mod_cores = 2;
        }
        self.select_weapon_mod(1);
        let first_view_color = self.weapon_mod_view_color(weapon);
        let first = self
            .state
            .as_ref()
            .map(|state| (state.weapon_mods[weapon.slot()], state.weapon_mod_cores))
            .unwrap_or_default();
        let first_changes_stats = weapon_mod_for(weapon, 1)
            .map(|entry| {
                entry.damage_mult != 1.0
                    || entry.cooldown_mult != 1.0
                    || entry.range_mult != 1.0
                    || entry.recoil_mult != 1.0
                    || entry.bloom_mult != 1.0
            })
            .unwrap_or(false);
        self.select_weapon_mod(2);
        let second_view_color = self.weapon_mod_view_color(weapon);
        let second = self
            .state
            .as_ref()
            .map(|state| (state.weapon_mods[weapon.slot()], state.weapon_mod_cores))
            .unwrap_or_default();
        let mut snapshot = VarDictionary::new();
        snapshot.set("total", mods.len() as i64);
        snapshot.set("localized", localized as i64);
        snapshot.set("visual_complete", visual_complete as i64);
        snapshot.set("visual_profiles", visual_signatures.len() as i64);
        snapshot.set("effective_profiles", effective_signatures.len() as i64);
        snapshot.set("branch_pairs_distinct", branch_pairs_distinct as i64);
        snapshot.set("branch_impact_fx", branch_impact_fx as i64);
        snapshot.set("branch_impact_lights", branch_impact_lights as i64);
        snapshot.set("first_selected", first.0 as i64);
        snapshot.set("first_cores", first.1 as i64);
        snapshot.set("second_selected", second.0 as i64);
        snapshot.set("second_cores", second.1 as i64);
        snapshot.set("first_changes_stats", first_changes_stats);
        snapshot.set(
            "view_identity_distinct",
            first_view_color != second_view_color,
        );
        snapshot.set(
            "view_visual_applied",
            self.weapon_rect
                .as_ref()
                .map(|weapon_rect| weapon_rect.get_modulate() == second_view_color)
                .unwrap_or(false),
        );
        snapshot
    }

    #[func]
    fn runtime_smoke_weak_points(&mut self) -> VarDictionary {
        let configured = self
            .cfg
            .as_ref()
            .map(|cfg| {
                cfg.enemies
                    .iter()
                    .filter(|enemy| {
                        (0.5..=0.95).contains(&enemy.weak_point.height)
                            && (1.0..=3.0).contains(&enemy.weak_point.multiplier)
                    })
                    .count()
            })
            .unwrap_or(0);
        let boss_ids = [
            "crypt_warden",
            "blood_oracle",
            "archive_sentinel",
            "heart_tyrant",
        ];
        let boss_profiles = self
            .cfg
            .as_ref()
            .map(|cfg| {
                boss_ids
                    .iter()
                    .filter_map(|id| cfg.enemy(id))
                    .filter(|enemy| {
                        enemy.weak_point.height != 0.7 || enemy.weak_point.multiplier != 1.6
                    })
                    .count()
            })
            .unwrap_or(0);

        let mut normal_damage = 0.0;
        let mut critical_damage = 0.0;
        let mut critical = false;
        if let Some(mut enemy) = self
            .enemies
            .iter()
            .find(|enemy| enemy.bind().alive)
            .cloned()
        {
            let origin = enemy.get_global_position();
            let max_hp = enemy.bind().max_hp;
            enemy.bind_mut().hp = max_hp;
            (normal_damage, _) = Self::apply_precise_damage(
                &mut enemy,
                origin + Vector3::UP * 0.1,
                8.0,
                DmgType::Physical,
            );
            enemy.bind_mut().hp = max_hp;
            (critical_damage, critical) = Self::apply_precise_damage(
                &mut enemy,
                origin + Vector3::UP * 100.0,
                8.0,
                DmgType::Physical,
            );
            enemy.bind_mut().hp = max_hp;
        }

        self.hit_confirm_timer = 0.0;
        self.critical_confirm_timer = 0.0;
        self.kill_confirm_timer = 0.0;
        self.register_weapon_hit(false, true);
        self.update_crosshair();
        let critical_crosshair = self
            .crosshair
            .as_ref()
            .map(|crosshair| crosshair.get_text().to_string().contains("CRIT"))
            .unwrap_or(false);

        let mut snapshot = VarDictionary::new();
        snapshot.set("configured", configured as i64);
        snapshot.set("boss_profiles", boss_profiles as i64);
        snapshot.set("normal_damage", normal_damage as f64);
        snapshot.set("critical_damage", critical_damage as f64);
        snapshot.set("critical", critical);
        snapshot.set("critical_crosshair", critical_crosshair);
        snapshot.set("localized", true);
        snapshot
    }

    #[func]
    fn runtime_smoke_dialogue_conditions(&mut self) -> VarDictionary {
        let (original_flags, original_stats) = self
            .state
            .as_ref()
            .map(|state| (state.flags.clone(), state.stats.clone()))
            .unwrap_or_else(|| {
                (
                    std::collections::HashSet::new(),
                    crate::character::Stats::new("Smoke"),
                )
            });
        if let Some(state) = self.state.as_mut() {
            for boss in [
                "crypt_warden",
                "blood_oracle",
                "archive_sentinel",
                "heart_tyrant",
            ] {
                state.flags.insert(format!("boss_defeated_{boss}"));
                state.flags.remove(&format!("discussed_{boss}"));
            }
            state.stats.intelligence = 5;
            state.stats.willpower = 5;
            state.flags.remove("heart_answer_self");
        }

        let expected = [
            ("victor", "hub_victor_warden_aftermath"),
            ("elena", "hub_elena_oracle_aftermath"),
            ("vale", "hub_vale_archive_aftermath"),
            ("stranger", "hub_stranger_tyrant_aftermath"),
        ];
        let reactive = self.state.as_ref().map_or(0, |state| {
            expected
                .iter()
                .filter(|(npc, scene)| npc_scene_id(npc, state) == *scene)
                .count()
        });
        let localized = expected
            .iter()
            .filter_map(|(_, scene)| self.resolve_scene(scene))
            .filter(|scene| {
                scene.lines.iter().all(|line| {
                    !crate::dialogue::localized(&line.text, "ru").is_empty()
                        && !crate::dialogue::localized(&line.text, "en").is_empty()
                }) && scene.choices.iter().all(|choice| {
                    !crate::dialogue::localized(&choice.text, "ru").is_empty()
                        && !crate::dialogue::localized(&choice.text, "en").is_empty()
                })
            })
            .count();

        let vale = self.resolve_scene("hub_vale_archive_aftermath");
        let low_stat_choices = vale
            .as_ref()
            .zip(self.state.as_ref())
            .map(|(scene, state)| {
                scene
                    .choices
                    .iter()
                    .filter(|choice| {
                        choice
                            .requires
                            .as_ref()
                            .is_none_or(|requirement| requirement.is_met(state))
                    })
                    .count()
            })
            .unwrap_or(0);
        if let Some(state) = self.state.as_mut() {
            state.stats.intelligence = 10;
        }
        let high_stat_choices = vale
            .as_ref()
            .zip(self.state.as_ref())
            .map(|(scene, state)| {
                scene
                    .choices
                    .iter()
                    .filter(|choice| {
                        choice
                            .requires
                            .as_ref()
                            .is_none_or(|requirement| requirement.is_met(state))
                    })
                    .count()
            })
            .unwrap_or(0);

        let stranger = self.resolve_scene("hub_stranger_tyrant_aftermath");
        let not_flag_before = stranger
            .as_ref()
            .and_then(|scene| scene.choices.get(1))
            .and_then(|choice| choice.requires.as_ref())
            .zip(self.state.as_ref())
            .is_some_and(|(requirement, state)| requirement.is_met(state));
        if let Some(state) = self.state.as_mut() {
            state.flags.insert("heart_answer_self".into());
        }
        let not_flag_after = stranger
            .as_ref()
            .and_then(|scene| scene.choices.get(1))
            .and_then(|choice| choice.requires.as_ref())
            .zip(self.state.as_ref())
            .is_some_and(|(requirement, state)| requirement.is_met(state));
        let quest_conditions = self
            .resolve_scene("hub_elena_oracle_aftermath")
            .map(|scene| {
                scene
                    .choices
                    .iter()
                    .filter(|choice| {
                        choice
                            .requires
                            .as_ref()
                            .is_some_and(|requirement| requirement.quest_done.is_some())
                    })
                    .count()
            })
            .unwrap_or(0);

        if let Some(state) = self.state.as_mut() {
            state.flags = original_flags;
            state.stats = original_stats;
        }
        let mut snapshot = VarDictionary::new();
        snapshot.set("reactive", reactive as i64);
        snapshot.set("localized", localized as i64);
        snapshot.set("low_stat_choices", low_stat_choices as i64);
        snapshot.set("high_stat_choices", high_stat_choices as i64);
        snapshot.set("not_flag_before", not_flag_before);
        snapshot.set("not_flag_after", not_flag_after);
        snapshot.set("quest_conditions", quest_conditions as i64);
        snapshot
    }

    #[func]
    fn runtime_smoke_complete_quest_chains(&mut self) -> VarDictionary {
        self.autosave_enabled = false;
        if let Some(state) = self.state.as_mut() {
            state.quests = crate::quest::QuestLog::default();
            state.quest_kills.clear();
            state.dungeons_cleared = 0;
            state.inventory = crate::item::Inventory::default();
        }

        let quests = self
            .cfg
            .as_ref()
            .map(|config| config.quests.clone())
            .unwrap_or_default();
        let mut offered = 0usize;
        let mut completed = 0usize;
        let mut stalled = false;

        while completed < quests.len() {
            let next = quests
                .iter()
                .find(|quest| {
                    self.state.as_ref().is_some_and(|state| {
                        !state.quests.has(&quest.id)
                            && quest
                                .requires
                                .iter()
                                .all(|required| state.quests.is_completed(required))
                    })
                })
                .cloned();
            let Some(quest) = next else {
                stalled = true;
                break;
            };

            let offer_effects =
                self.make_quest_scene("Quest Smoke", &quest.id)
                    .and_then(|scene| {
                        scene
                            .choices
                            .into_iter()
                            .find(|choice| {
                            choice.effects.iter().any(
                                |effect| matches!(effect, Effect::Quest { id, .. } if id == &quest.id),
                            )
                            })
                            .map(|choice| choice.effects)
                    });
            let Some(offer_effects) = offer_effects else {
                stalled = true;
                break;
            };
            if let Some(state) = self.state.as_mut() {
                state.apply(&offer_effects, "en");
            }
            if !self
                .state
                .as_ref()
                .is_some_and(|state| state.quests.is_active(&quest.id))
            {
                stalled = true;
                break;
            }
            offered += 1;

            if let Some(state) = self.state.as_mut() {
                if quest.kind == "clear_dungeon" {
                    state.dungeons_cleared = state.dungeons_cleared.max(quest.count);
                } else {
                    state.quest_kills.insert(quest.id.clone(), quest.count);
                }
            }

            let completion_effects =
                self.make_quest_scene("Quest Smoke", &quest.id)
                    .and_then(|scene| {
                        scene
                            .choices
                            .into_iter()
                            .find(|choice| {
                                choice.effects.iter().any(
                                |effect| matches!(effect, Effect::QuestDone(id) if id == &quest.id),
                            )
                            })
                            .map(|choice| choice.effects)
                    });
            let Some(completion_effects) = completion_effects else {
                stalled = true;
                break;
            };
            if let Some(state) = self.state.as_mut() {
                state.apply(&completion_effects, "en");
            }
            if !self
                .state
                .as_ref()
                .is_some_and(|state| state.quests.is_completed(&quest.id))
            {
                stalled = true;
                break;
            }
            completed += 1;
        }

        let mut snapshot = VarDictionary::new();
        let chain_names: std::collections::HashSet<_> =
            quests.iter().map(|quest| quest.chain_en.as_str()).collect();
        let reward_quests = quests
            .iter()
            .filter(|quest| !quest.reward_items.is_empty())
            .count();
        let expected_reward_qty: u32 = quests
            .iter()
            .flat_map(|quest| quest.reward_items.iter())
            .map(|reward| reward.qty)
            .sum();
        let awarded_reward_qty: u32 = self
            .state
            .as_ref()
            .map(|state| state.inventory.items.iter().map(|item| item.qty).sum())
            .unwrap_or(0);
        if let Some(state) = self.state.as_mut() {
            if let Some(first) = state.quests.quests.first_mut() {
                first.state = crate::quest::QuestState::Active;
            }
        }
        self.update_quest_label("ru");
        let journal_ru = self
            .quest_label
            .as_ref()
            .map(|label| label.get_text().to_string())
            .unwrap_or_default();
        self.update_quest_label("en");
        let journal_en = self
            .quest_label
            .as_ref()
            .map(|label| label.get_text().to_string())
            .unwrap_or_default();
        snapshot.set("total", quests.len() as i64);
        snapshot.set("offered", offered as i64);
        snapshot.set("completed", completed as i64);
        snapshot.set("stalled", stalled);
        snapshot.set("chains", chain_names.len() as i64);
        snapshot.set("reward_quests", reward_quests as i64);
        snapshot.set("expected_reward_qty", expected_reward_qty as i64);
        snapshot.set("awarded_reward_qty", awarded_reward_qty as i64);
        snapshot.set(
            "journal_localized",
            journal_ru.contains("Этап") && journal_en.contains("Stage"),
        );
        snapshot.set(
            "exploration_completed",
            self.state.as_ref().map_or(0, |state| {
                ["relay_resonance", "archive_whispers"]
                    .iter()
                    .filter(|quest_id| state.quests.is_completed(quest_id))
                    .count() as i64
            }),
        );
        snapshot
    }

    #[func]
    fn runtime_smoke_quest_world_changes(&mut self) -> VarDictionary {
        self.refresh_quest_world_changes();
        let changes: Vec<_> = self
            .cfg
            .as_ref()
            .map(|config| {
                config
                    .quests
                    .iter()
                    .filter_map(|quest| {
                        quest
                            .world_change
                            .as_ref()
                            .map(|change| (quest.id.clone(), change.clone()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut built = 0usize;
        let mut nodes = 0i64;
        let mut patterns = std::collections::HashSet::new();
        let mut activity_profiles = std::collections::HashSet::new();
        let expected_activities: u32 = changes
            .iter()
            .map(|(_, change)| change.activity_count)
            .sum();
        let mut activities = 0usize;
        let mut first_activity_path = None;
        for (_, change) in &changes {
            patterns.insert(change.pattern.as_str());
            activity_profiles.insert(change.activity.as_str());
            let path = NodePath::from(format!("QuestEvolution_{}", change.id).as_str());
            if let Some(root) = self.base().get_node_or_null(&path) {
                built += 1;
                nodes += root.get_child_count() as i64;
            }
            for index in 0..change.activity_count {
                let actor_path = format!(
                    "QuestEvolution_{}/QuestActivity_{}_{}",
                    change.id, change.id, index
                );
                if self
                    .base()
                    .get_node_or_null(&NodePath::from(actor_path.as_str()))
                    .is_some()
                {
                    activities += 1;
                    first_activity_path.get_or_insert(actor_path);
                }
            }
        }
        let actor_position = |game: &Game3D, path: &str| {
            game.base()
                .get_node_or_null(&NodePath::from(path))
                .and_then(|node| node.try_cast::<Node3D>().ok())
                .map(|node| node.get_position())
        };
        let (activity_moved, activity_paused) = first_activity_path
            .as_deref()
            .and_then(|path| actor_position(self, path).map(|before| (path, before)))
            .map(|(path, before)| {
                let previous_mode = self.mode;
                self.mode = Mode::Explore;
                self.tick_map_ambient(0.43);
                let after = actor_position(self, path).unwrap_or(before);
                self.mode = Mode::Paused;
                self.tick_map_ambient(0.61);
                let paused = actor_position(self, path).unwrap_or(after);
                self.mode = previous_mode;
                (
                    before.distance_to(after) > 0.001,
                    after.distance_to(paused) < 0.0001,
                )
            })
            .unwrap_or((false, false));
        let persisted = self.state.as_ref().is_some_and(|state| {
            let data = crate::save::SaveData::from_game(state, 100.0, &self.arsenal);
            let (restored, _, _) = data.into_game();
            changes
                .iter()
                .all(|(quest_id, _)| restored.quests.is_completed(quest_id))
        });
        let mut snapshot = VarDictionary::new();
        snapshot.set("configured", changes.len() as i64);
        snapshot.set("built", built as i64);
        snapshot.set("nodes", nodes);
        snapshot.set("patterns", patterns.len() as i64);
        snapshot.set("activity_profiles", activity_profiles.len() as i64);
        snapshot.set("expected_activities", expected_activities as i64);
        snapshot.set("activities", activities as i64);
        snapshot.set("activity_moved", activity_moved);
        snapshot.set("activity_paused", activity_paused);
        snapshot.set("persisted", persisted);
        snapshot
    }

    #[func]
    fn runtime_smoke_quest_navigation(&mut self) -> VarDictionary {
        let original_states: Vec<_> = self
            .state
            .as_ref()
            .map(|state| {
                state
                    .quests
                    .quests
                    .iter()
                    .map(|quest| quest.state)
                    .collect()
            })
            .unwrap_or_default();
        if let Some(state) = self.state.as_mut() {
            for quest in state.quests.quests.iter_mut().take(2) {
                quest.state = crate::quest::QuestState::Active;
            }
        }
        self.tracked_quest_index = 0;
        self.update_quest_navigation("ru");
        let ru = self
            .quest_objective_label
            .as_ref()
            .map(|label| label.get_text().to_string())
            .unwrap_or_default();
        self.update_quest_navigation("en");
        let first = self
            .quest_objective_label
            .as_ref()
            .map(|label| label.get_text().to_string())
            .unwrap_or_default();
        self.tracked_quest_index = 1;
        self.update_quest_navigation("en");
        let second = self
            .quest_objective_label
            .as_ref()
            .map(|label| label.get_text().to_string())
            .unwrap_or_default();
        let marker_visible = self
            .quest_objective_marker
            .as_ref()
            .is_some_and(|marker| marker.is_visible());
        if let Some(state) = self.state.as_mut() {
            for (quest, original) in state.quests.quests.iter_mut().zip(original_states) {
                quest.state = original;
            }
        }
        let mut snapshot = VarDictionary::new();
        snapshot.set(
            "localized",
            ru.contains("ЦЕЛЬ") && first.contains("TRACKED"),
        );
        snapshot.set("described", first.lines().count() >= 2);
        snapshot.set("distance", first.contains(" m"));
        snapshot.set("target", self.quest_objective_target.is_some());
        snapshot.set("marker_visible", marker_visible);
        snapshot.set("cycled", first != second);
        snapshot
    }

    #[func]
    fn runtime_smoke_quest_journal(&mut self) -> VarDictionary {
        let original_lang = self.settings.lang.clone();
        let original_states: Vec<_> = self
            .state
            .as_ref()
            .map(|state| {
                state
                    .quests
                    .quests
                    .iter()
                    .map(|quest| quest.state)
                    .collect()
            })
            .unwrap_or_default();
        if let Some(state) = self.state.as_mut() {
            for quest in state.quests.quests.iter_mut().take(2) {
                quest.state = crate::quest::QuestState::Active;
            }
        }
        self.settings.lang = "ru".into();
        self.open_journal();
        let ru = self
            .journal_list
            .as_ref()
            .map(|label| label.get_text().to_string())
            .unwrap_or_default();
        self.settings.lang = "en".into();
        self.refresh_quest_journal_ui();
        let en = self
            .journal_list
            .as_ref()
            .map(|label| label.get_text().to_string())
            .unwrap_or_default();
        self.track_journal_quest(1);
        let tracked = self.tracked_quest_index == 1
            && self
                .journal_list
                .as_ref()
                .is_some_and(|label| label.get_text().to_string().contains("[2] ▶"));
        let panel_visible = self
            .journal_panel
            .as_ref()
            .is_some_and(|panel| panel.is_visible());
        self.close_journal();
        if let Some(state) = self.state.as_mut() {
            for (quest, original) in state.quests.quests.iter_mut().zip(original_states) {
                quest.state = original;
            }
        }
        self.settings.lang = original_lang;
        let mut snapshot = VarDictionary::new();
        snapshot.set(
            "localized",
            ru.contains("АКТИВНЫЕ") && en.contains("ACTIVE:"),
        );
        snapshot.set("chains", en.matches("━━").count() as i64 / 2);
        snapshot.set(
            "states",
            en.contains("◆ ACTIVE") && en.contains("✓ COMPLETED"),
        );
        snapshot.set("rewards", en.contains(" XP") && en.contains("gold"));
        snapshot.set("panel_visible", panel_visible);
        snapshot.set("tracked", tracked);
        snapshot.set("closed", self.mode == Mode::Explore);
        snapshot
    }

    #[func]
    fn runtime_smoke_dungeon_quest_navigation(&mut self) -> VarDictionary {
        let original_states: Vec<_> = self
            .state
            .as_ref()
            .map(|state| {
                state
                    .quests
                    .quests
                    .iter()
                    .map(|quest| quest.state)
                    .collect()
            })
            .unwrap_or_default();
        if let Some(state) = self.state.as_mut() {
            for quest in &mut state.quests.quests {
                quest.state = if quest.id == "archive_whispers" {
                    crate::quest::QuestState::Active
                } else {
                    crate::quest::QuestState::Completed
                };
            }
        }
        self.tracked_quest_index = 0;
        self.update_quest_navigation("en");
        let target = self.quest_objective_target;
        let points_to_echo = target.is_some_and(|target| {
            self.dungeon_events
                .iter()
                .any(|event| event.kind == "story_echo" && !event.used && event.pos == target)
        });
        let label = self
            .quest_objective_label
            .as_ref()
            .map(|label| label.get_text().to_string())
            .unwrap_or_default();
        if let Some(state) = self.state.as_mut() {
            for (quest, original) in state.quests.quests.iter_mut().zip(original_states) {
                quest.state = original;
            }
        }
        let mut snapshot = VarDictionary::new();
        snapshot.set("target", target.is_some());
        snapshot.set("points_to_echo", points_to_echo);
        snapshot.set("localized", label.contains("TRACKED"));
        snapshot.set(
            "marker_visible",
            self.quest_objective_marker
                .as_ref()
                .is_some_and(|marker| marker.is_visible()),
        );
        snapshot
    }

    #[func]
    fn runtime_smoke_prepare_quest_events(&mut self) -> VarDictionary {
        self.autosave_enabled = false;
        if let Some(state) = self.state.as_mut() {
            for quest_id in ["echo_contact", "forgotten_cache", "heart_execution"] {
                if !state.quests.has(quest_id) {
                    state.quests.add(quest_id, quest_id, "runtime smoke");
                } else if let Some(quest) = state
                    .quests
                    .quests
                    .iter_mut()
                    .find(|quest| quest.id == quest_id)
                {
                    quest.state = crate::quest::QuestState::Active;
                }
                state.quest_kills.insert(quest_id.to_string(), 0);
            }
        }

        let stranger = self.npcs.iter().position(|npc| npc.id == "stranger");
        if let Some(index) = stranger {
            self.start_dialogue(index);
            self.scene = None;
            self.line_idx = 0;
            self.at_choices = false;
            if let Some(panel) = self.dlg_panel.as_mut() {
                panel.set_visible(false);
            }
            self.set_mode_explore();
        }

        let mut snapshot = VarDictionary::new();
        snapshot.set("stranger_found", stranger.is_some());
        snapshot.set(
            "interact_progress",
            self.state
                .as_ref()
                .and_then(|state| state.quest_kills.get("echo_contact"))
                .copied()
                .unwrap_or(0) as i64,
        );
        snapshot
    }

    #[func]
    fn runtime_smoke_pick_weapon(&mut self) -> VarDictionary {
        let weapon_index = self
            .world_items
            .iter()
            .position(|item| item.in_dungeon && matches!(&item.payload, Payload::Weapon(_)));
        if let Some(index) = weapon_index {
            self.pick_up_item_impl(index, false);
        }

        let mut snapshot = VarDictionary::new();
        snapshot.set("weapon_found", weapon_index.is_some());
        snapshot.set(
            "discover_progress",
            self.state
                .as_ref()
                .and_then(|state| state.quest_kills.get("forgotten_cache"))
                .copied()
                .unwrap_or(0) as i64,
        );
        snapshot
    }

    #[func]
    fn runtime_smoke_probe_boss(&mut self) -> VarDictionary {
        let boss_index = self
            .enemies
            .iter()
            .position(|enemy| enemy.get_position().x > 250.0 && enemy.bind().is_boss);

        let mut boss_id = String::new();
        let mut phase_count = 0;
        let mut middle_phase = 0;
        let mut final_phase = 0;
        let mut tactical_role = String::new();
        let mut telegraph_scale = 0.0;
        let mut phase_events = 0usize;
        let mut phase_animation = false;
        let lights_before = self.light_fx.len();
        if let Some(index) = boss_index {
            let mut boss = self.enemies[index].bind_mut();
            boss_id = boss.cfg_id.to_string();
            tactical_role = boss.tactical_role_id().to_string();
            phase_count = boss.boss_phase_count();
            boss.hp = boss.max_hp * 0.5;
            boss.take_damage(0.0, DmgType::Physical);
            middle_phase = boss.boss_phase_index();
            boss.hp = boss.max_hp * 0.2;
            boss.take_damage(0.0, DmgType::Physical);
            final_phase = boss.boss_phase_index();
            telegraph_scale = boss.strongest_telegraph_scale();
            phase_animation = boss.phase_transition_active();
            phase_events = boss.pending_phase_events();
        }
        self.collect_enemy_requests();
        let phase_fx = self.light_fx.len().saturating_sub(lights_before);

        let mut snapshot = VarDictionary::new();
        snapshot.set("boss_found", boss_index.is_some());
        snapshot.set("boss_id", boss_id);
        snapshot.set("phase_count", phase_count as i64);
        snapshot.set("middle_phase", middle_phase as i64);
        snapshot.set("final_phase", final_phase as i64);
        snapshot.set("tactical_role", tactical_role);
        snapshot.set("telegraph_scale", telegraph_scale as f64);
        snapshot.set("phase_events", phase_events as i64);
        snapshot.set("phase_animation", phase_animation);
        snapshot.set("phase_fx", phase_fx as i64);
        snapshot
    }

    #[func]
    fn runtime_smoke_exit_dungeon(&mut self) -> bool {
        self.exit_dungeon_impl(false);
        self.loc == Loc::World
            && self.dungeon_root.is_none()
            && self.dungeon_nav.is_none()
            && self.dungeon_hazards.is_empty()
    }
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

/// Динамический выбор сцены для NPC.
fn npc_scene_id(npc_id: &str, state: &GameState) -> &'static str {
    match npc_id {
        "vale" => {
            if state.has("boss_defeated_archive_sentinel")
                && !state.has("discussed_archive_sentinel")
            {
                "hub_vale_archive_aftermath"
            } else if state.has("met_vale") {
                "hub_vale_repeat"
            } else {
                "hub_vale_intro"
            }
        }
        "victor" => {
            if state.has("boss_defeated_crypt_warden") && !state.has("discussed_crypt_warden") {
                "hub_victor_warden_aftermath"
            } else if state.has("met_victor") {
                "hub_victor_repeat"
            } else {
                "hub_victor_intro"
            }
        }
        "elena" => {
            if state.has("boss_defeated_blood_oracle") && !state.has("discussed_blood_oracle") {
                "hub_elena_oracle_aftermath"
            } else if state.has("met_elena") {
                "hub_elena_repeat"
            } else {
                "hub_elena_intro"
            }
        }
        "sofia" => {
            if state.has("met_sofia") {
                "hub_sofia_repeat"
            } else {
                "hub_sofia_intro"
            }
        }
        "guard" => {
            if state.has("met_guard") {
                "hub_guard_repeat"
            } else {
                "hub_guard_intro"
            }
        }
        "merchant" => {
            if state.has("met_merchant") {
                "hub_merchant_repeat"
            } else {
                "hub_merchant_intro"
            }
        }
        "stranger" => {
            if state.has("boss_defeated_heart_tyrant") && !state.has("discussed_heart_tyrant") {
                "hub_stranger_tyrant_aftermath"
            } else if state.has("met_stranger") {
                "hub_stranger_repeat"
            } else {
                "hub_stranger_intro"
            }
        }
        _ => "",
    }
}

// ── Подмодули: реализация Game3D разложена по концернам ───────────────────────
mod class_select;
mod combat;
mod conversation;
mod delve;
mod environment;
mod gameplay;
mod hud;
mod hud_update;
mod items;
