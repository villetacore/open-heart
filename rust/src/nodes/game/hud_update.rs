//! Обновление HUD, анимация NPC, флэш-сообщения, сохранение.

use super::*;

impl Game3D {
    pub(super) fn build_quest_navigation_marker(&mut self) {
        let Some(mut marker) = make_billboard(
            &mut self.cache,
            "res://assets/sprites/pickups/soul.png",
            Vector3::ZERO,
            0.013,
        ) else {
            return;
        };
        marker.set_name("TrackedQuestMarker");
        marker.set_visible(false);
        marker.add_to_group("quest_objective_markers");
        self.base_mut().add_child(&marker);
        self.quest_objective_marker = Some(marker);
    }

    fn quest_navigation_color(giver: &str) -> Color {
        match giver {
            "hunter" => Color::from_rgba(1.0, 0.28, 0.2, 1.0),
            "medic" => Color::from_rgba(0.25, 1.0, 0.58, 1.0),
            "collector" => Color::from_rgba(1.0, 0.78, 0.2, 1.0),
            "scientist" => Color::from_rgba(0.25, 0.82, 1.0, 1.0),
            _ => Color::from_rgba(1.0, 0.3, 0.72, 1.0),
        }
    }

    fn quest_navigation_target(&self, quest: &crate::config::QuestCfg) -> Option<Vector3> {
        let player_position = self.player.as_ref().map(|player| player.get_position())?;
        let in_dungeon = self.loc == Loc::Dungeon;
        let nearest_enemy = self
            .enemies
            .iter()
            .filter_map(|enemy| {
                let binding = enemy.bind();
                let matches = binding.alive
                    && (quest.kind == "kill_any"
                        || (quest.kind == "boss" && binding.is_boss)
                        || binding.cfg_id.to_string() == quest.target);
                matches.then(|| enemy.get_position())
            })
            .min_by(|left, right| {
                left.distance_squared_to(player_position)
                    .total_cmp(&right.distance_squared_to(player_position))
            });
        if matches!(quest.kind.as_str(), "kill" | "kill_any" | "boss") {
            if let Some(target) = nearest_enemy {
                return Some(target);
            }
        }

        if matches!(
            quest.kind.as_str(),
            "collect" | "collect_category" | "discover_weapon"
        ) {
            if let Some(target) = self
                .world_items
                .iter()
                .filter(|item| item.in_dungeon == in_dungeon)
                .filter(|item| match quest.kind.as_str() {
                    "collect" => item.item_id == quest.target,
                    "collect_category" => self
                        .cfg
                        .as_ref()
                        .and_then(|config| config.item(&item.item_id))
                        .is_some_and(|config_item| config_item.category == quest.target),
                    "discover_weapon" => matches!(item.payload, Payload::Weapon(_)),
                    _ => false,
                })
                .map(|item| item.node.get_position())
                .min_by(|left, right| {
                    left.distance_squared_to(player_position)
                        .total_cmp(&right.distance_squared_to(player_position))
                })
            {
                return Some(target);
            }
        }

        if quest.kind == "interact" && !in_dungeon {
            if let Some((index, _)) = self
                .npcs
                .iter()
                .enumerate()
                .find(|(_, npc)| npc.id == quest.target)
            {
                return self
                    .npc_sprites
                    .get(index)
                    .map(|sprite| sprite.get_position());
            }
        }
        if in_dungeon {
            let event_kind = match quest.kind.as_str() {
                "solve_puzzle" => Some("puzzle_switch"),
                "discover_lore" => Some("story_echo"),
                _ => None,
            };
            if let Some(kind) = event_kind {
                if let Some(event) = self
                    .dungeon_events
                    .iter()
                    .find(|event| event.kind == kind && !event.used)
                {
                    return Some(event.pos);
                }
            }
            if matches!(quest.kind.as_str(), "clear_dungeon" | "enter_dungeon") {
                return Some(self.next_portal);
            }
            if quest.kind == "interact" {
                return Some(self.exit_portal);
            }
        } else if matches!(
            quest.kind.as_str(),
            "clear_dungeon"
                | "enter_dungeon"
                | "solve_puzzle"
                | "discover_lore"
                | "discover_weapon"
                | "boss"
        ) {
            return Some(self.gate_pos);
        }
        None
    }

    pub(super) fn update_quest_navigation(&mut self, lang: &str) {
        let active: Vec<_> = self
            .state
            .as_ref()
            .into_iter()
            .flat_map(|state| state.quests.quests.iter())
            .filter(|quest| quest.state == crate::quest::QuestState::Active)
            .filter_map(|quest| self.cfg.as_ref()?.quest(&quest.id).cloned())
            .collect();
        if active.is_empty() {
            self.quest_objective_target = None;
            if let Some(marker) = self.quest_objective_marker.as_mut() {
                marker.set_visible(false);
            }
            if let Some(label) = self.quest_objective_label.as_mut() {
                label.set_visible(false);
            }
            return;
        }
        self.tracked_quest_index %= active.len();
        let quest = &active[self.tracked_quest_index];
        let target = self.quest_navigation_target(quest);
        let target_changed = target != self.quest_objective_target;
        self.quest_objective_target = target;
        let distance = target
            .zip(self.player.as_ref().map(|player| player.get_position()))
            .map(|(target, player)| target.distance_to(player).round() as i32);
        let progress = self.quest_progress(quest).min(quest.count);
        let distance_text = distance.map_or_else(String::new, |meters| {
            if lang == "en" {
                format!("  ·  {meters} m")
            } else {
                format!("  ·  {meters} м")
            }
        });
        let text = format!(
            "[T] {}: {}  {}/{}{}\n{}",
            if lang == "en" { "TRACKED" } else { "ЦЕЛЬ" },
            quest.title(lang),
            progress,
            quest.count,
            distance_text,
            quest.description(lang)
        );
        if let Some(label) = self.quest_objective_label.as_mut() {
            label.set_text(&text);
            label.set_visible(true);
            label.set_modulate(Self::quest_navigation_color(&quest.giver));
        }
        if let Some(marker) = self.quest_objective_marker.as_mut() {
            if let Some(position) = target {
                marker.set_position(position + Vector3::UP * (2.2 + self.game_time.sin() * 0.35));
                marker.set_scale(Vector3::ONE * (1.0 + self.game_time.sin().abs() * 0.18));
                marker.set_modulate(Self::quest_navigation_color(&quest.giver));
                marker.set_visible(true);
            } else {
                marker.set_visible(false);
            }
        }
        if target_changed && self.loc == Loc::Dungeon {
            self.build_minimap_texture();
        }
    }

    // ── Обновление HUD ────────────────────────────────────────────────────────

    pub(super) fn update_hp_bar(&mut self) {
        let lang = self.settings.lang.clone();
        let (hp, max_hp) = if let Some(ref p) = self.player {
            if let Ok(player) = p.clone().try_cast::<Player>() {
                (player.bind().hp, player.bind().max_hp)
            } else {
                (100.0, 100.0)
            }
        } else {
            (100.0, 100.0)
        };
        // цель для плавного бара — ширину/цвет крутит tick_hud_anim
        self.hp_target = (hp / max_hp).clamp(0.0, 1.0);
        if let Some(ref mut lbl) = self.hp_label {
            lbl.set_text(&format!("{}: {:.0}/{:.0}", t("hud_hp", &lang), hp, max_hp));
        }
        // статусы игрока (иконки)
        let st = self.player_statuses.summary();
        if let Some(ref mut lbl) = self.status_label {
            lbl.set_text(&st);
        }
    }

    pub(super) fn update_xp_bar(&mut self) {
        let (level, xp, next) = self
            .state
            .as_ref()
            .map(|s| (s.level, s.xp, xp_to_next(s.level)))
            .unwrap_or((1, 0, 100));
        self.xp_target = (xp as f32 / next as f32).clamp(0.0, 1.0);
        if let Some(ref mut lbl) = self.xp_label {
            lbl.set_text(&format!(
                "{} {}  ({}/{})",
                if self.settings.lang == "en" {
                    "lvl."
                } else {
                    "ур."
                },
                level,
                xp,
                next
            ));
        }
    }

    /// Плавная анимация HUD: бары лерпят к цели, HP-бар пульсирует и краснеет
    /// при низком здоровье + красная виньетка через пост-процесс.
    pub(super) fn tick_hud_anim(&mut self, dt: f32) {
        self.hud_time += dt;
        let k = (dt * 9.0).min(1.0);
        self.hp_shown += (self.hp_target - self.hp_shown) * k;
        self.xp_shown += (self.xp_target - self.xp_shown) * (dt * 6.0).min(1.0);
        let hp = self.hp_shown;

        // низкое HP: доля тревоги 0..1 + пульс
        let low = ((0.35 - hp) / 0.35).clamp(0.0, 1.0);
        let pulse = 0.6 + 0.4 * (self.hud_time * 5.5).sin();

        if let Some(ref mut fg) = self.hp_bar_fg {
            fg.set_size(Vector2::new(218.0 * hp, 22.0));
            let c = Color::from_rgba(0.9 - hp * 0.5, 0.1 + hp * 0.3, 0.1, 1.0);
            fg.add_theme_stylebox_override("panel", &make_style(c, Color::TRANSPARENT_BLACK, 0));
            // при низком HP бар пульсирует яркостью
            let a = 1.0 - low * (1.0 - pulse) * 0.5;
            fg.set_modulate(Color::from_rgba(1.0, 1.0, 1.0, a));
        }
        if let Some(ref mut fg) = self.xp_bar_fg {
            fg.set_size(Vector2::new(220.0 * self.xp_shown, 8.0));
        }
        // красная виньетка по здоровью (пульсирует)
        if let Some(mat) = self.post_mat.as_mut() {
            mat.set_shader_parameter("lowhp", &(low * pulse).to_variant());
        }
    }

    pub(super) fn update_ammo_hud(&mut self) {
        let has_any = self.arsenal.owned.iter().any(|o| *o);
        if !has_any {
            if let Some(ref mut l) = self.ammo_label {
                l.set_text("");
            }
            return;
        }
        let def = weapon_def(self.arsenal.current);
        let text = match def.ammo {
            None => "∞".to_string(),
            Some((t, _)) if def.magazine > 0 => format!(
                "{}  {}/{}",
                t.name(&self.settings.lang),
                self.arsenal.clips[self.arsenal.current.slot()],
                self.arsenal.ammo_of(t)
            ),
            Some((t, _)) => format!(
                "{}  {}",
                t.name(&self.settings.lang),
                self.arsenal.ammo_of(t)
            ),
        };
        if let Some(ref mut l) = self.ammo_label {
            l.set_text(&text);
        }
    }

    pub(super) fn update_inv_label(&mut self) {
        let lang = self.settings.lang.clone();
        if let Some(ref state) = self.state {
            let text = if state.inventory.is_empty() {
                format!("{}: {}", t("hud_gold", &lang), state.gold)
            } else {
                let items: Vec<_> = state
                    .inventory
                    .items
                    .iter()
                    .map(|i| format!("{} ×{}", i.name, i.qty))
                    .collect();
                format!(
                    "{}  |  {}: {}  [ I ]",
                    items.join(", "),
                    t("hud_gold", &lang),
                    state.gold
                )
            };
            if let Some(ref mut lbl) = self.inv_label {
                lbl.set_text(&text);
            }
        }
    }

    pub(super) fn update_quest_label(&mut self, lang: &str) {
        if let Some(ref state) = self.state {
            let mut lines: Vec<String> = Vec::new();
            if state.perk_points > 0 {
                lines.push(format!(
                    "[P] {}: {} {}!",
                    if lang == "en" { "Perks" } else { "Перки" },
                    state.perk_points,
                    if lang == "en" { "pts." } else { "очк." }
                ));
            }
            let active: Vec<_> = state
                .quests
                .quests
                .iter()
                .filter(|q| q.state == crate::quest::QuestState::Active)
                .collect();
            if !active.is_empty() {
                lines.push(t("hud_quests", lang).to_string());
                for q in active.iter().take(5) {
                    if let Some(config_quest) =
                        self.cfg.as_ref().and_then(|config| config.quest(&q.id))
                    {
                        let progress = self.quest_progress(config_quest).min(config_quest.count);
                        let chain = config_quest.chain(lang);
                        let stage = if config_quest.stage > 0 {
                            format!(
                                "{} {}",
                                if lang == "en" { "Stage" } else { "Этап" },
                                config_quest.stage
                            )
                        } else {
                            String::new()
                        };
                        let prefix = [chain, stage.as_str()]
                            .into_iter()
                            .filter(|part| !part.is_empty())
                            .collect::<Vec<_>>()
                            .join(" · ");
                        lines.push(format!(
                            "• {}{}  {}/{}",
                            if prefix.is_empty() {
                                String::new()
                            } else {
                                format!("[{prefix}] ")
                            },
                            config_quest.title(lang),
                            progress,
                            config_quest.count
                        ));
                        continue;
                    }
                    lines.push(format!("• {}", q.title));
                }
            }
            let text = lines.join("\n");
            if let Some(ref mut lbl) = self.quest_label {
                lbl.set_text(&text);
            }
        }
    }

    pub(super) fn update_targeting_hud(&mut self) {
        let is_en = self.settings.lang == "en";
        let text = if let Some(idx) = self.near_enemy {
            if idx < self.enemies.len() {
                let eb = self.enemies[idx].bind();
                if eb.alive {
                    let ratio = (eb.hp / eb.max_hp).clamp(0.0, 1.0);
                    let filled = (ratio * 10.0).round() as usize;
                    let bar: String = "█".repeat(filled) + &"░".repeat(10 - filled.min(10));
                    let boss = if eb.is_boss {
                        if is_en {
                            "GUARDIAN "
                        } else {
                            "СТРАЖ "
                        }
                    } else if eb.is_elite() {
                        "⭐ "
                    } else {
                        ""
                    };
                    let st = eb.status_summary();
                    let st = if st.is_empty() {
                        String::new()
                    } else {
                        format!("  {st}")
                    };
                    let cast = if eb.is_casting() {
                        if is_en {
                            "  ⚠ CASTING"
                        } else {
                            "  ⚠ ПОДГОТОВКА АТАКИ"
                        }
                    } else {
                        ""
                    };
                    let weak_point = if is_en {
                        format!("  ◇ WEAK ×{:.2}", eb.weak_point_multiplier())
                    } else {
                        format!("  ◇ СЛАБОЕ МЕСТО ×{:.2}", eb.weak_point_multiplier())
                    };
                    format!(
                        "{}[{}]  {}  {:.0}/{:.0}{}{}{}",
                        boss,
                        eb.display_name(),
                        bar,
                        eb.hp,
                        eb.max_hp,
                        st,
                        cast,
                        weak_point
                    )
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if let Some(ref mut lbl) = self.targeting_label {
            if text.is_empty() {
                lbl.set_visible(false);
            } else {
                lbl.set_text(&text);
                lbl.set_visible(true);
            }
        }
    }

    /// Семантическая текстура миникарты: маршруты, комнаты, ловушки и порталы.
    pub(super) fn build_minimap_texture(&mut self) {
        let g = dungeon::GRID;
        if self.minimap_floor.len() < g * g {
            return;
        }
        let Some(mut img) = Image::create_empty(g as i32, g as i32, false, Format::RGBA8) else {
            return;
        };
        let background = Color::from_rgba(0.025, 0.018, 0.055, 0.96);
        let floor = Color::from_rgba(0.23, 0.17, 0.34, 1.0);
        let edge = Color::from_rgba(0.48, 0.34, 0.66, 1.0);
        img.fill(background);
        for j in 0..g {
            for i in 0..g {
                if self.minimap_floor[j * g + i] {
                    let boundary = [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)].iter().any(
                        |(offset_x, offset_z)| {
                            let x = i as i32 + offset_x;
                            let z = j as i32 + offset_z;
                            x < 0
                                || z < 0
                                || x >= g as i32
                                || z >= g as i32
                                || !self.minimap_floor[z as usize * g + x as usize]
                        },
                    );
                    img.set_pixel(i as i32, j as i32, if boundary { edge } else { floor });
                }
            }
        }
        let to_cell = |position: Vector3| -> (i32, i32) {
            (
                (position.x / dungeon::CELL + g as f32 * 0.5 - 0.5).round() as i32,
                (position.z / dungeon::CELL + g as f32 * 0.5 - 0.5).round() as i32,
            )
        };
        let mut draw_marker = |x: i32, z: i32, radius: i32, color: Color| {
            for offset_z in -radius..=radius {
                for offset_x in -radius..=radius {
                    let pixel_x = x + offset_x;
                    let pixel_z = z + offset_z;
                    if pixel_x >= 0 && pixel_z >= 0 && pixel_x < g as i32 && pixel_z < g as i32 {
                        img.set_pixel(pixel_x, pixel_z, color);
                    }
                }
            }
        };
        for (role, position) in &self.dungeon_room_markers {
            let color = match role.as_str() {
                "safe" => Color::from_rgba(0.28, 1.0, 0.68, 1.0),
                "treasure" => Color::from_rgba(1.0, 0.78, 0.18, 1.0),
                "ritual" => Color::from_rgba(0.9, 0.26, 1.0, 1.0),
                "traversal" => Color::from_rgba(0.18, 0.78, 1.0, 1.0),
                "puzzle" => Color::from_rgba(0.25, 1.0, 0.82, 1.0),
                "story" => Color::from_rgba(0.72, 0.58, 1.0, 1.0),
                "antechamber" => Color::from_rgba(1.0, 0.28, 0.48, 1.0),
                _ => Color::from_rgba(0.68, 0.5, 0.82, 1.0),
            };
            let (x, z) = to_cell(*position);
            draw_marker(x, z, 0, color);
        }
        for hazard in &self.dungeon_hazards {
            let (x, z) = to_cell(hazard.pos - DUNGEON_OFFSET);
            draw_marker(x, z, 1, hazard.kind.color());
        }
        if let Some(target) = self.quest_objective_target {
            let (x, z) = to_cell(target - DUNGEON_OFFSET);
            draw_marker(x, z, 1, Color::from_rgba(1.0, 0.86, 0.22, 1.0));
        }
        let (exit_x, exit_z) = to_cell(self.exit_portal - DUNGEON_OFFSET);
        draw_marker(exit_x, exit_z, 1, Color::from_rgba(0.32, 0.92, 1.0, 1.0));
        let (next_x, next_z) = to_cell(self.next_portal - DUNGEON_OFFSET);
        draw_marker(next_x, next_z, 1, Color::from_rgba(1.0, 0.24, 0.62, 1.0));
        if let Some(tex) = ImageTexture::create_from_image(&img) {
            let tex2d = tex.upcast::<Texture2D>();
            if let Some(ref mut rect) = self.minimap_rect {
                rect.set_texture(&tex2d);
            }
        }
    }

    /// Точка игрока на миникарте.
    pub(super) fn update_minimap(&mut self) {
        if self.loc != Loc::Dungeon {
            return;
        }
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

        let dot_x = MAP_X + pi * scale - 4.0;
        let dot_y = MAP_Y + pj * scale - 2.0;
        if let Some(ref mut dot) = self.minimap_dot {
            dot.set_position(Vector2::new(dot_x, dot_y));
            if let Ok(player) = p.clone().try_cast::<Player>() {
                dot.set_rotation(-player.bind().yaw());
            }
            dot.set_modulate(if self.dungeon_hazard_active {
                Color::from_rgba(1.0, 0.35, 0.2, 1.0)
            } else {
                Color::WHITE
            });
        }
    }

    pub(super) fn update_compass(&mut self) {
        let yaw = if let Some(ref p) = self.player {
            if let Ok(player) = p.clone().try_cast::<Player>() {
                player.bind().yaw()
            } else {
                0.0
            }
        } else {
            0.0
        };
        let tau = std::f32::consts::TAU;
        let sector = ((yaw.rem_euclid(tau) / tau * 8.0 + 0.5) as usize) % 8;
        let dirs = ["N", "NW", "W", "SW", "S", "SE", "E", "NE"];
        if let Some(ref mut lbl) = self.compass_label {
            lbl.set_text(dirs[sector]);
        }
    }

    // ── Анимация NPC ─────────────────────────────────────────────────────────

    pub(super) fn tick_npc_anim(&mut self, dt: f32) {
        self.npc_anim_timer += dt;
        if self.npc_anim_timer < 1.0 / IDLE_FPS {
            return;
        }
        self.npc_anim_timer = 0.0;
        self.npc_anim_frame = (self.npc_anim_frame + 1) % NPC_IDLE_FRAMES.len();
        let (x, y, w, h) = NPC_IDLE_FRAMES[self.npc_anim_frame];
        let rect = Rect2::new(Vector2::new(x, y), Vector2::new(w, h));
        for sprite in self.npc_sprites.iter_mut() {
            sprite.set_region_rect(rect);
        }
    }

    // ── Флэш-сообщения ────────────────────────────────────────────────────────

    pub(super) fn show_flash(&mut self, msg: &str) {
        if let Some(ref mut lbl) = self.flash_label {
            lbl.set_text(msg);
            lbl.set_visible(true);
            lbl.add_theme_color_override("font_color", C_GOLD);
        }
        self.flash_timer = 2.5;
    }

    pub(super) fn tick_flash(&mut self, dt: f32) {
        if self.flash_timer > 0.0 {
            self.flash_timer -= dt;
            if self.flash_timer <= 0.0 {
                if let Some(ref mut lbl) = self.flash_label {
                    lbl.set_visible(false);
                    lbl.set_scale(Vector2::ONE);
                }
            } else {
                let alpha = if self.flash_timer < 0.8 {
                    self.flash_timer / 0.8
                } else {
                    1.0
                };
                let c = Color::from_rgba(C_GOLD.r, C_GOLD.g, C_GOLD.b, alpha);
                // «поп» появления: короткий overshoot-скейл в первые 0.2 c
                let age = 2.5 - self.flash_timer;
                let scale = if age < 0.2 {
                    1.0 + (1.0 - age / 0.2) * 0.18
                } else {
                    1.0
                };
                if let Some(ref mut lbl) = self.flash_label {
                    lbl.add_theme_color_override("font_color", c);
                    let sz = lbl.get_size();
                    lbl.set_pivot_offset(sz * 0.5);
                    lbl.set_scale(Vector2::new(scale, scale));
                }
            }
        }
    }

    pub(super) fn tick_damage_flash(&mut self, dt: f32) {
        if self.damage_flash_timer > 0.0 {
            self.damage_flash_timer -= dt;
            let alpha = (self.damage_flash_timer / 0.35).clamp(0.0, 1.0) * 0.42;
            let style = make_style(
                Color::from_rgba(0.8, 0.0, 0.0, alpha),
                Color::TRANSPARENT_BLACK,
                0,
            );
            if let Some(ref mut df) = self.damage_flash {
                df.add_theme_stylebox_override("panel", &style);
                df.set_visible(true);
            }
            if self.damage_flash_timer <= 0.0 {
                if let Some(ref mut df) = self.damage_flash {
                    df.set_visible(false);
                }
            }
        }
    }

    // ── Сохранение ───────────────────────────────────────────────────────────

    pub(super) fn auto_save(&mut self) {
        if !self.autosave_enabled {
            return;
        }
        if let Some(ref state) = self.state {
            let hp = if let Some(ref p) = self.player {
                if let Ok(player) = p.clone().try_cast::<Player>() {
                    player.bind().hp
                } else {
                    100.0
                }
            } else {
                100.0
            };
            save::save(state, hp, &self.arsenal);
        }
    }
}
