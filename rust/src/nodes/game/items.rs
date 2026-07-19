//! Подбор предметов, инвентарь и перки.

use super::*;

impl Game3D {
    // ── Подбор предметов ─────────────────────────────────────────────────────

    pub(super) fn pick_up_item(&mut self, idx: usize) {
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

    pub(super) fn use_first_consumable(&mut self) {
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

    pub(super) fn spawn_fx_on_player(&mut self, tex: &str) {
        if let Some(ref p) = self.player {
            let pos = p.get_global_position() + Vector3::new(0.0, 1.2, 0.0);
            self.spawn_fx(tex, pos, 0.010, 0.5);
        }
    }

    // ── Инвентарь ────────────────────────────────────────────────────────────

    pub(super) fn open_inventory(&mut self) {
        self.mode = Mode::Inventory;
        self.freeze_player(true);
        self.refresh_inventory_ui();
        if let Some(ref mut p) = self.inv_panel { p.set_visible(true); }
        if let Some(ref mut lbl) = self.hint_label { lbl.set_visible(false); }
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
    }

    pub(super) fn close_inventory(&mut self) {
        if let Some(ref mut p) = self.inv_panel { p.set_visible(false); }
        self.set_mode_explore();
    }

    pub(super) fn refresh_inventory_ui(&mut self) {
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

    pub(super) fn open_perks(&mut self) {
        self.mode = Mode::Perks;
        self.freeze_player(true);
        self.refresh_perk_ui();
        if let Some(ref mut p) = self.perk_panel { p.set_visible(true); }
        if let Some(ref mut lbl) = self.hint_label { lbl.set_visible(false); }
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
    }

    pub(super) fn close_perks(&mut self) {
        if let Some(ref mut p) = self.perk_panel { p.set_visible(false); }
        self.set_mode_explore();
    }

    pub(super) fn process_perks(&mut self) {
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

    pub(super) fn buy_perk_at(&mut self, n: usize) {
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

    pub(super) fn refresh_perk_ui(&mut self) {
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

}
