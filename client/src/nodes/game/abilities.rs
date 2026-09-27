use super::*;
use openheart_core::combat::ability::AbilityDef;

impl Game3D {
    fn player_ability(&self, slot: usize) -> Option<AbilityDef> {
        let class = self.state.as_ref()?.class_idx?;
        self.cfg.as_ref()?.player_abilities.iter().find(|a| a.class == class && a.slot == slot).cloned()
    }

    pub(super) fn try_player_ability(&mut self, slot: usize) {
        if self.mode != Mode::Explore { return; }
        let Some(def) = self.player_ability(slot) else { return; };
        let Some(player) = self.player().and_then(|p| p.try_cast::<Player>().ok()) else { return; };
        let (alive, stunned) = { let p = player.bind(); (!p.dead && p.hp > 0.0, p.stunned) };
        if !alive || stunned { return; }
        if !self.state.as_ref().unwrap().abilities.ready(slot) {
            self.show_flash(if self.settings.lang == "en" { "Ability is recovering" } else { "Умение восстанавливается" });
            return;
        }
        if self.net_ready() {
            let net = self.net.as_mut().unwrap();
            net.send_command("choose_class", serde_json::json!({"class":def.class}));
            net.send_command("ability", serde_json::json!({"slot":slot}));
            return;
        }
        // Offline умение восстанавливается быстрее с учётом cd_mult спека и перков.
        let cd_mult = self.loadout.cd_mult;
        if !self.state.as_mut().unwrap().abilities.activate(&def, cd_mult, alive, stunned) { return; }
        let (origin, forward) = { let p = player.bind(); (p.eye_pos(), p.aim_dir()) };
        let from = convert::to_core(origin);
        let mut candidates: Vec<_> = self.enemies.iter().enumerate().filter_map(|(i,e)| {
            let target = e.get_global_position() + Vector3::UP;
            if e.bind().alive && def.hits(from, convert::to_core(forward), convert::to_core(target)) {
                Some((i, origin.distance_to(target), target))
            } else { None }
        }).collect();
        candidates.sort_by(|a,b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        let mut hits = 0;
        for (index, _, target) in candidates {
            if hits >= def.max_targets { break; }
            // A target must be the first physical hit: abilities never pass through walls.
            if let Some((_, Some(hit_enemy))) = self.raycast(origin, target) {
                if hit_enemy.instance_id() != self.enemies[index].instance_id() { continue; }
            } else { continue; }
            let dealt = self.enemies[index].bind_mut().take_damage(def.damage, DmgType::from_id(&def.damage_type).unwrap());
            if dealt <= 0.0 { continue; }
            hits += 1;
            if def.slow > 0.0 {
                let status = self.cfg.as_ref().and_then(|cfg| cfg.statuses.iter().find(|s| s.kind == "slow")).cloned();
                if let Some(mut status) = status {
                    status.id = "player_ability_slow".into();
                    status.duration = def.duration;
                    let slow = if self.enemies[index].bind().is_boss { def.slow.min(0.25) } else { def.slow };
                    status.amount = slow;
                    self.enemies[index].bind_mut().apply_status(&status);
                }
            }
            self.spawn_tinted_fx("res://assets/effects/effect_energy.png", target, 0.016, 0.35, Self::ability_color(def.icon));
        }
        if def.heal_per_hit > 0.0 { player.clone().bind_mut().heal(hits as f32 * def.heal_per_hit); }
        self.ability_feedback(&def, origin);
    }

    pub(super) fn ability_feedback(&mut self, def: &AbilityDef, origin: Vector3) {
        self.spawn_light_fx(origin, Self::ability_color(def.icon), 1.8, 5.0, 0.25);
        self.play_sfx_at(&["res://assets/sounds/An Evil Robot Is Walking.wav"], origin);
        self.show_flash(def.name(&self.settings.lang).to_string().as_str());
    }

    fn ability_color(icon: usize) -> Color {
        match icon {
            0 | 1 => Color::from_rgb(1.0, 0.12, 0.35),
            2 => Color::from_rgb(1.0, 0.65, 0.2),
            5 => Color::from_rgb(0.65, 0.3, 1.0),
            _ => Color::from_rgb(0.2, 0.85, 1.0),
        }
    }

    pub(super) fn on_ability_cast(&mut self, event: &openheart_core::sim::Event) {
        let Some(def) = self.cfg.as_ref().and_then(|c| c.player_abilities.iter().find(|a| a.id == event.text)).cloned() else { return; };
        if event.actor == self.local_peer {
            // Сетевой каст спек-нейтрален (cd_mult=1.0) — кулдаун совпадает с сервером.
            if let Some(state) = self.state.as_mut() { state.abilities.activate(&def, 1.0, true, false); }
        }
        if let Some(pos) = event.pos { self.ability_feedback(&def, convert::to_godot(pos)); }
    }

    pub(super) fn build_ability_hud(&mut self) {
        let mut layer = CanvasLayer::new_alloc();
        layer.set_layer(2);
        let texture = self.cache.get("res://assets/icons/abilities_atlas.png");
        for slot in 0..2 {
            let mut panel = Panel::new_alloc();
            place(&panel, 0.5, 1.0, 790.0 + slot as f32 * 180.0, 976.0, 172.0, 94.0);
            panel.add_theme_stylebox_override("panel", &make_style(C_UI_BG, C_BORDER, 1));
            let mut icon = TextureRect::new_alloc();
            icon.set_position(Vector2::new(8.0,8.0));
            icon.set_size(Vector2::new(66.0,66.0));
            icon.set_expand_mode(godot::classes::texture_rect::ExpandMode::IGNORE_SIZE);
            let mut atlas = AtlasTexture::new_gd();
            if let Some(texture) = texture.as_ref() { atlas.set_atlas(texture); }
            atlas.set_region(Rect2::new(Vector2::ZERO,Vector2::new(512.0,512.0)));
            icon.set_texture(&atlas);
            panel.add_child(&icon);
            let mut label = Label::new_alloc();
            label.set_position(Vector2::new(80.0,6.0));
            label.set_size(Vector2::new(88.0,84.0));
            label.add_theme_font_size_override("font_size",16);
            label.set_autowrap_mode(godot::classes::text_server::AutowrapMode::WORD_SMART);
            panel.add_child(&label);
            layer.add_child(&panel);
            self.ability_labels.push(label);
            self.ability_icons.push(atlas);
        }
        self.base_mut().add_child(&layer);
    }

    pub(super) fn update_ability_hud(&mut self) {
        for slot in 0..2 {
            let Some(def) = self.player_ability(slot) else { continue; };
            let (cd, charges, max) = {
                let a = &self.state.as_ref().unwrap().abilities;
                (a.cooldowns[slot], a.charges[slot], a.max_charges[slot].max(1))
            };
            let ready = charges > 0;
            let en = self.settings.lang == "en";
            if let Some(label) = self.ability_labels.get_mut(slot) {
                let status = if ready {
                    let base = if en { "READY" } else { "ГОТОВО" };
                    if max > 1 { format!("{base} ×{charges}") } else { base.to_string() }
                } else {
                    format!("{:.1}s", cd)
                };
                label.set_text(&format!("[{}] {}\n{}", if slot==0 {"F"} else {"G"}, def.name(&self.settings.lang), status));
                label.add_theme_color_override("font_color", if ready {Self::ability_color(def.icon)} else {C_DIM});
                label.set_tooltip_text(def.description(&self.settings.lang));
            }
            if let Some(atlas) = self.ability_icons.get_mut(slot) {
                atlas.set_region(Rect2::new(Vector2::new((def.icon%3) as f32*512.0,(def.icon/3) as f32*512.0),Vector2::new(512.0,512.0)));
            }
        }
    }
}
