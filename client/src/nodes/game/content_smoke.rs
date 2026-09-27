//! Runtime integration assertions for the content and creative update.
use super::*;

impl Game3D {
    fn smoke_giver_choice(&mut self, giver: &str) {
        self.scene = self.make_giver_scene(giver, giver);
        self.line_idx = 0;
        self.at_choices = false;
        self.mode = Mode::Dialogue;
        for _ in 0..10 {
            if self.at_choices || self.mode != Mode::Dialogue { break; }
            self.advance_dialogue();
        }
        if self.at_choices { self.select_choice(0); }
    }
}

#[godot_api(secondary)]
impl Game3D {
    #[func]
    fn runtime_evening_smoke(&mut self) -> VarDictionary {
        let mut result = VarDictionary::new();
        self.autosave_enabled = false;
        self.state = Some(GameState::new("Evening test"));
        self.confirm_class(1, 0);
        self.creative = false;
        let steps = [
            ("evening_circuit", "ash", "solve_puzzle", "2"),
            ("evening_memory", "noel", "discover_lore", "3"),
            ("evening_music", "lucien", "interact", "yves"),
            ("evening_table", "emil", "interact", "ren"),
        ];
        // Existing side stories are entry requirements, not part of this smoke.
        for giver in ["ash", "noel", "lucien", "emil"] {
            for stage in 1..=2 {
                let id = format!("neighbors_{giver}_{stage}");
                let state = self.state.as_mut().unwrap();
                state.quests.add(&id, &id, "test prerequisite");
                state.quests.complete(&id);
            }
        }
        // A direct link to an authored offer must not bypass dependencies.
        self.scene = self.resolve_scene("evening_table_offer");
        self.mode = Mode::Dialogue;
        self.at_choices = true;
        self.select_choice(0);
        result.set("locked_offer", !self.state.as_ref().unwrap().quests.has("evening_table"));
        let gold_before = self.state.as_ref().unwrap().gold;
        for (id, giver, event, target) in steps {
            self.bump_quests(event, target);
            result.set(format!("{id}_fresh").as_str(), !self.state.as_ref().unwrap().quest_kills.contains_key(id));
            self.smoke_giver_choice(giver);
            result.set(format!("{id}_accepted").as_str(), self.state.as_ref().unwrap().quests.is_active(id));
            // A direct completion link is insufficient without its event.
            self.scene = self.resolve_scene(&format!("{id}_complete"));
            self.mode = Mode::Dialogue;
            self.at_choices = true;
            self.select_choice(0);
            result.set(format!("{id}_no_early_reward").as_str(), self.state.as_ref().unwrap().quests.is_active(id));
            self.bump_quests(event, "wrong_target");
            // Wildcard puzzle/lore objectives accept any depth, social ones do not.
            if event == "interact" {
                result.set(format!("{id}_target").as_str(), !self.state.as_ref().unwrap().quest_kills.contains_key(id));
            }
            self.bump_quests(event, target);
            let json = save::SaveData::from_game(self.state.as_ref().unwrap(), 100.0, &self.arsenal).to_json().unwrap();
            let (restored, _, _) = save::SaveData::from_json(&json).unwrap().into_game();
            self.state = Some(restored);
            result.set(format!("{id}_saved").as_str(), self.state.as_ref().unwrap().quest_kills.get(id) == Some(&1));
            self.smoke_giver_choice(giver);
            result.set(format!("{id}_completed").as_str(), self.state.as_ref().unwrap().quests.is_completed(id));
            let repeat = self.make_quest_scene(giver, id).unwrap();
            result.set(format!("{id}_no_repeat").as_str(), !repeat.choices.iter().flat_map(|c| &c.effects).any(|e| matches!(e, Effect::QuestDone(_))));
        }
        result.set("reward_total", self.state.as_ref().unwrap().gold == gold_before + 235);
        result.set("ending_flag", self.state.as_ref().unwrap().has("shared_evening"));
        result.set("food_reward", self.state.as_ref().unwrap().inventory.items.iter().any(|i| i.id == "bread" && i.qty == 2));
        self.refresh_quest_world_changes();
        self.refresh_quest_world_changes();
        let tables = self.base().get_children().iter_shared().filter(|n| n.get_name() == "QuestEvolution_shared_table").count();
        result.set("one_table", tables == 1);
        self.scene = self.resolve_scene("evening_epilogue");
        self.line_idx = 0;
        self.refresh_dlg_ui();
        result.set("ending_art", self.dialogue_portrait.as_ref().is_some_and(|p| p.is_visible() && p.get_texture().is_some()));
        self.scene = Some(Scene { id: "unknown_art".into(), lines: vec![Line::new("", "unknown_npc", "Test")], choices: vec![] });
        self.refresh_dlg_ui();
        result.set("old_art_cleared", self.dialogue_portrait.as_ref().is_some_and(|p| !p.is_visible() && p.get_texture().is_none()));
        for neighbor in ["ivo", "noel", "lucien", "ash", "emil", "yves"] {
            self.scene = self.resolve_scene(&format!("neighbors_{neighbor}_city"));
            self.refresh_dlg_ui();
            result.set(format!("{neighbor}_city_art").as_str(), self.dialogue_portrait.as_ref().is_some_and(|p| p.is_visible() && p.get_texture().is_some()));
        }
        self.scene = self.resolve_scene("evening_epilogue");
        self.mode = Mode::Dialogue;
        self.at_choices = false;
        if let Some(panel) = self.dlg_panel.as_mut() { panel.set_visible(true); }
        self.refresh_dlg_ui();
        result
    }

    #[func]
    fn runtime_content_smoke(&mut self) -> VarDictionary {
        let mut result = VarDictionary::new();
        result.set("creative_session", self.creative && !self.autosave_enabled && self.net.is_none());
        self.autosave_enabled = false;
        self.state = Some(GameState::new("Content test"));
        self.confirm_class(1, 0);
        let static_npcs = self.npc_sprites.iter().filter(|s| s.has_meta("static_atlas")).count();
        result.set("static_npcs", static_npcs as i64);
        result.set("decor", self.base().try_get_node_as::<Node3D>("QuarterProps").map(|n| n.get_child_count()).unwrap_or(0));
        self.creative = false;
        result.set("initial_styles", self.wardrobe_scene().choices.len() as i64);
        self.smoke_giver_choice("stylist");
        result.set("accepted", self.state.as_ref().unwrap().quests.quests.iter().any(|q| q.id == "atelier_neighbors"));
        let early = self.make_giver_scene("Silas", "stylist").unwrap();
        result.set("no_early_reward", !early.choices.iter().flat_map(|c| &c.effects).any(|e| matches!(e, Effect::QuestDone(_))));
        self.bump_quests("interact", "medic");
        self.smoke_giver_choice("stylist");
        result.set("stage_one", self.state.as_ref().unwrap().quests.is_completed("atelier_neighbors"));
        self.smoke_giver_choice("stylist");
        for _ in 0..3 { self.bump_quests("collect", "neon_shard"); }
        self.smoke_giver_choice("stylist");
        result.set("collection_unlocked", self.state.as_ref().unwrap().quests.is_completed("atelier_supplies") && self.wardrobe_scene().choices.len() == 37);
        self.open_wardrobe();
        self.select_choice(4);
        let state = self.state.as_ref().unwrap();
        let saved = save::SaveData::from_game(state, 100.0, &self.arsenal).to_json().unwrap();
        let (restored, _, _) = save::SaveData::from_json(&saved).unwrap().into_game();
        result.set("style_saved", restored.has("style_rose_hair_bow"));
        let gold_before = state.gold;
        let repeat = self.make_giver_scene("Silas", "stylist").unwrap();
        result.set("no_repeat_reward", !repeat.choices.iter().flat_map(|c| &c.effects).any(|e| matches!(e, Effect::QuestDone(id) if id == "atelier_supplies")) && self.state.as_ref().unwrap().gold == gold_before);
        self.end_dialogue();
        self.creative = true;
        self.creative_refill();
        let mut player = self.player().unwrap().cast::<Player>();
        let before = player.bind().hp;
        self.damage_player(10000.0);
        result.set("invulnerable", player.bind().hp == before);
        self.arsenal.ammo = [0; 4];
        self.state.as_mut().unwrap().abilities.cooldowns = [99.0; 2];
        self.creative_refill();
        result.set("unlimited", self.arsenal.owned.iter().all(|v| *v) && self.arsenal.ammo.iter().all(|v| *v > 0) && self.state.as_ref().unwrap().abilities.cooldowns == [0.0; 2]);
        self.creative_action(-2);
        self.creative_action(18);
        result.set("class_switch", self.state.as_ref().is_some_and(|s| s.class_idx == Some(2) && s.spec_idx == 2));
        self.creative_action(-2);
        self.creative_action(108);
        result.set("depth_access", self.loc == Loc::Dungeon && self.dungeon_depth == 8);
        self.creative_action(-2);
        self.creative_action(0);
        result.set("return_home", self.loc == Loc::World);
        player.bind_mut().hp = before;
        self.scene = Some(Scene { id: "empty_test".into(), lines: vec![], choices: vec![Choice::simple("Leave", vec![])] });
        self.mode = Mode::Dialogue;
        self.refresh_dlg_ui();
        result.set("empty_dialogue_safe", self.at_choices);
        self.end_dialogue();
        result
    }
}
