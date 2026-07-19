//! Диалоговая система.

use super::*;

impl Game3D {
    // ── Диалог ───────────────────────────────────────────────────────────────

    /// Сцена по id: сначала dialogues.json пресета (данные приоритетнее кода —
    /// пресет может переопределять встроенные сцены), затем story.rs.
    pub(super) fn resolve_scene(&self, id: &str) -> Option<Scene> {
        if id.is_empty() { return None; }
        if let Some(s) = self.cfg.as_ref().and_then(|c| c.dialogue(id)) {
            return Some(s.clone());
        }
        self.state.as_ref().and_then(|st| get_scene(id, st))
    }

    pub(super) fn start_dialogue(&mut self, npc_idx: usize) {
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
    pub(super) fn make_giver_scene(&self, npc_name: &str, npc_id: &str) -> Option<Scene> {
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
    pub(super) fn make_quest_scene(&self, npc_name: &str, quest_id: &str) -> Option<Scene> {
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
    pub(super) fn quest_progress(&self, q: &crate::config::QuestCfg) -> u32 {
        let Some(st) = self.state.as_ref() else { return 0 };
        match q.kind.as_str() {
            "clear_dungeon" => st.dungeons_cleared,
            _ => st.quest_kills.get(&q.id).copied().unwrap_or(0),
        }
    }

    /// Инкремент прогресса kill/collect-квестов по событию.
    pub(super) fn bump_quests(&mut self, kind: &str, target: &str) {
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

    pub(super) fn advance_dialogue(&mut self) {
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

    pub(super) fn select_choice(&mut self, idx: usize) {
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

    pub(super) fn end_dialogue(&mut self) {
        self.scene = None;
        self.line_idx = 0;
        self.at_choices = false;
        if let Some(ref mut p) = self.dlg_panel { p.set_visible(false); }
        self.set_mode_explore();
        self.auto_save();
    }

    pub(super) fn refresh_dlg_ui(&mut self) {
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

}
