//! Подбор предметов, инвентарь и перки.

use super::*;

impl Game3D {
    // ── Подбор предметов ─────────────────────────────────────────────────────

    pub(super) fn pick_up_item(&mut self, idx: usize) {
        self.pick_up_item_impl(idx, true);
    }

    pub(super) fn pick_up_item_impl(&mut self, idx: usize, save_after: bool) {
        if idx >= self.world_items.len() {
            return;
        }
        let lang = self.settings.lang.clone();
        let wi = self.world_items.remove(idx);
        wi.node.free();
        self.near_item = None;
        self.punch_pick();
        let name = wi.name.clone();

        // прогресс collect-квестов
        let picked_id = wi.item_id.clone();
        self.bump_quests("collect", &picked_id);
        let picked_category = self
            .cfg
            .as_ref()
            .and_then(|cfg| cfg.item(&picked_id))
            .map(|item| item.category.clone());
        if let Some(category) = picked_category {
            self.bump_quests("collect_category", &category);
        }

        match wi.payload {
            Payload::Gold(v) => {
                if let Some(ref mut st) = self.state {
                    st.gold += v;
                }
                self.show_flash(&format!(
                    "+{} {}",
                    v,
                    if lang == "en" { "gold" } else { "зол." }
                ));
            }
            Payload::Ammo(t, n) => {
                let added = self.arsenal.add_ammo(t, n, self.loadout.ammo_mult);
                self.show_flash(&format!("+{} {}", added, t.name(&lang)));
            }
            Payload::Weapon(w) => {
                let is_new = self.arsenal.give_weapon(w);
                self.bump_quests("discover_weapon", weapon_def(w).id.id());
                if let Some((t, _)) = weapon_def(w).ammo {
                    self.arsenal
                        .add_ammo(t, t.pack_size(), self.loadout.ammo_mult);
                }
                if is_new {
                    self.arsenal.current = w;
                    self.refresh_weapon_sheet();
                    self.show_flash(&format!(
                        "{}: {}!",
                        if lang == "en" {
                            "NEW WEAPON"
                        } else {
                            "НОВОЕ ОРУЖИЕ"
                        },
                        weapon_def(w).name(&lang)
                    ));
                } else {
                    self.show_flash(&format!(
                        "+{} ({})",
                        if lang == "en" {
                            "ammo"
                        } else {
                            "боеприпасы"
                        },
                        weapon_def(w).name(&lang)
                    ));
                }
            }
            Payload::Heart => {
                if let Some(ref mut st) = self.state {
                    st.add_heart();
                }
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
                self.show_flash(if lang == "en" {
                    "HEART OF LIFE: +15 max HP, fully healed!"
                } else {
                    "СЕРДЦЕ ЖИЗНИ: +15 макс. HP, полное лечение!"
                });
            }
            Payload::KeyItem | Payload::Consumable { .. } => {
                if let Some(ref mut st) = self.state {
                    use crate::item::Item;
                    st.inventory.add(Item::new(&wi.item_id, &name, "", 1));
                }
                self.show_flash(&format!("{}: {}", t("msg_picked_up", &lang), name));
            }
        }
        if save_after {
            self.auto_save();
        }
    }

    pub(super) fn use_first_consumable(&mut self) {
        let lang = self.settings.lang.clone();
        let heal_data = self.state.as_ref().and_then(|s| {
            s.inventory
                .items
                .iter()
                .find(|i| {
                    matches!(
                        i.id.as_str(),
                        "medkit" | "armor_shard" | "potion" | "bread" | "energy_drink"
                    )
                })
                .map(|i| {
                    let amt = match i.id.as_str() {
                        "medkit" => 30.0,
                        "armor_shard" => 20.0,
                        "potion" => 50.0,
                        "energy_drink" => 15.0,
                        _ => 10.0,
                    };
                    (i.id.clone(), amt)
                })
        });
        if let Some((id, amount)) = heal_data {
            let full = self
                .player
                .as_ref()
                .and_then(|p| p.clone().try_cast::<Player>().ok())
                .map(|pl| pl.bind().hp >= pl.bind().max_hp)
                .unwrap_or(true);
            if full {
                self.show_flash(if lang == "en" {
                    "Health is already full"
                } else {
                    "Здоровье уже полное"
                });
                return;
            }
            if let Some(ref mut state) = self.state {
                state.inventory.remove_one(&id);
            }
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
        if let Some(ref mut p) = self.inv_panel {
            p.set_visible(true);
        }
        if let Some(ref mut lbl) = self.hint_label {
            lbl.set_visible(false);
        }
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
    }

    pub(super) fn close_inventory(&mut self) {
        if let Some(ref mut p) = self.inv_panel {
            p.set_visible(false);
        }
        self.set_mode_explore();
    }

    pub(super) fn open_journal(&mut self) {
        self.mode = Mode::Journal;
        self.freeze_player(true);
        self.refresh_quest_journal_ui();
        if let Some(panel) = self.journal_panel.as_mut() {
            panel.set_visible(true);
        }
        if let Some(label) = self.hint_label.as_mut() {
            label.set_visible(false);
        }
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
    }

    pub(super) fn close_journal(&mut self) {
        if let Some(panel) = self.journal_panel.as_mut() {
            panel.set_visible(false);
        }
        self.set_mode_explore();
    }

    pub(super) fn process_journal(&mut self) {
        let input = Input::singleton();
        if input.is_action_just_pressed("journal") || input.is_action_just_pressed("escape") {
            self.close_journal();
            return;
        }
        for (index, action) in ["choice_1", "choice_2", "choice_3", "choice_4", "weapon_5"]
            .into_iter()
            .enumerate()
        {
            if input.is_action_just_pressed(action) {
                self.track_journal_quest(index);
                return;
            }
        }
    }

    pub(super) fn track_journal_quest(&mut self, index: usize) {
        let active_count = self
            .state
            .as_ref()
            .map(|state| {
                state
                    .quests
                    .quests
                    .iter()
                    .filter(|quest| quest.state == crate::quest::QuestState::Active)
                    .count()
            })
            .unwrap_or(0);
        if index >= active_count {
            return;
        }
        self.tracked_quest_index = index;
        let lang = self.settings.lang.clone();
        self.update_quest_navigation(&lang);
        self.refresh_quest_journal_ui();
    }

    pub(super) fn refresh_quest_journal_ui(&mut self) {
        let lang = self.settings.lang.clone();
        let quests = self
            .cfg
            .as_ref()
            .map(|config| config.quests.clone())
            .unwrap_or_default();
        let quest_states: std::collections::HashMap<_, _> = self
            .state
            .as_ref()
            .into_iter()
            .flat_map(|state| state.quests.quests.iter())
            .map(|quest| (quest.id.clone(), quest.state))
            .collect();
        let active_ids: Vec<_> = self
            .state
            .as_ref()
            .into_iter()
            .flat_map(|state| state.quests.quests.iter())
            .filter(|quest| quest.state == crate::quest::QuestState::Active)
            .map(|quest| quest.id.clone())
            .collect();
        let mut chains = Vec::new();
        for quest in &quests {
            if !chains.contains(&quest.chain_en) {
                chains.push(quest.chain_en.clone());
            }
        }
        let mut lines = Vec::new();
        lines.push(if lang == "en" {
            format!(
                "ACTIVE: {}   ·   COMPLETED: {}   ·   CHAINS: {}",
                active_ids.len(),
                quest_states
                    .values()
                    .filter(|state| **state == crate::quest::QuestState::Completed)
                    .count(),
                chains.len()
            )
        } else {
            format!(
                "АКТИВНЫЕ: {}   ·   ЗАВЕРШЕНО: {}   ·   ЦЕПОЧКИ: {}",
                active_ids.len(),
                quest_states
                    .values()
                    .filter(|state| **state == crate::quest::QuestState::Completed)
                    .count(),
                chains.len()
            )
        });
        lines.push(String::new());

        for chain_id in chains {
            let mut chain_quests: Vec<_> = quests
                .iter()
                .filter(|quest| quest.chain_en == chain_id)
                .collect();
            chain_quests.sort_by_key(|quest| quest.stage);
            let chain_title = chain_quests
                .first()
                .map(|quest| quest.chain(&lang))
                .unwrap_or(chain_id.as_str());
            let completed = chain_quests
                .iter()
                .filter(|quest| {
                    quest_states.get(&quest.id) == Some(&crate::quest::QuestState::Completed)
                })
                .count();
            lines.push(format!(
                "━━ {}  {}/{} ━━",
                chain_title,
                completed,
                chain_quests.len()
            ));
            for quest in chain_quests {
                let active_index = active_ids.iter().position(|id| id == &quest.id);
                let state = quest_states.get(&quest.id).copied();
                let status = match state {
                    Some(crate::quest::QuestState::Completed) => {
                        if lang == "en" {
                            "✓ COMPLETED"
                        } else {
                            "✓ ЗАВЕРШЕНО"
                        }
                    }
                    Some(crate::quest::QuestState::Active) => {
                        if lang == "en" {
                            "◆ ACTIVE"
                        } else {
                            "◆ АКТИВНО"
                        }
                    }
                    None => {
                        if lang == "en" {
                            "◇ LOCKED"
                        } else {
                            "◇ ЗАКРЫТО"
                        }
                    }
                };
                let tracked = active_index == Some(self.tracked_quest_index);
                let key =
                    active_index.map_or_else(String::new, |index| format!("[{}] ", index + 1));
                lines.push(format!(
                    "{}{}{} · {} — {}",
                    key,
                    if tracked { "▶ " } else { "  " },
                    status,
                    if lang == "en" { "Stage" } else { "Этап" },
                    quest.stage
                ));
                lines.push(format!("    {}", quest.title(&lang)));
                if state.is_some() {
                    let progress = self.quest_progress(quest).min(quest.count);
                    lines.push(format!(
                        "    {}/{} · {} XP · {} {}",
                        progress,
                        quest.count,
                        quest.reward_xp,
                        quest.reward_gold,
                        if lang == "en" { "gold" } else { "зол." }
                    ));
                    lines.push(format!("    {}", quest.description(&lang)));
                }
            }
            lines.push(String::new());
        }
        if let Some(label) = self.journal_list.as_mut() {
            label.set_text(&lines.join("\n"));
        }
    }

    pub(super) fn refresh_inventory_ui(&mut self) {
        let lang = self.settings.lang.clone();
        let text = if let Some(ref state) = self.state {
            let mut lines = Vec::new();
            if let Some(ci) = state.class_idx {
                let c = &classes()[ci.min(classes().len() - 1)];
                lines.push(format!(
                    "{} / {}   {} {}   XP {}/{}",
                    c.name(&lang),
                    c.specs[state.spec_idx.min(2)].name(&lang),
                    if lang == "en" { "lvl." } else { "ур." },
                    state.level,
                    state.xp,
                    xp_to_next(state.level)
                ));
                lines.push(String::new());
            }
            lines.push(format!(
                "{}: {} {}",
                t("hud_gold", &lang),
                state.gold,
                if lang == "en" { "gold" } else { "зол." }
            ));
            lines.push(String::new());
            lines.push(
                if lang == "en" {
                    "Ammunition:"
                } else {
                    "Боезапас:"
                }
                .to_string(),
            );
            for t in AmmoType::ALL {
                lines.push(format!("  {}: {}", t.name(&lang), self.arsenal.ammo_of(t)));
            }
            lines.push(String::new());
            let current_weapon = self.arsenal.current;
            let selected_branch = state.weapon_mods[current_weapon.slot()];
            lines.push(format!(
                "{}: {}  |  {}: {}",
                if lang == "en" {
                    "Weapon mods"
                } else {
                    "Моды оружия"
                },
                weapon_def(current_weapon).name(&lang),
                if lang == "en" {
                    "boss cores"
                } else {
                    "ядра боссов"
                },
                state.weapon_mod_cores,
            ));
            for branch in 1..=2u8 {
                if let Some(weapon_mod) = weapon_mod_for(current_weapon, branch) {
                    let marker = if selected_branch == branch {
                        "◆"
                    } else {
                        "◇"
                    };
                    lines.push(format!(
                        "  [{branch}] {marker} {} — {}",
                        weapon_mod.name(&lang),
                        weapon_mod.description(&lang)
                    ));
                }
            }
            lines.push(if lang == "en" {
                "Choose/switch branch with 1 or 2 (cost: 1 boss core).".to_string()
            } else {
                "Выбор/смена ветви: 1 или 2 (цена: 1 ядро босса).".to_string()
            });
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
            lines.push(String::new());
            lines.push(
                if lang == "en" {
                    "━━━━━━━━━━  QUESTS  ━━━━━━━━━━"
                } else {
                    "━━━━━━━━━━  КВЕСТЫ  ━━━━━━━━━━"
                }
                .to_string(),
            );
            let mut has_quests = false;
            for quest in state
                .quests
                .quests
                .iter()
                .filter(|quest| quest.state == crate::quest::QuestState::Active)
            {
                has_quests = true;
                lines.push(format!("◆ {}", quest.title));
                lines.push(format!("    {}", quest.description));
            }
            for quest in state
                .quests
                .quests
                .iter()
                .filter(|quest| quest.state == crate::quest::QuestState::Completed)
            {
                has_quests = true;
                lines.push(format!(
                    "✓ {}  [{}]",
                    quest.title,
                    if lang == "en" {
                        "completed"
                    } else {
                        "завершено"
                    }
                ));
            }
            if !has_quests {
                lines.push(
                    if lang == "en" {
                        "No active quests."
                    } else {
                        "Активных заданий нет."
                    }
                    .to_string(),
                );
            }
            lines.join("\n")
        } else {
            String::new()
        };
        if let Some(ref mut lbl) = self.inv_list {
            lbl.set_text(&text);
        }
    }

    pub(super) fn select_weapon_mod(&mut self, branch: u8) {
        let weapon = self.arsenal.current;
        let lang = self.settings.lang.clone();
        let Some(weapon_mod) = weapon_mod_for(weapon, branch) else {
            return;
        };
        let result = {
            let Some(state) = self.state.as_mut() else {
                return;
            };
            if state.weapon_mods[weapon.slot()] == branch {
                0
            } else if state.weapon_mod_cores == 0 {
                -1
            } else {
                state.weapon_mod_cores -= 1;
                state.weapon_mods[weapon.slot()] = branch;
                1
            }
        };
        let message = match result {
            1 => format!(
                "{}: {}",
                if lang == "en" {
                    "MOD INSTALLED"
                } else {
                    "МОД УСТАНОВЛЕН"
                },
                weapon_mod.name(&lang)
            ),
            0 => if lang == "en" {
                "This branch is already active"
            } else {
                "Эта ветвь уже активна"
            }
            .to_string(),
            _ => if lang == "en" {
                "A boss core is required"
            } else {
                "Требуется ядро босса"
            }
            .to_string(),
        };
        self.refresh_inventory_ui();
        if result == 1 {
            self.update_weapon_mod_visual();
            self.update_crosshair();
        }
        self.show_flash(&message);
        if result == 1 {
            self.auto_save();
        }
    }

    // ── Перки ────────────────────────────────────────────────────────────────

    pub(super) fn open_perks(&mut self) {
        self.mode = Mode::Perks;
        self.freeze_player(true);
        self.refresh_perk_ui();
        if let Some(ref mut p) = self.perk_panel {
            p.set_visible(true);
        }
        if let Some(ref mut lbl) = self.hint_label {
            lbl.set_visible(false);
        }
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
    }

    pub(super) fn close_perks(&mut self) {
        if let Some(ref mut p) = self.perk_panel {
            p.set_visible(false);
        }
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
        let lang = self.settings.lang.clone();
        // детерминированный список доступных перков (тот же, что в refresh_perk_ui)
        let picked = {
            let Some(st) = self.state.as_ref() else {
                return;
            };
            let avail = crate::perk::available(&st.perks, st.perk_points);
            avail
                .get(n)
                .map(|p| (p.id.clone(), p.cost, p.name(&lang).to_string(), p.max_ranks))
        };
        let Some((id, cost, name, max_ranks)) = picked else {
            return;
        };

        let new_rank = {
            let st = self.state.as_mut().unwrap();
            if st.perk_points < cost {
                return;
            }
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
                if hp > max {
                    pl.bind_mut().hp = max;
                }
            }
        }
        self.refresh_perk_ui();
        self.show_flash(&format!(
            "{}: {} ({}/{})",
            if self.settings.lang == "en" {
                "Perk upgraded"
            } else {
                "Перк улучшен"
            },
            name,
            new_rank,
            max_ranks
        ));
        self.auto_save();
    }

    pub(super) fn refresh_perk_ui(&mut self) {
        let lang = self.settings.lang.clone();
        use crate::perk::{available, perks, reqs_met, synergies, synergy_active};
        let text = if let Some(ref st) = self.state {
            let owned = &st.perks;
            let points = st.perk_points;
            // номера покупки — по порядку available()
            let avail_ids: Vec<String> = available(owned, points)
                .iter()
                .map(|p| p.id.clone())
                .collect();

            let mut lines = vec![
                format!(
                    "{}: {}",
                    if lang == "en" {
                        "Perk points"
                    } else {
                        "Очки перков"
                    },
                    points
                ),
                String::new(),
            ];

            for (branch, title_ru, title_en) in [
                ("survival", "◆ ЖИВУЧЕСТЬ", "◆ SURVIVAL"),
                ("offense", "◆ УРОН", "◆ OFFENSE"),
                ("utility", "◆ УТИЛИТИ", "◆ UTILITY"),
            ] {
                lines.push(if lang == "en" { title_en } else { title_ru }.to_string());
                for p in perks().iter().filter(|p| p.branch == branch) {
                    let rank = owned.get(&p.id).copied().unwrap_or(0);
                    let tag = if rank >= p.max_ranks {
                        if lang == "en" {
                            "  [MAX]"
                        } else {
                            "  [МАКС]"
                        }
                        .to_string()
                    } else if !reqs_met(&p.requires, owned) {
                        let need: Vec<String> = p
                            .requires
                            .iter()
                            .map(|r| {
                                let id = r.split_once(':').map(|(a, _)| a).unwrap_or(r);
                                crate::perk::perk_by_id(id)
                                    .map(|d| d.name(&lang).to_string())
                                    .unwrap_or_else(|| id.to_string())
                            })
                            .collect();
                        format!(
                            "  🔒 {}: {}",
                            if lang == "en" {
                                "requires"
                            } else {
                                "нужно"
                            },
                            need.join(", ")
                        )
                    } else if let Some(pos) = avail_ids.iter().position(|x| *x == p.id) {
                        format!(
                            "  ◀ [{}] {} ({} {})",
                            pos + 1,
                            if lang == "en" { "buy" } else { "купить" },
                            p.cost,
                            if lang == "en" { "pts." } else { "оч." }
                        )
                    } else {
                        String::new()
                    };
                    lines.push(format!(
                        "  {} {}/{}{}",
                        p.name(&lang),
                        rank,
                        p.max_ranks,
                        tag
                    ));
                    lines.push(format!("      {}", p.description(&lang)));
                }
                lines.push(String::new());
            }

            lines.push(
                if lang == "en" {
                    "◆ SYNERGIES"
                } else {
                    "◆ СИНЕРГИИ"
                }
                .to_string(),
            );
            for s in synergies() {
                let on = synergy_active(s, owned);
                let mark = if on { "✔" } else { "…" };
                lines.push(format!(
                    "  {} {} — {}",
                    mark,
                    s.name(&lang),
                    s.description(&lang)
                ));
            }
            lines.join("\n")
        } else {
            String::new()
        };
        if let Some(ref mut lbl) = self.perk_list {
            lbl.set_text(&text);
        }
    }
}
