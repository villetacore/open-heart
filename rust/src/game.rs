//! Game3D — главный узел 3D-мира.
//! Строит уровень (6 комнат + коридоры), управляет NPC/врагами/предметами, HUD.

use godot::prelude::*;
use godot::classes::{
    CanvasLayer, CharacterBody3D, CollisionShape3D, BoxShape3D,
    DirectionalLight3D, INode3D, Image, ImageTexture, Input,
    Label, MeshInstance3D, BoxMesh, StandardMaterial3D,
    Node3D, OmniLight3D, Panel, Sprite3D,
    StaticBody3D, StyleBoxFlat, Texture2D, VBoxContainer,
};
use godot::classes::base_material_3d::{BillboardMode, TextureParam};
use godot::classes::sprite_base_3d::AlphaCutMode;
use godot::global::HorizontalAlignment;

use crate::config::GameConfig;
use crate::dialogue::Scene;
use crate::enemy::Enemy;
use crate::game_state::GameState;
use crate::locale::t;
use crate::player::Player;
use crate::save;
use crate::settings::Settings;
use crate::story::get_scene;

// ── NPC-конфигурация ─────────────────────────────────────────────────────────

struct NpcCfg {
    id:       &'static str,
    name:     &'static str,
    scene_id: &'static str,
    pos:      Vector3,
    color:    Color,
    tex:      &'static str,
}

// Пути: assets/sprites/characters/npc_<id>.png (новые) с fallback на старые.
fn npc_sprite_tex(id: &str) -> (&'static str, &'static str) {
    match id {
        "vale"       => ("res://assets/sprites/characters/npc_vale.png",       "res://assets/sprites/femboy_pink.png"),
        "victor"     => ("res://assets/sprites/characters/npc_victor.png",     "res://assets/sprites/femboy_dark2.png"),
        "elena"      => ("res://assets/sprites/characters/npc_elena.png",      "res://assets/sprites/femboy_dark1.png"),
        "sofia"      => ("res://assets/sprites/characters/npc_sofia.png",      "res://assets/sprites/femboy_pink.png"),
        "guard"      => ("res://assets/sprites/characters/npc_guard.png",      "res://assets/sprites/femboy_dark2.png"),
        "merchant"   => ("res://assets/sprites/characters/npc_merchant.png",   "res://assets/sprites/femboy_pink.png"),
        "scientist"  => ("res://assets/sprites/characters/npc_scientist.png",  "res://assets/sprites/femboy_dark1.png"),
        "stranger"   => ("res://assets/sprites/characters/npc_stranger.png",   "res://assets/sprites/femboy_dark2.png"),
        _            => ("res://assets/sprites/femboy_dark1.png",              "res://assets/sprites/femboy_dark1.png"),
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
        _              => "",
    }
}

const NPC_DATA: &[NpcCfg] = &[
    NpcCfg {
        id: "vale", name: "Ms. Вейл", scene_id: "meet_vale",
        pos: Vector3::new(-5.0, 0.0, -7.0),
        color: Color::from_rgba(1.0, 0.5, 0.7, 1.0),
        tex: "res://assets/sprites/femboy_pink.png",
    },
    NpcCfg {
        id: "victor", name: "Виктор", scene_id: "intro_victor",
        pos: Vector3::new(5.0, 0.0, -7.0),
        color: Color::from_rgba(0.4, 0.8, 0.5, 1.0),
        tex: "res://assets/sprites/femboy_dark2.png",
    },
    NpcCfg {
        id: "elena", name: "Елена", scene_id: "first_elena",
        pos: Vector3::new(-4.0, 0.0, -26.0),
        color: Color::from_rgba(0.4, 0.5, 0.9, 1.0),
        tex: "res://assets/sprites/femboy_dark1.png",
    },
    NpcCfg {
        id: "sofia", name: "София", scene_id: "meet_sofia",
        pos: Vector3::new(0.0, 0.0, 27.0),
        color: Color::from_rgba(0.9, 0.8, 0.3, 1.0),
        tex: "res://assets/sprites/femboy_pink.png",
    },
    NpcCfg {
        id: "guard", name: "Охранник", scene_id: "meet_guard",
        pos: Vector3::new(14.0, 0.0, 0.0),
        color: Color::from_rgba(0.6, 0.6, 0.6, 1.0),
        tex: "res://assets/sprites/femboy_dark2.png",
    },
    NpcCfg {
        id: "merchant", name: "Торговец", scene_id: "meet_merchant",
        pos: Vector3::new(26.0, 0.0, -5.0),
        color: Color::from_rgba(0.9, 0.65, 0.2, 1.0),
        tex: "res://assets/sprites/femboy_pink.png",
    },
    NpcCfg {
        id: "scientist", name: "Учёный", scene_id: "meet_scientist",
        pos: Vector3::new(-26.0, 0.0, -5.0),
        color: Color::from_rgba(0.3, 0.9, 0.9, 1.0),
        tex: "res://assets/sprites/femboy_dark1.png",
    },
    NpcCfg {
        id: "stranger", name: "Незнакомец", scene_id: "meet_stranger",
        pos: Vector3::new(0.0, 0.0, -43.0),
        color: Color::from_rgba(0.5, 0.3, 0.7, 1.0),
        tex: "res://assets/sprites/femboy_dark2.png",
    },
];

// ── Режим игры ────────────────────────────────────────────────────────────────

#[derive(PartialEq, Clone, Copy)]
enum Mode { Explore, Dialogue, Dead, Inventory }

// ── Предмет в мире ────────────────────────────────────────────────────────────

struct WorldItemNode {
    node:    Gd<StaticBody3D>,
    item_id: String,
    name:    String,
    heal:    Option<f32>,
    gold:    i32,
}

// ── Главная структура ─────────────────────────────────────────────────────────

#[derive(GodotClass)]
#[class(base = Node3D)]
pub struct Game3D {
    base: Base<Node3D>,

    player:      Option<Gd<CharacterBody3D>>,
    npc_sprites: Vec<Gd<Sprite3D>>,
    enemies:     Vec<Gd<Enemy>>,
    world_items: Vec<WorldItemNode>,

    state:       Option<GameState>,
    settings:    Settings,
    mode:        Mode,
    scene:       Option<Scene>,
    line_idx:    usize,
    near_npc:    Option<usize>,
    near_enemy:  Option<usize>,
    near_item:   Option<usize>,
    at_choices:  bool,

    shoot_cd:    f32,

    npc_anim_timer: f32,
    npc_anim_frame: usize,

    // HUD виджеты
    hint_label:      Option<Gd<Label>>,
    hp_bar_bg:       Option<Gd<Panel>>,
    hp_bar_fg:       Option<Gd<Panel>>,
    hp_label:        Option<Gd<Label>>,
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
    crosshair:       Option<Gd<Label>>,
    dead_panel:      Option<Gd<Panel>>,
    compass_label:   Option<Gd<Label>>,
    targeting_label: Option<Gd<Label>>,
    damage_flash:    Option<Gd<Panel>>,
    damage_flash_timer: f32,
    weapon_sprite:   Option<Gd<Sprite3D>>,  // HUD-оружие (2D overlay через 3D нет — будет Sprite2D)
    game_time:       f32,
}

// ── Константы ─────────────────────────────────────────────────────────────────

const WALL_H: f32     = 3.2;
const INTERACT_R: f32 = 2.8;
const PICKUP_R: f32   = 1.4;
const SHOOT_RANGE: f32 = 18.0;
const SHOOT_CD: f32    = 0.4;
const PIXEL_SZ: f32   = 0.010;
const HUD_W: f32      = 1920.0;
const HUD_H: f32      = 1080.0;

// Стандартный формат: 512×256, 4 фрейма 128×256 (idle_0|idle_1|walk_0|walk_1)
const NPC_FRAME_W: f32 = 128.0;
const NPC_FRAME_H: f32 = 256.0;
const NPC_IDLE_FRAMES: [(f32,f32,f32,f32); 2] = [
    (0.0,   0.0, 128.0, 256.0),
    (128.0, 0.0, 128.0, 256.0),
];
const IDLE_FPS: f32 = 3.0;

const C_WALL:   Color = Color::from_rgba(0.10, 0.06, 0.08, 1.0);
const C_FLOOR:  Color = Color::from_rgba(0.08, 0.04, 0.06, 1.0);
const C_CEIL:   Color = Color::from_rgba(0.04, 0.02, 0.04, 1.0);
const C_UI_BG:  Color = Color::from_rgba(0.04, 0.03, 0.07, 0.94);
const C_BORDER: Color = Color::from_rgba(0.65, 0.30, 0.52, 1.0);
const C_MAIN:   Color = Color::from_rgba(0.95, 0.92, 0.98, 1.0);
const C_DIM:    Color = Color::from_rgba(0.58, 0.52, 0.66, 1.0);
const C_PINK:   Color = Color::from_rgba(1.00, 0.55, 0.80, 1.0);
const C_GOLD:   Color = Color::from_rgba(1.00, 0.84, 0.30, 1.0);
const C_RED:    Color = Color::from_rgba(0.90, 0.15, 0.15, 1.0);
const C_GREEN:  Color = Color::from_rgba(0.20, 0.85, 0.30, 1.0);
const C_CYAN:   Color = Color::from_rgba(0.40, 0.90, 1.00, 1.0);

// ── INode3D ───────────────────────────────────────────────────────────────────

#[godot_api]
impl INode3D for Game3D {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            player: None, npc_sprites: Vec::new(),
            enemies: Vec::new(), world_items: Vec::new(),
            state: None, settings: Settings::default(),
            mode: Mode::Explore, scene: None, line_idx: 0,
            near_npc: None, near_enemy: None, near_item: None,
            at_choices: false, shoot_cd: 0.0,
            npc_anim_timer: 0.0, npc_anim_frame: 0,
            hint_label: None,
            hp_bar_bg: None, hp_bar_fg: None, hp_label: None,
            dlg_panel: None, dlg_speaker: None, dlg_text: None,
            choice_box: None,
            cl0: None, cl1: None, cl2: None, cl3: None,
            flash_label: None, flash_timer: 0.0,
            inv_label: None, quest_label: None,
            inv_panel: None, inv_list: None,
            crosshair: None, dead_panel: None,
            compass_label: None, targeting_label: None,
            damage_flash: None, damage_flash_timer: 0.0,
            weapon_sprite: None, game_time: 0.0,
        }
    }

    fn ready(&mut self) {
        self.settings = Settings::load();
        let lang = self.settings.lang.clone();

        let (state, player_hp) = if let Some((st, hp)) = save::load() {
            (st, hp)
        } else {
            (GameState::new("Игрок"), 100.0)
        };
        self.state = Some(state);

        let cfg = GameConfig::load();
        self.build_level();
        self.build_lighting();
        self.build_npcs();
        self.build_enemies(&cfg);
        self.build_world_items(&cfg);
        self.build_hud(&lang);

        let player_gd = self.base().get_node_as::<CharacterBody3D>("Player");
        if let Ok(mut p) = player_gd.clone().try_cast::<Player>() {
            p.bind_mut().hp = player_hp;
        }
        self.player = Some(player_gd);
    }

    fn process(&mut self, delta: f64) {
        let dt = delta as f32;
        self.game_time += dt;
        self.shoot_cd = (self.shoot_cd - dt).max(0.0);
        self.tick_flash(dt);
        self.tick_damage_flash(dt);
        self.tick_npc_anim(dt);
        self.tick_items(dt);
        self.collect_enemy_damage(dt);
        self.update_compass();

        match self.mode {
            Mode::Explore   => self.process_explore(),
            Mode::Dialogue  => self.process_dialogue(),
            Mode::Dead      => {}
            Mode::Inventory => self.process_inventory(),
        }

        self.update_hp_bar();
        self.update_targeting_hud();
    }
}

// ── Строительство уровня ──────────────────────────────────────────────────────

impl Game3D {
    fn build_level(&mut self) {
        const H: f32  = WALL_H;
        const T: f32  = 0.22;  // толщина стены
        const DW: f32 = 5.0;   // ширина двери

        let world = Image::load_from_file("res://assets/textures_raw/world_complete.png");
        let img   = world.as_ref();
        let tf  = crop_tex(img, 1344, 640, 192, 128); // пол
        let tc  = crop_tex(img,  576,   0, 192, 128); // потолок
        let tw1 = crop_tex(img,    0,   0, 192, 128); // стена 1
        let tw2 = crop_tex(img,    0, 640, 192, 128); // стена 2
        let tw3 = crop_tex(img,  192, 640, 192, 128); // стена 3
        let tw4 = crop_tex(img, 1152, 768, 192, 128); // стена 4

        // ── ЦЕНТРАЛЬНЫЙ ЗАЛ (0,0) 20×20 ─────────────────────────────────────
        self.add_fc(0.0, 0.0, 20.0, 20.0, H, tf.as_ref(), tc.as_ref());
        self.add_wh(0.0, -10.0, 20.0, H, T, DW, tw1.as_ref());  // N стена, дверь
        self.add_wh(0.0,  10.0, 20.0, H, T, DW, tw1.as_ref());  // S стена, дверь
        self.add_wv(10.0,  0.0, 20.0, H, T, DW, tw1.as_ref());  // E стена, дверь
        self.add_wv(-10.0, 0.0, 20.0, H, T, DW, tw1.as_ref()); // W стена, дверь
        // Колонны
        for (px, pz) in [(-8.0f32,-8.0),(8.0,-8.0),(-8.0,8.0),(8.0,8.0)] {
            let p = make_box(Vector3::new(px, H*0.5, pz), Vector3::new(0.75, H, 0.75), C_WALL, None, 1.0);
            self.base_mut().add_child(&p);
        }
        // Центральный пьедестал
        let ped = make_box(Vector3::new(0.0, 0.2, 0.0), Vector3::new(2.0, 0.4, 2.0), C_WALL, None, 1.0);
        self.base_mut().add_child(&ped);
        // Диагональные срезы углов (сглаживают 90° стыки)
        use std::f32::consts::FRAC_PI_4;
        self.add_diag(-9.3, -9.3, 2.8, -FRAC_PI_4, tw1.as_ref());
        self.add_diag( 9.3, -9.3, 2.8,  FRAC_PI_4, tw1.as_ref());
        self.add_diag(-9.3,  9.3, 2.8,  FRAC_PI_4, tw1.as_ref());
        self.add_diag( 9.3,  9.3, 2.8, -FRAC_PI_4, tw1.as_ref());
        // Обломки у северной стены
        self.add_rubble(-6.0, -8.5);
        self.add_rubble( 6.0, -8.0);

        // ── N КОРИДОР (0, -14) 5×8 ───────────────────────────────────────────
        self.add_fc(0.0, -14.0, DW, 8.0, H, tf.as_ref(), tc.as_ref());
        let c = make_box(Vector3::new( 2.75, H*0.5, -14.0), Vector3::new(T, H, 8.0), C_WALL, tw1.as_ref(), 1.0);
        self.base_mut().add_child(&c);
        let c = make_box(Vector3::new(-2.75, H*0.5, -14.0), Vector3::new(T, H, 8.0), C_WALL, tw1.as_ref(), 1.0);
        self.base_mut().add_child(&c);

        // ── АРХИВ (0, -26) 24×16 ─────────────────────────────────────────────
        self.add_fc(0.0, -26.0, 24.0, 16.0, H, tf.as_ref(), tc.as_ref());
        self.add_wh( 0.0, -18.0, 24.0, H, T, DW, tw2.as_ref()); // S стена, дверь
        self.add_wh( 0.0, -34.0, 24.0, H, T, DW, tw2.as_ref()); // N стена, дверь
        self.add_wv(-12.0, -26.0, 16.0, H, T, 0.0, tw2.as_ref()); // W
        self.add_wv( 12.0, -26.0, 16.0, H, T, 0.0, tw2.as_ref()); // E
        // Стеллажи (слегка нерегулярно расставленные)
        for (px, pz) in [(-9.0f32,-26.5f32),(-4.5,-25.0),(4.5,-27.0),(9.0,-26.0)] {
            let sh = make_box(Vector3::new(px, 0.9, pz), Vector3::new(1.1, 1.8, 3.0), C_WALL, None, 1.0);
            self.base_mut().add_child(&sh);
        }
        // Читальный подиум (+1 ступень)
        self.add_step(0.0, -30.0, 5.0, 3.0, 0.35, tf.as_ref());
        let desk = make_box(Vector3::new(0.0, 0.35+0.35, -30.0), Vector3::new(2.5, 0.35, 1.2), C_WALL, None, 1.0);
        self.base_mut().add_child(&desk);
        // Ниша в восточной стене архива
        self.add_fc(15.0, -26.0, 3.0, 4.0, H, tf.as_ref(), tc.as_ref());
        let n1 = make_box(Vector3::new(15.0, H*0.5, -24.0), Vector3::new(3.0, H, T), C_WALL, tw2.as_ref(), 1.0);
        self.base_mut().add_child(&n1);
        let n2 = make_box(Vector3::new(15.0, H*0.5, -28.0), Vector3::new(3.0, H, T), C_WALL, tw2.as_ref(), 1.0);
        self.base_mut().add_child(&n2);
        // Диагональный угол у NW
        self.add_diag(-10.5, -33.2, 2.4, FRAC_PI_4, tw2.as_ref());

        // ── N2 КОРИДОР (0, -38) 5×8 ──────────────────────────────────────────
        self.add_fc(0.0, -38.0, DW, 8.0, H, tf.as_ref(), tc.as_ref());
        let c = make_box(Vector3::new( 2.75, H*0.5, -38.0), Vector3::new(T, H, 8.0), C_WALL, tw3.as_ref(), 1.0);
        self.base_mut().add_child(&c);
        let c = make_box(Vector3::new(-2.75, H*0.5, -38.0), Vector3::new(T, H, 8.0), C_WALL, tw3.as_ref(), 1.0);
        self.base_mut().add_child(&c);

        // ── ТРОННЫЙ ЗАЛ (0, -48) 18×12 ──────────────────────────────────────
        self.add_fc(0.0, -48.0, 18.0, 12.0, H, tf.as_ref(), tc.as_ref());
        self.add_wh( 0.0, -42.0, 18.0, H, T, DW, tw3.as_ref()); // S дверь
        self.add_wh( 0.0, -54.0, 18.0, H, T, 0.0, tw3.as_ref()); // N глухая
        self.add_wv(-9.0, -48.0, 12.0, H, T, 0.0, tw3.as_ref()); // W
        self.add_wv( 9.0, -48.0, 12.0, H, T, 0.0, tw3.as_ref()); // E
        // Алтарь на поднятой платформе (2 ступени)
        self.add_step(0.0, -50.5, 7.0, 4.5, 0.32, tf.as_ref());
        self.add_step(0.0, -51.5, 4.5, 3.0, 0.64, tf.as_ref());
        let alt = make_box(Vector3::new(0.0, 0.64+0.45, -52.0), Vector3::new(3.5, 0.7, 1.5), C_WALL, None, 1.0);
        self.base_mut().add_child(&alt);
        // Укрытия асимметрично
        for (bx, bz, bw, bd) in [(-5.5f32,-44.5f32,1.2f32,0.9f32),(5.5,-44.5,0.9,1.2),(-4.2,-49.5,1.0,0.9),(4.6,-48.8,0.8,1.0)] {
            let b = make_box(Vector3::new(bx, 0.5, bz), Vector3::new(bw, 1.0, bd), C_WALL, None, 1.0);
            self.base_mut().add_child(&b);
        }
        // Диагональные углы тронного зала
        self.add_diag(-7.8, -42.8, 2.2, -FRAC_PI_4, tw3.as_ref());
        self.add_diag( 7.8, -42.8, 2.2,  FRAC_PI_4, tw3.as_ref());
        self.add_diag(-7.8, -53.2, 2.2,  FRAC_PI_4, tw3.as_ref());
        self.add_diag( 7.8, -53.2, 2.2, -FRAC_PI_4, tw3.as_ref());

        // ── E КОРИДОР (14, 0) 8×5 ────────────────────────────────────────────
        self.add_fc(14.0, 0.0, 8.0, DW, H, tf.as_ref(), tc.as_ref());
        let c = make_box(Vector3::new(14.0, H*0.5,  2.75), Vector3::new(8.0, H, T), C_WALL, tw1.as_ref(), 1.0);
        self.base_mut().add_child(&c);
        let c = make_box(Vector3::new(14.0, H*0.5, -2.75), Vector3::new(8.0, H, T), C_WALL, tw1.as_ref(), 1.0);
        self.base_mut().add_child(&c);

        // ── ВОСТОЧНЫЙ РЫНОК (26, 0) 16×22 ────────────────────────────────────
        self.add_fc(26.0, 0.0, 16.0, 22.0, H, tf.as_ref(), tc.as_ref());
        self.add_wv(18.0,  0.0, 22.0, H, T, DW, tw4.as_ref()); // W дверь
        self.add_wv(34.0,  0.0, 22.0, H, T, 0.0, tw4.as_ref());
        self.add_wh(26.0, -11.0, 16.0, H, T, 0.0, tw4.as_ref());
        self.add_wh(26.0,  11.0, 16.0, H, T, 0.0, tw4.as_ref());
        // Торговые прилавки — нерегулярно, разные высоты
        for (bx, bz, bw, bh, bd) in [
            (21.5f32, 6.5f32,  2.2, 1.2, 1.3),
            (21.5,   -6.5,     2.0, 0.9, 1.5),
            (30.5,    6.0,     2.4, 1.1, 1.2),
            (30.5,   -5.5,     1.8, 1.4, 1.0),
            (25.8,    0.5,     2.0, 1.0, 1.4),
            (27.5,    2.5,     1.4, 1.3, 0.9),
        ] {
            let s = make_box(Vector3::new(bx, bh*0.5, bz), Vector3::new(bw, bh, bd), C_WALL, None, 1.0);
            self.base_mut().add_child(&s);
        }
        // Приподнятая витрина у восточной стены
        self.add_step(33.0, 0.0, 1.5, 8.0, 0.45, tf.as_ref());
        // Диагональный срез у входа
        self.add_diag(18.8, -2.4, 3.0, FRAC_PI_4, tw4.as_ref());

        // ── W КОРИДОР (-14, 0) 8×5 ───────────────────────────────────────────
        self.add_fc(-14.0, 0.0, 8.0, DW, H, tf.as_ref(), tc.as_ref());
        let c = make_box(Vector3::new(-14.0, H*0.5,  2.75), Vector3::new(8.0, H, T), C_WALL, tw2.as_ref(), 1.0);
        self.base_mut().add_child(&c);
        let c = make_box(Vector3::new(-14.0, H*0.5, -2.75), Vector3::new(8.0, H, T), C_WALL, tw2.as_ref(), 1.0);
        self.base_mut().add_child(&c);

        // ── ЗАПАДНАЯ ЛАБОРАТОРИЯ (-26, 0) 16×22 ──────────────────────────────
        self.add_fc(-26.0, 0.0, 16.0, 22.0, H, tf.as_ref(), tc.as_ref());
        self.add_wv(-18.0,  0.0, 22.0, H, T, DW, tw2.as_ref()); // E дверь
        self.add_wv(-34.0,  0.0, 22.0, H, T, 0.0, tw2.as_ref());
        self.add_wh(-26.0, -11.0, 16.0, H, T, 0.0, tw2.as_ref());
        self.add_wh(-26.0,  11.0, 16.0, H, T, 0.0, tw2.as_ref());
        // Оборудование лаборатории — под разными углами
        for (bx, bz, bw, bh, bd, ry) in [
            (-22.0f32, -7.0f32, 1.4, 1.2, 0.8, 0.0f32),
            (-30.0,    -7.0,    1.6, 0.9, 0.9, 0.3),
            (-22.0,     7.0,    1.3, 1.1, 0.8, -0.2),
            (-30.0,     7.0,    1.5, 1.0, 1.0, 0.1),
            (-26.0,     0.0,    2.0, 1.3, 0.7, 0.0),
            (-24.5,    -3.5,    1.0, 0.8, 1.4, 0.4),
            (-27.5,     3.5,    1.2, 1.4, 0.7, -0.3),
        ] {
            let mut body = StaticBody3D::new_alloc();
            body.set_position(Vector3::new(bx, bh*0.5, bz));
            body.set_rotation(Vector3::new(0.0, ry, 0.0));
            let mut mi = MeshInstance3D::new_alloc();
            let mut mesh = BoxMesh::new_gd(); mesh.set_size(Vector3::new(bw, bh, bd)); mi.set_mesh(&mesh);
            let mut mat = StandardMaterial3D::new_gd(); mat.set_albedo(C_WALL); mi.set_surface_override_material(0, &mat);
            let mut col = CollisionShape3D::new_alloc(); let mut sh = BoxShape3D::new_gd(); sh.set_size(Vector3::new(bw, bh, bd)); col.set_shape(&sh);
            body.add_child(&mi); body.add_child(&col);
            self.base_mut().add_child(&body);
        }
        // Наблюдательная площадка
        self.add_step(-26.0, 9.5, 7.0, 4.0, 0.55, tf.as_ref());
        // Диагональный вход
        self.add_diag(-18.8, 2.4, 3.0, -FRAC_PI_4, tw2.as_ref());

        // ── S КОРИДОР (0, 14) 5×8 ────────────────────────────────────────────
        self.add_fc(0.0, 14.0, DW, 8.0, H, tf.as_ref(), tc.as_ref());
        let c = make_box(Vector3::new( 2.75, H*0.5, 14.0), Vector3::new(T, H, 8.0), C_WALL, tw4.as_ref(), 1.0);
        self.base_mut().add_child(&c);
        let c = make_box(Vector3::new(-2.75, H*0.5, 14.0), Vector3::new(T, H, 8.0), C_WALL, tw4.as_ref(), 1.0);
        self.base_mut().add_child(&c);

        // ── ЮЖНАЯ АРЕНА (0, 27) 24×18 ────────────────────────────────────────
        self.add_fc(0.0, 27.0, 24.0, 18.0, H, tf.as_ref(), tc.as_ref());
        self.add_wh( 0.0, 18.0, 24.0, H, T, DW, tw4.as_ref()); // N дверь
        self.add_wh( 0.0, 36.0, 24.0, H, T, 0.0, tw4.as_ref());
        self.add_wv(-12.0, 27.0, 18.0, H, T, 0.0, tw4.as_ref());
        self.add_wv( 12.0, 27.0, 18.0, H, T, 0.0, tw4.as_ref());
        // Двухуровневая арена — центральный ринг приподнят
        self.add_step(0.0, 27.0, 10.0, 8.0, 0.28, tf.as_ref());
        // Укрытия — асимметрично и разные размеры
        for (bx, bz, bw, bh, bd) in [
            (-8.0f32, 22.5f32, 1.0, 0.95, 1.2),
            ( 8.5,    22.8,    1.3, 0.85, 0.9),
            (-8.2,    31.0,    0.9, 1.10, 1.0),
            ( 7.8,    30.5,    1.1, 0.90, 1.3),
            ( 0.0,    27.2,    1.5, 0.75, 0.9),
            (-4.2,    24.3,    0.8, 1.00, 1.0),
            ( 4.5,    24.0,    1.0, 0.85, 0.8),
            (-2.5,    30.0,    0.9, 1.20, 0.8),
            ( 3.0,    28.5,    0.7, 0.95, 1.0),
        ] {
            let cov = make_box(Vector3::new(bx, bh*0.5, bz), Vector3::new(bw, bh, bd), C_WALL, None, 1.0);
            self.base_mut().add_child(&cov);
        }
        // Диагонали у входа арены
        self.add_diag(-2.4, 18.8, 3.0, -FRAC_PI_4, tw4.as_ref());
        self.add_diag( 2.4, 18.8, 3.0,  FRAC_PI_4, tw4.as_ref());
        // Обломки
        self.add_rubble(10.0, 33.5);
        self.add_rubble(-10.5, 33.0);
    }

    // ── Помощники для стен/полов ───────────────────────────────────────────────

    fn add_fc(&mut self, cx: f32, cz: f32, w: f32, d: f32, h: f32,
              tf: Option<&Gd<Texture2D>>, tc: Option<&Gd<Texture2D>>) {
        let fl = make_box(Vector3::new(cx, -0.1, cz), Vector3::new(w, 0.2, d), C_FLOOR, tf, 4.0);
        self.base_mut().add_child(&fl);
        let ce = make_box(Vector3::new(cx, h+0.1, cz), Vector3::new(w, 0.2, d), C_CEIL, tc, 4.0);
        self.base_mut().add_child(&ce);
    }

    // Горизонтальная стена (вдоль X) на позиции z, центр cx, ширина width.
    fn add_wh(&mut self, cx: f32, z: f32, width: f32, h: f32, t: f32,
              door_w: f32, tex: Option<&Gd<Texture2D>>) {
        if door_w <= 0.0 {
            let w = make_box(Vector3::new(cx, h*0.5, z), Vector3::new(width, h, t), C_WALL, tex, 2.0);
            self.base_mut().add_child(&w);
        } else {
            let side = (width - door_w) * 0.5;
            if side > 0.05 {
                let l = make_box(
                    Vector3::new(cx - width*0.5 + side*0.5, h*0.5, z),
                    Vector3::new(side, h, t), C_WALL, tex, 2.0,
                );
                self.base_mut().add_child(&l);
                let r = make_box(
                    Vector3::new(cx + width*0.5 - side*0.5, h*0.5, z),
                    Vector3::new(side, h, t), C_WALL, tex, 2.0,
                );
                self.base_mut().add_child(&r);
            }
        }
    }

    // Диагональная стена (box повёрнут на rot_y радиан вокруг Y).
    fn add_diag(&mut self, cx: f32, cz: f32, len: f32, rot_y: f32, tex: Option<&Gd<Texture2D>>) {
        const H: f32 = WALL_H;
        const T: f32 = 0.22;
        let mut body = StaticBody3D::new_alloc();
        body.set_position(Vector3::new(cx, H * 0.5, cz));
        body.set_rotation(Vector3::new(0.0, rot_y, 0.0));

        let mut mi = MeshInstance3D::new_alloc();
        let mut mesh = BoxMesh::new_gd();
        mesh.set_size(Vector3::new(len, H, T));
        mi.set_mesh(&mesh);
        let mut mat = StandardMaterial3D::new_gd();
        if let Some(t) = tex { mat.set_albedo(Color::WHITE); mat.set_texture(TextureParam::ALBEDO, t); }
        else { mat.set_albedo(C_WALL); }
        mi.set_surface_override_material(0, &mat);

        let mut col = CollisionShape3D::new_alloc();
        let mut shape = BoxShape3D::new_gd();
        shape.set_size(Vector3::new(len, H, T));
        col.set_shape(&shape);

        body.add_child(&mi);
        body.add_child(&col);
        self.base_mut().add_child(&body);
    }

    // Поднятая платформа-ступень (w×rh×d, верхняя поверхность на высоте rh).
    fn add_step(&mut self, cx: f32, cz: f32, w: f32, d: f32, rh: f32, tex: Option<&Gd<Texture2D>>) {
        let b = make_box(Vector3::new(cx, rh * 0.5, cz), Vector3::new(w, rh, d), C_WALL, tex, 1.0);
        self.base_mut().add_child(&b);
    }

    // Куча обломков из нескольких маленьких ящиков (псевдо-случайный паттерн).
    fn add_rubble(&mut self, cx: f32, cz: f32) {
        let pieces: &[(f32, f32, f32, f32, f32)] = &[
            ( 0.00,  0.00, 0.55, 0.32, 0.40),
            ( 0.55,  0.28, 0.38, 0.22, 0.30),
            (-0.48,  0.18, 0.30, 0.18, 0.35),
            ( 0.22, -0.55, 0.42, 0.28, 0.25),
            (-0.30, -0.25, 0.25, 0.38, 0.22),
            ( 0.65, -0.15, 0.20, 0.15, 0.20),
        ];
        for (ox, oz, sw, sh, sd) in pieces {
            let b = make_box(
                Vector3::new(cx + ox, sh * 0.5, cz + oz),
                Vector3::new(*sw, *sh, *sd),
                C_WALL, None, 1.0,
            );
            self.base_mut().add_child(&b);
        }
    }

    // Вертикальная стена (вдоль Z) на позиции x, центр cz, глубина depth.
    fn add_wv(&mut self, x: f32, cz: f32, depth: f32, h: f32, t: f32,
              door_w: f32, tex: Option<&Gd<Texture2D>>) {
        if door_w <= 0.0 {
            let w = make_box(Vector3::new(x, h*0.5, cz), Vector3::new(t, h, depth), C_WALL, tex, 2.0);
            self.base_mut().add_child(&w);
        } else {
            let side = (depth - door_w) * 0.5;
            if side > 0.05 {
                let t1 = make_box(
                    Vector3::new(x, h*0.5, cz - depth*0.5 + side*0.5),
                    Vector3::new(t, h, side), C_WALL, tex, 2.0,
                );
                self.base_mut().add_child(&t1);
                let t2 = make_box(
                    Vector3::new(x, h*0.5, cz + depth*0.5 - side*0.5),
                    Vector3::new(t, h, side), C_WALL, tex, 2.0,
                );
                self.base_mut().add_child(&t2);
            }
        }
    }

    fn build_lighting(&mut self) {
        use godot::classes::light_3d::Param;

        let mut dir = DirectionalLight3D::new_alloc();
        dir.set_rotation(Vector3::new(-1.0, 0.4, 0.0));
        dir.set_param(Param::ENERGY, 0.25);
        dir.set_color(Color::from_rgba(1.0, 0.88, 0.92, 1.0));
        dir.set_shadow(false);
        self.base_mut().add_child(&dir);

        // Свет в каждой комнате
        let lights: &[(Vector3, f32, Color, f32)] = &[
            (Vector3::new(  0.0, 2.8,   0.0), 2.4, Color::from_rgba(0.85, 0.30, 0.48, 1.0), 26.0), // зал
            (Vector3::new(  0.0, 2.8, -26.0), 1.8, Color::from_rgba(0.60, 0.60, 0.95, 1.0), 22.0), // архив
            (Vector3::new(  0.0, 2.5, -48.0), 2.8, Color::from_rgba(0.95, 0.10, 0.12, 1.0), 18.0), // трон
            (Vector3::new( 26.0, 2.8,   0.0), 2.0, Color::from_rgba(1.00, 0.80, 0.35, 1.0), 20.0), // рынок
            (Vector3::new(-26.0, 2.8,   0.0), 1.8, Color::from_rgba(0.35, 0.95, 0.60, 1.0), 20.0), // лаб
            (Vector3::new(  0.0, 2.8,  27.0), 2.2, Color::from_rgba(0.95, 0.55, 0.20, 1.0), 24.0), // арена
            // Доп. точечные в центральном зале
            (Vector3::new(-9.0, 2.6, -9.0),   1.0, Color::from_rgba(1.0, 0.60, 0.75, 1.0), 14.0),
            (Vector3::new( 9.0, 2.6, -9.0),   1.0, Color::from_rgba(1.0, 0.60, 0.75, 1.0), 14.0),
            (Vector3::new(-9.0, 2.6,  9.0),   1.0, Color::from_rgba(1.0, 0.60, 0.75, 1.0), 14.0),
            (Vector3::new( 9.0, 2.6,  9.0),   1.0, Color::from_rgba(1.0, 0.60, 0.75, 1.0), 14.0),
        ];

        for (pos, energy, color, range) in lights {
            let mut omni = OmniLight3D::new_alloc();
            omni.set_position(*pos);
            omni.set_param(Param::RANGE, *range);
            omni.set_param(Param::ENERGY, *energy);
            omni.set_color(*color);
            self.base_mut().add_child(&omni);
        }
    }

    fn build_npcs(&mut self) {
        let mut sprites: Vec<Gd<Sprite3D>> = Vec::new();
        for cfg in NPC_DATA.iter() {
            let mut sprite = Sprite3D::new_alloc();
            sprite.set_position(cfg.pos + Vector3::new(0.0, 0.81, 0.0));
            sprite.set_pixel_size(PIXEL_SZ);
            sprite.set_billboard_mode(BillboardMode::ENABLED);

            let (new_path, fallback_path) = npc_sprite_tex(cfg.id);
            let loaded = Image::load_from_file(new_path)
                .or_else(|| Image::load_from_file(fallback_path));
            if let Some(img) = loaded {
                if let Some(itex) = ImageTexture::create_from_image(&img) {
                    sprite.set_texture(&itex.upcast::<Texture2D>());
                    sprite.set_region_enabled(true);
                    let (x, y, w, h) = NPC_IDLE_FRAMES[0];
                    sprite.set_region_rect(Rect2::new(Vector2::new(x,y), Vector2::new(w,h)));
                }
            }
            sprite.set_alpha_cut_mode(AlphaCutMode::DISCARD);
            sprite.set_modulate(cfg.color);
            self.base_mut().add_child(&sprite);
            sprites.push(sprite);
        }
        self.npc_sprites = sprites;
    }

    fn build_enemies(&mut self, cfg: &GameConfig) {
        for spawn in &cfg.level.spawn_enemies {
            let Some(ecfg) = cfg.enemy(&spawn.kind) else { continue };
            let mut e = Enemy::new_alloc();
            let pos = Vector3::new(spawn.x, 0.0, spawn.z);
            e.set_position(pos);
            self.base_mut().add_child(&e);
            let color = Color::from_rgba(ecfg.color_r, ecfg.color_g, ecfg.color_b, 1.0);
            e.bind_mut().configure(
                &ecfg.id, ecfg.hp, ecfg.speed, ecfg.attack_damage,
                ecfg.attack_range, ecfg.attack_cooldown, ecfg.chase_range,
                ecfg.patrol_radius, color, pos,
            );
            self.enemies.push(e);
        }
    }

    fn build_world_items(&mut self, cfg: &GameConfig) {
        for spawn in &cfg.level.spawn_items {
            let Some(icfg) = cfg.item(&spawn.kind) else { continue };
            let pos   = Vector3::new(spawn.x, 0.35, spawn.z);
            let color = Color::from_rgba(icfg.color_r, icfg.color_g, icfg.color_b, 1.0);

            // Пробуем billboard-спрайт; иначе — цветной куб
            let sprite_path = item_sprite_tex(&icfg.id);
            let has_sprite = !sprite_path.is_empty();

            let mut node = make_box(pos, Vector3::new(0.28, 0.28, 0.28), color, None, 1.0);
            self.base_mut().add_child(&node);

            if has_sprite {
                if let Some(img) = Image::load_from_file(sprite_path) {
                    if let Some(itex) = ImageTexture::create_from_image(&img) {
                        let mut sp = Sprite3D::new_alloc();
                        sp.set_position(Vector3::new(0.0, 0.0, 0.0));
                        sp.set_pixel_size(0.008);
                        sp.set_billboard_mode(BillboardMode::ENABLED);
                        sp.set_alpha_cut_mode(AlphaCutMode::DISCARD);
                        sp.set_texture(&itex.upcast::<Texture2D>());
                        sp.set_region_enabled(true);
                        // Один фрейм 64×64 из двухфреймового листа 128×64
                        sp.set_region_rect(Rect2::new(Vector2::ZERO, Vector2::new(64.0, 64.0)));
                        node.add_child(&sp);
                    }
                }
            }

            let gold = if icfg.category == "currency" { icfg.value } else { 0 };
            let name = self.item_name(icfg);
            self.world_items.push(WorldItemNode { node, item_id: icfg.id.clone(), name, heal: icfg.heal, gold });
        }
    }

    fn item_name(&self, icfg: &crate::config::ItemCfg) -> String {
        if self.settings.lang == "en" { icfg.name_en.clone() } else { icfg.name_ru.clone() }
    }
}

// ── HUD ───────────────────────────────────────────────────────────────────────

impl Game3D {
    fn build_hud(&mut self, lang: &str) {
        let mut layer = CanvasLayer::new_alloc();
        self.base_mut().add_child(&layer);

        // Урон-флэш (полный экран, красный)
        let mut df = Panel::new_alloc();
        df.set_position(Vector2::ZERO);
        df.set_size(Vector2::new(HUD_W, HUD_H));
        df.add_theme_stylebox_override("panel",
            &make_style(Color::from_rgba(0.8, 0.0, 0.0, 0.0), Color::TRANSPARENT_BLACK, 0));
        df.set_visible(false);
        layer.add_child(&df);
        self.damage_flash = Some(df);

        // Прицел
        let mut cross = Label::new_alloc();
        cross.set_text("+");
        cross.set_position(Vector2::new(HUD_W*0.5-8.0, HUD_H*0.5-12.0));
        cross.set_size(Vector2::new(16.0, 24.0));
        cross.set_horizontal_alignment(HorizontalAlignment::CENTER);
        cross.add_theme_font_size_override("font_size", 20);
        cross.add_theme_color_override("font_color", C_MAIN);
        layer.add_child(&cross);
        self.crosshair = Some(cross);

        // Таргетинг — HP врага над прицелом
        let mut tgt = Label::new_alloc();
        tgt.set_position(Vector2::new(HUD_W*0.5-200.0, HUD_H*0.5-46.0));
        tgt.set_size(Vector2::new(400.0, 24.0));
        tgt.set_horizontal_alignment(HorizontalAlignment::CENTER);
        tgt.add_theme_font_size_override("font_size", 13);
        tgt.add_theme_color_override("font_color", C_RED);
        tgt.set_visible(false);
        layer.add_child(&tgt);
        self.targeting_label = Some(tgt);

        // Компас (сверху по центру)
        let mut cmp = Label::new_alloc();
        cmp.set_text("N");
        cmp.set_position(Vector2::new(HUD_W*0.5-40.0, 10.0));
        cmp.set_size(Vector2::new(80.0, 30.0));
        cmp.set_horizontal_alignment(HorizontalAlignment::CENTER);
        cmp.add_theme_font_size_override("font_size", 18);
        cmp.add_theme_color_override("font_color", C_CYAN);
        layer.add_child(&cmp);
        self.compass_label = Some(cmp);

        // HP бар — фон
        let mut hp_bg = Panel::new_alloc();
        hp_bg.set_position(Vector2::new(24.0, HUD_H - 58.0));
        hp_bg.set_size(Vector2::new(222.0, 26.0));
        hp_bg.add_theme_stylebox_override("panel", &make_style(
            Color::from_rgba(0.08,0.01,0.01,0.92), Color::from_rgba(0.35,0.08,0.08,1.0), 1));
        layer.add_child(&hp_bg);
        self.hp_bar_bg = Some(hp_bg);

        // HP бар — заполнение
        let mut hp_fg = Panel::new_alloc();
        hp_fg.set_position(Vector2::new(26.0, HUD_H - 56.0));
        hp_fg.set_size(Vector2::new(218.0, 22.0));
        hp_fg.add_theme_stylebox_override("panel", &make_style(C_RED, Color::TRANSPARENT_BLACK, 0));
        layer.add_child(&hp_fg);
        self.hp_bar_fg = Some(hp_fg);

        // HP текст
        let mut hp_lbl = Label::new_alloc();
        hp_lbl.set_position(Vector2::new(24.0, HUD_H - 84.0));
        hp_lbl.set_size(Vector2::new(220.0, 24.0));
        hp_lbl.add_theme_font_size_override("font_size", 14);
        hp_lbl.add_theme_color_override("font_color", C_RED);
        layer.add_child(&hp_lbl);
        self.hp_label = Some(hp_lbl);

        // Подсказка взаимодействия
        let mut hint = Label::new_alloc();
        hint.set_position(Vector2::new(HUD_W*0.5-280.0, HUD_H-96.0));
        hint.set_size(Vector2::new(560.0, 28.0));
        hint.set_horizontal_alignment(HorizontalAlignment::CENTER);
        hint.add_theme_font_size_override("font_size", 16);
        hint.add_theme_color_override("font_color", C_GOLD);
        hint.set_visible(false);
        layer.add_child(&hint);
        self.hint_label = Some(hint);

        // Инвентарь (строка вверху-справа)
        let mut inv = Label::new_alloc();
        inv.set_position(Vector2::new(HUD_W-420.0, 10.0));
        inv.set_size(Vector2::new(408.0, 24.0));
        inv.set_horizontal_alignment(HorizontalAlignment::RIGHT);
        inv.add_theme_font_size_override("font_size", 13);
        inv.add_theme_color_override("font_color", C_DIM);
        layer.add_child(&inv);
        self.inv_label = Some(inv);

        // Журнал квестов (левый верх)
        let mut ql = Label::new_alloc();
        ql.set_position(Vector2::new(24.0, 44.0));
        ql.set_size(Vector2::new(360.0, 150.0));
        ql.add_theme_font_size_override("font_size", 13);
        ql.add_theme_color_override("font_color", C_DIM);
        ql.set_autowrap_mode(godot::classes::text_server::AutowrapMode::WORD);
        layer.add_child(&ql);
        self.quest_label = Some(ql);

        // Флэш-сообщение
        let mut flash = Label::new_alloc();
        flash.set_position(Vector2::new(HUD_W*0.5-300.0, HUD_H*0.5-80.0));
        flash.set_size(Vector2::new(600.0, 34.0));
        flash.set_horizontal_alignment(HorizontalAlignment::CENTER);
        flash.add_theme_font_size_override("font_size", 18);
        flash.add_theme_color_override("font_color", C_GOLD);
        flash.set_visible(false);
        layer.add_child(&flash);
        self.flash_label = Some(flash);

        // Инвентарный экран
        {
            let pw = 720.0; let ph = 520.0;
            let mut ip = Panel::new_alloc();
            ip.set_position(Vector2::new((HUD_W-pw)*0.5, (HUD_H-ph)*0.5));
            ip.set_size(Vector2::new(pw, ph));
            ip.add_theme_stylebox_override("panel", &make_style(C_UI_BG, C_BORDER, 2));
            ip.set_visible(false);

            let mut title = Label::new_alloc();
            title.set_text(t("inv_title", lang));
            title.set_position(Vector2::new(24.0, 16.0));
            title.set_size(Vector2::new(pw-48.0, 32.0));
            title.add_theme_font_size_override("font_size", 22);
            title.add_theme_color_override("font_color", C_PINK);
            ip.add_child(&title);

            let mut il = Label::new_alloc();
            il.set_position(Vector2::new(24.0, 60.0));
            il.set_size(Vector2::new(pw-48.0, ph-110.0));
            il.add_theme_font_size_override("font_size", 15);
            il.add_theme_color_override("font_color", C_MAIN);
            il.set_autowrap_mode(godot::classes::text_server::AutowrapMode::WORD);
            ip.add_child(&il);

            let mut hint_i = Label::new_alloc();
            hint_i.set_text(t("inv_close", lang));
            hint_i.set_position(Vector2::new(24.0, ph-42.0));
            hint_i.set_size(Vector2::new(pw-48.0, 28.0));
            hint_i.add_theme_font_size_override("font_size", 13);
            hint_i.add_theme_color_override("font_color", C_DIM);
            ip.add_child(&hint_i);

            layer.add_child(&ip);
            self.inv_list  = Some(il);
            self.inv_panel = Some(ip);
        }

        // Диалоговая панель
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
            text.set_size(Vector2::new(HUD_W-48.0, 145.0));
            text.add_theme_font_size_override("font_size", 16);
            text.add_theme_color_override("font_color", C_MAIN);
            text.set_autowrap_mode(godot::classes::text_server::AutowrapMode::WORD);
            panel.add_child(&text);

            let mut vbox = VBoxContainer::new_alloc();
            vbox.set_position(Vector2::new(24.0, 205.0));
            vbox.set_size(Vector2::new(HUD_W-48.0, 170.0));
            panel.add_child(&vbox);

            let choice_lbls: [_; 4] = std::array::from_fn(|i| {
                let mut lbl = Label::new_alloc();
                lbl.set_text(&format!("{}.", i+1));
                lbl.add_theme_font_size_override("font_size", 15);
                lbl.add_theme_color_override("font_color", C_DIM);
                lbl.set_visible(false);
                vbox.add_child(&lbl);
                lbl
            });
            let [c0,c1,c2,c3] = choice_lbls;

            layer.add_child(&panel);
            self.dlg_panel   = Some(panel);
            self.dlg_speaker = Some(speaker);
            self.dlg_text    = Some(text);
            self.choice_box  = Some(vbox);
            self.cl0 = Some(c0); self.cl1 = Some(c1);
            self.cl2 = Some(c2); self.cl3 = Some(c3);
        }

        // Оружие — HUD-спрайт (нижний правый угол, как в DOOM)
        // Sprite2D не импортирован, используем CanvasItem через Label-placeholder,
        // настоящий спрайт будет через отдельный TextureRect при необходимости.
        // Сейчас: текстовый символ пистолета.
        {
            let mut wlbl = Label::new_alloc();
            wlbl.set_text("🔫");
            wlbl.set_position(Vector2::new(HUD_W - 160.0, HUD_H - 120.0));
            wlbl.set_size(Vector2::new(120.0, 80.0));
            wlbl.add_theme_font_size_override("font_size", 52);
            layer.add_child(&wlbl);
            // TODO: заменить на TextureRect с weapon_pistol.png после генерации ассетов
        }

        // Экран смерти
        {
            let mut dp = Panel::new_alloc();
            dp.set_position(Vector2::ZERO);
            dp.set_size(Vector2::new(HUD_W, HUD_H));
            dp.add_theme_stylebox_override("panel",
                &make_style(Color::from_rgba(0.3,0.0,0.0,0.88), Color::TRANSPARENT_BLACK, 0));
            dp.set_visible(false);

            let mut lbl = Label::new_alloc();
            lbl.set_text(t("msg_died", lang));
            lbl.set_position(Vector2::new(0.0, HUD_H*0.4));
            lbl.set_size(Vector2::new(HUD_W, 60.0));
            lbl.set_horizontal_alignment(HorizontalAlignment::CENTER);
            lbl.add_theme_font_size_override("font_size", 56);
            lbl.add_theme_color_override("font_color", C_RED);
            dp.add_child(&lbl);

            let mut sub = Label::new_alloc();
            sub.set_text("Enter — перезапустить");
            sub.set_position(Vector2::new(0.0, HUD_H*0.4+70.0));
            sub.set_size(Vector2::new(HUD_W, 30.0));
            sub.set_horizontal_alignment(HorizontalAlignment::CENTER);
            sub.add_theme_font_size_override("font_size", 18);
            sub.add_theme_color_override("font_color", C_DIM);
            dp.add_child(&sub);

            layer.add_child(&dp);
            self.dead_panel = Some(dp);
        }
    }
}

/// Динамический выбор сцены для NPC.
fn npc_scene_id(npc_id: &str, state: &crate::game_state::GameState) -> &'static str {
    match npc_id {
        "vale" => {
            if !state.has("met_vale")                  { "meet_vale" }
            else if !state.has("vale_chat_1_done")     { "vale_class_chat" }
            else if state.rel("vale") < 30             { "vale_office_1" }
            else if state.rel("vale") < 55             { "vale_office_2" }
            else                                       { "vale_office_deep" }
        }
        "victor" => {
            if !state.has("met_victor")                { "intro_victor" }
            else if !state.has("victor_quest_given")   { "victor_chat_2" }
            else if state.has("victor_quest_done")     { "victor_chat_end" }
            else                                       { "victor_quest_check" }
        }
        "elena" => {
            if !state.has("met_elena")                 { "first_elena" }
            else if !state.has("elena_lib_1")          { "elena_library_1" }
            else if !state.has("elena_quest_given")    { "elena_chat_2" }
            else if state.has("elena_quest_done")      { "elena_chat_end" }
            else                                       { "elena_quest_check" }
        }
        "sofia" => {
            if !state.has("met_sofia")                 { "meet_sofia" }
            else if !state.has("sofia_deep_done")      { "sofia_chat" }
            else                                       { "sofia_chat_3" }
        }
        "guard" => {
            if !state.has("met_guard")                 { "meet_guard" }
            else if !state.has("guard_quest_given")    { "guard_quest_offer" }
            else if state.has("guard_quest_done")      { "guard_quest_end" }
            else                                       { "guard_quest_check" }
        }
        "merchant" => {
            if !state.has("met_merchant")              { "meet_merchant" }
            else if !state.has("merchant_bought")      { "merchant_shop" }
            else                                       { "merchant_again" }
        }
        "scientist" => {
            if !state.has("met_scientist")                 { "meet_scientist" }
            else if !state.has("scientist_quest_given")    { "scientist_quest_offer" }
            else if state.has("scientist_quest_done")      { "scientist_quest_end" }
            else                                           { "scientist_quest_check" }
        }
        "stranger" => {
            if !state.has("met_stranger")              { "meet_stranger" }
            else                                       { "stranger_again" }
        }
        _ => "",
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

// ── Игровой процесс ───────────────────────────────────────────────────────────

impl Game3D {
    fn process_explore(&mut self) {
        let lang = self.settings.lang.clone();
        self.update_nearby();
        self.update_inv_label();
        self.update_quest_label(&lang);

        let input = Input::singleton();

        if input.is_action_just_pressed("interact") {
            if let Some(idx) = self.near_item {
                self.pick_up_item(idx);
            } else if let Some(idx) = self.near_npc {
                self.start_dialogue(idx);
            }
        }

        if input.is_action_just_pressed("shoot") && self.shoot_cd == 0.0 {
            self.try_shoot();
        }

        if input.is_action_just_pressed("inventory") {
            self.open_inventory();
        }

        let player_dead = self.player.as_ref()
            .and_then(|p| p.clone().try_cast::<Player>().ok())
            .map(|pl| pl.bind().dead)
            .unwrap_or(false);
        if player_dead && self.mode != Mode::Dead {
            self.mode = Mode::Dead;
            if let Some(ref mut dp) = self.dead_panel { dp.set_visible(true); }
            Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
            save::delete();
        }
    }

    fn process_dialogue(&mut self) {
        if let Some(ref mut p) = self.player {
            let mut vel = p.get_velocity();
            vel.x = 0.0; vel.z = 0.0;
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
        if Input::singleton().is_action_just_pressed("inventory") {
            self.close_inventory();
        }
        if Input::singleton().is_action_just_pressed("interact") {
            self.use_first_consumable();
        }
    }

    // ── Поблизости ───────────────────────────────────────────────────────────

    fn update_nearby(&mut self) {
        let lang = self.settings.lang.clone();
        let player_pos = match self.player.as_ref() {
            Some(p) => p.get_global_position(),
            None    => { self.near_npc = None; return; }
        };

        let mut near_npc: Option<usize> = None;
        let mut best_n = INTERACT_R;
        for (i, sp) in self.npc_sprites.iter().enumerate() {
            let d = (player_pos - sp.get_global_position()).length();
            if d < best_n { best_n = d; near_npc = Some(i); }
        }
        self.near_npc = near_npc;

        let mut near_item: Option<usize> = None;
        let mut best_i = PICKUP_R;
        for (i, wi) in self.world_items.iter().enumerate() {
            let d = (player_pos - wi.node.get_global_position()).length();
            if d < best_i { best_i = d; near_item = Some(i); }
        }
        self.near_item = near_item;

        // Ближайший враг (для таргетинга)
        let mut near_enemy: Option<usize> = None;
        let mut best_e = SHOOT_RANGE * 0.7;
        for (i, e) in self.enemies.iter().enumerate() {
            if e.bind().alive {
                let d = (player_pos - e.bind().base().get_global_position()).length();
                if d < best_e { best_e = d; near_enemy = Some(i); }
            }
        }
        self.near_enemy = near_enemy;

        let hint_text = if let Some(idx) = near_item {
            let name = self.world_items[idx].name.clone();
            format!("{}: {}", t("hud_pickup", &lang), name)
        } else if let Some(idx) = near_npc {
            format!("{} {}", t("hud_interact", &lang), NPC_DATA[idx].name)
        } else {
            String::new()
        };

        if let Some(ref mut lbl) = self.hint_label {
            if hint_text.is_empty() { lbl.set_visible(false); }
            else { lbl.set_text(&hint_text); lbl.set_visible(true); }
        }
    }

    // ── Стрельба ─────────────────────────────────────────────────────────────

    fn try_shoot(&mut self) {
        self.shoot_cd = SHOOT_CD;
        let lang = self.settings.lang.clone();

        let (player_pos, facing) = match self.player.as_ref() {
            Some(p) => {
                let pos = p.get_global_position();
                let fwd = if let Ok(pl) = p.clone().try_cast::<Player>() {
                    pl.bind().facing_dir()
                } else {
                    Vector3::new(0.0, 0.0, -1.0)
                };
                (pos, fwd)
            }
            None => return,
        };

        let mut hit_idx: Option<usize> = None;
        let mut best_score = 0.0f32;

        for (i, e) in self.enemies.iter().enumerate() {
            let eb = e.bind();
            if !eb.alive { continue; }
            let epos = eb.base().get_global_position();
            let dist = Vector3::new(epos.x-player_pos.x, 0.0, epos.z-player_pos.z).length();
            if dist > SHOOT_RANGE { continue; }
            let to_e = Vector3::new(epos.x-player_pos.x, 0.0, epos.z-player_pos.z).normalized();
            let dot  = facing.dot(to_e);
            if dot > 0.62 {
                let score = dot / (dist + 0.1);
                if score > best_score { best_score = score; hit_idx = Some(i); }
            }
        }

        if let Some(idx) = hit_idx {
            self.enemies[idx].bind_mut().take_damage(25.0);
            let alive = self.enemies[idx].bind().alive;
            if alive {
                let hp  = self.enemies[idx].bind().hp;
                let max = self.enemies[idx].bind().max_hp;
                self.show_flash(&format!("{} ({:.0}/{:.0})", t("msg_hit", &lang), hp, max));
            } else {
                self.show_flash(t("msg_enemy_dead", &lang));
            }
        } else {
            self.show_flash(t("msg_miss", &lang));
        }

        self.enemies.retain(|e| e.bind().alive);
    }

    // ── Урон от врагов ────────────────────────────────────────────────────────

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
            let maybe_player = self.player.as_ref().map(|p| p.clone());
            if let Some(p_gd) = maybe_player {
                if let Ok(mut player) = p_gd.try_cast::<Player>() {
                    player.bind_mut().take_damage(total_dmg);
                }
            }
            self.damage_flash_timer = 0.35;
            self.show_flash(&format!("-{:.0} HP", total_dmg));
        }
    }

    // ── Подбор предметов ─────────────────────────────────────────────────────

    fn pick_up_item(&mut self, idx: usize) {
        let lang = self.settings.lang.clone();
        let wi   = self.world_items.remove(idx);
        wi.node.clone().free();
        self.near_item = None;
        let name = wi.name.clone();
        if let Some(ref mut state) = self.state {
            if wi.gold > 0 { state.gold += wi.gold; }
            if wi.heal.is_some() {
                use crate::item::Item;
                state.inventory.add(Item::new(&wi.item_id, &name, "", 1));
            }
        }
        self.show_flash(&format!("{}: {}", t("msg_picked_up", &lang), name));
        self.auto_save();
    }

    fn use_first_consumable(&mut self) {
        let lang = self.settings.lang.clone();
        let heal_data = self.state.as_ref().and_then(|s| {
            s.inventory.items.iter()
                .find(|i| i.id == "medkit" || i.id == "armor_shard" || i.id == "potion"
                          || i.id == "bread" || i.id == "energy_drink")
                .map(|i| {
                    let amt = match i.id.as_str() {
                        "medkit"        => 30.0,
                        "armor_shard"   => 20.0,
                        "potion"        => 50.0,
                        "energy_drink"  => 15.0,
                        _               => 10.0,
                    };
                    (i.id.clone(), amt)
                })
        });
        if let Some((id, amount)) = heal_data {
            if let Some(ref mut state) = self.state { state.inventory.remove_one(&id); }
            if let Some(ref p) = self.player {
                if let Ok(mut player) = p.clone().try_cast::<Player>() {
                    player.bind_mut().heal(amount);
                }
            }
            self.show_flash(t("msg_healed", &lang));
        }
    }

    // ── Инвентарь ────────────────────────────────────────────────────────────

    fn open_inventory(&mut self) {
        self.mode = Mode::Inventory;
        self.refresh_inventory_ui();
        if let Some(ref mut p) = self.inv_panel { p.set_visible(true); }
        if let Some(ref mut lbl) = self.hint_label { lbl.set_visible(false); }
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
    }

    fn close_inventory(&mut self) {
        self.mode = Mode::Explore;
        if let Some(ref mut p) = self.inv_panel { p.set_visible(false); }
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::CAPTURED);
    }

    fn refresh_inventory_ui(&mut self) {
        let lang = self.settings.lang.clone();
        let text = if let Some(ref state) = self.state {
            if state.inventory.is_empty() && state.gold == 0 {
                t("hud_inv_empty", &lang).to_string()
            } else {
                let mut lines = vec![format!("{}: {}  зол.", t("hud_gold", &lang), state.gold)];
                for item in &state.inventory.items {
                    lines.push(format!("• {} ×{}", item.name, item.qty));
                }
                lines.push(String::new());
                lines.push(format!("[ E ] — {}", t("inv_use", &lang)));
                lines.join("\n")
            }
        } else { String::new() };
        if let Some(ref mut lbl) = self.inv_list { lbl.set_text(&text); }
    }

    // ── Диалог ───────────────────────────────────────────────────────────────

    fn start_dialogue(&mut self, npc_idx: usize) {
        if npc_idx >= NPC_DATA.len() { return; }
        let dynamic = self.state.as_ref()
            .map(|s| npc_scene_id(NPC_DATA[npc_idx].id, s))
            .unwrap_or("");
        let scene_id = if dynamic.is_empty() { NPC_DATA[npc_idx].scene_id } else { dynamic };
        let scene = match self.state.as_ref().and_then(|s| get_scene(scene_id, s)) {
            Some(s) => s, None => return,
        };
        self.scene    = Some(scene);
        self.line_idx = 0;
        self.mode     = Mode::Dialogue;
        self.at_choices = false;
        if let Some(ref mut p) = self.dlg_panel { p.set_visible(true); }
        if let Some(ref mut lbl) = self.hint_label { lbl.set_visible(false); }
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
        self.refresh_dlg_ui();
    }

    fn advance_dialogue(&mut self) {
        let (total, has_choices) = match self.scene.as_ref() {
            Some(s) => (s.lines.len(), !s.choices.is_empty()),
            None    => { self.end_dialogue(); return; }
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
                .filter(|c| c.requires.as_ref().map_or(true, |(st,mn)| state.stat(st) >= *mn))
                .collect();
            if idx >= avail.len() { return; }
            (avail[idx].effects.clone(), avail[idx].next.clone())
        };
        let msgs = self.state.as_mut().unwrap().apply(&effects);
        for m in msgs { self.show_flash(&m); }
        if let Some(next_id) = next {
            let new_scene = self.state.as_ref().and_then(|s| get_scene(&next_id, s));
            if let Some(sc) = new_scene {
                self.scene    = Some(sc);
                self.line_idx = 0;
                self.at_choices = false;
                self.refresh_dlg_ui();
                return;
            }
        }
        self.end_dialogue();
    }

    fn end_dialogue(&mut self) {
        self.scene    = None;
        self.line_idx = 0;
        self.mode     = Mode::Explore;
        self.at_choices = false;
        if let Some(ref mut p) = self.dlg_panel { p.set_visible(false); }
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::CAPTURED);
        self.auto_save();
    }

    fn refresh_dlg_ui(&mut self) {
        let (speaker, text, choices_text): (String, String, Vec<String>) = {
            let scene = match self.scene.as_ref() { Some(s) => s, None => return };
            let state = match self.state.as_ref() { Some(s) => s, None => return };
            let line = &scene.lines[self.line_idx.min(scene.lines.len().saturating_sub(1))];
            let ct: Vec<String> = if self.at_choices {
                scene.choices.iter()
                    .filter(|c| c.requires.as_ref().map_or(true, |(st,mn)| state.stat(st) >= *mn))
                    .enumerate()
                    .map(|(i,c)| format!("{}. {}", i+1, c.text))
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
                } else { lbl.set_visible(false); }
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
            let c = Color::from_rgba(0.9 - ratio*0.5, 0.1 + ratio*0.3, 0.1, 1.0);
            fg.add_theme_stylebox_override("panel", &make_style(c, Color::TRANSPARENT_BLACK, 0));
        }
        if let Some(ref mut lbl) = self.hp_label {
            lbl.set_text(&format!("{}: {:.0}/{:.0}", t("hud_hp", &lang), hp, max_hp));
        }
    }

    fn update_inv_label(&mut self) {
        let lang = self.settings.lang.clone();
        if let Some(ref state) = self.state {
            let text = if state.inventory.is_empty() {
                format!("{}  |  {}: {}", t("hud_inv_empty", &lang), t("hud_gold", &lang), state.gold)
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
            let active: Vec<_> = state.quests.quests.iter()
                .filter(|q| q.state == crate::quest::QuestState::Active)
                .collect();
            let text = if active.is_empty() { String::new() } else {
                let mut lines = vec![t("hud_quests", lang).to_string()];
                for q in active.iter().take(5) {
                    lines.push(format!("• {}", q.title));
                }
                lines.join("\n")
            };
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
                    format!("[{}]  {}  {:.0}/{:.0}", eb.cfg_id, bar, eb.hp, eb.max_hp)
                } else { String::new() }
            } else { String::new() }
        } else { String::new() };

        if let Some(ref mut lbl) = self.targeting_label {
            if text.is_empty() { lbl.set_visible(false); }
            else { lbl.set_text(&text); lbl.set_visible(true); }
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
        let (x,y,w,h) = NPC_IDLE_FRAMES[self.npc_anim_frame];
        let rect = Rect2::new(Vector2::new(x,y), Vector2::new(w,h));
        for sprite in self.npc_sprites.iter_mut() { sprite.set_region_rect(rect); }
    }

    fn tick_items(&mut self, _dt: f32) {
        // Боб-эффект: предметы-спрайты вращаются (game_time использован не здесь т.к. бorrow)
        // Реализовано через transform напрямую в build_world_items с AnimationPlayer при необходимости.
        // Пока оставляем статичными — CSS-стиль у объектов в мире достаточно.
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
                // Fade alpha in last 0.8 seconds
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
        let lang = self.settings.lang.clone();
        if let Some(ref state) = self.state {
            let hp = if let Some(ref p) = self.player {
                if let Ok(player) = p.clone().try_cast::<Player>() { player.bind().hp } else { 100.0 }
            } else { 100.0 };
            if save::save(state, hp) { self.show_flash(t("msg_saved", &lang)); }
        }
    }
}

// ── Вспомогательные функции ───────────────────────────────────────────────────

fn make_box(
    pos: Vector3, size: Vector3, color: Color,
    tex: Option<&Gd<Texture2D>>, uv: f32,
) -> Gd<StaticBody3D> {
    let mut body = StaticBody3D::new_alloc();
    body.set_position(pos);

    let mut mi = MeshInstance3D::new_alloc();
    let mut mesh = BoxMesh::new_gd();
    mesh.set_size(size);
    mi.set_mesh(&mesh);

    let mut mat = StandardMaterial3D::new_gd();
    if let Some(t) = tex {
        mat.set_albedo(Color::WHITE);
        mat.set_texture(TextureParam::ALBEDO, t);
        mat.set_uv1_scale(Vector3::new(uv, uv, 1.0));
    } else {
        mat.set_albedo(color);
    }
    mi.set_surface_override_material(0, &mat);

    let mut col = CollisionShape3D::new_alloc();
    let mut shape = BoxShape3D::new_gd();
    shape.set_size(size);
    col.set_shape(&shape);

    body.add_child(&mi);
    body.add_child(&col);
    body
}

fn crop_tex(img: Option<&Gd<Image>>, x: i32, y: i32, w: i32, h: i32) -> Option<Gd<Texture2D>> {
    let img = img?;
    let region = img.get_region(Rect2i::new(Vector2i::new(x,y), Vector2i::new(w,h)))?;
    let itex = ImageTexture::create_from_image(&region)?;
    Some(itex.upcast::<Texture2D>())
}
