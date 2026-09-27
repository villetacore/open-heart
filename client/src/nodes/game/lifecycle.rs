//! Жизненный цикл ноды `Game3D`: init / ready / input / process.
//!
//! Godot-виртуальные методы вынесены из `mod.rs` без изменения поведения.
use super::*;

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
            station_zones: Vec::new(),
            map_ambient: Vec::new(),
            active_district: None,
            players: HashMap::new(),
            local_peer: LOCAL_PEER,
            next_entity_id: 1,
            net: None,
            remote_players: HashMap::new(),
            net_enemies: HashMap::new(),
            net_items: HashMap::new(),
            net_item_kinds: HashMap::new(),
            net_correction: None,
            craft_panel: None,
            craft_list: None,
            craft_shown: Vec::new(),
            craft_stations: Vec::new(),
            ground_snap_ticks: 0,
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
            ability_labels: Vec::new(),
            ability_icons: Vec::new(),
            inventory_grid: None,
            inventory_detail: None,
            inventory_use: None,
            selected_item: String::new(),
            inventory_filter: 0,
            perk_grid: None,
            journal_buttons: None,
            journal_panel: None,
            journal_list: None,
            perk_panel: None,
            perk_list: None,
            perk_respec: None,
            perk_craft: None,
            shop_layer: None,
            crosshair: None,
            dead_panel: None,
            pause_panel: None,
            epilogue_panel: None,
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
            class_portraits: Vec::new(),
            game_time: 0.0,
            autosave_enabled: true,
            creative: false,
            creative_panel: None,
            dialogue_actions: None,
            fashion_icon: None,
            weapon_icon: None,
            dialogue_portrait: None,
            net_save_timer: 0.0,
        }
    }

    fn ready(&mut self) {
        self.settings = Settings::load();
        self.settings.apply_global(); // окно/vsync/громкость
        let lang = self.settings.lang.clone();

        self.creative = creative::take_request();
        self.autosave_enabled = !self.creative;
        let loaded = if self.creative { None } else { save::load() };
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
                self.station_zones = built.stations.clone();
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
        self.build_ability_hud();
        self.build_modern_panels();
        self.build_presentation();
        self.build_quest_navigation_marker();
        self.build_select_ui();

        let player_gd = self.base().get_node_as::<CharacterBody3D>("Player");
        self.set_local_player(player_gd.clone());
        // Сетевой режим: адрес из аргумента запуска либо из меню (настройки).
        let server = Self::server_from_cmdline().or_else(|| {
            let saved = self.settings.server.trim().to_string();
            (!saved.is_empty()).then_some(saved)
        });
        if let Some(url) = server.filter(|_| !self.creative) {
            self.net_connect(&url);
        }
        // FOV камеры из настроек
        if let Some(mut cam) = player_gd.try_get_node_as::<godot::classes::Camera3D>("Camera3D") {
            cam.set_fov(self.settings.fov);
        }
        if let Ok(mut p) = player_gd.try_cast::<Player>() {
            p.bind_mut().teleport(spawn);
        }
        // врагам нужна ссылка на игрока
        let pl = self.player();
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
            if let Some(p) = self.player() {
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
        if self.creative {
            self.confirm_class(0, 0);
            self.creative_refill();
            self.build_creative_panel();
            // Кнопка «Арена» в меню: сразу высадить игрока в бой с волной врагов.
            if creative::take_arena_request() {
                self.spawn_arena_wave("mixed");
            }
        }
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        // F11 — быстрый тумблер полного экрана прямо в игре
        if let Ok(k) = event.try_cast::<InputEventKey>() {
            if k.is_pressed() && !k.is_echo() && k.get_physical_keycode() == godot::global::Key::F2 && self.creative {
                self.creative_action(if self.mode == Mode::Creative { -1 } else { -2 });
            }
            if k.is_pressed() && !k.is_echo() && k.get_physical_keycode() == godot::global::Key::F11
            {
                self.settings.fullscreen = !self.settings.fullscreen;
                self.settings.apply_video();
                self.settings.save();
            }
        }
    }

    fn process(&mut self, delta: f64) {
        self.creative_refill();
        self.refresh_fashion();
        let dt = delta as f32;
        self.game_time += dt;
        self.shoot_cd = (self.shoot_cd - dt).max(0.0);
        if self.ground_snap_ticks > 0 {
            self.ground_snap_ticks -= 1;
            self.snap_world_pickups();
        }
        if self.net.is_some() {
            self.tick_net(dt);
        }
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
        if in_gameplay || self.net_ready() {
            if let Some(state) = self.state.as_mut() { state.abilities.tick(dt); }
        }
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
            Mode::Craft => self.process_craft(),
            Mode::Journal => self.process_journal(),
            Mode::Perks => self.process_perks(),
            Mode::Dead => self.process_dead(),
            Mode::Paused => self.process_paused(),
            Mode::Epilogue => { if Input::singleton().is_action_just_pressed("interact") { self.finish_epilogue(); } },
            Mode::Shop => self.process_shop(),
            Mode::Creative => { if Input::singleton().is_action_just_pressed("escape") { self.creative_action(-1); } },
        }

        self.update_hp_bar();
        self.update_xp_bar();
        self.update_ammo_hud();
        self.update_targeting_hud();
        self.update_minimap();
        self.update_ability_hud();
    }
}
