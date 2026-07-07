//! Game3D — главный узел игры.
//!
//! Открытый мир (хаб + пустоши) и процедурные данжи, RPG-классы (3×3 спека),
//! DOOM-боёвка: hitscan / мили / снаряды, FP-спрайт оружия на HUD.

use godot::prelude::*;
use godot::classes::{
    AtlasTexture, CanvasLayer, CharacterBody3D, DirectionalLight3D,
    Environment, Image, ImageTexture, Input, Label, Node3D, OmniLight3D,
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
    sprite_fx:   Vec<SpriteFx>,
    light_fx:    Vec<LightFx>,

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

    // HUD
    hint_label:      Option<Gd<Label>>,
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
            projectiles: Vec::new(), sprite_fx: Vec::new(), light_fx: Vec::new(),
            state: None, settings: Settings::default(),
            arsenal: Arsenal::new(),
            loadout: compute_loadout(0, 0, 1),
            mode: Mode::Explore, loc: Loc::World,
            dungeon_root: None, dungeon_depth: 0, dungeon_name: String::new(),
            exit_portal: Vector3::ZERO, next_portal: Vector3::ZERO,
            boss_alive: false,
            scene: None, line_idx: 0, at_choices: false,
            near_npc: None, near_enemy: None, near_item: None, near_portal: None,
            shoot_cd: 0.0,
            weapon_anim: WeaponAnim::Idle, anim_timer: 0.0, idle_frame: 0,
            npc_anim_timer: 0.0, npc_anim_frame: 0,
            class_pick: 0,
            hint_label: None,
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

    fn process(&mut self, delta: f64) {
        let dt = delta as f32;
        self.game_time += dt;
        self.shoot_cd = (self.shoot_cd - dt).max(0.0);
        self.tick_flash(dt);
        self.tick_damage_flash(dt);
        self.tick_npc_anim(dt);
        self.tick_fx(dt);
        self.tick_weapon_anim(dt);
        self.tick_muzzle(dt);
        self.update_compass();

        // Бой идёт только в Explore: в меню (инвентарь/перки/диалог/пауза/смерть)
        // снаряды замирают, враги заморожены и не наносят урона.
        let in_gameplay = self.mode == Mode::Explore;
        if in_gameplay {
            self.tick_projectiles(dt);
            self.collect_enemy_damage(dt);
        }
        if self.enemies_frozen == in_gameplay {
            let frozen = !in_gameplay;
            for e in self.enemies.iter_mut() {
                let mut b = e.bind_mut();
                b.frozen = frozen;
                // Удар, успевший лечь в pending_dmg в тик перехода в меню,
                // сгорает: иначе он «прилетел бы из паузы» после закрытия.
                if frozen { b.pending_dmg = 0.0; }
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

// ── Окружение и мир ───────────────────────────────────────────────────────────

impl Game3D {
    fn build_environment(&mut self, map_env: Option<&crate::map::MapEnv>) {
        use godot::classes::light_3d::Param;

        let sky_name = map_env
            .and_then(|e| e.sky.clone())
            .unwrap_or_else(|| "sky_purple".to_string());
        let ambient = map_env.and_then(|e| e.ambient).unwrap_or([0.32, 0.22, 0.38]);
        let ambient_energy = map_env.and_then(|e| e.ambient_energy).unwrap_or(1.15);
        let fog_density = map_env.and_then(|e| e.fog_density).unwrap_or(0.010);

        let mut env = Environment::new_gd();
        let sky_path = format!("res://assets/textures/sky/{}.png", sky_name);
        if let Some(sky_tex) = self.cache.get(&sky_path) {
            let mut sky_mat = PanoramaSkyMaterial::new_gd();
            sky_mat.set_panorama(&sky_tex);
            let mut sky = Sky::new_gd();
            sky.set_material(&sky_mat);
            env.set_background(BgMode::SKY);
            env.set_sky(&sky);
        }
        env.set_ambient_source(AmbientSource::COLOR);
        env.set_ambient_light_color(Color::from_rgba(ambient[0], ambient[1], ambient[2], 1.0));
        env.set_ambient_light_energy(ambient_energy);
        env.set_fog_enabled(true);
        env.set_fog_light_color(Color::from_rgba(0.10, 0.05, 0.14, 1.0));
        env.set_fog_density(fog_density);

        // bloom/glow — неоновая эстетика (Forward Mobile рендерер)
        env.set_glow_enabled(true);
        env.set_glow_intensity(0.65);
        env.set_glow_strength(1.0);
        env.set_glow_bloom(0.18);
        env.set_glow_hdr_bleed_threshold(1.0);

        // tone mapping
        env.set_tonemapper(ToneMapper::ACES);
        env.set_tonemap_exposure(1.08);

        let mut we = WorldEnvironment::new_alloc();
        we.set_environment(&env);
        self.base_mut().add_child(&we);

        let mut dir = DirectionalLight3D::new_alloc();
        dir.set_rotation(Vector3::new(-0.9, 0.3, 0.0));
        dir.set_param(Param::ENERGY, 0.35);
        dir.set_color(Color::from_rgba(0.8, 0.6, 0.85, 1.0));
        dir.set_shadow(false);
        self.base_mut().add_child(&dir);
    }

    fn build_npcs(&mut self) {
        // NPC из npcs.json пресета; legacy-таблица NPC_DATA — только если файла нет вовсе.
        let (cfg_npcs, file_present): (Vec<crate::config::NpcCfg>, bool) = self.cfg.as_ref()
            .map(|c| (c.npcs.clone(), c.npcs_file_present))
            .unwrap_or_default();

        let mut sprites: Vec<Gd<Sprite3D>> = Vec::new();
        let mut npcs: Vec<NpcRt> = Vec::new();

        if cfg_npcs.is_empty() && !file_present {
            for cfg in NPC_DATA.iter() {
                let (new_path, fallback) = npc_sprite_tex(cfg.id);
                let path = if self.cache.get(new_path).is_some() { new_path } else { fallback };
                if let Some(mut sprite) = make_billboard(&mut self.cache, path,
                                                         cfg.pos + Vector3::new(0.0, 1.28, 0.0), PIXEL_SZ) {
                    sprite.set_region_enabled(true);
                    let (x, y, w, h) = NPC_IDLE_FRAMES[0];
                    sprite.set_region_rect(Rect2::new(Vector2::new(x, y), Vector2::new(w, h)));
                    sprite.set_modulate(cfg.color);
                    self.base_mut().add_child(&sprite);
                    sprites.push(sprite);
                    npcs.push(NpcRt {
                        id: cfg.id.to_string(),
                        name: cfg.name.to_string(),
                        scene: Some("story".to_string()),
                        quest: None,
                    });
                }
            }
        } else {
            for nc in &cfg_npcs {
                let sprite_name = if nc.sprite.is_empty() { format!("npc_{}", nc.id) } else { nc.sprite.clone() };
                let path = format!("res://assets/sprites/characters/{}.png", sprite_name);
                let path = if self.cache.get(&path).is_some() {
                    path
                } else {
                    "res://assets/sprites/femboy_dark1.png".to_string()
                };
                if let Some(mut sprite) = make_billboard(&mut self.cache, &path,
                        Vector3::new(nc.pos[0], 1.28, nc.pos[1]), PIXEL_SZ) {
                    sprite.set_region_enabled(true);
                    let (x, y, w, h) = NPC_IDLE_FRAMES[0];
                    sprite.set_region_rect(Rect2::new(Vector2::new(x, y), Vector2::new(w, h)));
                    if let Some(c) = nc.color {
                        sprite.set_modulate(Color::from_rgba(c[0], c[1], c[2], 1.0));
                    }
                    self.base_mut().add_child(&sprite);
                    sprites.push(sprite);
                    npcs.push(NpcRt {
                        id: nc.id.clone(),
                        name: nc.name_ru.clone(),
                        scene: nc.scene.clone(),
                        quest: nc.quest.clone(),
                    });
                }
            }
        }
        self.npc_sprites = sprites;
        self.npcs = npcs;
    }

    /// Спавны открытого мира из legacy data/level.json (когда у пресета нет карты).
    fn build_world_spawns(&mut self) {
        let Some(cfg) = self.cfg.take() else { return };
        let level = cfg.level.clone();
        self.cfg = Some(cfg);
        self.spawn_from_level(&level);
    }

    /// Заспавнить врагов/предметы/патроны/оружие из структуры LevelCfg (карта или level.json).
    fn spawn_from_level(&mut self, level: &crate::config::LevelCfg) {
        let Some(cfg) = self.cfg.take() else { return };

        for spawn in &level.spawn_enemies {
            self.spawn_enemy(&cfg, &spawn.kind,
                             Vector3::new(spawn.x, 0.0, spawn.z), 1.0, false, false);
        }
        for spawn in &level.spawn_items {
            self.spawn_item(&cfg, &spawn.kind, Vector3::new(spawn.x, 0.0, spawn.z), false);
        }
        for spawn in &level.spawn_ammo {
            let t = match spawn.kind.as_str() {
                "shells"  => AmmoType::Shells,
                "rockets" => AmmoType::Rockets,
                "cells"   => AmmoType::Cells,
                _         => AmmoType::Bullets,
            };
            self.spawn_ammo_pickup(t, spawn.amount, Vector3::new(spawn.x, 0.0, spawn.z), false);
        }
        for spawn in &level.spawn_weapons {
            if let Some(w) = weapon_by_name(&spawn.kind) {
                self.spawn_weapon_pickup(w, Vector3::new(spawn.x, 0.0, spawn.z), false);
            }
        }
        self.cfg = Some(cfg);
    }

    fn spawn_enemy(&mut self, cfg: &GameConfig, kind: &str, pos: Vector3, mult: f32,
                   is_boss: bool, in_dungeon: bool) {
        let Some(ecfg) = cfg.enemy(kind) else {
            godot_warn!("[spawn] враг '{kind}' не найден в enemies.json пресета — пропускаю");
            return;
        };
        let mut e = Enemy::new_alloc();
        e.set_position(pos);
        self.base_mut().add_child(&e);
        let color = Color::from_rgba(ecfg.color_r, ecfg.color_g, ecfg.color_b, 1.0);
        e.bind_mut().configure(
            &ecfg.id, ecfg.hp, ecfg.speed, ecfg.attack_damage,
            ecfg.attack_range, ecfg.attack_cooldown, ecfg.chase_range,
            ecfg.patrol_radius, color, pos, ecfg.xp, mult, is_boss,
            ecfg.resist.arr(),
            ecfg.sprite.as_deref().unwrap_or(&ecfg.id), ecfg.scale,
        );
        if let Some(ref p) = self.player {
            e.bind_mut().set_player(p.clone());
        }
        let _ = in_dungeon;
        self.enemies.push(e);
    }

    fn make_pickup_node(&mut self, tex_path: &str, pos: Vector3, px: f32) -> Gd<Node3D> {
        let mut node = Node3D::new_alloc();
        node.set_position(pos + Vector3::new(0.0, 0.55, 0.0));
        if let Some(sp) = make_billboard(&mut self.cache, tex_path, Vector3::ZERO, px) {
            node.add_child(&sp);
        }
        self.base_mut().add_child(&node);
        node
    }

    fn spawn_item(&mut self, cfg: &GameConfig, kind: &str, pos: Vector3, in_dungeon: bool) {
        // специальные предметы вне items.json
        if kind == "heart_1up" {
            let node = self.make_pickup_node("res://assets/sprites/pickups/heart_1up.png", pos, 0.010);
            self.world_items.push(WorldItemNode {
                node, item_id: "heart_1up".into(), name: "Сердце жизни".into(),
                payload: Payload::Heart, in_dungeon,
            });
            return;
        }
        let Some(icfg) = cfg.item(kind) else {
            godot_warn!("[spawn] предмет '{kind}' не найден в items.json пресета — пропускаю");
            return;
        };
        let tex = item_sprite_tex(&icfg.id);
        let node = if tex.is_empty() {
            self.make_pickup_node("res://assets/sprites/items/item_potion.png", pos, 0.008)
        } else {
            self.make_pickup_node(tex, pos, 0.008)
        };
        let name = if self.settings.lang == "en" { icfg.name_en.clone() } else { icfg.name_ru.clone() };
        let payload = if icfg.category == "currency" {
            Payload::Gold(icfg.value as i32)
        } else if icfg.category == "key" {
            Payload::KeyItem
        } else {
            Payload::Consumable { heal: icfg.heal.unwrap_or(10.0) }
        };
        self.world_items.push(WorldItemNode {
            node, item_id: icfg.id.clone(), name, payload, in_dungeon,
        });
    }

    fn spawn_ammo_pickup(&mut self, t: AmmoType, amount: u32, pos: Vector3, in_dungeon: bool) {
        let node = self.make_pickup_node(t.pickup_tex(), pos, 0.009);
        self.world_items.push(WorldItemNode {
            node,
            item_id: format!("ammo_{}", t.idx()),
            name: t.name_ru().to_string(),
            payload: Payload::Ammo(t, amount),
            in_dungeon,
        });
    }

    fn spawn_weapon_pickup(&mut self, w: WeaponId, pos: Vector3, in_dungeon: bool) {
        let def = weapon_def(w);
        let mut node = Node3D::new_alloc();
        node.set_position(pos + Vector3::new(0.0, 0.65, 0.0));
        if let Some(mut sp) = make_billboard(&mut self.cache, &def.sheet, Vector3::ZERO, 0.012) {
            sp.set_region_enabled(true);
            sp.set_region_rect(Rect2::new(Vector2::ZERO, Vector2::new(FRAME_W, def.frame_h)));
            node.add_child(&sp);
        }
        let l = make_light(Vector3::new(0.0, 0.4, 0.0), C_PINK, 0.7, 4.0);
        node.add_child(&l);
        self.base_mut().add_child(&node);
        self.world_items.push(WorldItemNode {
            node,
            item_id: format!("weapon_{}", def.name_ru),
            name: def.name_ru.to_string(),
            payload: Payload::Weapon(w),
            in_dungeon,
        });
    }
}

fn weapon_by_name(s: &str) -> Option<WeaponId> {
    Some(match s {
        "sword"    => WeaponId::Sword,
        "chainsaw" => WeaponId::Chainsaw,
        "pistol"   => WeaponId::Pistol,
        "shotgun"  => WeaponId::Shotgun,
        "rifle"    => WeaponId::Rifle,
        "nailgun"  => WeaponId::Nailgun,
        "plasma"   => WeaponId::Plasma,
        "rocket"   => WeaponId::Rocket,
        _ => return None,
    })
}

// ── Данж ──────────────────────────────────────────────────────────────────────

impl Game3D {
    fn enter_dungeon(&mut self, depth: u32) {
        // cfg берём ДО разрушительных шагов: ранний выход не должен оставить
        // игрока в пустоте с уже снесённым данжем
        let Some(cfg) = self.cfg.take() else { return };
        self.clear_dungeon();

        let seed = {
            let st = self.state.as_mut().unwrap();
            st.dungeon_seed = st.dungeon_seed.wrapping_add(1);
            st.dungeon_seed
        };
        // Дроп с убийств тоже зависит от сида — забег полностью воспроизводим
        // (раньше сессионный Rng стартовал с константы).
        self.rng = Rng::new(seed ^ 0x00D1_CED0);

        let plan: DungeonPlan = dungeon::generate(depth, seed, &mut self.cache, &cfg);

        let mut root = plan.root.clone();
        root.set_position(DUNGEON_OFFSET);
        self.base_mut().add_child(&root);
        self.dungeon_root = Some(root);
        self.dungeon_depth = depth;
        self.dungeon_name = plan.theme_name.clone();
        self.minimap_floor = plan.floor_map.clone();
        self.exit_portal = DUNGEON_OFFSET + plan.exit_portal;
        self.next_portal = DUNGEON_OFFSET + plan.next_portal;
        self.boss_alive = plan.enemies.iter().any(|e| e.is_boss);

        for es in &plan.enemies {
            self.spawn_enemy(&cfg, &es.kind, DUNGEON_OFFSET + es.pos, es.mult, es.is_boss, true);
        }
        for (kind, pos) in &plan.items {
            self.spawn_item(&cfg, kind, DUNGEON_OFFSET + *pos, true);
        }
        self.cfg = Some(cfg);
        for (t, n, pos) in &plan.ammo {
            self.spawn_ammo_pickup(*t, *n, DUNGEON_OFFSET + *pos, true);
        }
        for (w, pos) in &plan.weapons {
            self.spawn_weapon_pickup(*w, DUNGEON_OFFSET + *pos, true);
        }

        if let Some(ref p) = self.player {
            if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                pl.bind_mut().teleport(DUNGEON_OFFSET + plan.player_spawn + Vector3::new(0.0, 1.0, 0.0));
            }
        }
        self.loc = Loc::Dungeon;

        // квест при первом входе
        let add_quest = {
            let st = self.state.as_mut().unwrap();
            if !st.has("dungeon_quest") {
                st.flags.insert("dungeon_quest".into());
                st.quests.add("dungeon_heart", "Сердце данжа", "Убей стража на дне данжа.");
                true
            } else { false }
        };
        if add_quest {
            self.show_flash("Новый квест: «Сердце данжа»");
        }
        self.show_flash(&format!("«{}» — глубина {}", plan.theme_name, depth));
        self.build_minimap_texture();
        if let Some(ref mut bg) = self.minimap_bg  { bg.set_visible(true); }
        if let Some(ref mut mr) = self.minimap_rect { mr.set_visible(true); }
        if let Some(ref mut d)  = self.minimap_dot  { d.set_visible(true); }
        self.update_loc_label();
    }

    fn exit_dungeon(&mut self) {
        self.clear_dungeon();
        self.loc = Loc::World;
        if let Some(ref p) = self.player {
            if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                let gate = self.gate_pos;
                pl.bind_mut().teleport(gate + Vector3::new(0.0, 1.0, 3.5));
            }
        }
        if let Some(ref mut bg) = self.minimap_bg  { bg.set_visible(false); }
        if let Some(ref mut mr) = self.minimap_rect { mr.set_visible(false); }
        if let Some(ref mut d)  = self.minimap_dot  { d.set_visible(false); }
        self.minimap_floor.clear();
        self.show_flash("Пустоши Неонового Сердца");
        self.update_loc_label();
        self.auto_save();
    }

    fn clear_dungeon(&mut self) {
        if let Some(root) = self.dungeon_root.take() {
            root.free();
        }
        // убрать врагов и предметы данжа
        let pl = self.player.clone();
        let _ = pl;
        let mut i = 0;
        while i < self.enemies.len() {
            let in_d = self.enemies[i].get_position().x > 250.0;
            if in_d {
                let e = self.enemies.remove(i);
                e.free();
            } else { i += 1; }
        }
        let mut i = 0;
        while i < self.world_items.len() {
            if self.world_items[i].in_dungeon {
                let it = self.world_items.remove(i);
                it.node.free();
            } else { i += 1; }
        }
        for p in self.projectiles.drain(..) {
            p.node.free();
        }
        self.boss_alive = false;
    }
}

// ── Выбор класса ──────────────────────────────────────────────────────────────

impl Game3D {
    fn build_select_ui(&mut self) {
        let mut layer = CanvasLayer::new_alloc();
        layer.set_layer(5);
        self.base_mut().add_child(&layer);

        let mut panel = Panel::new_alloc();
        panel.set_position(Vector2::ZERO);
        panel.set_size(Vector2::new(HUD_W, HUD_H));
        panel.add_theme_stylebox_override("panel",
            &make_style(Color::from_rgba(0.02, 0.01, 0.04, 0.97), C_BORDER, 0));
        panel.set_visible(false);

        let mut title = Label::new_alloc();
        title.set_text("ВЫБЕРИ КЛАСС");
        title.set_position(Vector2::new(0.0, 90.0));
        title.set_size(Vector2::new(HUD_W, 70.0));
        title.set_horizontal_alignment(HorizontalAlignment::CENTER);
        title.add_theme_font_size_override("font_size", 52);
        title.add_theme_color_override("font_color", C_PINK);
        panel.add_child(&title);
        self.select_title = Some(title);

        let card_w = 480.0;
        let card_h = 560.0;
        let gap = 60.0;
        let total = card_w * 3.0 + gap * 2.0;
        let x0 = (HUD_W - total) * 0.5;
        let y0 = 240.0;

        for i in 0..3 {
            let mut card = Panel::new_alloc();
            card.set_position(Vector2::new(x0 + i as f32 * (card_w + gap), y0));
            card.set_size(Vector2::new(card_w, card_h));
            card.add_theme_stylebox_override("panel", &make_style(C_UI_BG, C_BORDER, 2));

            let mut key = Label::new_alloc();
            key.set_text(&format!("[ {} ]", i + 1));
            key.set_position(Vector2::new(0.0, 22.0));
            key.set_size(Vector2::new(card_w, 40.0));
            key.set_horizontal_alignment(HorizontalAlignment::CENTER);
            key.add_theme_font_size_override("font_size", 30);
            key.add_theme_color_override("font_color", C_GOLD);
            card.add_child(&key);

            let mut ct = Label::new_alloc();
            ct.set_position(Vector2::new(0.0, 80.0));
            ct.set_size(Vector2::new(card_w, 46.0));
            ct.set_horizontal_alignment(HorizontalAlignment::CENTER);
            ct.add_theme_font_size_override("font_size", 34);
            ct.add_theme_color_override("font_color", C_PINK);
            card.add_child(&ct);
            self.card_titles.push(ct);

            let mut cb = Label::new_alloc();
            cb.set_position(Vector2::new(28.0, 150.0));
            cb.set_size(Vector2::new(card_w - 56.0, card_h - 180.0));
            cb.add_theme_font_size_override("font_size", 19);
            cb.add_theme_color_override("font_color", C_MAIN);
            cb.set_autowrap_mode(godot::classes::text_server::AutowrapMode::WORD);
            card.add_child(&cb);
            self.card_bodies.push(cb);

            panel.add_child(&card);
        }

        let mut hint = Label::new_alloc();
        hint.set_text("Нажми 1, 2 или 3");
        hint.set_position(Vector2::new(0.0, HUD_H - 120.0));
        hint.set_size(Vector2::new(HUD_W, 40.0));
        hint.set_horizontal_alignment(HorizontalAlignment::CENTER);
        hint.add_theme_font_size_override("font_size", 22);
        hint.add_theme_color_override("font_color", C_DIM);
        panel.add_child(&hint);

        layer.add_child(&panel);
        self.select_panel = Some(panel);
    }

    fn open_class_select(&mut self) {
        self.mode = Mode::ClassSelect;
        self.freeze_player(true);
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
        if let Some(ref mut t) = self.select_title { t.set_text("ВЫБЕРИ КЛАСС"); }
        for (i, c) in classes().iter().enumerate() {
            self.fill_class_card(i, c);
        }
        if let Some(ref mut p) = self.select_panel { p.set_visible(true); }
    }

    fn fill_class_card(&mut self, i: usize, c: &ClassDef) {
        if let Some(t) = self.card_titles.get_mut(i) {
            t.set_text(&c.name_ru);
        }
        if let Some(b) = self.card_bodies.get_mut(i) {
            let weapons: Vec<&str> = c.start_weapons.iter()
                .map(|w| weapon_def(*w).name_ru.as_str()).collect();
            b.set_text(&format!(
                "Роль: {}\n\n{}\n\nHP: {:.0}\nСкорость: {:.1}\nОружие: {}\n\nСпеки:\n• {}\n• {}\n• {}",
                c.role_ru, c.desc_ru, c.base_hp, c.speed, weapons.join(", "),
                c.specs[0].name_ru, c.specs[1].name_ru, c.specs[2].name_ru,
            ));
        }
    }

    fn open_spec_select(&mut self, class_idx: usize) {
        self.mode = Mode::SpecSelect;
        self.class_pick = class_idx;
        let c = &classes()[class_idx];
        if let Some(ref mut t) = self.select_title {
            t.set_text(&format!("{} — ВЫБЕРИ СПЕЦИАЛИЗАЦИЮ", c.name_ru));
        }
        for i in 0..3 {
            let s = &c.specs[i];
            if let Some(t) = self.card_titles.get_mut(i) {
                t.set_text(&s.name_ru);
            }
            if let Some(b) = self.card_bodies.get_mut(i) {
                b.set_text(&format!("{}\n\n(Esc — назад к классам)", s.desc_ru));
            }
        }
    }

    fn process_class_select(&mut self) {
        let input = Input::singleton();
        for i in 0..3usize {
            let act = ["choice_1", "choice_2", "choice_3"][i];
            if input.is_action_just_pressed(act) {
                self.open_spec_select(i);
                return;
            }
        }
    }

    fn process_spec_select(&mut self) {
        let input = Input::singleton();
        if input.is_action_just_pressed("escape") {
            self.open_class_select();
            return;
        }
        for i in 0..3usize {
            let act = ["choice_1", "choice_2", "choice_3"][i];
            if input.is_action_just_pressed(act) {
                self.confirm_class(self.class_pick, i);
                return;
            }
        }
    }

    fn confirm_class(&mut self, class_idx: usize, spec_idx: usize) {
        {
            let st = self.state.as_mut().unwrap();
            st.class_idx = Some(class_idx);
            st.spec_idx = spec_idx;
            st.perk_points += 1;   // стартовое очко перка
        }
        self.apply_loadout(class_idx, spec_idx, true);
        if let Some(ref mut p) = self.select_panel { p.set_visible(false); }
        self.set_mode_explore();
        self.refresh_weapon_sheet();
        let c = &classes()[class_idx];
        self.show_flash(&format!("{} / {}. Вперёд!", c.name_ru, c.specs[spec_idx].name_ru));
        self.update_loc_label();
        self.auto_save();
    }

    /// Пересчитать статы из класса/спека/уровня. give_kit — выдать стартовый набор.
    fn apply_loadout(&mut self, class_idx: usize, spec_idx: usize, give_kit: bool) {
        let level = self.state.as_ref().map(|s| s.level).unwrap_or(1);
        self.loadout = compute_loadout(class_idx, spec_idx, level);
        let hearts = self.state.as_ref().map(|s| s.stat_hearts()).unwrap_or(0);
        self.loadout.max_hp += hearts as f32 * 15.0;

        // модификаторы перков и активных синергий
        let owned_perks = self.state.as_ref().map(|s| s.perks.clone()).unwrap_or_default();
        let mods = crate::perk::mods_for(&owned_perks);
        self.loadout.max_hp    += mods.max_hp_add;
        self.loadout.speed     *= mods.speed_mult;
        self.loadout.dmg_mult  += mods.dmg_add;
        self.loadout.cd_mult   *= mods.cd_mult;
        self.loadout.lifesteal += mods.lifesteal_add;
        self.loadout.ammo_mult += mods.ammo_add;
        self.loadout.max_hp = self.loadout.max_hp.max(40.0);

        let c = &classes()[class_idx];
        let s = &c.specs[spec_idx];

        if give_kit {
            self.arsenal = Arsenal::new();
            for w in &c.start_weapons {
                self.arsenal.give_weapon(*w);
            }
            if let Some(w) = s.extra_weapon {
                self.arsenal.give_weapon(w);
                if let Some((t, _)) = weapon_def(w).ammo {
                    self.arsenal.add_ammo(t, t.pack_size() * 2, self.loadout.ammo_mult);
                }
            }
            for (t, n) in &c.start_ammo {
                self.arsenal.add_ammo(*t, *n, self.loadout.ammo_mult);
            }
            self.arsenal.current = c.start_weapons[0];
        }

        let (max_hp, speed) = (self.loadout.max_hp, self.loadout.speed);
        if let Some(ref p) = self.player {
            if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                let mut b = pl.bind_mut();
                b.max_hp = max_hp;
                if give_kit { b.hp = max_hp; }
                else { b.hp = b.hp.min(max_hp); }
                b.speed = speed;
            }
        }
    }

    fn freeze_player(&mut self, frozen: bool) {
        if let Some(ref p) = self.player {
            if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                pl.bind_mut().frozen = frozen;
            }
        }
    }

    fn set_mode_explore(&mut self) {
        self.mode = Mode::Explore;
        self.freeze_player(false);
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::CAPTURED);
    }
}

// ── HUD ───────────────────────────────────────────────────────────────────────

/// Полноэкранная постобработка: виньетка + лёгкая хроматическая аберрация +
/// тонкие сканлайны. Canvas-шейдер — работает и в GL Compatibility.
const POST_FX_SHADER: &str = r#"
shader_type canvas_item;
uniform sampler2D screen_tex : hint_screen_texture, filter_linear;
uniform float vignette : hint_range(0.0, 1.0) = 0.34;
uniform float aberration = 1.4;
uniform float scanline = 0.045;

void fragment() {
    vec2 uv = SCREEN_UV;
    vec2 off = (uv - 0.5) * aberration * 0.0018;
    float r = texture(screen_tex, uv + off).r;
    float g = texture(screen_tex, uv).g;
    float b = texture(screen_tex, uv - off).b;
    vec3 col = vec3(r, g, b);
    float d = length(uv - 0.5);
    col *= 1.0 - vignette * smoothstep(0.32, 0.86, d);
    col *= 1.0 - scanline * (0.5 + 0.5 * sin(uv.y * 620.0));
    COLOR = vec4(col, 1.0);
}
"#;

impl Game3D {
    /// Слой постобработки поверх 3D, но ПОД HUD (layer -1 < HUD default 1... нет:
    /// экранная текстура читается до HUD, поэтому вешаем на слой 0 ниже HUD-слоя 1).
    fn build_post_fx(&mut self) {
        use godot::classes::{ColorRect, Shader, ShaderMaterial};
        use godot::classes::control::MouseFilter;

        let mut shader = Shader::new_gd();
        shader.set_code(POST_FX_SHADER);
        let mut mat = ShaderMaterial::new_gd();
        mat.set_shader(&shader);

        let mut rect = ColorRect::new_alloc();
        rect.set_anchors_preset(godot::classes::control::LayoutPreset::FULL_RECT);
        rect.set_material(&mat);
        rect.set_mouse_filter(MouseFilter::IGNORE);

        let mut layer = CanvasLayer::new_alloc();
        layer.set_layer(0); // под HUD (1), поверх 3D-вьюпорта
        layer.add_child(&rect);
        self.base_mut().add_child(&layer);
    }

    fn build_hud(&mut self, lang: &str) {
        self.build_post_fx();

        let mut layer = CanvasLayer::new_alloc();
        self.base_mut().add_child(&layer);

        // урон-флэш
        let mut df = Panel::new_alloc();
        df.set_position(Vector2::ZERO);
        df.set_size(Vector2::new(HUD_W, HUD_H));
        df.add_theme_stylebox_override("panel",
            &make_style(Color::from_rgba(0.8, 0.0, 0.0, 0.0), Color::TRANSPARENT_BLACK, 0));
        df.set_visible(false);
        layer.add_child(&df);
        self.damage_flash = Some(df);

        // FP-оружие (низ по центру)
        {
            let mut wr = TextureRect::new_alloc();
            wr.set_position(Vector2::new(HUD_W * 0.5 - 260.0, HUD_H - 560.0));
            wr.set_size(Vector2::new(520.0, 560.0));
            wr.set_expand_mode(godot::classes::texture_rect::ExpandMode::IGNORE_SIZE);
            wr.set_stretch_mode(godot::classes::texture_rect::StretchMode::SCALE);
            wr.set_texture_filter(godot::classes::canvas_item::TextureFilter::NEAREST);
            wr.set_visible(false);
            layer.add_child(&wr);
            self.weapon_rect = Some(wr);
        }

        // прицел
        let mut cross = Label::new_alloc();
        cross.set_text("+");
        cross.set_position(Vector2::new(HUD_W * 0.5 - 8.0, HUD_H * 0.5 - 12.0));
        cross.set_size(Vector2::new(16.0, 24.0));
        cross.set_horizontal_alignment(HorizontalAlignment::CENTER);
        cross.add_theme_font_size_override("font_size", 20);
        cross.add_theme_color_override("font_color", C_MAIN);
        layer.add_child(&cross);
        self.crosshair = Some(cross);

        // таргетинг
        let mut tgt = Label::new_alloc();
        tgt.set_position(Vector2::new(HUD_W * 0.5 - 200.0, HUD_H * 0.5 - 46.0));
        tgt.set_size(Vector2::new(400.0, 24.0));
        tgt.set_horizontal_alignment(HorizontalAlignment::CENTER);
        tgt.add_theme_font_size_override("font_size", 13);
        tgt.add_theme_color_override("font_color", C_RED);
        tgt.set_visible(false);
        layer.add_child(&tgt);
        self.targeting_label = Some(tgt);

        // компас
        let mut cmp = Label::new_alloc();
        cmp.set_text("N");
        cmp.set_position(Vector2::new(HUD_W * 0.5 - 40.0, 10.0));
        cmp.set_size(Vector2::new(80.0, 30.0));
        cmp.set_horizontal_alignment(HorizontalAlignment::CENTER);
        cmp.add_theme_font_size_override("font_size", 18);
        cmp.add_theme_color_override("font_color", C_CYAN);
        layer.add_child(&cmp);
        self.compass_label = Some(cmp);

        // локация
        let mut ll = Label::new_alloc();
        ll.set_position(Vector2::new(HUD_W * 0.5 - 300.0, 40.0));
        ll.set_size(Vector2::new(600.0, 26.0));
        ll.set_horizontal_alignment(HorizontalAlignment::CENTER);
        ll.add_theme_font_size_override("font_size", 14);
        ll.add_theme_color_override("font_color", C_DIM);
        layer.add_child(&ll);
        self.loc_label = Some(ll);

        // Иконка HP (сердце)
        if let Some(tex) = self.cache.get("res://assets/ui/ui_heart.png") {
            let mut ic = TextureRect::new_alloc();
            ic.set_texture(&tex);
            ic.set_position(Vector2::new(24.0, HUD_H - 100.0));
            ic.set_size(Vector2::new(34.0, 34.0));
            ic.set_expand_mode(godot::classes::texture_rect::ExpandMode::IGNORE_SIZE);
            ic.set_stretch_mode(godot::classes::texture_rect::StretchMode::SCALE);
            ic.set_texture_filter(godot::classes::canvas_item::TextureFilter::NEAREST);
            layer.add_child(&ic);
        }

        // Иконка боезапаса
        if let Some(tex) = self.cache.get("res://assets/ui/ui_ammo.png") {
            let mut ic = TextureRect::new_alloc();
            ic.set_texture(&tex);
            ic.set_position(Vector2::new(HUD_W - 404.0, HUD_H - 66.0));
            ic.set_size(Vector2::new(36.0, 36.0));
            ic.set_expand_mode(godot::classes::texture_rect::ExpandMode::IGNORE_SIZE);
            ic.set_stretch_mode(godot::classes::texture_rect::StretchMode::SCALE);
            ic.set_texture_filter(godot::classes::canvas_item::TextureFilter::NEAREST);
            layer.add_child(&ic);
        }

        // HP бар
        let mut hp_bg = Panel::new_alloc();
        hp_bg.set_position(Vector2::new(24.0, HUD_H - 58.0));
        hp_bg.set_size(Vector2::new(222.0, 26.0));
        hp_bg.add_theme_stylebox_override("panel", &make_style(
            Color::from_rgba(0.08, 0.01, 0.01, 0.92), Color::from_rgba(0.35, 0.08, 0.08, 1.0), 1));
        layer.add_child(&hp_bg);

        let mut hp_fg = Panel::new_alloc();
        hp_fg.set_position(Vector2::new(26.0, HUD_H - 56.0));
        hp_fg.set_size(Vector2::new(218.0, 22.0));
        hp_fg.add_theme_stylebox_override("panel", &make_style(C_RED, Color::TRANSPARENT_BLACK, 0));
        layer.add_child(&hp_fg);
        self.hp_bar_fg = Some(hp_fg);

        let mut hp_lbl = Label::new_alloc();
        hp_lbl.set_position(Vector2::new(24.0, HUD_H - 84.0));
        hp_lbl.set_size(Vector2::new(220.0, 24.0));
        hp_lbl.add_theme_font_size_override("font_size", 14);
        hp_lbl.add_theme_color_override("font_color", C_RED);
        layer.add_child(&hp_lbl);
        self.hp_label = Some(hp_lbl);

        // XP бар
        let mut xp_bg = Panel::new_alloc();
        xp_bg.set_position(Vector2::new(24.0, HUD_H - 28.0));
        xp_bg.set_size(Vector2::new(222.0, 10.0));
        xp_bg.add_theme_stylebox_override("panel", &make_style(
            Color::from_rgba(0.05, 0.03, 0.10, 0.92), Color::from_rgba(0.25, 0.15, 0.4, 1.0), 1));
        layer.add_child(&xp_bg);

        let mut xp_fg = Panel::new_alloc();
        xp_fg.set_position(Vector2::new(25.0, HUD_H - 27.0));
        xp_fg.set_size(Vector2::new(0.0, 8.0));
        xp_fg.add_theme_stylebox_override("panel", &make_style(C_XP, Color::TRANSPARENT_BLACK, 0));
        layer.add_child(&xp_fg);
        self.xp_bar_fg = Some(xp_fg);

        let mut xp_lbl = Label::new_alloc();
        xp_lbl.set_position(Vector2::new(252.0, HUD_H - 34.0));
        xp_lbl.set_size(Vector2::new(260.0, 22.0));
        xp_lbl.add_theme_font_size_override("font_size", 13);
        xp_lbl.add_theme_color_override("font_color", C_XP);
        layer.add_child(&xp_lbl);
        self.xp_label = Some(xp_lbl);

        // Патроны и оружие (низ справа)
        let mut am = Label::new_alloc();
        am.set_position(Vector2::new(HUD_W - 360.0, HUD_H - 64.0));
        am.set_size(Vector2::new(336.0, 34.0));
        am.set_horizontal_alignment(HorizontalAlignment::RIGHT);
        am.add_theme_font_size_override("font_size", 26);
        am.add_theme_color_override("font_color", C_GOLD);
        layer.add_child(&am);
        self.ammo_label = Some(am);

        let mut wn = Label::new_alloc();
        wn.set_position(Vector2::new(HUD_W - 360.0, HUD_H - 92.0));
        wn.set_size(Vector2::new(336.0, 24.0));
        wn.set_horizontal_alignment(HorizontalAlignment::RIGHT);
        wn.add_theme_font_size_override("font_size", 14);
        wn.add_theme_color_override("font_color", C_DIM);
        layer.add_child(&wn);
        self.weapon_label = Some(wn);

        // подсказка
        let mut hint = Label::new_alloc();
        hint.set_position(Vector2::new(HUD_W * 0.5 - 280.0, HUD_H - 130.0));
        hint.set_size(Vector2::new(560.0, 28.0));
        hint.set_horizontal_alignment(HorizontalAlignment::CENTER);
        hint.add_theme_font_size_override("font_size", 16);
        hint.add_theme_color_override("font_color", C_GOLD);
        hint.set_visible(false);
        layer.add_child(&hint);
        self.hint_label = Some(hint);

        // инвентарь (строка)
        let mut inv = Label::new_alloc();
        inv.set_position(Vector2::new(HUD_W - 460.0, 10.0));
        inv.set_size(Vector2::new(448.0, 24.0));
        inv.set_horizontal_alignment(HorizontalAlignment::RIGHT);
        inv.add_theme_font_size_override("font_size", 13);
        inv.add_theme_color_override("font_color", C_DIM);
        layer.add_child(&inv);
        self.inv_label = Some(inv);

        // квесты
        let mut ql = Label::new_alloc();
        ql.set_position(Vector2::new(24.0, 44.0));
        ql.set_size(Vector2::new(360.0, 150.0));
        ql.add_theme_font_size_override("font_size", 13);
        ql.add_theme_color_override("font_color", C_DIM);
        ql.set_autowrap_mode(godot::classes::text_server::AutowrapMode::WORD);
        layer.add_child(&ql);
        self.quest_label = Some(ql);

        // флэш
        let mut flash = Label::new_alloc();
        flash.set_position(Vector2::new(HUD_W * 0.5 - 300.0, HUD_H * 0.5 - 110.0));
        flash.set_size(Vector2::new(600.0, 34.0));
        flash.set_horizontal_alignment(HorizontalAlignment::CENTER);
        flash.add_theme_font_size_override("font_size", 18);
        flash.add_theme_color_override("font_color", C_GOLD);
        flash.set_visible(false);
        layer.add_child(&flash);
        self.flash_label = Some(flash);

        // экран инвентаря
        {
            let pw = 720.0;
            let ph = 520.0;
            let mut ip = Panel::new_alloc();
            ip.set_position(Vector2::new((HUD_W - pw) * 0.5, (HUD_H - ph) * 0.5));
            ip.set_size(Vector2::new(pw, ph));
            ip.add_theme_stylebox_override("panel", &make_style(C_UI_BG, C_BORDER, 2));
            ip.set_visible(false);

            let mut title = Label::new_alloc();
            title.set_text(t("inv_title", lang));
            title.set_position(Vector2::new(24.0, 16.0));
            title.set_size(Vector2::new(pw - 48.0, 32.0));
            title.add_theme_font_size_override("font_size", 22);
            title.add_theme_color_override("font_color", C_PINK);
            ip.add_child(&title);

            let mut il = Label::new_alloc();
            il.set_position(Vector2::new(24.0, 60.0));
            il.set_size(Vector2::new(pw - 48.0, ph - 110.0));
            il.add_theme_font_size_override("font_size", 15);
            il.add_theme_color_override("font_color", C_MAIN);
            il.set_autowrap_mode(godot::classes::text_server::AutowrapMode::WORD);
            ip.add_child(&il);

            let mut hint_i = Label::new_alloc();
            hint_i.set_text(t("inv_close", lang));
            hint_i.set_position(Vector2::new(24.0, ph - 42.0));
            hint_i.set_size(Vector2::new(pw - 48.0, 28.0));
            hint_i.add_theme_font_size_override("font_size", 13);
            hint_i.add_theme_color_override("font_color", C_DIM);
            ip.add_child(&hint_i);

            layer.add_child(&ip);
            self.inv_list = Some(il);
            self.inv_panel = Some(ip);
        }

        // экран перков
        {
            let pw = 900.0;
            let ph = 640.0;
            let mut pp = Panel::new_alloc();
            pp.set_position(Vector2::new((HUD_W - pw) * 0.5, (HUD_H - ph) * 0.5));
            pp.set_size(Vector2::new(pw, ph));
            pp.add_theme_stylebox_override("panel", &make_style(C_UI_BG, C_BORDER, 2));
            pp.set_visible(false);

            let mut title = Label::new_alloc();
            title.set_text("ДЕРЕВО ПЕРКОВ");
            title.set_position(Vector2::new(24.0, 14.0));
            title.set_size(Vector2::new(pw - 48.0, 32.0));
            title.add_theme_font_size_override("font_size", 22);
            title.add_theme_color_override("font_color", C_XP);
            pp.add_child(&title);

            let mut pl = Label::new_alloc();
            pl.set_position(Vector2::new(24.0, 54.0));
            pl.set_size(Vector2::new(pw - 48.0, ph - 100.0));
            pl.add_theme_font_size_override("font_size", 14);
            pl.add_theme_color_override("font_color", C_MAIN);
            pl.set_autowrap_mode(godot::classes::text_server::AutowrapMode::WORD);
            pp.add_child(&pl);

            let mut hint_p = Label::new_alloc();
            hint_p.set_text("[ 1–8 ] купить перк   ·   [ P / Esc ] закрыть");
            hint_p.set_position(Vector2::new(24.0, ph - 40.0));
            hint_p.set_size(Vector2::new(pw - 48.0, 28.0));
            hint_p.add_theme_font_size_override("font_size", 13);
            hint_p.add_theme_color_override("font_color", C_DIM);
            pp.add_child(&hint_p);

            layer.add_child(&pp);
            self.perk_list = Some(pl);
            self.perk_panel = Some(pp);
        }

        // диалоговая панель
        {
            let panel_y = HUD_H * 0.60;
            let panel_h = HUD_H * 0.40;
            let mut panel = Panel::new_alloc();
            panel.set_position(Vector2::new(0.0, panel_y));
            panel.set_size(Vector2::new(HUD_W, panel_h));
            panel.add_theme_stylebox_override("panel", &make_style(C_UI_BG, C_BORDER, 2));
            panel.set_visible(false);

            let mut speaker = Label::new_alloc();
            speaker.set_position(Vector2::new(24.0, 16.0));
            speaker.set_size(Vector2::new(500.0, 30.0));
            speaker.add_theme_font_size_override("font_size", 20);
            speaker.add_theme_color_override("font_color", C_PINK);
            panel.add_child(&speaker);

            let mut text = Label::new_alloc();
            text.set_position(Vector2::new(24.0, 54.0));
            text.set_size(Vector2::new(HUD_W - 48.0, 145.0));
            text.add_theme_font_size_override("font_size", 16);
            text.add_theme_color_override("font_color", C_MAIN);
            text.set_autowrap_mode(godot::classes::text_server::AutowrapMode::WORD);
            panel.add_child(&text);

            let mut vbox = VBoxContainer::new_alloc();
            vbox.set_position(Vector2::new(24.0, 205.0));
            vbox.set_size(Vector2::new(HUD_W - 48.0, 170.0));
            panel.add_child(&vbox);

            let choice_lbls: [_; 4] = std::array::from_fn(|i| {
                let mut lbl = Label::new_alloc();
                lbl.set_text(&format!("{}.", i + 1));
                lbl.add_theme_font_size_override("font_size", 15);
                lbl.add_theme_color_override("font_color", C_DIM);
                lbl.set_visible(false);
                vbox.add_child(&lbl);
                lbl
            });
            let [c0, c1, c2, c3] = choice_lbls;

            layer.add_child(&panel);
            self.dlg_panel = Some(panel);
            self.dlg_speaker = Some(speaker);
            self.dlg_text = Some(text);
            self.choice_box = Some(vbox);
            self.cl0 = Some(c0); self.cl1 = Some(c1);
            self.cl2 = Some(c2); self.cl3 = Some(c3);
        }

        // миникарта данжа (правый верхний угол)
        {
            const MAP_SZ: f32 = 176.0;
            const MAP_X: f32 = HUD_W - MAP_SZ - 16.0;
            const MAP_Y: f32 = 40.0;

            let mut bg = Panel::new_alloc();
            bg.set_position(Vector2::new(MAP_X - 4.0, MAP_Y - 4.0));
            bg.set_size(Vector2::new(MAP_SZ + 8.0, MAP_SZ + 8.0));
            bg.add_theme_stylebox_override("panel",
                &make_style(Color::from_rgba(0.02, 0.01, 0.05, 0.88), C_BORDER, 1));
            bg.set_visible(false);
            layer.add_child(&bg);
            self.minimap_bg = Some(bg);

            let mut mr = TextureRect::new_alloc();
            mr.set_position(Vector2::new(MAP_X, MAP_Y));
            mr.set_size(Vector2::new(MAP_SZ, MAP_SZ));
            mr.set_expand_mode(godot::classes::texture_rect::ExpandMode::IGNORE_SIZE);
            mr.set_stretch_mode(godot::classes::texture_rect::StretchMode::SCALE);
            mr.set_texture_filter(godot::classes::canvas_item::TextureFilter::NEAREST);
            mr.set_visible(false);
            layer.add_child(&mr);
            self.minimap_rect = Some(mr);

            // точка игрока
            let mut dot = Panel::new_alloc();
            dot.set_size(Vector2::new(6.0, 6.0));
            dot.add_theme_stylebox_override("panel",
                &make_style(C_PINK, Color::TRANSPARENT_BLACK, 0));
            dot.set_visible(false);
            layer.add_child(&dot);
            self.minimap_dot = Some(dot);
        }

        // экран смерти
        {
            let mut dp = Panel::new_alloc();
            dp.set_position(Vector2::ZERO);
            dp.set_size(Vector2::new(HUD_W, HUD_H));
            dp.add_theme_stylebox_override("panel",
                &make_style(Color::from_rgba(0.3, 0.0, 0.0, 0.88), Color::TRANSPARENT_BLACK, 0));
            dp.set_visible(false);

            let mut lbl = Label::new_alloc();
            lbl.set_text(t("msg_died", lang));
            lbl.set_position(Vector2::new(0.0, HUD_H * 0.4));
            lbl.set_size(Vector2::new(HUD_W, 60.0));
            lbl.set_horizontal_alignment(HorizontalAlignment::CENTER);
            lbl.add_theme_font_size_override("font_size", 56);
            lbl.add_theme_color_override("font_color", C_RED);
            dp.add_child(&lbl);

            let mut sub = Label::new_alloc();
            sub.set_text("E — вернуться в хаб (−25% золота)");
            sub.set_position(Vector2::new(0.0, HUD_H * 0.4 + 70.0));
            sub.set_size(Vector2::new(HUD_W, 30.0));
            sub.set_horizontal_alignment(HorizontalAlignment::CENTER);
            sub.add_theme_font_size_override("font_size", 18);
            sub.add_theme_color_override("font_color", C_DIM);
            dp.add_child(&sub);

            layer.add_child(&dp);
            self.dead_panel = Some(dp);
        }

        // экран паузы
        {
            let mut pp = Panel::new_alloc();
            pp.set_position(Vector2::ZERO);
            pp.set_size(Vector2::new(HUD_W, HUD_H));
            pp.add_theme_stylebox_override("panel",
                &make_style(Color::from_rgba(0.02, 0.01, 0.05, 0.82), Color::TRANSPARENT_BLACK, 0));
            pp.set_visible(false);

            let mut title = Label::new_alloc();
            title.set_text("ПАУЗА");
            title.set_position(Vector2::new(0.0, HUD_H * 0.36));
            title.set_size(Vector2::new(HUD_W, 60.0));
            title.set_horizontal_alignment(HorizontalAlignment::CENTER);
            title.add_theme_font_size_override("font_size", 52);
            title.add_theme_color_override("font_color", C_PINK);
            pp.add_child(&title);

            let mut lines = Label::new_alloc();
            lines.set_text("[ 1 / Esc ]  Продолжить\n\n[ 2 ]  Выйти в главное меню");
            lines.set_position(Vector2::new(0.0, HUD_H * 0.36 + 90.0));
            lines.set_size(Vector2::new(HUD_W, 120.0));
            lines.set_horizontal_alignment(HorizontalAlignment::CENTER);
            lines.add_theme_font_size_override("font_size", 22);
            lines.add_theme_color_override("font_color", C_MAIN);
            pp.add_child(&lines);

            let mut note = Label::new_alloc();
            note.set_text("Прогресс сохраняется автоматически");
            note.set_position(Vector2::new(0.0, HUD_H * 0.36 + 230.0));
            note.set_size(Vector2::new(HUD_W, 30.0));
            note.set_horizontal_alignment(HorizontalAlignment::CENTER);
            note.add_theme_font_size_override("font_size", 14);
            note.add_theme_color_override("font_color", C_DIM);
            pp.add_child(&note);

            layer.add_child(&pp);
            self.pause_panel = Some(pp);
        }
    }

    /// Обновить AtlasTexture под текущее оружие.
    fn refresh_weapon_sheet(&mut self) {
        let def = weapon_def(self.arsenal.current);
        let Some(tex) = self.cache.get(&def.sheet) else { return };
        let mut at = AtlasTexture::new_gd();
        at.set_atlas(&tex);
        at.set_region(Rect2::new(Vector2::ZERO, Vector2::new(FRAME_W, def.frame_h)));
        if let Some(ref mut wr) = self.weapon_rect {
            wr.set_texture(&at);
            // масштаб: высота на экране пропорциональна высоте кадра
            let k = 6.0;
            let w = FRAME_W * k;
            let h = def.frame_h * k;
            wr.set_size(Vector2::new(w, h));
            wr.set_position(Vector2::new(HUD_W * 0.5 - w * 0.5, HUD_H - h));
            wr.set_visible(true);
        }
        self.weapon_atlas = Some(at);
        self.weapon_anim = WeaponAnim::Switch(0.22);
        self.set_weapon_frame(def.idle_frames[0]);
        if let Some(ref mut wl) = self.weapon_label {
            wl.set_text(&format!("[{}] {}  ({})", def.id.slot() + 1, def.name_ru, def.dmg_type.name_ru()));
        }
    }

    fn set_weapon_frame(&mut self, frame: usize) {
        let def = weapon_def(self.arsenal.current);
        if let Some(ref mut at) = self.weapon_atlas {
            at.set_region(Rect2::new(
                Vector2::new(frame as f32 * FRAME_W, 0.0),
                Vector2::new(FRAME_W, def.frame_h),
            ));
        }
    }

    fn tick_weapon_anim(&mut self, dt: f32) {
        if self.mode != Mode::Explore && self.mode != Mode::Dialogue { return; }
        let def = weapon_def(self.arsenal.current);
        self.anim_timer += dt;

        match self.weapon_anim {
            WeaponAnim::Fire(i) => {
                let frame_time = 1.0 / def.fire_fps;
                if self.anim_timer >= frame_time {
                    self.anim_timer = 0.0;
                    let next = i + 1;
                    if next < def.fire_frames.len() {
                        self.weapon_anim = WeaponAnim::Fire(next);
                        self.set_weapon_frame(def.fire_frames[next]);
                    } else {
                        self.weapon_anim = WeaponAnim::Idle;
                        self.set_weapon_frame(def.idle_frames[0]);
                    }
                }
            }
            WeaponAnim::Switch(t) => {
                let t2 = t - dt;
                if t2 <= 0.0 {
                    self.weapon_anim = WeaponAnim::Idle;
                } else {
                    self.weapon_anim = WeaponAnim::Switch(t2);
                }
            }
            WeaponAnim::Idle => {
                if def.idle_frames.len() > 1 && self.anim_timer >= 0.16 {
                    self.anim_timer = 0.0;
                    self.idle_frame = (self.idle_frame + 1) % def.idle_frames.len();
                    self.set_weapon_frame(def.idle_frames[self.idle_frame]);
                }
            }
        }

        // позиция: бо́б при ходьбе + провал при смене
        let moving = self.player.as_ref()
            .and_then(|p| p.clone().try_cast::<Player>().ok())
            .map(|p| p.bind().moving)
            .unwrap_or(false);
        let k = 6.0;
        let h = def.frame_h * k;
        let w = FRAME_W * k;
        let bob = if moving { (self.game_time * 9.0).sin() * 10.0 } else { (self.game_time * 2.0).sin() * 3.0 };
        let dip = match self.weapon_anim {
            WeaponAnim::Switch(t) => (t / 0.22) * 240.0,
            _ => 0.0,
        };
        if let Some(ref mut wr) = self.weapon_rect {
            wr.set_position(Vector2::new(
                HUD_W * 0.5 - w * 0.5 + if moving { (self.game_time * 4.5).sin() * 14.0 } else { 0.0 },
                HUD_H - h + 10.0 + bob + dip,
            ));
        }
    }

    fn update_loc_label(&mut self) {
        let text = match self.loc {
            Loc::World => self.world_name.clone(),
            Loc::Dungeon => format!("{} — глубина {}", self.dungeon_name, self.dungeon_depth),
        };
        if let Some(ref mut l) = self.loc_label { l.set_text(&text); }
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

// ── Игровой процесс ───────────────────────────────────────────────────────────

impl Game3D {
    fn process_explore(&mut self) {
        let lang = self.settings.lang.clone();
        self.update_nearby();
        self.update_inv_label();
        self.update_quest_label(&lang);

        let input = Input::singleton();

        // Смерть проверяется ДО обработки ввода: Esc в кадр смерти не должен
        // открыть паузу поверх мёртвого игрока (и дать сохраниться с hp=0).
        let player_dead = self.player.as_ref()
            .and_then(|p| p.clone().try_cast::<Player>().ok())
            .map(|pl| pl.bind().dead)
            .unwrap_or(false);
        if player_dead {
            self.mode = Mode::Dead;
            if let Some(ref mut dp) = self.dead_panel { dp.set_visible(true); }
            Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
            // Сейв НЕ стирается: смерть = возврат в хаб со штрафом (см. DESIGN_PLAN §13).
            return;
        }

        if input.is_action_just_pressed("escape") {
            self.open_pause();
            return;
        }

        if input.is_action_just_pressed("interact") {
            if let Some(kind) = self.near_portal {
                self.use_portal(kind);
            } else if let Some(idx) = self.near_item {
                self.pick_up_item(idx);
            } else if let Some(idx) = self.near_npc {
                self.start_dialogue(idx);
            }
        }

        // смена оружия: клавиши 1-8
        for slot in 0..8usize {
            let act = format!("weapon_{}", slot + 1);
            if input.is_action_just_pressed(&act) {
                let w = WeaponId::from_slot(slot);
                if self.arsenal.has(w) && self.arsenal.current != w {
                    self.arsenal.current = w;
                    self.refresh_weapon_sheet();
                }
            }
        }
        if input.is_action_just_pressed("weapon_next") {
            let w = self.arsenal.cycle(1);
            if w != self.arsenal.current {
                self.arsenal.current = w;
                self.refresh_weapon_sheet();
            }
        }
        if input.is_action_just_pressed("weapon_prev") {
            let w = self.arsenal.cycle(-1);
            if w != self.arsenal.current {
                self.arsenal.current = w;
                self.refresh_weapon_sheet();
            }
        }

        // стрельба
        let has_any_weapon = self.arsenal.owned.iter().any(|o| *o);
        if has_any_weapon {
            let def = weapon_def(self.arsenal.current);
            let want_fire = if def.auto {
                input.is_action_pressed("shoot")
            } else {
                input.is_action_just_pressed("shoot")
            };
            if want_fire && self.shoot_cd <= 0.0 {
                self.try_fire();
            }
        }

        // быстрое лечение
        if input.is_action_just_pressed("use_med") {
            self.use_first_consumable();
        }

        if input.is_action_just_pressed("inventory") {
            self.open_inventory();
        }

        if input.is_action_just_pressed("perks") {
            self.open_perks();
        }

    }

    fn process_dead(&mut self) {
        let input = Input::singleton();
        if input.is_action_just_pressed("interact") {
            self.respawn_at_hub();
        }
    }

    // ── Пауза ────────────────────────────────────────────────────────────────

    fn open_pause(&mut self) {
        self.mode = Mode::Paused;
        self.freeze_player(true);
        if let Some(ref mut p) = self.pause_panel { p.set_visible(true); }
        if let Some(ref mut lbl) = self.hint_label { lbl.set_visible(false); }
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
    }

    fn close_pause(&mut self) {
        if let Some(ref mut p) = self.pause_panel { p.set_visible(false); }
        self.set_mode_explore();
    }

    fn process_paused(&mut self) {
        let input = Input::singleton();
        if input.is_action_just_pressed("escape") || input.is_action_just_pressed("choice_1") {
            self.close_pause();
            return;
        }
        if input.is_action_just_pressed("choice_2") {
            self.auto_save();
            self.base().get_tree().change_scene_to_file("res://main_menu.tscn");
        }
    }

    /// Смерть → возврат в хаб: −25 % золота, данж сгорает, сейв сохраняется.
    fn respawn_at_hub(&mut self) {
        let lost = {
            let st = self.state.as_mut().unwrap();
            let lost = st.gold / 4;
            st.gold -= lost;
            lost
        };
        if self.loc == Loc::Dungeon {
            self.clear_dungeon();
            self.loc = Loc::World;
        }
        if let Some(ref p) = self.player {
            if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                let max = pl.bind().max_hp;
                {
                    let mut b = pl.bind_mut();
                    b.dead = false;
                    b.hp = max;
                }
                pl.bind_mut().teleport(Vector3::new(0.0, 1.1, 10.0));
            }
        }
        self.mode = Mode::Explore;
        if let Some(ref mut dp) = self.dead_panel { dp.set_visible(false); }
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::CAPTURED);
        self.update_loc_label();
        self.show_flash(&format!("Ты очнулся в хабе. Потеряно золота: {}.", lost));
        self.auto_save();
    }

    fn process_dialogue(&mut self) {
        if let Some(ref mut p) = self.player {
            let mut vel = p.get_velocity();
            vel.x = 0.0;
            vel.z = 0.0;
            p.set_velocity(vel);
        }
        let input = Input::singleton();
        if !self.at_choices {
            if input.is_action_just_pressed("interact") { self.advance_dialogue(); }
        } else {
            if input.is_action_just_pressed("choice_1") { self.select_choice(0); }
            if input.is_action_just_pressed("choice_2") { self.select_choice(1); }
            if input.is_action_just_pressed("choice_3") { self.select_choice(2); }
            if input.is_action_just_pressed("choice_4") { self.select_choice(3); }
        }
    }

    fn process_inventory(&mut self) {
        if Input::singleton().is_action_just_pressed("inventory")
            || Input::singleton().is_action_just_pressed("escape") {
            self.close_inventory();
        }
        if Input::singleton().is_action_just_pressed("interact") {
            self.use_first_consumable();
            self.refresh_inventory_ui();
        }
    }

    // ── Поблизости ───────────────────────────────────────────────────────────

    fn update_nearby(&mut self) {
        let lang = self.settings.lang.clone();
        let player_pos = match self.player.as_ref() {
            Some(p) => p.get_global_position(),
            None => { self.near_npc = None; return; }
        };

        // порталы
        self.near_portal = None;
        match self.loc {
            Loc::World => {
                if (player_pos - self.gate_pos).length() < PORTAL_R {
                    self.near_portal = Some(PortalKind::EnterDungeon);
                }
            }
            Loc::Dungeon => {
                if (player_pos - self.exit_portal).length() < PORTAL_R {
                    self.near_portal = Some(PortalKind::ExitDungeon);
                } else if (player_pos - self.next_portal).length() < PORTAL_R {
                    self.near_portal = Some(PortalKind::DeeperDungeon);
                }
            }
        }

        let mut near_npc: Option<usize> = None;
        if self.loc == Loc::World {
            let mut best_n = INTERACT_R;
            for (i, sp) in self.npc_sprites.iter().enumerate() {
                let d = (player_pos - sp.get_global_position()).length();
                if d < best_n { best_n = d; near_npc = Some(i); }
            }
        }
        self.near_npc = near_npc;

        let mut near_item: Option<usize> = None;
        let mut best_i = PICKUP_R;
        for (i, wi) in self.world_items.iter().enumerate() {
            let d = (player_pos - wi.node.get_global_position()).length();
            if d < best_i { best_i = d; near_item = Some(i); }
        }
        self.near_item = near_item;

        // ближайший враг для таргет-инфо
        let mut near_enemy: Option<usize> = None;
        let mut best_e = 24.0;
        for (i, e) in self.enemies.iter().enumerate() {
            if e.bind().alive {
                let d = (player_pos - e.get_global_position()).length();
                if d < best_e { best_e = d; near_enemy = Some(i); }
            }
        }
        self.near_enemy = near_enemy;

        let hint_text = if let Some(kind) = self.near_portal {
            match kind {
                PortalKind::EnterDungeon => {
                    let depth = self.state.as_ref().map(|s| s.dungeons_cleared + 1).unwrap_or(1);
                    format!("[E] Войти в данж (глубина {})", depth)
                }
                PortalKind::ExitDungeon => "[E] Вернуться в мир".to_string(),
                PortalKind::DeeperDungeon => {
                    if self.boss_alive {
                        "Портал запечатан — убей стража данжа".to_string()
                    } else {
                        format!("[E] Спуститься глубже (глубина {})", self.dungeon_depth + 1)
                    }
                }
            }
        } else if let Some(idx) = self.near_item {
            format!("{}: {}", t("hud_pickup", &lang), self.world_items[idx].name)
        } else if let Some(idx) = self.near_npc {
            format!("{} {}", t("hud_interact", &lang),
                    self.npcs.get(idx).map(|n| n.name.as_str()).unwrap_or("?"))
        } else {
            String::new()
        };

        if let Some(ref mut lbl) = self.hint_label {
            if hint_text.is_empty() { lbl.set_visible(false); }
            else { lbl.set_text(&hint_text); lbl.set_visible(true); }
        }
    }

    fn use_portal(&mut self, kind: PortalKind) {
        match kind {
            PortalKind::EnterDungeon => {
                let depth = self.state.as_ref().map(|s| s.dungeons_cleared + 1).unwrap_or(1);
                self.enter_dungeon(depth);
            }
            PortalKind::ExitDungeon => self.exit_dungeon(),
            PortalKind::DeeperDungeon => {
                if self.boss_alive {
                    self.show_flash("Портал запечатан! Сначала убей стража.");
                } else {
                    let d = self.dungeon_depth + 1;
                    self.enter_dungeon(d);
                }
            }
        }
    }

    // ── Боёвка ───────────────────────────────────────────────────────────────

    fn player_aim(&self) -> Option<(Vector3, Vector3)> {
        let p = self.player.as_ref()?;
        let pl = p.clone().try_cast::<Player>().ok()?;
        let b = pl.bind();
        Some((b.eye_pos(), b.aim_dir()))
    }

    fn try_fire(&mut self) {
        let cur = self.arsenal.current;
        let def = weapon_def(cur);

        if !self.arsenal.can_fire(cur) {
            self.shoot_cd = 0.35;
            self.show_flash(&format!("Нет боеприпасов: {}",
                def.ammo.map(|(t, _)| t.name_ru()).unwrap_or("—")));
            return;
        }

        self.shoot_cd = def.cooldown * self.loadout.cd_mult;
        self.arsenal.consume(cur);
        self.weapon_anim = WeaponAnim::Fire(0);
        self.anim_timer = 0.0;
        self.set_weapon_frame(def.fire_frames[0]);
        self.flash_muzzle();

        let Some((eye, dir)) = self.player_aim() else { return };
        let dmg = def.damage * self.loadout.dmg_mult;
        let dtype = def.dmg_type;

        match def.kind {
            FireKind::Melee => self.fire_melee(dmg, def.range, dtype),
            FireKind::Hitscan { pellets, spread } => {
                for _ in 0..pellets {
                    let sx = (self.rng.f32() - 0.5) * 2.0 * spread;
                    let sy = (self.rng.f32() - 0.5) * 2.0 * spread;
                    // разброс в плоскости, перпендикулярной взгляду
                    let right = dir.cross(Vector3::UP).normalized();
                    let up = right.cross(dir).normalized();
                    let d = (dir + right * sx + up * sy).normalized();
                    self.fire_ray(eye, d, def.range, dmg, dtype);
                }
            }
            FireKind::Projectile { speed, splash } => {
                self.spawn_projectile(eye + dir * 0.6, dir * speed, dmg, dtype, splash,
                                      def.range / speed, cur);
            }
        }
        self.process_kills();
    }

    fn fire_melee(&mut self, dmg: f32, range: f32, dtype: DmgType) {
        let Some((eye, dir)) = self.player_aim() else { return };
        let flat_dir = Vector3::new(dir.x, 0.0, dir.z).normalized();
        let mut best: Option<(usize, f32)> = None;
        for (i, e) in self.enemies.iter().enumerate() {
            let eb = e.bind();
            if !eb.alive { continue; }
            let epos = e.get_global_position();
            let to = Vector3::new(epos.x - eye.x, 0.0, epos.z - eye.z);
            let d = to.length();
            if d > range { continue; }
            let dot = flat_dir.dot(to.normalized());
            if dot > 0.45 {
                let score = dot / (d + 0.1);
                if best.map(|(_, s)| score > s).unwrap_or(true) {
                    best = Some((i, score));
                }
            }
        }
        if let Some((idx, _)) = best {
            let epos = self.enemies[idx].get_global_position();
            let dealt = self.enemies[idx].bind_mut().take_damage(dmg, dtype);
            self.spawn_fx("res://assets/effects/effect_blood.png",
                          epos + Vector3::new(0.0, 1.1, 0.0), 0.010, 0.28);
            // вампиризм — от фактически нанесённого урона
            if self.loadout.lifesteal > 0.0 {
                let heal = dealt * self.loadout.lifesteal;
                if let Some(ref p) = self.player {
                    if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                        pl.bind_mut().heal(heal);
                    }
                }
            }
        }
    }

    fn fire_ray(&mut self, from: Vector3, dir: Vector3, range: f32, dmg: f32, dtype: DmgType) {
        let to = from + dir * range;
        let hit = self.raycast(from, to);
        match hit {
            Some((pos, Some(mut enemy))) => {
                enemy.bind_mut().take_damage(dmg, dtype);
                self.spawn_fx("res://assets/effects/effect_blood.png",
                              pos, 0.008, 0.25);
            }
            Some((pos, None)) => {
                self.spawn_fx("res://assets/effects/effect_bullet.png",
                              pos - dir * 0.1, 0.004, 0.15);
            }
            None => {}
        }
    }

    /// Луч: возвращает точку и врага (если попали в него).
    fn raycast(&mut self, from: Vector3, to: Vector3) -> Option<(Vector3, Option<Gd<Enemy>>)> {
        let world = self.base().get_world_3d()?;
        let mut space = world.clone().get_direct_space_state()?;
        let mut query = PhysicsRayQueryParameters3D::create(from, to)?;
        if let Some(ref p) = self.player {
            let mut excl: godot::builtin::Array<Rid> = godot::builtin::Array::new();
            excl.push(p.get_rid());
            query.set_exclude(&excl);
        }
        let hit = space.intersect_ray(&query);
        if hit.is_empty() { return None; }
        let pos = hit.get("position")?.try_to::<Vector3>().ok()?;
        let enemy = hit.get("collider")
            .and_then(|cv| cv.try_to::<Gd<godot::classes::Node>>().ok())
            .and_then(|n| n.try_cast::<Enemy>().ok());
        Some((pos, enemy))
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_projectile(&mut self, pos: Vector3, vel: Vector3, dmg: f32, dmg_type: DmgType,
                        splash: f32, ttl: f32, weapon: WeaponId) {
        let mut node = Node3D::new_alloc();
        node.set_position(pos);
        let (tex, px, color) = match weapon {
            WeaponId::Rocket => ("res://assets/sprites/projectiles/rocket.png", 0.010, Color::WHITE),
            _ => ("res://assets/effects/effect_energy.png", 0.006, C_PINK),
        };
        if let Some(mut sp) = make_billboard(&mut self.cache, tex, Vector3::ZERO, px) {
            sp.set_modulate(color);
            node.add_child(&sp);
        }
        let l = make_light(Vector3::ZERO, C_PINK, 0.8, 5.0);
        node.add_child(&l);
        self.base_mut().add_child(&node);
        self.projectiles.push(Projectile { node, pos, vel, dmg, dmg_type, splash, ttl });
    }

    fn tick_projectiles(&mut self, dt: f32) {
        let mut exploded: Vec<(Vector3, f32, DmgType, f32)> = Vec::new(); // pos, dmg, type, splash
        let mut direct_hits: Vec<(Gd<Enemy>, f32, DmgType, Vector3)> = Vec::new();

        let mut i = 0;
        while i < self.projectiles.len() {
            let new_pos = self.projectiles[i].pos + self.projectiles[i].vel * dt;
            let from = self.projectiles[i].pos;
            let hit = self.raycast(from, new_pos);
            let mut remove = false;

            match hit {
                Some((pos, Some(enemy))) => {
                    let pr = &self.projectiles[i];
                    if pr.splash > 0.0 {
                        exploded.push((pos, pr.dmg, pr.dmg_type, pr.splash));
                    } else {
                        direct_hits.push((enemy, pr.dmg, pr.dmg_type, pos));
                    }
                    remove = true;
                }
                Some((pos, None)) => {
                    let pr = &self.projectiles[i];
                    if pr.splash > 0.0 {
                        exploded.push((pos, pr.dmg, pr.dmg_type, pr.splash));
                    } else {
                        self.spawn_fx("res://assets/effects/effect_bullet.png", pos, 0.004, 0.15);
                    }
                    remove = true;
                }
                None => {
                    self.projectiles[i].pos = new_pos;
                    self.projectiles[i].node.set_position(new_pos);
                    self.projectiles[i].ttl -= dt;
                    if self.projectiles[i].ttl <= 0.0 { remove = true; }
                }
            }

            if remove {
                let p = self.projectiles.remove(i);
                p.node.free();
            } else {
                i += 1;
            }
        }

        for (enemy, dmg, dtype, pos) in direct_hits {
            let mut e = enemy;
            e.bind_mut().take_damage(dmg, dtype);
            self.spawn_fx("res://assets/effects/effect_blood.png", pos, 0.008, 0.25);
        }
        for (pos, dmg, dtype, splash) in exploded {
            self.explode(pos, dmg, dtype, splash);
        }
        self.process_kills();
    }

    fn explode(&mut self, pos: Vector3, dmg: f32, dtype: DmgType, radius: f32) {
        self.spawn_fx("res://assets/effects/effect_explosion.png", pos, 0.022, 0.4);
        self.spawn_light_fx(pos, Color::from_rgba(1.0, 0.5, 0.6, 1.0), 2.6, radius * 2.2, 0.35);

        for e in self.enemies.iter_mut() {
            let alive = e.bind().alive;
            if !alive { continue; }
            let d = (e.get_global_position() - pos).length();
            if d < radius {
                let fall = 1.0 - (d / radius) * 0.55;
                e.bind_mut().take_damage(dmg * fall, dtype);
            }
        }
        // самоурон
        if let Some(ref p) = self.player {
            let d = (p.get_global_position() - pos).length();
            if d < radius * 0.8 {
                if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                    let fall = 1.0 - d / (radius * 0.8);
                    pl.bind_mut().take_damage(dmg * 0.35 * fall);
                    self.damage_flash_timer = 0.3;
                }
            }
        }
    }

    /// Обработка убитых врагов: XP, дроп, эффекты, квест босса.
    fn process_kills(&mut self) {
        let mut kills: Vec<(Vector3, f32, bool, String)> = Vec::new();
        let mut i = 0;
        while i < self.enemies.len() {
            let alive = self.enemies[i].bind().alive;
            if !alive {
                let pos = self.enemies[i].get_global_position();
                let xp = self.enemies[i].bind().xp_value;
                let is_boss = self.enemies[i].bind().is_boss;
                let kind = self.enemies[i].bind().cfg_id.to_string();
                kills.push((pos, xp, is_boss, kind));
                let e = self.enemies.remove(i);
                e.free();
            } else {
                i += 1;
            }
        }

        for (pos, xp, is_boss, kind) in kills {
            self.spawn_fx("res://assets/effects/effect_blood.png",
                          pos + Vector3::new(0.0, 0.9, 0.0), 0.014, 0.4);
            // прогресс kill-квестов
            self.bump_quests("kill", &kind);

            // XP и уровни
            let levels = {
                let st = self.state.as_mut().unwrap();
                st.add_xp(xp as u32)
            };
            if levels > 0 {
                let (ci, si, lvl) = {
                    let st = self.state.as_ref().unwrap();
                    (st.class_idx.unwrap_or(0), st.spec_idx, st.level)
                };
                self.apply_loadout(ci, si, false);
                // подлечить при апе
                if let Some(ref p) = self.player {
                    if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                        let heal = pl.bind().max_hp * 0.35;
                        pl.bind_mut().heal(heal);
                    }
                }
                self.show_flash(&format!("УРОВЕНЬ {}!", lvl));
            }

            // дроп
            let in_dungeon = self.loc == Loc::Dungeon;
            // Дроп по таблице kill_drops (loot.json): один бросок, записи
            // кумулятивны по chance, остаток вероятности — «ничего».
            let drops = self.cfg.as_ref().map(|c| c.loot.kill_drops.clone()).unwrap_or_default();
            let roll = self.rng.f32();
            let mut acc = 0.0f32;
            for d in &drops {
                acc += d.chance;
                if roll >= acc { continue; }
                match d.kind.as_str() {
                    "ammo" => {
                        let t = AmmoType::from_idx(self.rng.below(4) as usize);
                        self.spawn_ammo_pickup(t, t.pack_size() / 2 + 1, pos, in_dungeon);
                    }
                    _ => {
                        let id = d.id.as_deref().unwrap_or("");
                        if id == "heart_1up" {
                            self.spawn_item_heart(pos, in_dungeon);
                        } else if !id.is_empty() {
                            let cfg = self.cfg.take();
                            if let Some(ref cfg) = cfg {
                                self.spawn_item(cfg, id, pos, in_dungeon);
                            }
                            self.cfg = cfg;
                        }
                    }
                }
                break;
            }

            if is_boss {
                self.boss_alive = false;
                let msgs = {
                    let st = self.state.as_mut().unwrap();
                    st.dungeons_cleared = st.dungeons_cleared.max(self.dungeon_depth);
                    st.gold += 50;
                    let done = st.quests.quests.iter()
                        .any(|q| q.id == "dungeon_heart"
                             && q.state == crate::quest::QuestState::Active);
                    if done { st.quests.complete("dungeon_heart"); }
                    st.add_xp(120)
                };
                let _ = msgs;
                self.show_flash("СТРАЖ ПОВЕРЖЕН! Портал вглубь открыт (+50 зол.)");
                self.auto_save();
            }
        }
    }

    fn spawn_item_heart(&mut self, pos: Vector3, in_dungeon: bool) {
        let node = self.make_pickup_node("res://assets/sprites/pickups/heart_1up.png", pos, 0.010);
        self.world_items.push(WorldItemNode {
            node, item_id: "heart_1up".into(), name: "Сердце жизни".into(),
            payload: Payload::Heart, in_dungeon,
        });
    }

    // ── Эффекты ──────────────────────────────────────────────────────────────

    fn spawn_fx(&mut self, tex: &str, pos: Vector3, px: f32, ttl: f32) {
        if let Some(sp) = make_billboard(&mut self.cache, tex, pos, px) {
            self.base_mut().add_child(&sp);
            self.sprite_fx.push(SpriteFx { node: sp, ttl, total: ttl });
        }
    }

    fn spawn_light_fx(&mut self, pos: Vector3, color: Color, energy: f32, range: f32, ttl: f32) {
        let l = make_light(pos, color, energy, range);
        self.base_mut().add_child(&l);
        self.light_fx.push(LightFx { node: l, ttl, total: ttl, energy });
    }

    fn tick_fx(&mut self, dt: f32) {
        let mut i = 0;
        while i < self.sprite_fx.len() {
            self.sprite_fx[i].ttl -= dt;
            if self.sprite_fx[i].ttl <= 0.0 {
                let fx = self.sprite_fx.remove(i);
                fx.node.free();
            } else {
                let a = (self.sprite_fx[i].ttl / self.sprite_fx[i].total).clamp(0.0, 1.0);
                let n = self.sprite_fx[i].node.clone();
                let mut n = n;
                n.set_modulate(Color::from_rgba(1.0, 1.0, 1.0, a));
                i += 1;
            }
        }
        let mut i = 0;
        while i < self.light_fx.len() {
            self.light_fx[i].ttl -= dt;
            if self.light_fx[i].ttl <= 0.0 {
                let fx = self.light_fx.remove(i);
                fx.node.free();
            } else {
                use godot::classes::light_3d::Param;
                let a = (self.light_fx[i].ttl / self.light_fx[i].total).clamp(0.0, 1.0);
                let e = self.light_fx[i].energy * a;
                let n = self.light_fx[i].node.clone();
                let mut n = n;
                n.set_param(Param::ENERGY, e);
                i += 1;
            }
        }
    }

    fn flash_muzzle(&mut self) {
        self.muzzle_timer = 0.09;
        if self.muzzle_light.is_none() {
            if let Some(ref p) = self.player {
                let l = make_light(Vector3::new(0.0, 0.6, -0.8),
                                   Color::from_rgba(1.0, 0.6, 0.8, 1.0), 0.0, 7.0);
                let mut p2 = p.clone();
                p2.add_child(&l);
                self.muzzle_light = Some(l);
            }
        }
        if let Some(ref mut l) = self.muzzle_light {
            use godot::classes::light_3d::Param;
            l.set_param(Param::ENERGY, 1.6);
        }
    }

    fn tick_muzzle(&mut self, dt: f32) {
        if self.muzzle_timer > 0.0 {
            self.muzzle_timer -= dt;
            if self.muzzle_timer <= 0.0 {
                if let Some(ref mut l) = self.muzzle_light {
                    use godot::classes::light_3d::Param;
                    l.set_param(Param::ENERGY, 0.0);
                }
            }
        }
    }

    // ── Урон от врагов ───────────────────────────────────────────────────────

    fn collect_enemy_damage(&mut self, _dt: f32) {
        let mut total_dmg = 0.0f32;
        for e in self.enemies.iter_mut() {
            let dmg = e.bind().pending_dmg;
            if dmg > 0.0 {
                total_dmg += dmg;
                e.bind_mut().pending_dmg = 0.0;
            }
        }
        if total_dmg > 0.0 {
            let maybe_player = self.player.clone();
            if let Some(p_gd) = maybe_player {
                if let Ok(mut player) = p_gd.try_cast::<Player>() {
                    player.bind_mut().take_damage(total_dmg);
                }
            }
            self.damage_flash_timer = 0.35;
        }
    }

    // ── Подбор предметов ─────────────────────────────────────────────────────

    fn pick_up_item(&mut self, idx: usize) {
        if idx >= self.world_items.len() { return; }
        let lang = self.settings.lang.clone();
        let wi = self.world_items.remove(idx);
        wi.node.free();
        self.near_item = None;
        let name = wi.name.clone();

        // прогресс collect-квестов
        let picked_id = wi.item_id.clone();
        self.bump_quests("collect", &picked_id);

        match wi.payload {
            Payload::Gold(v) => {
                if let Some(ref mut st) = self.state { st.gold += v; }
                self.show_flash(&format!("+{} зол.", v));
            }
            Payload::Ammo(t, n) => {
                let added = self.arsenal.add_ammo(t, n, self.loadout.ammo_mult);
                self.show_flash(&format!("+{} {}", added, t.name_ru()));
            }
            Payload::Weapon(w) => {
                let is_new = self.arsenal.give_weapon(w);
                if let Some((t, _)) = weapon_def(w).ammo {
                    self.arsenal.add_ammo(t, t.pack_size(), self.loadout.ammo_mult);
                }
                if is_new {
                    self.arsenal.current = w;
                    self.refresh_weapon_sheet();
                    self.show_flash(&format!("НОВОЕ ОРУЖИЕ: {}!", weapon_def(w).name_ru));
                } else {
                    self.show_flash(&format!("+боеприпасы ({})", weapon_def(w).name_ru));
                }
            }
            Payload::Heart => {
                if let Some(ref mut st) = self.state { st.add_heart(); }
                let (ci, si) = {
                    let st = self.state.as_ref().unwrap();
                    (st.class_idx.unwrap_or(0), st.spec_idx)
                };
                self.apply_loadout(ci, si, false);
                if let Some(ref p) = self.player {
                    if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                        let mh = pl.bind().max_hp;
                        pl.bind_mut().hp = mh;
                    }
                }
                self.show_flash("СЕРДЦЕ ЖИЗНИ: +15 макс. HP, полное лечение!");
            }
            Payload::KeyItem | Payload::Consumable { .. } => {
                if let Some(ref mut st) = self.state {
                    use crate::item::Item;
                    st.inventory.add(Item::new(&wi.item_id, &name, "", 1));
                }
                self.show_flash(&format!("{}: {}", t("msg_picked_up", &lang), name));
            }
        }
        self.auto_save();
    }

    fn use_first_consumable(&mut self) {
        let lang = self.settings.lang.clone();
        let heal_data = self.state.as_ref().and_then(|s| {
            s.inventory.items.iter()
                .find(|i| matches!(i.id.as_str(),
                    "medkit" | "armor_shard" | "potion" | "bread" | "energy_drink"))
                .map(|i| {
                    let amt = match i.id.as_str() {
                        "medkit"       => 30.0,
                        "armor_shard"  => 20.0,
                        "potion"       => 50.0,
                        "energy_drink" => 15.0,
                        _              => 10.0,
                    };
                    (i.id.clone(), amt)
                })
        });
        if let Some((id, amount)) = heal_data {
            let full = self.player.as_ref()
                .and_then(|p| p.clone().try_cast::<Player>().ok())
                .map(|pl| pl.bind().hp >= pl.bind().max_hp)
                .unwrap_or(true);
            if full {
                self.show_flash("Здоровье уже полное");
                return;
            }
            if let Some(ref mut state) = self.state { state.inventory.remove_one(&id); }
            if let Some(ref p) = self.player {
                if let Ok(mut player) = p.clone().try_cast::<Player>() {
                    player.bind_mut().heal(amount);
                }
            }
            self.spawn_fx_on_player("res://assets/effects/effect_heal.png");
            self.show_flash(t("msg_healed", &lang));
        }
    }

    fn spawn_fx_on_player(&mut self, tex: &str) {
        if let Some(ref p) = self.player {
            let pos = p.get_global_position() + Vector3::new(0.0, 1.2, 0.0);
            self.spawn_fx(tex, pos, 0.010, 0.5);
        }
    }

    // ── Инвентарь ────────────────────────────────────────────────────────────

    fn open_inventory(&mut self) {
        self.mode = Mode::Inventory;
        self.freeze_player(true);
        self.refresh_inventory_ui();
        if let Some(ref mut p) = self.inv_panel { p.set_visible(true); }
        if let Some(ref mut lbl) = self.hint_label { lbl.set_visible(false); }
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
    }

    fn close_inventory(&mut self) {
        if let Some(ref mut p) = self.inv_panel { p.set_visible(false); }
        self.set_mode_explore();
    }

    fn refresh_inventory_ui(&mut self) {
        let lang = self.settings.lang.clone();
        let text = if let Some(ref state) = self.state {
            let mut lines = Vec::new();
            if let Some(ci) = state.class_idx {
                let c = &classes()[ci.min(classes().len() - 1)];
                lines.push(format!("{} / {}   ур. {}   XP {}/{}",
                    c.name_ru, c.specs[state.spec_idx.min(2)].name_ru,
                    state.level, state.xp, xp_to_next(state.level)));
                lines.push(String::new());
            }
            lines.push(format!("{}: {} зол.", t("hud_gold", &lang), state.gold));
            lines.push(String::new());
            lines.push("Боезапас:".to_string());
            for t in AmmoType::ALL {
                lines.push(format!("  {}: {}", t.name_ru(), self.arsenal.ammo_of(t)));
            }
            lines.push(String::new());
            if state.inventory.is_empty() {
                lines.push(t("hud_inv_empty", &lang).to_string());
            } else {
                for item in &state.inventory.items {
                    lines.push(format!("• {} ×{}", item.name, item.qty));
                }
                lines.push(String::new());
                lines.push(format!("[ E ] — {}", t("inv_use", &lang)));
            }
            lines.join("\n")
        } else { String::new() };
        if let Some(ref mut lbl) = self.inv_list { lbl.set_text(&text); }
    }

    // ── Перки ────────────────────────────────────────────────────────────────

    fn open_perks(&mut self) {
        self.mode = Mode::Perks;
        self.freeze_player(true);
        self.refresh_perk_ui();
        if let Some(ref mut p) = self.perk_panel { p.set_visible(true); }
        if let Some(ref mut lbl) = self.hint_label { lbl.set_visible(false); }
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
    }

    fn close_perks(&mut self) {
        if let Some(ref mut p) = self.perk_panel { p.set_visible(false); }
        self.set_mode_explore();
    }

    fn process_perks(&mut self) {
        let input = Input::singleton();
        if input.is_action_just_pressed("perks") || input.is_action_just_pressed("escape") {
            self.close_perks();
            return;
        }
        for n in 0..8usize {
            let act = format!("weapon_{}", n + 1);
            if input.is_action_just_pressed(&act) {
                self.buy_perk_at(n);
                return;
            }
        }
    }

    fn buy_perk_at(&mut self, n: usize) {
        // детерминированный список доступных перков (тот же, что в refresh_perk_ui)
        let picked = {
            let Some(st) = self.state.as_ref() else { return };
            let avail = crate::perk::available(&st.perks, st.perk_points);
            avail.get(n).map(|p| (p.id.clone(), p.cost, p.name_ru.clone(),
                                  p.max_ranks))
        };
        let Some((id, cost, name, max_ranks)) = picked else { return };

        let new_rank = {
            let st = self.state.as_mut().unwrap();
            if st.perk_points < cost { return; }
            st.perk_points -= cost;
            let r = st.perks.entry(id.clone()).or_insert(0);
            *r += 1;
            *r
        };

        let (ci, si) = {
            let st = self.state.as_ref().unwrap();
            (st.class_idx.unwrap_or(0), st.spec_idx)
        };
        self.apply_loadout(ci, si, false);
        // если максимум HP вырос — не даём текущему HP «отстать» слишком сильно
        if let Some(ref p) = self.player {
            if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                let max = pl.bind().max_hp;
                let hp = pl.bind().hp;
                if hp > max { pl.bind_mut().hp = max; }
            }
        }
        self.refresh_perk_ui();
        self.show_flash(&format!("Перк улучшен: {} ({}/{})", name, new_rank, max_ranks));
        self.auto_save();
    }

    fn refresh_perk_ui(&mut self) {
        use crate::perk::{available, perks, reqs_met, synergies, synergy_active};
        let text = if let Some(ref st) = self.state {
            let owned = &st.perks;
            let points = st.perk_points;
            // номера покупки — по порядку available()
            let avail_ids: Vec<String> = available(owned, points).iter().map(|p| p.id.clone()).collect();

            let mut lines = vec![format!("Очки перков: {}", points), String::new()];

            for (branch, title) in [
                ("survival", "◆ ЖИВУЧЕСТЬ"),
                ("offense",  "◆ УРОН"),
                ("utility",  "◆ УТИЛИТИ"),
            ] {
                lines.push(title.to_string());
                for p in perks().iter().filter(|p| p.branch == branch) {
                    let rank = owned.get(&p.id).copied().unwrap_or(0);
                    let tag = if rank >= p.max_ranks {
                        "  [МАКС]".to_string()
                    } else if !reqs_met(&p.requires, owned) {
                        let need: Vec<String> = p.requires.iter().map(|r| {
                            let id = r.split_once(':').map(|(a, _)| a).unwrap_or(r);
                            crate::perk::perk_by_id(id).map(|d| d.name_ru.clone()).unwrap_or_else(|| id.to_string())
                        }).collect();
                        format!("  🔒 нужно: {}", need.join(", "))
                    } else if let Some(pos) = avail_ids.iter().position(|x| *x == p.id) {
                        format!("  ◀ [{}] купить ({} оч.)", pos + 1, p.cost)
                    } else {
                        String::new()
                    };
                    lines.push(format!("  {} {}/{}{}", p.name_ru, rank, p.max_ranks, tag));
                    lines.push(format!("      {}", p.desc_ru));
                }
                lines.push(String::new());
            }

            lines.push("◆ СИНЕРГИИ".to_string());
            for s in synergies() {
                let on = synergy_active(s, owned);
                let mark = if on { "✔" } else { "…" };
                lines.push(format!("  {} {} — {}", mark, s.name_ru, s.desc_ru));
            }
            lines.join("\n")
        } else { String::new() };
        if let Some(ref mut lbl) = self.perk_list { lbl.set_text(&text); }
    }

    // ── Диалог ───────────────────────────────────────────────────────────────

    /// Сцена по id: сначала dialogues.json пресета (данные приоритетнее кода —
    /// пресет может переопределять встроенные сцены), затем story.rs.
    fn resolve_scene(&self, id: &str) -> Option<Scene> {
        if id.is_empty() { return None; }
        if let Some(s) = self.cfg.as_ref().and_then(|c| c.dialogue(id)) {
            return Some(s.clone());
        }
        self.state.as_ref().and_then(|st| get_scene(id, st))
    }

    fn start_dialogue(&mut self, npc_idx: usize) {
        let Some(npc) = self.npcs.get(npc_idx) else { return };
        let (npc_id, npc_name) = (npc.id.clone(), npc.name.clone());
        let scene_kind = npc.scene.clone();
        let quest_id = npc.quest.clone();

        // 1) story-персонажи: динамический выбор сцены из story.rs
        // 2) конкретный scene_id
        // 3) квест-гивер: сгенерированная сцена выдачи/прогресса/сдачи
        let scene = match scene_kind.as_deref() {
            Some("story") => {
                let dynamic = self.state.as_ref()
                    .map(|s| npc_scene_id(&npc_id, s))
                    .unwrap_or("");
                self.resolve_scene(dynamic)
            }
            Some(id) if !id.is_empty() => self.resolve_scene(id),
            _ => None,
        };
        let _ = quest_id;
        let scene = scene.or_else(|| self.make_giver_scene(&npc_name, &npc_id));
        let Some(scene) = scene else { return };

        self.scene = Some(scene);
        self.line_idx = 0;
        self.mode = Mode::Dialogue;
        self.freeze_player(true);
        self.at_choices = false;
        if let Some(ref mut p) = self.dlg_panel { p.set_visible(true); }
        if let Some(ref mut lbl) = self.hint_label { lbl.set_visible(false); }
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
        self.refresh_dlg_ui();
    }

    /// Сцена для квест-гивера: NPC выдаёт свои квесты (giver == npc_id) по цепочке —
    /// первый незавершённый; когда всё сдано — благодарность.
    fn make_giver_scene(&self, npc_name: &str, npc_id: &str) -> Option<Scene> {
        let cfg = self.cfg.as_ref()?;
        let st = self.state.as_ref()?;
        let next = cfg.quests.iter().find(|q| {
            q.giver == npc_id && !st.quests.quests.iter()
                .any(|x| x.id == q.id && x.state == crate::quest::QuestState::Completed)
        });
        match next {
            Some(q) => self.make_quest_scene(npc_name, &q.id.clone()),
            None => {
                let has_any = cfg.quests.iter().any(|q| q.giver == npc_id);
                if !has_any { return None; }
                Some(Scene {
                    id: format!("auto_thanks_{npc_id}"),
                    lines: vec![Line::new(npc_name, "",
                        "Ты сделал всё, о чём я просил. Квартал этого не забудет.")],
                    choices: vec![],
                })
            }
        }
    }

    /// Сгенерировать сцену диалога для конкретного квеста по его состоянию.
    fn make_quest_scene(&self, npc_name: &str, quest_id: &str) -> Option<Scene> {
        let cfg = self.cfg.as_ref()?;
        let q = cfg.quest(quest_id)?.clone();
        let st = self.state.as_ref()?;

        let taken = st.quests.quests.iter().any(|x| x.id == q.id);
        let done  = st.quests.quests.iter()
            .any(|x| x.id == q.id && x.state == crate::quest::QuestState::Completed);
        let progress = self.quest_progress(&q);
        let ready = progress >= q.count;

        let mut lines = Vec::new();
        let mut choices = Vec::new();

        if done {
            lines.push(Line::new(npc_name, "", "Спасибо ещё раз. Ты уже помог мне — заходи просто так."));
        } else if !taken {
            lines.push(Line::new(npc_name, "", &q.desc_ru));
            lines.push(Line::new(npc_name, "",
                &format!("Награда: {} XP, {} зол. Возьмёшься?", q.reward_xp, q.reward_gold)));
            choices.push(Choice {
                text: format!("Взять задание «{}»", q.title_ru),
                requires: None,
                effects: vec![Effect::Quest {
                    id: q.id.clone(), title: q.title_ru.clone(), desc: q.desc_ru.clone(),
                }],
                next: None,
            });
            choices.push(Choice::simple("Не сейчас.", vec![]));
        } else if ready {
            lines.push(Line::new(npc_name, "", "Сделано? Отлично. Вот твоя награда."));
            choices.push(Choice {
                text: "Сдать задание".into(),
                requires: None,
                effects: vec![
                    Effect::QuestDone(q.id.clone()),
                    Effect::Xp(q.reward_xp),
                    Effect::Gold(q.reward_gold),
                ],
                next: None,
            });
            choices.push(Choice::simple("Ещё вернусь.", vec![]));
        } else {
            lines.push(Line::new(npc_name, "",
                &format!("Как продвигается? {} — {}/{}.", q.title_ru, progress, q.count)));
        }

        Some(Scene { id: format!("auto_quest_{}", q.id), lines, choices })
    }

    /// Текущий прогресс квеста (kill/collect — счётчик, clear_dungeon — глубина).
    fn quest_progress(&self, q: &crate::config::QuestCfg) -> u32 {
        let Some(st) = self.state.as_ref() else { return 0 };
        match q.kind.as_str() {
            "clear_dungeon" => st.dungeons_cleared,
            _ => st.quest_kills.get(&q.id).copied().unwrap_or(0),
        }
    }

    /// Инкремент прогресса kill/collect-квестов по событию.
    fn bump_quests(&mut self, kind: &str, target: &str) {
        let Some(cfg) = self.cfg.as_ref() else { return };
        let matching: Vec<(String, u32, String)> = cfg.quests.iter()
            .filter(|q| q.kind == kind && q.target == target)
            .map(|q| (q.id.clone(), q.count, q.title_ru.clone()))
            .collect();
        if matching.is_empty() { return; }

        let Some(st) = self.state.as_mut() else { return };
        let mut notices = Vec::new();
        for (qid, count, title) in matching {
            let active = st.quests.quests.iter()
                .any(|x| x.id == qid && x.state == crate::quest::QuestState::Active);
            if !active { continue; }
            let c = st.quest_kills.entry(qid.clone()).or_insert(0);
            if *c < count {
                *c += 1;
                if *c >= count {
                    notices.push(format!("Задание готово к сдаче: «{}»", title));
                } else {
                    notices.push(format!("{}: {}/{}", title, *c, count));
                }
            }
        }
        for n in notices { self.show_flash(&n); }
    }

    fn advance_dialogue(&mut self) {
        let (total, has_choices) = match self.scene.as_ref() {
            Some(s) => (s.lines.len(), !s.choices.is_empty()),
            None => { self.end_dialogue(); return; }
        };
        if self.line_idx + 1 < total {
            self.line_idx += 1;
            self.refresh_dlg_ui();
        } else if has_choices {
            self.at_choices = true;
            self.refresh_dlg_ui();
        } else {
            self.end_dialogue();
        }
    }

    fn select_choice(&mut self, idx: usize) {
        let (effects, next) = {
            let scene = match self.scene.as_ref() { Some(s) => s, None => return };
            let state = match self.state.as_ref() { Some(s) => s, None => return };
            let avail: Vec<_> = scene.choices.iter()
                .filter(|c| c.requires.as_ref().is_none_or(|(st, mn)| state.stat(st) >= *mn))
                .collect();
            if idx >= avail.len() { return; }
            (avail[idx].effects.clone(), avail[idx].next.clone())
        };
        let lvl_before = self.state.as_ref().map(|s| s.level).unwrap_or(1);
        let msgs = self.state.as_mut().unwrap().apply(&effects);
        for m in msgs { self.show_flash(&m); }
        // Effect::Xp мог поднять уровень — пересчитать статы
        let lvl_after = self.state.as_ref().map(|s| s.level).unwrap_or(1);
        if lvl_after != lvl_before {
            let (ci, si) = {
                let st = self.state.as_ref().unwrap();
                (st.class_idx.unwrap_or(0), st.spec_idx)
            };
            self.apply_loadout(ci, si, false);
        }
        if let Some(next_id) = next {
            let new_scene = self.resolve_scene(&next_id);
            if let Some(sc) = new_scene {
                self.scene = Some(sc);
                self.line_idx = 0;
                self.at_choices = false;
                self.refresh_dlg_ui();
                return;
            }
        }
        self.end_dialogue();
    }

    fn end_dialogue(&mut self) {
        self.scene = None;
        self.line_idx = 0;
        self.at_choices = false;
        if let Some(ref mut p) = self.dlg_panel { p.set_visible(false); }
        self.set_mode_explore();
        self.auto_save();
    }

    fn refresh_dlg_ui(&mut self) {
        let (speaker, text, choices_text): (String, String, Vec<String>) = {
            let scene = match self.scene.as_ref() { Some(s) => s, None => return };
            let state = match self.state.as_ref() { Some(s) => s, None => return };
            let line = &scene.lines[self.line_idx.min(scene.lines.len().saturating_sub(1))];
            let ct: Vec<String> = if self.at_choices {
                scene.choices.iter()
                    .filter(|c| c.requires.as_ref().is_none_or(|(st, mn)| state.stat(st) >= *mn))
                    .enumerate()
                    .map(|(i, c)| format!("{}. {}", i + 1, c.text))
                    .collect()
            } else { vec![] };
            (line.speaker.clone(), line.text.clone(), ct)
        };
        if let Some(ref mut lbl) = self.dlg_speaker { lbl.set_text(&speaker); }
        let display = if !self.at_choices {
            format!("{}\n\n  [ E — далее ]", text)
        } else { text };
        if let Some(ref mut lbl) = self.dlg_text { lbl.set_text(&display); }
        let cl = [self.cl0.as_mut(), self.cl1.as_mut(), self.cl2.as_mut(), self.cl3.as_mut()];
        for (i, lbl_opt) in cl.into_iter().enumerate() {
            if let Some(lbl) = lbl_opt {
                if i < choices_text.len() {
                    lbl.set_text(&choices_text[i]);
                    lbl.set_visible(true);
                    lbl.add_theme_color_override("font_color", C_PINK);
                } else {
                    lbl.set_visible(false);
                }
            }
        }
        if let Some(ref mut vbox) = self.choice_box {
            vbox.set_visible(self.at_choices && !choices_text.is_empty());
        }
    }

    // ── Обновление HUD ────────────────────────────────────────────────────────

    fn update_hp_bar(&mut self) {
        let lang = self.settings.lang.clone();
        let (hp, max_hp) = if let Some(ref p) = self.player {
            if let Ok(player) = p.clone().try_cast::<Player>() {
                (player.bind().hp, player.bind().max_hp)
            } else { (100.0, 100.0) }
        } else { (100.0, 100.0) };
        let ratio = (hp / max_hp).clamp(0.0, 1.0);
        if let Some(ref mut fg) = self.hp_bar_fg {
            fg.set_size(Vector2::new(218.0 * ratio, 22.0));
            let c = Color::from_rgba(0.9 - ratio * 0.5, 0.1 + ratio * 0.3, 0.1, 1.0);
            fg.add_theme_stylebox_override("panel", &make_style(c, Color::TRANSPARENT_BLACK, 0));
        }
        if let Some(ref mut lbl) = self.hp_label {
            lbl.set_text(&format!("{}: {:.0}/{:.0}", t("hud_hp", &lang), hp, max_hp));
        }
    }

    fn update_xp_bar(&mut self) {
        let (level, xp, next) = self.state.as_ref()
            .map(|s| (s.level, s.xp, xp_to_next(s.level)))
            .unwrap_or((1, 0, 100));
        let ratio = (xp as f32 / next as f32).clamp(0.0, 1.0);
        if let Some(ref mut fg) = self.xp_bar_fg {
            fg.set_size(Vector2::new(220.0 * ratio, 8.0));
        }
        if let Some(ref mut lbl) = self.xp_label {
            lbl.set_text(&format!("ур. {}  ({}/{})", level, xp, next));
        }
    }

    fn update_ammo_hud(&mut self) {
        let has_any = self.arsenal.owned.iter().any(|o| *o);
        if !has_any {
            if let Some(ref mut l) = self.ammo_label { l.set_text(""); }
            return;
        }
        let def = weapon_def(self.arsenal.current);
        let text = match def.ammo {
            None => "∞".to_string(),
            Some((t, _)) => format!("{}  {}", t.name_ru(), self.arsenal.ammo_of(t)),
        };
        if let Some(ref mut l) = self.ammo_label { l.set_text(&text); }
    }

    fn update_inv_label(&mut self) {
        let lang = self.settings.lang.clone();
        if let Some(ref state) = self.state {
            let text = if state.inventory.is_empty() {
                format!("{}: {}", t("hud_gold", &lang), state.gold)
            } else {
                let items: Vec<_> = state.inventory.items.iter()
                    .map(|i| format!("{} ×{}", i.name, i.qty))
                    .collect();
                format!("{}  |  {}: {}  [ I ]", items.join(", "), t("hud_gold", &lang), state.gold)
            };
            if let Some(ref mut lbl) = self.inv_label { lbl.set_text(&text); }
        }
    }

    fn update_quest_label(&mut self, lang: &str) {
        if let Some(ref state) = self.state {
            let mut lines: Vec<String> = Vec::new();
            if state.perk_points > 0 {
                lines.push(format!("[P] Перки: {} очк.!", state.perk_points));
            }
            let active: Vec<_> = state.quests.quests.iter()
                .filter(|q| q.state == crate::quest::QuestState::Active)
                .collect();
            if !active.is_empty() {
                lines.push(t("hud_quests", lang).to_string());
                for q in active.iter().take(5) {
                    lines.push(format!("• {}", q.title));
                }
            }
            let text = lines.join("\n");
            if let Some(ref mut lbl) = self.quest_label { lbl.set_text(&text); }
        }
    }

    fn update_targeting_hud(&mut self) {
        let text = if let Some(idx) = self.near_enemy {
            if idx < self.enemies.len() {
                let eb = self.enemies[idx].bind();
                if eb.alive {
                    let ratio = (eb.hp / eb.max_hp).clamp(0.0, 1.0);
                    let filled = (ratio * 10.0).round() as usize;
                    let bar: String = "█".repeat(filled) + &"░".repeat(10 - filled.min(10));
                    let boss = if eb.is_boss { "СТРАЖ " } else { "" };
                    format!("{}[{}]  {}  {:.0}/{:.0}", boss, eb.cfg_id, bar, eb.hp, eb.max_hp)
                } else { String::new() }
            } else { String::new() }
        } else { String::new() };

        if let Some(ref mut lbl) = self.targeting_label {
            if text.is_empty() { lbl.set_visible(false); }
            else { lbl.set_text(&text); lbl.set_visible(true); }
        }
    }

    /// Текстура миникарты из floor_map генератора (1 пиксель = 1 клетка).
    fn build_minimap_texture(&mut self) {
        let g = dungeon::GRID;
        if self.minimap_floor.len() < g * g { return; }
        let Some(mut img) = Image::create_empty(g as i32, g as i32, false, Format::RGBA8)
            else { return };
        img.fill(Color::from_rgba(0.06, 0.04, 0.10, 0.9));
        for j in 0..g {
            for i in 0..g {
                if self.minimap_floor[j * g + i] {
                    img.set_pixel(i as i32, j as i32,
                                  Color::from_rgba(0.42, 0.30, 0.58, 1.0));
                }
            }
        }
        if let Some(tex) = ImageTexture::create_from_image(&img) {
            let tex2d = tex.upcast::<Texture2D>();
            if let Some(ref mut rect) = self.minimap_rect {
                rect.set_texture(&tex2d);
            }
        }
    }

    /// Точка игрока на миникарте.
    fn update_minimap(&mut self) {
        if self.loc != Loc::Dungeon { return; }
        let Some(ref p) = self.player else { return };
        let pos = p.get_position() - DUNGEON_OFFSET;

        const MAP_SZ: f32 = 176.0;
        const MAP_X: f32 = HUD_W - MAP_SZ - 16.0;
        const MAP_Y: f32 = 40.0;
        let scale = MAP_SZ / dungeon::GRID as f32;
        let pi = (pos.x / dungeon::CELL + dungeon::GRID as f32 * 0.5)
            .clamp(0.0, dungeon::GRID as f32 - 1.0);
        let pj = (pos.z / dungeon::CELL + dungeon::GRID as f32 * 0.5)
            .clamp(0.0, dungeon::GRID as f32 - 1.0);

        let dot_x = MAP_X + pi * scale - 3.0;
        let dot_y = MAP_Y + pj * scale - 3.0;
        if let Some(ref mut dot) = self.minimap_dot {
            dot.set_position(Vector2::new(dot_x, dot_y));
        }
    }

    fn update_compass(&mut self) {
        let yaw = if let Some(ref p) = self.player {
            if let Ok(player) = p.clone().try_cast::<Player>() {
                player.bind().yaw()
            } else { 0.0 }
        } else { 0.0 };
        let tau = std::f32::consts::TAU;
        let sector = ((yaw.rem_euclid(tau) / tau * 8.0 + 0.5) as usize) % 8;
        let dirs = ["N", "NW", "W", "SW", "S", "SE", "E", "NE"];
        if let Some(ref mut lbl) = self.compass_label { lbl.set_text(dirs[sector]); }
    }

    // ── Анимация NPC ─────────────────────────────────────────────────────────

    fn tick_npc_anim(&mut self, dt: f32) {
        self.npc_anim_timer += dt;
        if self.npc_anim_timer < 1.0 / IDLE_FPS { return; }
        self.npc_anim_timer = 0.0;
        self.npc_anim_frame = (self.npc_anim_frame + 1) % NPC_IDLE_FRAMES.len();
        let (x, y, w, h) = NPC_IDLE_FRAMES[self.npc_anim_frame];
        let rect = Rect2::new(Vector2::new(x, y), Vector2::new(w, h));
        for sprite in self.npc_sprites.iter_mut() {
            sprite.set_region_rect(rect);
        }
    }

    // ── Флэш-сообщения ────────────────────────────────────────────────────────

    fn show_flash(&mut self, msg: &str) {
        if let Some(ref mut lbl) = self.flash_label {
            lbl.set_text(msg);
            lbl.set_visible(true);
            lbl.add_theme_color_override("font_color", C_GOLD);
        }
        self.flash_timer = 2.5;
    }

    fn tick_flash(&mut self, dt: f32) {
        if self.flash_timer > 0.0 {
            self.flash_timer -= dt;
            if self.flash_timer <= 0.0 {
                if let Some(ref mut lbl) = self.flash_label { lbl.set_visible(false); }
            } else {
                let alpha = if self.flash_timer < 0.8 { self.flash_timer / 0.8 } else { 1.0 };
                let c = Color::from_rgba(C_GOLD.r, C_GOLD.g, C_GOLD.b, alpha);
                if let Some(ref mut lbl) = self.flash_label {
                    lbl.add_theme_color_override("font_color", c);
                }
            }
        }
    }

    fn tick_damage_flash(&mut self, dt: f32) {
        if self.damage_flash_timer > 0.0 {
            self.damage_flash_timer -= dt;
            let alpha = (self.damage_flash_timer / 0.35).clamp(0.0, 1.0) * 0.42;
            let style = make_style(Color::from_rgba(0.8, 0.0, 0.0, alpha), Color::TRANSPARENT_BLACK, 0);
            if let Some(ref mut df) = self.damage_flash {
                df.add_theme_stylebox_override("panel", &style);
                df.set_visible(true);
            }
            if self.damage_flash_timer <= 0.0 {
                if let Some(ref mut df) = self.damage_flash { df.set_visible(false); }
            }
        }
    }

    // ── Сохранение ───────────────────────────────────────────────────────────

    fn auto_save(&mut self) {
        if let Some(ref state) = self.state {
            let hp = if let Some(ref p) = self.player {
                if let Ok(player) = p.clone().try_cast::<Player>() { player.bind().hp } else { 100.0 }
            } else { 100.0 };
            save::save(state, hp, &self.arsenal);
        }
    }
}
