//! Окружение, мир и способности врагов.

use super::*;

// ── Окружение и мир ───────────────────────────────────────────────────────────

impl Game3D {
    pub(super) fn tick_map_ambient(&mut self, dt: f32) {
        if self.mode == Mode::Paused {
            return;
        }
        for ambient in &mut self.map_ambient {
            ambient.phase += dt * ambient.speed;
            ambient
                .node
                .set_position(ambient.origin + Vector3::UP * (ambient.phase.sin() * ambient.bob));
            if ambient.spin.abs() > f32::EPSILON {
                ambient.node.rotate_y(ambient.spin * dt);
            }
        }
    }

    pub(super) fn build_boss_aftermath(&mut self) {
        use godot::classes::base_material_3d::BillboardMode;
        use godot::classes::{CylinderMesh, MeshInstance3D, StandardMaterial3D};

        let flags = self
            .state
            .as_ref()
            .map(|state| state.flags.clone())
            .unwrap_or_default();
        let memorials = [
            (
                "crypt_warden",
                Vector3::new(-7.0, 0.0, -2.0),
                Color::from_rgba(0.3, 0.85, 1.0, 1.0),
                "heavy",
            ),
            (
                "blood_oracle",
                Vector3::new(-2.0, 0.0, -7.0),
                Color::from_rgba(1.0, 0.12, 0.4, 1.0),
                "void_priest",
            ),
            (
                "archive_sentinel",
                Vector3::new(7.0, 0.0, -2.0),
                Color::from_rgba(0.35, 0.5, 1.0, 1.0),
                "heavy",
            ),
            (
                "heart_tyrant",
                Vector3::new(2.0, 0.0, 5.0),
                Color::from_rgba(1.0, 0.2, 0.65, 1.0),
                "heart_tyrant",
            ),
        ];

        for (boss_id, position, color, sprite_id) in memorials {
            if !flags.contains(&format!("boss_defeated_{boss_id}")) {
                continue;
            }
            let node_name = format!("BossMemorial_{boss_id}");
            if self
                .base()
                .get_node_or_null(&NodePath::from(node_name.as_str()))
                .is_some()
            {
                continue;
            }

            let mut memorial = Node3D::new_alloc();
            memorial.set_name(&node_name);
            memorial.set_position(position);
            memorial.add_to_group("boss_memorials");

            let mut pedestal_mesh = CylinderMesh::new_gd();
            pedestal_mesh.set_top_radius(1.15);
            pedestal_mesh.set_bottom_radius(1.35);
            pedestal_mesh.set_height(0.55);
            let mut pedestal = MeshInstance3D::new_alloc();
            pedestal.set_mesh(&pedestal_mesh);
            pedestal.set_position(Vector3::new(0.0, 0.275, 0.0));
            let mut material = StandardMaterial3D::new_gd();
            material.set_albedo(color.darkened(0.55));
            material.set_emission(color);
            material.set_emission_energy_multiplier(2.2);
            pedestal.set_surface_override_material(0, &material);
            memorial.add_child(&pedestal);

            let texture_path = format!("res://assets/sprites/characters/enemy_{sprite_id}.png");
            if let Some(texture) = self.cache.get(&texture_path) {
                let mut hologram = Sprite3D::new_alloc();
                hologram.set_texture(&texture);
                hologram.set_region_enabled(true);
                hologram.set_region_rect(Rect2::new(Vector2::ZERO, Vector2::new(128.0, 256.0)));
                hologram.set_pixel_size(0.009);
                hologram.set_position(Vector3::new(0.0, 1.65, 0.0));
                hologram.set_billboard_mode(BillboardMode::ENABLED);
                hologram.set_modulate(Color::from_rgba(color.r, color.g, color.b, 0.72));
                hologram.set_transparency(0.28);
                memorial.add_child(&hologram);
            }

            let light = make_light(Vector3::new(0.0, 1.1, 0.0), color, 1.65, 5.5);
            memorial.add_child(&light);
            self.base_mut().add_child(&memorial);
        }
    }

    pub(super) fn refresh_quest_world_changes(&mut self) {
        let changes: Vec<_> = self
            .cfg
            .as_ref()
            .map(|config| {
                config
                    .quests
                    .iter()
                    .filter_map(|quest| {
                        let completed = self
                            .state
                            .as_ref()
                            .is_some_and(|state| state.quests.is_completed(&quest.id));
                        completed.then(|| quest.world_change.clone()).flatten()
                    })
                    .collect()
            })
            .unwrap_or_default();

        for change in changes {
            let node_name = format!("QuestEvolution_{}", change.id);
            if self
                .base()
                .get_node_or_null(&NodePath::from(node_name.as_str()))
                .is_some()
            {
                continue;
            }
            let color = Color::from_rgba(change.color[0], change.color[1], change.color[2], 1.0);
            let scale = change.scale;
            let mut root = Node3D::new_alloc();
            root.set_name(&node_name);
            root.set_position(Vector3::new(change.pos[0], change.pos[1], change.pos[2]));
            root.add_to_group("quest_world_evolutions");

            let slab = |position: Vector3, size: Vector3| {
                make_glow_slab(position * scale, size * scale, None, color, 1.0)
            };
            match change.pattern.as_str() {
                "table" => {
                    let wood = Color::from_rgb(0.24, 0.12, 0.16);
                    let mut parts = vec![
                        (Vector3::new(0.0, 0.9, 0.0), Vector3::new(3.6, 0.16, 1.4)),
                        (Vector3::new(0.0, 0.5, -1.15), Vector3::new(3.6, 0.14, 0.5)),
                        (Vector3::new(0.0, 0.5, 1.15), Vector3::new(3.6, 0.14, 0.5)),
                    ];
                    for x in [-1.4, 1.4] {
                        for z in [-0.5, 0.5] {
                            parts.push((Vector3::new(x, 0.45, z), Vector3::new(0.16, 0.9, 0.16)));
                        }
                        for z in [-1.15, 1.15] {
                            parts.push((Vector3::new(x, 0.25, z), Vector3::new(0.16, 0.5, 0.35)));
                        }
                    }
                    for (position, size) in parts {
                        root.add_child(&make_glow_slab(position * scale, size * scale, None, wood, 0.12));
                    }
                    for x in [-1.1, 0.0, 1.1] {
                        root.add_child(&slab(Vector3::new(x, 1.08, 0.0), Vector3::new(0.13, 0.25, 0.13)));
                    }
                }
                "garden" => {
                    root.add_child(&slab(
                        Vector3::new(0.0, 0.08, 0.0),
                        Vector3::new(5.2, 0.16, 5.2),
                    ));
                    for offset in [
                        Vector3::new(-1.7, 0.45, -1.7),
                        Vector3::new(1.7, 0.45, -1.7),
                        Vector3::new(-1.7, 0.45, 1.7),
                        Vector3::new(1.7, 0.45, 1.7),
                    ] {
                        root.add_child(&slab(offset, Vector3::new(0.55, 0.9, 0.55)));
                    }
                }
                "gallery" => {
                    root.add_child(&slab(
                        Vector3::new(0.0, 0.12, 0.0),
                        Vector3::new(6.0, 0.24, 2.2),
                    ));
                    for x in [-2.25, -0.75, 0.75, 2.25] {
                        root.add_child(&slab(
                            Vector3::new(x, 1.0, 0.0),
                            Vector3::new(0.18, 2.0, 1.4),
                        ));
                    }
                }
                "archive" => {
                    for radius in [1.1, 2.2, 3.3] {
                        root.add_child(&slab(
                            Vector3::new(0.0, 0.12, -radius),
                            Vector3::new(radius * 2.0, 0.16, 0.16),
                        ));
                        root.add_child(&slab(
                            Vector3::new(0.0, 0.12, radius),
                            Vector3::new(radius * 2.0, 0.16, 0.16),
                        ));
                    }
                }
                "shrine" => {
                    root.add_child(&slab(
                        Vector3::new(0.0, 0.2, 0.0),
                        Vector3::new(4.8, 0.4, 4.8),
                    ));
                    root.add_child(&slab(
                        Vector3::new(0.0, 0.65, 0.0),
                        Vector3::new(3.2, 0.5, 3.2),
                    ));
                    root.add_child(&slab(
                        Vector3::new(0.0, 1.25, 0.0),
                        Vector3::new(1.6, 0.7, 1.6),
                    ));
                    for offset in [
                        Vector3::LEFT,
                        Vector3::RIGHT,
                        Vector3::FORWARD,
                        Vector3::BACK,
                    ] {
                        root.add_child(&slab(
                            offset * 2.4 + Vector3::UP * 0.7,
                            Vector3::new(0.25, 1.4, 0.25),
                        ));
                    }
                }
                _ => {
                    root.add_child(&slab(
                        Vector3::new(0.0, 0.15, 0.0),
                        Vector3::new(4.5, 0.3, 4.5),
                    ));
                    root.add_child(&slab(
                        Vector3::new(0.0, 1.2, 0.0),
                        Vector3::new(0.45, 2.4, 0.45),
                    ));
                    for offset in [
                        Vector3::LEFT,
                        Vector3::RIGHT,
                        Vector3::FORWARD,
                        Vector3::BACK,
                    ] {
                        root.add_child(&slab(
                            offset * 1.7 + Vector3::UP * 0.45,
                            Vector3::new(0.3, 0.9, 0.3),
                        ));
                    }
                }
            }
            if !change.sprite.is_empty() {
                if let Some(mut sprite) = make_billboard(
                    &mut self.cache,
                    &change.sprite,
                    Vector3::new(0.0, 2.2 * scale, 0.0),
                    0.009 * scale,
                ) {
                    sprite.set_modulate(color.lightened(0.18));
                    root.add_child(&sprite);
                }
            }
            // A shared table is static decor, not orbiting NPC stand-ins.
            let activity_count = if change.pattern == "table" { 0 } else { change.activity_count };
            for index in 0..activity_count {
                let angle = std::f32::consts::TAU * index as f32 / change.activity_count as f32;
                let radius = change.activity_radius * scale;
                let position = Vector3::new(angle.cos() * radius, 0.7, angle.sin() * radius);
                let mut actor = Node3D::new_alloc();
                actor.set_name(&format!("QuestActivity_{}_{}", change.id, index));
                actor.set_position(position);
                actor.add_to_group("quest_world_activities");
                if let Some(mut sprite) = make_billboard(
                    &mut self.cache,
                    &change.sprite,
                    Vector3::ZERO,
                    0.0065 * scale,
                ) {
                    let activity_tint = match change.activity.as_str() {
                        "support" => color.lightened(0.3),
                        "crowd" => color.lerp(Color::WHITE, index as f64 * 0.08),
                        "echo" => Color::from_rgba(color.r, color.g, color.b, 0.58),
                        "guardian" => color.darkened(0.12),
                        _ => color,
                    };
                    sprite.set_modulate(activity_tint);
                    actor.add_child(&sprite);
                }
                actor.add_child(&make_glow_slab(
                    Vector3::new(0.0, -0.62, 0.0),
                    Vector3::new(0.55, 0.06, 0.55) * scale,
                    None,
                    color,
                    1.0,
                ));
                root.add_child(&actor);
                self.map_ambient.push(crate::map::MapAmbient {
                    node: actor,
                    origin: position,
                    phase: angle,
                    speed: change.activity_speed * (1.0 + index as f32 * 0.07),
                    bob: match change.activity.as_str() {
                        "echo" => 0.42,
                        "support" => 0.28,
                        _ => 0.16,
                    } * scale,
                    spin: match change.activity.as_str() {
                        "patrol" => 0.55,
                        "guardian" => -0.28,
                        "echo" => 0.8,
                        _ => 0.22,
                    } * change.activity_speed,
                });
            }
            root.add_child(&make_light(
                Vector3::new(0.0, 1.5 * scale, 0.0),
                color,
                1.7,
                7.0 * scale,
            ));
            self.base_mut().add_child(&root);
        }
    }

    pub(super) fn build_environment(&mut self, map_env: Option<&crate::map::MapEnv>) {
        use godot::classes::light_3d::Param;

        let sky_name = map_env
            .and_then(|e| e.sky.clone())
            .unwrap_or_else(|| "sky_purple".to_string());
        let ambient = map_env
            .and_then(|e| e.ambient)
            .unwrap_or([0.32, 0.22, 0.38]);
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

        // bloom/glow — неоновая эстетика (Forward Mobile рендерер), управляется настройкой
        env.set_glow_enabled(self.settings.glow);
        env.set_glow_intensity(0.65);
        env.set_glow_strength(1.0);
        env.set_glow_bloom(0.18);
        env.set_glow_hdr_bleed_threshold(1.0);

        // ambient occlusion — глубина в углах (только когда включены тени/качество)
        let display_backend = godot::classes::DisplayServer::singleton().get_name();
        if self.settings.shadows && display_backend != "headless" {
            env.set_ssao_enabled(true);
        }

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
        // тени direction-света — по настройке (стильные мягкие тени)
        dir.set_shadow(self.settings.shadows);
        self.base_mut().add_child(&dir);
    }

    pub(super) fn build_npcs(&mut self) {
        // NPC из npcs.json пресета; legacy-таблица NPC_DATA — только если файла нет вовсе.
        let (cfg_npcs, file_present): (Vec<crate::config::NpcCfg>, bool) = self
            .cfg
            .as_ref()
            .map(|c| (c.npcs.clone(), c.npcs_file_present))
            .unwrap_or_default();

        let mut sprites: Vec<Gd<Sprite3D>> = Vec::new();
        let mut npcs: Vec<NpcRt> = Vec::new();

        if cfg_npcs.is_empty() && !file_present {
            for cfg in NPC_DATA.iter() {
                let (new_path, fallback) = npc_sprite_tex(cfg.id);
                let path = if self.cache.get(new_path).is_some() {
                    new_path
                } else {
                    fallback
                };
                if let Some(mut sprite) = make_billboard(
                    &mut self.cache,
                    path,
                    cfg.pos + Vector3::new(0.0, 1.28, 0.0),
                    PIXEL_SZ,
                ) {
                    sprite.set_region_enabled(true);
                    sprite.set_texture_filter(TextureFilter::NEAREST);
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
                let sprite_name = if nc.sprite.is_empty() {
                    format!("npc_{}", nc.id)
                } else {
                    nc.sprite.clone()
                };
                let path = format!("res://assets/sprites/characters/{}.png", sprite_name);
                let path = if self.cache.get(&path).is_some() {
                    path
                } else {
                    "res://assets/sprites/femboy_dark1.png".to_string()
                };
                if let Some(mut sprite) = make_billboard(
                    &mut self.cache,
                    &path,
                    Vector3::new(nc.pos[0], 1.28, nc.pos[1]),
                    PIXEL_SZ,
                ) {
                    sprite.set_region_enabled(true);
                    sprite.set_texture_filter(TextureFilter::NEAREST);
                    let (x, y, w, h) = NPC_IDLE_FRAMES[0];
                    sprite.set_region_rect(Rect2::new(Vector2::new(x, y), Vector2::new(w, h)));
                    if let Some(c) = nc.color {
                        sprite.set_modulate(Color::from_rgba(c[0], c[1], c[2], 1.0));
                    }
                    if let Some([x, y, w, h]) = nc.sprite_region.filter(|r| r.iter().all(|v| v.is_finite()) && r[0] >= 0.0 && r[1] >= 0.0 && r[2] > 0.0 && r[3] > 0.0) {
                        let height = nc.sprite_height.filter(|v| v.is_finite() && *v > 0.0).unwrap_or(2.0);
                        sprite.set_region_rect(Rect2::new(Vector2::new(x, y), Vector2::new(w, h)));
                        sprite.set_pixel_size(height / h);
                        sprite.set_position(Vector3::new(nc.pos[0], height * 0.5, nc.pos[1]));
                        sprite.set_texture_filter(TextureFilter::LINEAR);
                        sprite.set_meta("static_atlas", &true.to_variant());
                    }
                    self.base_mut().add_child(&sprite);
                    sprites.push(sprite);
                    npcs.push(NpcRt {
                        id: nc.id.clone(),
                        name: nc.name(&self.settings.lang).to_string(),
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
    pub(super) fn build_world_spawns(&mut self) {
        let Some(cfg) = self.cfg.take() else { return };
        let level = cfg.level.clone();
        self.cfg = Some(cfg);
        self.spawn_from_level(&level);
    }

    /// Заспавнить врагов/предметы/патроны/оружие из структуры LevelCfg (карта или level.json).
    pub(super) fn spawn_from_level(&mut self, level: &crate::config::LevelCfg) {
        let Some(cfg) = self.cfg.take() else { return };

        for spawn in &level.spawn_enemies {
            self.spawn_enemy(
                &cfg,
                &spawn.kind,
                Vector3::new(spawn.x, 0.0, spawn.z),
                1.0,
                false,
                false,
                &[],
            );
        }
        for spawn in &level.spawn_items {
            self.spawn_item(
                &cfg,
                &spawn.kind,
                Vector3::new(spawn.x, 0.0, spawn.z),
                false,
            );
        }
        for spawn in &level.spawn_ammo {
            let t = match spawn.kind.as_str() {
                "shells" => AmmoType::Shells,
                "rockets" => AmmoType::Rockets,
                "cells" => AmmoType::Cells,
                _ => AmmoType::Bullets,
            };
            self.spawn_ammo_pickup(t, spawn.amount, Vector3::new(spawn.x, 0.0, spawn.z), false);
        }
        for spawn in &level.spawn_weapons {
            if let Some(w) = weapon_by_name(&spawn.kind) {
                self.spawn_weapon_pickup(w, Vector3::new(spawn.x, 0.0, spawn.z), false);
            }
        }
        self.cfg = Some(cfg);
        // Y спавнов карты жёстко 0; приземляем пикапы на реальную поверхность,
        // как только физика геометрии готова (несколько кадров запаса).
        self.ground_snap_ticks = 3;
    }

    /// Опустить наземные пикапы (не данжевые) на поверхность под ними: луч вниз
    /// из точки над предметом ищет верхнюю грань (пол/рампа/платформа), и спрайт
    /// садится на неё с зазором `ground_offset`. Идемпотентно — можно звать
    /// несколько кадров, пока коллайдеры не появятся в физическом мире.
    pub(super) fn snap_world_pickups(&mut self) {
        let Some(world) = self.base().get_world_3d() else { return; };
        let Some(mut space) = world.get_direct_space_state() else { return; };
        // Луч ловит только статичную геометрию: игрока и врагов исключаем, иначе
        // предмет мог бы «сесть» на голову врагу, стоящему в точке спавна.
        let mut exclude: godot::builtin::Array<Rid> = godot::builtin::Array::new();
        if let Some(p) = self.player() { exclude.push(p.get_rid()); }
        for e in self.enemies.iter() { exclude.push(e.get_rid()); }

        let targets: Vec<(usize, Vector3, f32)> = self
            .world_items
            .iter()
            .enumerate()
            .filter(|(_, it)| !it.in_dungeon)
            .map(|(i, it)| (i, it.node.get_position(), it.ground_offset))
            .collect();
        for (i, pos, offset) in targets {
            let from = Vector3::new(pos.x, 8.0, pos.z);
            let to = Vector3::new(pos.x, -3.0, pos.z);
            let Some(mut query) = PhysicsRayQueryParameters3D::create(from, to) else { continue; };
            query.set_exclude(&exclude);
            let hit = space.intersect_ray(&query);
            if hit.is_empty() { continue; }
            if let Some(y) = hit.get("position").and_then(|v| v.try_to::<Vector3>().ok()).map(|p| p.y) {
                if let Some(item) = self.world_items.get_mut(i) {
                    item.node.set_position(Vector3::new(pos.x, y + offset, pos.z));
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn spawn_enemy(
        &mut self,
        cfg: &GameConfig,
        kind: &str,
        pos: Vector3,
        mult: f32,
        is_boss: bool,
        in_dungeon: bool,
        affixes: &[String],
    ) {
        let Some(ecfg) = cfg.enemy(kind) else {
            godot_warn!("[spawn] враг '{kind}' не найден в enemies.json пресета — пропускаю");
            return;
        };
        // ВАЖНО: конфигурируем ДО add_child — ready() врага выполняется синхронно
        // внутри add_child и строит спрайт из pending-полей (цвет/лист/масштаб);
        // конфигурация после add_child давала белых «грунтов» вместо всех видов.
        let entity_id = self.take_entity_id();
        let mut e = Enemy::new_alloc();
        e.bind_mut().set_entity_id(entity_id);
        e.set_position(pos);
        let color = Color::from_rgba(ecfg.color_r, ecfg.color_g, ecfg.color_b, 1.0);
        // сложность из настроек масштабирует hp/урон/XP поверх глубинного множителя
        let mult = mult * self.settings.difficulty_mult();
        e.bind_mut().configure(
            &ecfg.id,
            ecfg.display_name(&self.settings.lang),
            ecfg.hp,
            ecfg.speed,
            ecfg.attack_damage,
            ecfg.attack_range,
            ecfg.attack_cooldown,
            ecfg.chase_range,
            ecfg.patrol_radius,
            color,
            pos,
            ecfg.xp,
            mult,
            is_boss,
            ecfg.resist.arr(),
            ecfg.sprite.as_deref().unwrap_or(&ecfg.id),
            ecfg.scale,
            crate::enemy::Behavior::from_id(ecfg.behavior.as_deref().unwrap_or("")),
            crate::enemy::TacticalRole::from_id(&ecfg.role),
            ecfg.animation.clone(),
        );
        e.bind_mut()
            .set_weak_point(ecfg.weak_point.height, ecfg.weak_point.multiplier);
        // способности из abilities.json + pain_chance (неизвестные id — с предупреждением)
        let abs: Vec<crate::config::AbilityCfg> = ecfg
            .abilities
            .iter()
            .filter_map(|id| {
                let a = cfg.ability(id).cloned();
                if a.is_none() {
                    godot_warn!(
                        "[spawn] враг '{}': способность '{id}' не найдена в abilities.json",
                        ecfg.id
                    );
                }
                a
            })
            .collect();
        if !abs.is_empty() || ecfg.pain_chance > 0.0 {
            // Сид врага — от id сущности и позиции, без game_time: игровое время
            // у двух клиентов разное, а мир обязан совпадать (docs/MULTIPLAYER.md §5).
            let seed = (entity_id as u64) << 32
                ^ (pos.x.to_bits() as u64) << 8
                ^ pos.z.to_bits() as u64;
            e.bind_mut().set_combat_extras(abs, ecfg.pain_chance, seed);
        }
        if !ecfg.phases.is_empty() {
            let phases = ecfg
                .phases
                .iter()
                .map(|phase| crate::enemy::BossPhaseDef {
                    threshold: phase.threshold,
                    abilities: phase
                        .abilities
                        .iter()
                        .filter_map(|id| cfg.ability(id).cloned())
                        .collect(),
                    damage_mult: phase.damage_mult,
                    speed_mult: phase.speed_mult,
                    tint: phase.tint,
                })
                .collect();
            e.bind_mut().set_boss_phases(phases);
        }
        // статус, который враг накладывает на игрока при атаке
        if let Some(s) = &ecfg.attack_status {
            e.bind_mut()
                .set_attack_status(Some((s.id.clone(), s.chance)));
        }
        // аффиксы элиты (после combat_extras — модифицируют и pain_chance)
        if !affixes.is_empty() {
            let resolved: Vec<crate::config::AffixCfg> = affixes
                .iter()
                .filter_map(|id| {
                    let a = cfg.affixes.iter().find(|a| &a.id == id).cloned();
                    if a.is_none() {
                        godot_warn!("[spawn] аффикс '{id}' не найден в affixes.json — пропускаю");
                    }
                    a
                })
                .collect();
            e.bind_mut().apply_affixes(&resolved);
        }
        if let Some(p) = self.player() {
            e.bind_mut().set_player(p.clone());
        }
        // в данже враги получают навигационную сетку — преследуют по A*
        if in_dungeon {
            if let Some(ref nav) = self.dungeon_nav {
                e.bind_mut().set_nav(nav.clone(), DUNGEON_OFFSET);
            }
        }
        // add_child В КОНЦЕ: ready() выполняется синхронно здесь и читает уже
        // сконфигурированные pending-поля (спрайт/цвет/тинт элиты/масштаб).
        self.base_mut().add_child(&e);
        self.enemies.push(e);
    }

    /// Разбудить врагов вокруг точки (шум выстрела/взрыва — слух работает сквозь стены).
    pub(super) fn alert_enemies(&mut self, pos: Vector3, radius: f32) {
        let mut woke_at: Option<Vector3> = None;
        for e in self.enemies.iter_mut() {
            if (e.get_global_position() - pos).length() < radius {
                let just_woke = e.bind_mut().alert();
                if just_woke && woke_at.is_none() {
                    woke_at = Some(e.get_global_position());
                }
            }
        }
        // проигрываем один звук «пробуждения», даже если проснулось несколько —
        // иначе будет каша из наложенных звуков.
        if let Some(p) = woke_at {
            self.play_sfx_at(&SFX_WALK, p);
        }
    }

    // ── Способности врагов ───────────────────────────────────────────────────

    /// Собрать запросы способностей у врагов (снаряды/призыв/лечение) и исполнить.
    pub(super) fn collect_enemy_requests(&mut self) {
        let mut shots = Vec::new();
        let mut summons = Vec::new();
        let mut heals = Vec::new();
        let mut phases = Vec::new();
        for e in self.enemies.iter_mut() {
            let (s, m, h, p) = e.bind_mut().drain_requests();
            shots.extend(s);
            summons.extend(m);
            heals.extend(h);
            phases.extend(p);
        }

        for req in shots {
            self.spawn_enemy_shot(req);
        }

        for req in summons {
            // spawn_enemy сам умножит на сложность — снимем её с mult кастера,
            // иначе миньоны получили бы её дважды
            let base_mult = req.mult / self.settings.difficulty_mult().max(0.01);
            let cfg = self.cfg.take();
            if let Some(ref cfg) = cfg {
                for i in 0..req.count {
                    let ang = i as f32 * 2.4 + 0.7;
                    let off = Vector3::new(ang.cos() * 1.6, 0.0, ang.sin() * 1.6);
                    self.spawn_enemy(
                        cfg,
                        &req.kind,
                        req.pos + off,
                        base_mult,
                        false,
                        self.loc == Loc::Dungeon,
                        &[],
                    );
                }
            }
            self.cfg = cfg;
            self.spawn_fx(
                "res://assets/effects/effect_teleport.png",
                req.pos + Vector3::new(0.0, 1.0, 0.0),
                0.014,
                0.4,
            );
        }

        for req in heals {
            for e in self.enemies.iter_mut() {
                if (e.get_global_position() - req.pos).length() < req.radius {
                    e.bind_mut().heal_hp(req.amount);
                }
            }
            self.spawn_fx(
                "res://assets/effects/effect_heal.png",
                req.pos + Vector3::new(0.0, 1.4, 0.0),
                0.013,
                0.5,
            );
        }

        for req in phases {
            self.show_flash(&if self.settings.lang == "en" {
                format!(
                    "{} — PHASE {}/{}",
                    req.boss_name.to_uppercase(),
                    req.phase,
                    req.total
                )
            } else {
                format!(
                    "{} — ФАЗА {}/{}",
                    req.boss_name.to_uppercase(),
                    req.phase,
                    req.total
                )
            });
            self.spawn_fx(
                "res://assets/effects/effect_teleport.png",
                req.pos + Vector3::new(0.0, 1.4, 0.0),
                0.024,
                0.9,
            );
            self.spawn_light_fx(
                req.pos + Vector3::new(0.0, 1.0, 0.0),
                req.color,
                3.0,
                14.0,
                0.9,
            );
            for offset in [
                Vector3::new(3.0, 0.25, 0.0),
                Vector3::new(-3.0, 0.25, 0.0),
                Vector3::new(0.0, 0.25, 3.0),
                Vector3::new(0.0, 0.25, -3.0),
            ] {
                self.spawn_light_fx(req.pos + offset, req.color, 1.2, 6.0, 0.55);
            }
            self.play_sfx_at(&SFX_DEATH, req.pos);
        }
    }

    /// Спавн снарядов врага (веер вокруг направления на игрока).
    pub(super) fn spawn_enemy_shot(&mut self, req: crate::enemy::ShotReq) {
        for i in 0..req.count {
            let a = (i as f32 - (req.count as f32 - 1.0) * 0.5) * req.spread;
            let (s, c) = a.sin_cos();
            let d = Vector3::new(
                req.dir.x * c - req.dir.z * s,
                req.dir.y,
                req.dir.x * s + req.dir.z * c,
            )
            .normalized();

            let mut node = Node3D::new_alloc();
            node.set_position(req.origin);
            if let Some(mut sp) = make_billboard(
                &mut self.cache,
                "res://assets/effects/effect_energy.png",
                Vector3::ZERO,
                0.007,
            ) {
                sp.set_modulate(req.color);
                node.add_child(&sp);
            }
            let l = make_light(Vector3::ZERO, req.color, 0.7, 4.5);
            node.add_child(&l);
            self.base_mut().add_child(&node);
            self.enemy_projectiles.push(EnemyProjectile {
                node,
                pos: req.origin,
                vel: d * req.speed,
                dmg: req.damage,
                ttl: 3.5,
                status: req.status.clone(),
            });
        }
    }

    /// Полёт вражеских снарядов: сегментный рейкаст, попадание в игрока — урон,
    /// любое другое тело (стена, свой же) гасит снаряд.
    pub(super) fn tick_enemy_projectiles(&mut self, dt: f32) {
        let mut player_dmg = 0.0f32;
        let mut player_statuses: Vec<String> = Vec::new();
        let mut wall_fx: Vec<Vector3> = Vec::new();

        let mut i = 0;
        while i < self.enemy_projectiles.len() {
            let from = self.enemy_projectiles[i].pos;
            let new_pos = from + self.enemy_projectiles[i].vel * dt;
            let mut remove = false;

            let hit = (|| {
                let world = self.base().get_world_3d()?;
                let mut space = world.clone().get_direct_space_state()?;
                let query = PhysicsRayQueryParameters3D::create(from, new_pos)?;
                let hit = space.intersect_ray(&query);
                if hit.is_empty() {
                    return None;
                }
                let pos = hit.get("position")?.try_to::<Vector3>().ok()?;
                let node = hit
                    .get("collider")
                    .and_then(|cv| cv.try_to::<Gd<godot::classes::Node>>().ok());
                Some((pos, node))
            })();

            match hit {
                Some((pos, node)) => {
                    let is_player = node
                        .map(|n| n.try_cast::<Player>().is_ok())
                        .unwrap_or(false);
                    if is_player {
                        player_dmg += self.enemy_projectiles[i].dmg;
                        if let Some(s) = self.enemy_projectiles[i].status.clone() {
                            player_statuses.push(s);
                        }
                    } else {
                        wall_fx.push(pos);
                    }
                    remove = true;
                }
                None => {
                    self.enemy_projectiles[i].pos = new_pos;
                    self.enemy_projectiles[i].node.set_position(new_pos);
                    self.enemy_projectiles[i].ttl -= dt;
                    if self.enemy_projectiles[i].ttl <= 0.0 {
                        remove = true;
                    }
                }
            }

            if remove {
                let p = self.enemy_projectiles.remove(i);
                p.node.free();
            } else {
                i += 1;
            }
        }

        for pos in wall_fx {
            self.spawn_fx("res://assets/effects/effect_bullet.png", pos, 0.005, 0.18);
        }
        if player_dmg > 0.0 {
            self.damage_player(player_dmg);
        }
        for id in player_statuses {
            self.apply_status_to_player(&id);
        }
    }

    pub(super) fn make_pickup_node(&mut self, tex_path: &str, pos: Vector3, px: f32) -> Gd<Node3D> {
        let mut node = Node3D::new_alloc();
        node.set_position(pos + Vector3::new(0.0, 0.55, 0.0));
        if let Some(mut sp) = make_billboard(&mut self.cache, tex_path, Vector3::ZERO, px) {
            if tex_path.contains("/sprites/items/") || tex_path.contains("/sprites/pickups/") {
                sp.set_region_enabled(true);
                sp.set_texture_filter(TextureFilter::NEAREST);
                sp.set_region_rect(Rect2::new(Vector2::ZERO, Vector2::new(64.0, 64.0)));
            }
            node.add_child(&sp);
        }
        self.base_mut().add_child(&node);
        node
    }

    pub(super) fn spawn_item(
        &mut self,
        cfg: &GameConfig,
        kind: &str,
        pos: Vector3,
        in_dungeon: bool,
    ) {
        // специальные предметы вне items.json
        if kind == "heart_1up" {
            let node =
                self.make_pickup_node("res://assets/sprites/pickups/heart_1up.png", pos, 0.010);
            self.world_items.push(WorldItemNode {
                node,
                item_id: "heart_1up".into(),
                name: if self.settings.lang == "en" {
                    "Heart of Life"
                } else {
                    "Сердце жизни"
                }
                .into(),
                payload: Payload::Heart,
                in_dungeon,
                ground_offset: 0.55,
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
        let name = if self.settings.lang == "en" {
            icfg.name_en.clone()
        } else {
            icfg.name_ru.clone()
        };
        let payload = if icfg.category == "currency" {
            Payload::Gold(icfg.value as i32)
        } else if icfg.category == "key" {
            Payload::KeyItem
        } else {
            Payload::Consumable {
                heal: icfg.heal.unwrap_or(10.0),
            }
        };
        self.world_items.push(WorldItemNode {
            node,
            item_id: icfg.id.clone(),
            name,
            payload,
            in_dungeon,
            ground_offset: 0.55,
        });
    }

    pub(super) fn spawn_ammo_pickup(
        &mut self,
        t: AmmoType,
        amount: u32,
        pos: Vector3,
        in_dungeon: bool,
    ) {
        let node = self.make_pickup_node(t.pickup_tex(), pos, 0.009);
        self.world_items.push(WorldItemNode {
            node,
            item_id: format!("ammo_{}", t.idx()),
            name: t.name(&self.settings.lang).to_string(),
            payload: Payload::Ammo(t, amount),
            in_dungeon,
            ground_offset: 0.55,
        });
    }

    pub(super) fn spawn_weapon_pickup(&mut self, w: WeaponId, pos: Vector3, in_dungeon: bool) {
        let def = weapon_def(w);
        let mut node = Node3D::new_alloc();
        node.set_position(pos + Vector3::new(0.0, 0.65, 0.0));
        if let Some(mut sp) = make_billboard(&mut self.cache, &def.sheet, Vector3::ZERO, 0.012) {
            sp.set_region_enabled(true);
            sp.set_texture_filter(TextureFilter::NEAREST);
            sp.set_region_rect(Rect2::new(
                Vector2::ZERO,
                Vector2::new(FRAME_W, def.frame_h),
            ));
            node.add_child(&sp);
        }
        let l = make_light(Vector3::new(0.0, 0.4, 0.0), C_PINK, 0.7, 4.0);
        node.add_child(&l);
        self.base_mut().add_child(&node);
        self.world_items.push(WorldItemNode {
            node,
            item_id: format!("weapon_{}", def.id.id()),
            name: def.name(&self.settings.lang).to_string(),
            payload: Payload::Weapon(w),
            in_dungeon,
            ground_offset: 0.65,
        });
    }
}

pub(super) fn weapon_by_name(s: &str) -> Option<WeaponId> {
    Some(match s {
        "sword" => WeaponId::Sword,
        "chainsaw" => WeaponId::Chainsaw,
        "pistol" => WeaponId::Pistol,
        "shotgun" => WeaponId::Shotgun,
        "rifle" => WeaponId::Rifle,
        "nailgun" => WeaponId::Nailgun,
        "plasma" => WeaponId::Plasma,
        "rocket" => WeaponId::Rocket,
        _ => return None,
    })
}
