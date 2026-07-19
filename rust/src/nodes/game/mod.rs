//! Game3D — главный узел игры.
//!
//! Открытый мир (хаб + пустоши) и процедурные данжи, RPG-классы (3×3 спека),
//! DOOM-боёвка: hitscan / мили / снаряды, FP-спрайт оружия на HUD.

use godot::prelude::*;
use godot::classes::{
    AtlasTexture, AudioStream, AudioStreamPlayer, AudioStreamPlayer3D,
    CanvasLayer, CharacterBody3D, DirectionalLight3D,
    Environment, Image, ImageTexture, Input, InputEvent, InputEventKey, Label,
    Node3D, OmniLight3D,
    PanoramaSkyMaterial, Panel, PhysicsRayQueryParameters3D, Sky, Sprite3D,
    StyleBoxFlat, Texture2D, TextureRect, VBoxContainer, WorldEnvironment, INode3D,
};
use godot::classes::environment::{AmbientSource, BgMode, ToneMapper};
use godot::classes::image::Format;
use godot::global::HorizontalAlignment;

use crate::classes::{classes, compute_loadout, xp_to_next, ClassDef, Loadout};
use crate::config::GameConfig;
use crate::dialogue::{Choice, Effect, Line, Scene};
use crate::dungeon::{self, DungeonPlan};
use crate::enemy::Enemy;
use crate::game_state::GameState;
use crate::gfx::{make_billboard, make_light, Rng, TexCache};
use crate::locale::t;
use crate::nav::NavGrid;
use crate::player::Player;
use crate::save;
use crate::settings::Settings;
use crate::story::get_scene;
use crate::weapon::{weapon_def, AmmoType, Arsenal, DmgType, FireKind, WeaponId, FRAME_W};
use crate::world;

// ── NPC ───────────────────────────────────────────────────────────────────────

#[allow(dead_code)]
struct NpcCfg {
    id:       &'static str,
    name:     &'static str,
    scene_id: &'static str,
    pos:      Vector3,
    color:    Color,
}

const NPC_DATA: &[NpcCfg] = &[
    NpcCfg { id: "vale",      name: "Ms. Вейл",   scene_id: "meet_vale",      pos: Vector3::new(-6.0, 0.0, -8.0),  color: Color::from_rgba(1.0, 0.75, 0.85, 1.0) },
    NpcCfg { id: "victor",    name: "Виктор",     scene_id: "intro_victor",   pos: Vector3::new(6.0, 0.0, -8.0),   color: Color::from_rgba(0.75, 1.0, 0.8, 1.0) },
    NpcCfg { id: "elena",     name: "Елена",      scene_id: "first_elena",    pos: Vector3::new(-11.0, 0.0, 3.0),  color: Color::from_rgba(0.75, 0.8, 1.0, 1.0) },
    NpcCfg { id: "sofia",     name: "София",      scene_id: "meet_sofia",     pos: Vector3::new(11.0, 0.0, 3.0),   color: Color::from_rgba(1.0, 0.95, 0.7, 1.0) },
    NpcCfg { id: "guard",     name: "Охранник",   scene_id: "meet_guard",     pos: Vector3::new(-2.5, 0.0, -18.0), color: Color::from_rgba(0.85, 0.85, 0.85, 1.0) },
    NpcCfg { id: "merchant",  name: "Торговец",   scene_id: "meet_merchant",  pos: Vector3::new(17.5, 0.0, -1.0),  color: Color::from_rgba(1.0, 0.85, 0.6, 1.0) },
    NpcCfg { id: "scientist", name: "Учёный",     scene_id: "meet_scientist", pos: Vector3::new(-18.0, 0.0, 0.0),  color: Color::from_rgba(0.7, 1.0, 1.0, 1.0) },
    NpcCfg { id: "stranger",  name: "Незнакомец", scene_id: "meet_stranger",  pos: Vector3::new(5.0, 0.0, 16.0),   color: Color::from_rgba(0.8, 0.65, 0.95, 1.0) },
];

fn npc_sprite_tex(id: &str) -> (&'static str, &'static str) {
    match id {
        "vale"      => ("res://assets/sprites/characters/npc_vale.png",      "res://assets/sprites/femboy_pink.png"),
        "victor"    => ("res://assets/sprites/characters/npc_victor.png",    "res://assets/sprites/femboy_dark2.png"),
        "elena"     => ("res://assets/sprites/characters/npc_elena.png",     "res://assets/sprites/femboy_dark1.png"),
        "sofia"     => ("res://assets/sprites/characters/npc_sofia.png",     "res://assets/sprites/femboy_pink.png"),
        "guard"     => ("res://assets/sprites/characters/npc_guard.png",     "res://assets/sprites/femboy_dark2.png"),
        "merchant"  => ("res://assets/sprites/characters/npc_merchant.png",  "res://assets/sprites/femboy_pink.png"),
        "scientist" => ("res://assets/sprites/characters/npc_scientist.png", "res://assets/sprites/femboy_dark1.png"),
        "stranger"  => ("res://assets/sprites/characters/npc_stranger.png",  "res://assets/sprites/femboy_dark2.png"),
        _           => ("res://assets/sprites/femboy_dark1.png",             "res://assets/sprites/femboy_dark1.png"),
    }
}

fn item_sprite_tex(id: &str) -> &'static str {
    match id {
        "medkit"       => "res://assets/sprites/items/item_medkit.png",
        "key"          => "res://assets/sprites/items/item_key.png",
        "gold_coin" | "gold_stack" => "res://assets/sprites/items/item_gold.png",
        "armor_shard"  => "res://assets/sprites/items/item_armor.png",
        "energy_drink" => "res://assets/sprites/items/item_energy_drink.png",
        "potion"       => "res://assets/sprites/items/item_potion.png",
        "ancient_ruby" => "res://assets/sprites/items/item_ruby.png",
        "heart_1up"    => "res://assets/sprites/pickups/heart_1up.png",
        "soul"         => "res://assets/sprites/pickups/soul.png",
        _              => "",
    }
}

// ── Режимы и полезная нагрузка предметов ─────────────────────────────────────

#[derive(PartialEq, Clone, Copy)]
enum Mode { ClassSelect, SpecSelect, Explore, Dialogue, Dead, Inventory, Perks, Paused }

#[derive(PartialEq, Clone, Copy)]
enum Loc { World, Dungeon }

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
    node:    Gd<Node3D>,
    item_id: String,
    name:    String,
    payload: Payload,
    in_dungeon: bool,
}

/// Рантайм-описание NPC (из npcs.json пресета или legacy NPC_DATA).
struct NpcRt {
    id:    String,
    name:  String,
    scene: Option<String>,   // "story" → динамика story.rs; иначе конкретный id сцены
    quest: Option<String>,   // id квеста из quests.json (гивер)
}

struct SpriteFx { node: Gd<Sprite3D>, ttl: f32, total: f32 }
struct LightFx  { node: Gd<OmniLight3D>, ttl: f32, total: f32, energy: f32 }

struct Projectile {
    node:     Gd<Node3D>,
    pos:      Vector3,
    vel:      Vector3,
    dmg:      f32,
    dmg_type: DmgType,
    splash:   f32,
    ttl:      f32,
    status:   Option<(String, f32)>,   // статус оружия при попадании
}

/// Прямое попадание снаряда: (враг, урон, тип, точка, статус оружия).
type DirectHit = (Gd<Enemy>, f32, DmgType, Vector3, Option<(String, f32)>);
/// Убитый враг для пост-обработки: (позиция, XP, босс?, id, посмертный взрыв).
type KillInfo = (Vector3, f32, bool, String, Option<(f32, f32)>);

/// Снаряд врага (abilities.json: projectile_burst) — летит в игрока, можно увернуться.
struct EnemyProjectile {
    node: Gd<Node3D>,
    pos:  Vector3,
    vel:  Vector3,
    dmg:  f32,
    ttl:  f32,
    status: Option<String>,   // статус на игрока при попадании (id)
}

#[derive(PartialEq, Clone, Copy)]
#[allow(clippy::enum_variant_names)]
enum PortalKind { EnterDungeon, ExitDungeon, DeeperDungeon }

// ── Анимация FP-оружия ────────────────────────────────────────────────────────

#[derive(PartialEq, Clone, Copy)]
enum WeaponAnim { Idle, Fire(usize), Switch(f32) }

// ── Главная структура ─────────────────────────────────────────────────────────

#[derive(GodotClass)]
#[class(base = Node3D)]
pub struct Game3D {
    base: Base<Node3D>,

    cache:       TexCache,
    rng:         Rng,
    cfg:         Option<GameConfig>,

    preset:      String,
    gate_pos:    Vector3,
    world_name:  String,

    player:      Option<Gd<CharacterBody3D>>,
    npc_sprites: Vec<Gd<Sprite3D>>,
    npcs:        Vec<NpcRt>,
    enemies:     Vec<Gd<Enemy>>,
    world_items: Vec<WorldItemNode>,
    projectiles: Vec<Projectile>,
    enemy_projectiles: Vec<EnemyProjectile>,
    sprite_fx:   Vec<SpriteFx>,
    light_fx:    Vec<LightFx>,
    sfx_2d:      Vec<Gd<AudioStreamPlayer>>,
    sfx_3d:      Vec<Gd<AudioStreamPlayer3D>>,

    // Реактивный пост-процесс: хэндл материала + затухающие импульсы эффектов
    post_mat: Option<Gd<godot::classes::ShaderMaterial>>,
    fx_hit:   f32,  // получен урон  → красная пульсация/аберрация
    fx_kill:  f32,  // убийство      → короткий яркий «панч»
    fx_pick:  f32,  // подбор предмета→ тёплое золотистое свечение

    state:       Option<GameState>,
    settings:    Settings,
    arsenal:     Arsenal,
    loadout:     Loadout,
    mode:        Mode,
    loc:         Loc,

    // данж
    dungeon_root:   Option<Gd<Node3D>>,
    dungeon_depth:  u32,
    dungeon_name:   String,
    dungeon_nav:    Option<std::sync::Arc<NavGrid>>,
    exit_portal:    Vector3,
    next_portal:    Vector3,
    boss_alive:     bool,

    // диалог
    scene:       Option<Scene>,
    line_idx:    usize,
    at_choices:  bool,

    near_npc:    Option<usize>,
    near_enemy:  Option<usize>,
    near_item:   Option<usize>,
    near_portal: Option<PortalKind>,

    shoot_cd:    f32,
    weapon_anim: WeaponAnim,
    anim_timer:  f32,
    idle_frame:  usize,

    npc_anim_timer: f32,
    npc_anim_frame: usize,

    class_pick:  usize,   // выбранный класс на этапе выбора спека

    // статусы игрока (горение/кровь/замедление/уязвимость от врагов)
    player_statuses: crate::status::StatusSet,

    // HUD
    hint_label:      Option<Gd<Label>>,
    status_label:    Option<Gd<Label>>,
    hp_bar_fg:       Option<Gd<Panel>>,
    hp_label:        Option<Gd<Label>>,
    xp_bar_fg:       Option<Gd<Panel>>,
    xp_label:        Option<Gd<Label>>,
    ammo_label:      Option<Gd<Label>>,
    weapon_label:    Option<Gd<Label>>,
    loc_label:       Option<Gd<Label>>,
    dlg_panel:       Option<Gd<Panel>>,
    dlg_speaker:     Option<Gd<Label>>,
    dlg_text:        Option<Gd<Label>>,
    choice_box:      Option<Gd<VBoxContainer>>,
    cl0: Option<Gd<Label>>, cl1: Option<Gd<Label>>,
    cl2: Option<Gd<Label>>, cl3: Option<Gd<Label>>,
    flash_label:     Option<Gd<Label>>,
    flash_timer:     f32,
    inv_label:       Option<Gd<Label>>,
    quest_label:     Option<Gd<Label>>,
    inv_panel:       Option<Gd<Panel>>,
    inv_list:        Option<Gd<Label>>,
    perk_panel:      Option<Gd<Panel>>,
    perk_list:       Option<Gd<Label>>,
    crosshair:       Option<Gd<Label>>,
    dead_panel:      Option<Gd<Panel>>,
    pause_panel:     Option<Gd<Panel>>,
    enemies_frozen:  bool,
    compass_label:   Option<Gd<Label>>,
    targeting_label: Option<Gd<Label>>,
    damage_flash:    Option<Gd<Panel>>,
    damage_flash_timer: f32,

    weapon_rect:     Option<Gd<TextureRect>>,
    weapon_atlas:    Option<Gd<AtlasTexture>>,
    muzzle_light:    Option<Gd<OmniLight3D>>,
    muzzle_timer:    f32,

    // миникарта данжа (floor_map генератора → текстура + точка игрока)
    minimap_bg:      Option<Gd<Panel>>,
    minimap_rect:    Option<Gd<TextureRect>>,
    minimap_dot:     Option<Gd<Panel>>,
    minimap_floor:   Vec<bool>,

    // выбор класса
    select_panel:    Option<Gd<Panel>>,
    select_title:    Option<Gd<Label>>,
    card_titles:     Vec<Gd<Label>>,
    card_bodies:     Vec<Gd<Label>>,

    game_time:       f32,
}

// ── Константы ─────────────────────────────────────────────────────────────────

const INTERACT_R: f32 = 2.8;
const PICKUP_R: f32   = 1.6;
const PORTAL_R: f32   = 2.4;
const PIXEL_SZ: f32   = 0.010;
const HUD_W: f32      = 1920.0;
const HUD_H: f32      = 1080.0;

const DUNGEON_OFFSET: Vector3 = Vector3::new(500.0, 0.0, 500.0);

// ── Звуковые эффекты (варианты — выбираем случайный для разнообразия) ──────────
const SFX_FIRE: [&str; 3] = [
    "res://assets/sounds/Plasma Gun.wav",
    "res://assets/sounds/Plasma Gun1.wav",
    "res://assets/sounds/Plasma Gun Shot.wav",
];
const SFX_MELEE: [&str; 2] = [
    "res://assets/sounds/Plasma Sword Strike.wav",
    "res://assets/sounds/Plasma Sword Strike1.wav",
];
const SFX_DEATH: [&str; 2] = [
    "res://assets/sounds/The Evil Robot Dies.wav",
    "res://assets/sounds/The Evil Robot Dies1.wav",
];
const SFX_WALK: [&str; 2] = [
    "res://assets/sounds/An Evil Robot Is Walking.wav",
    "res://assets/sounds/An Evil Robot Is Walking1.wav",
];

const NPC_IDLE_FRAMES: [(f32, f32, f32, f32); 2] = [
    (0.0,   0.0, 128.0, 256.0),
    (128.0, 0.0, 128.0, 256.0),
];
const IDLE_FPS: f32 = 3.0;

const C_UI_BG:  Color = Color::from_rgba(0.04, 0.03, 0.07, 0.94);
const C_BORDER: Color = Color::from_rgba(0.65, 0.30, 0.52, 1.0);
const C_MAIN:   Color = Color::from_rgba(0.95, 0.92, 0.98, 1.0);
const C_DIM:    Color = Color::from_rgba(0.58, 0.52, 0.66, 1.0);
const C_PINK:   Color = Color::from_rgba(1.00, 0.55, 0.80, 1.0);
const C_GOLD:   Color = Color::from_rgba(1.00, 0.84, 0.30, 1.0);
const C_RED:    Color = Color::from_rgba(0.90, 0.15, 0.15, 1.0);
const C_CYAN:   Color = Color::from_rgba(0.40, 0.90, 1.00, 1.0);
const C_XP:     Color = Color::from_rgba(0.55, 0.35, 0.95, 1.0);

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
            player: None, npc_sprites: Vec::new(), npcs: Vec::new(),
            enemies: Vec::new(), world_items: Vec::new(),
            projectiles: Vec::new(), enemy_projectiles: Vec::new(),
            sprite_fx: Vec::new(), light_fx: Vec::new(),
            sfx_2d: Vec::new(), sfx_3d: Vec::new(),
            post_mat: None, fx_hit: 0.0, fx_kill: 0.0, fx_pick: 0.0,
            state: None, settings: Settings::default(),
            arsenal: Arsenal::new(),
            loadout: compute_loadout(0, 0, 1),
            mode: Mode::Explore, loc: Loc::World,
            dungeon_root: None, dungeon_depth: 0, dungeon_name: String::new(),
            dungeon_nav: None,
            exit_portal: Vector3::ZERO, next_portal: Vector3::ZERO,
            boss_alive: false,
            scene: None, line_idx: 0, at_choices: false,
            near_npc: None, near_enemy: None, near_item: None, near_portal: None,
            shoot_cd: 0.0,
            weapon_anim: WeaponAnim::Idle, anim_timer: 0.0, idle_frame: 0,
            npc_anim_timer: 0.0, npc_anim_frame: 0,
            class_pick: 0,
            player_statuses: crate::status::StatusSet::new(),
            hint_label: None, status_label: None,
            hp_bar_fg: None, hp_label: None,
            xp_bar_fg: None, xp_label: None,
            ammo_label: None, weapon_label: None, loc_label: None,
            dlg_panel: None, dlg_speaker: None, dlg_text: None,
            choice_box: None,
            cl0: None, cl1: None, cl2: None, cl3: None,
            flash_label: None, flash_timer: 0.0,
            inv_label: None, quest_label: None,
            inv_panel: None, inv_list: None,
            perk_panel: None, perk_list: None,
            crosshair: None, dead_panel: None,
            pause_panel: None, enemies_frozen: false,
            compass_label: None, targeting_label: None,
            damage_flash: None, damage_flash_timer: 0.0,
            weapon_rect: None, weapon_atlas: None,
            muzzle_light: None, muzzle_timer: 0.0,
            minimap_bg: None, minimap_rect: None, minimap_dot: None,
            minimap_floor: Vec::new(),
            select_panel: None, select_title: None,
            card_titles: Vec::new(), card_bodies: Vec::new(),
            game_time: 0.0,
        }
    }

    fn ready(&mut self) {
        self.settings = Settings::load();
        self.settings.apply_global();   // окно/vsync/громкость
        let lang = self.settings.lang.clone();

        let loaded = save::load();
        let has_class = loaded.as_ref().map(|(s, _, _)| s.class_idx.is_some()).unwrap_or(false);
        // Пресет: у сейва приоритет (продолжаем ту игру, которую начали).
        let preset = loaded.as_ref()
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
                self.world_name = built.name_ru.to_uppercase();
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
        self.build_npcs();
        self.build_hud(&lang);
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
            if k.is_pressed() && !k.is_echo()
                && k.get_physical_keycode() == godot::global::Key::F11 {
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
        self.tick_fx(dt);
        self.tick_sfx();
        self.tick_post_fx(dt);
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
            Mode::SpecSelect  => self.process_spec_select(),
            Mode::Explore     => self.process_explore(),
            Mode::Dialogue    => self.process_dialogue(),
            Mode::Inventory   => self.process_inventory(),
            Mode::Perks       => self.process_perks(),
            Mode::Dead        => self.process_dead(),
            Mode::Paused      => self.process_paused(),
        }

        self.update_hp_bar();
        self.update_xp_bar();
        self.update_ammo_hud();
        self.update_targeting_hud();
        self.update_minimap();
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
    c.set_anchor(Side::LEFT,   ax);
    c.set_anchor(Side::RIGHT,  ax);
    c.set_anchor(Side::TOP,    ay);
    c.set_anchor(Side::BOTTOM, ay);
    c.set_offset(Side::LEFT,   x - ax * HUD_W);
    c.set_offset(Side::RIGHT,  x + w - ax * HUD_W);
    c.set_offset(Side::TOP,    y - ay * HUD_H);
    c.set_offset(Side::BOTTOM, y + h - ay * HUD_H);
}

/// Динамический выбор сцены для NPC.
fn npc_scene_id(npc_id: &str, state: &GameState) -> &'static str {
    match npc_id {
        "vale" => {
            if !state.has("met_vale")              { "meet_vale" }
            else if !state.has("vale_chat_1_done") { "vale_class_chat" }
            else if state.rel("vale") < 30         { "vale_office_1" }
            else if state.rel("vale") < 55         { "vale_office_2" }
            else                                   { "vale_office_deep" }
        }
        "victor" => {
            if !state.has("met_victor")              { "intro_victor" }
            else if !state.has("victor_quest_given") { "victor_chat_2" }
            else if state.has("victor_quest_done")   { "victor_chat_end" }
            else                                     { "victor_quest_check" }
        }
        "elena" => {
            if !state.has("met_elena")              { "first_elena" }
            else if !state.has("elena_lib_1")       { "elena_library_1" }
            else if !state.has("elena_quest_given") { "elena_chat_2" }
            else if state.has("elena_quest_done")   { "elena_chat_end" }
            else                                    { "elena_quest_check" }
        }
        "sofia" => {
            if !state.has("met_sofia")           { "meet_sofia" }
            else if !state.has("sofia_deep_done"){ "sofia_chat" }
            else                                 { "sofia_chat_3" }
        }
        "guard" => {
            if !state.has("met_guard")              { "meet_guard" }
            else if !state.has("guard_quest_given") { "guard_quest_offer" }
            else if state.has("guard_quest_done")   { "guard_quest_end" }
            else                                    { "guard_quest_check" }
        }
        "merchant" => {
            if !state.has("met_merchant")        { "meet_merchant" }
            else if !state.has("merchant_bought"){ "merchant_shop" }
            else                                 { "merchant_again" }
        }
        "scientist" => {
            if !state.has("met_scientist")              { "meet_scientist" }
            else if !state.has("scientist_quest_given") { "scientist_quest_offer" }
            else if state.has("scientist_quest_done")   { "scientist_quest_end" }
            else                                        { "scientist_quest_check" }
        }
        "stranger" => {
            if !state.has("met_stranger") { "meet_stranger" }
            else                          { "stranger_again" }
        }
        _ => "",
    }
}

// ── Подмодули: реализация Game3D разложена по концернам ───────────────────────
mod environment;
mod delve;
mod class_select;
mod hud;
mod gameplay;
mod combat;
mod items;
mod conversation;
mod hud_update;
