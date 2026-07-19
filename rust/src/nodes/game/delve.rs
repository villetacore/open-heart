//! Генерация и вход в данж.

use super::*;

// ── Данж ──────────────────────────────────────────────────────────────────────

impl Game3D {
    pub(super) fn enter_dungeon(&mut self, depth: u32) {
        // cfg берём ДО разрушительных шагов: ранний выход не должен оставить
        // игрока в пустоте с уже снесённым данжем
        let Some(cfg) = self.cfg.take() else { return };
        self.clear_dungeon();

        let seed = {
            let st = self.state.as_mut().unwrap();
            st.dungeon_seed = st.dungeon_seed.wrapping_add(1);
            st.dungeon_seed
        };
        // Дроп с убийств тоже зависит от сида — забег полностью воспроизводим
        // (раньше сессионный Rng стартовал с константы).
        self.rng = Rng::new(seed ^ 0x00D1_CED0);

        let plan: DungeonPlan = dungeon::generate(depth, seed, &mut self.cache, &cfg);

        let mut root = plan.root.clone();
        root.set_position(DUNGEON_OFFSET);
        self.base_mut().add_child(&root);
        self.dungeon_root = Some(root);
        self.dungeon_depth = depth;
        self.dungeon_name = plan.theme_name.clone();
        self.minimap_floor = plan.floor_map.clone();
        self.exit_portal = DUNGEON_OFFSET + plan.exit_portal;
        self.next_portal = DUNGEON_OFFSET + plan.next_portal;
        self.boss_alive = plan.enemies.iter().any(|e| e.is_boss);
        // навигация: сетка проходимости данжа для A* врагов (до спавнов!)
        self.dungeon_nav = Some(NavGrid::new(plan.floor_map.clone(), plan.floor_heights.clone()));

        for es in &plan.enemies {
            self.spawn_enemy(&cfg, &es.kind, DUNGEON_OFFSET + es.pos, es.mult, es.is_boss, true,
                             &es.affixes);
        }
        for (kind, pos) in &plan.items {
            self.spawn_item(&cfg, kind, DUNGEON_OFFSET + *pos, true);
        }
        self.cfg = Some(cfg);
        for (t, n, pos) in &plan.ammo {
            self.spawn_ammo_pickup(*t, *n, DUNGEON_OFFSET + *pos, true);
        }
        for (w, pos) in &plan.weapons {
            self.spawn_weapon_pickup(*w, DUNGEON_OFFSET + *pos, true);
        }

        if let Some(ref p) = self.player {
            if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                pl.bind_mut().teleport(DUNGEON_OFFSET + plan.player_spawn + Vector3::new(0.0, 1.0, 0.0));
            }
        }
        self.loc = Loc::Dungeon;

        // квест при первом входе
        let add_quest = {
            let st = self.state.as_mut().unwrap();
            if !st.has("dungeon_quest") {
                st.flags.insert("dungeon_quest".into());
                st.quests.add("dungeon_heart", "Сердце данжа", "Убей стража на дне данжа.");
                true
            } else { false }
        };
        if add_quest {
            self.show_flash("Новый квест: «Сердце данжа»");
        }
        self.show_flash(&format!("«{}» — глубина {}", plan.theme_name, depth));
        self.build_minimap_texture();
        if let Some(ref mut bg) = self.minimap_bg  { bg.set_visible(true); }
        if let Some(ref mut mr) = self.minimap_rect { mr.set_visible(true); }
        if let Some(ref mut d)  = self.minimap_dot  { d.set_visible(true); }
        self.update_loc_label();
    }

    pub(super) fn exit_dungeon(&mut self) {
        self.clear_dungeon();
        self.loc = Loc::World;
        if let Some(ref p) = self.player {
            if let Ok(mut pl) = p.clone().try_cast::<Player>() {
                let gate = self.gate_pos;
                pl.bind_mut().teleport(gate + Vector3::new(0.0, 1.0, 3.5));
            }
        }
        if let Some(ref mut bg) = self.minimap_bg  { bg.set_visible(false); }
        if let Some(ref mut mr) = self.minimap_rect { mr.set_visible(false); }
        if let Some(ref mut d)  = self.minimap_dot  { d.set_visible(false); }
        self.minimap_floor.clear();
        self.show_flash("Пустоши Неонового Сердца");
        self.update_loc_label();
        self.auto_save();
    }

    pub(super) fn clear_dungeon(&mut self) {
        self.dungeon_nav = None;
        if let Some(root) = self.dungeon_root.take() {
            root.free();
        }
        // убрать врагов и предметы данжа
        let pl = self.player.clone();
        let _ = pl;
        let mut i = 0;
        while i < self.enemies.len() {
            let in_d = self.enemies[i].get_position().x > 250.0;
            if in_d {
                let e = self.enemies.remove(i);
                e.free();
            } else { i += 1; }
        }
        let mut i = 0;
        while i < self.world_items.len() {
            if self.world_items[i].in_dungeon {
                let it = self.world_items.remove(i);
                it.node.free();
            } else { i += 1; }
        }
        for p in self.projectiles.drain(..) {
            p.node.free();
        }
        for p in self.enemy_projectiles.drain(..) {
            p.node.free();
        }
        self.boss_alive = false;
    }
}
