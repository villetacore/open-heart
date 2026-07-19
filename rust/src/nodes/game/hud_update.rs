//! Обновление HUD, анимация NPC, флэш-сообщения, сохранение.

use super::*;

impl Game3D {
    // ── Обновление HUD ────────────────────────────────────────────────────────

    pub(super) fn update_hp_bar(&mut self) {
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
        // статусы игрока (иконки)
        let st = self.player_statuses.summary();
        if let Some(ref mut lbl) = self.status_label {
            lbl.set_text(&st);
        }
    }

    pub(super) fn update_xp_bar(&mut self) {
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

    pub(super) fn update_ammo_hud(&mut self) {
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

    pub(super) fn update_inv_label(&mut self) {
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

    pub(super) fn update_quest_label(&mut self, lang: &str) {
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

    pub(super) fn update_targeting_hud(&mut self) {
        let text = if let Some(idx) = self.near_enemy {
            if idx < self.enemies.len() {
                let eb = self.enemies[idx].bind();
                if eb.alive {
                    let ratio = (eb.hp / eb.max_hp).clamp(0.0, 1.0);
                    let filled = (ratio * 10.0).round() as usize;
                    let bar: String = "█".repeat(filled) + &"░".repeat(10 - filled.min(10));
                    let boss = if eb.is_boss { "СТРАЖ " }
                               else if eb.is_elite() { "⭐ " } else { "" };
                    let st = eb.status_summary();
                    let st = if st.is_empty() { String::new() } else { format!("  {st}") };
                    format!("{}[{}]  {}  {:.0}/{:.0}{}", boss, eb.display_name(), bar, eb.hp, eb.max_hp, st)
                } else { String::new() }
            } else { String::new() }
        } else { String::new() };

        if let Some(ref mut lbl) = self.targeting_label {
            if text.is_empty() { lbl.set_visible(false); }
            else { lbl.set_text(&text); lbl.set_visible(true); }
        }
    }

    /// Текстура миникарты из floor_map генератора (1 пиксель = 1 клетка).
    pub(super) fn build_minimap_texture(&mut self) {
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
    pub(super) fn update_minimap(&mut self) {
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

    pub(super) fn update_compass(&mut self) {
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

    pub(super) fn tick_npc_anim(&mut self, dt: f32) {
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

    pub(super) fn tick_damage_flash(&mut self, dt: f32) {
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

    pub(super) fn auto_save(&mut self) {
        if let Some(ref state) = self.state {
            let hp = if let Some(ref p) = self.player {
                if let Ok(player) = p.clone().try_cast::<Player>() { player.bind().hp } else { 100.0 }
            } else { 100.0 };
            save::save(state, hp, &self.arsenal);
        }
    }
}
