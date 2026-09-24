//! Shared interactive UI components. Labels remain separate from artwork.
use super::*;
use godot::classes::{Button, Control, GridContainer};

fn button(text: &str, size: Vector2) -> Gd<Button> {
    let mut b = Button::new_alloc();
    b.set_text(text);
    b.set_custom_minimum_size(size);
    b.add_theme_font_size_override("font_size", 20);
    for (state, color) in [("normal", C_BORDER), ("hover", C_CYAN), ("focus", C_GOLD), ("pressed", C_PINK), ("disabled", C_DIM)] {
        b.add_theme_stylebox_override(state, &make_style(C_UI_BG, color, if state=="focus" {2} else {1}));
    }
    b
}

fn clear_children(node: &mut Gd<impl Inherits<godot::classes::Node>>) {
    let mut node = node.clone().upcast::<godot::classes::Node>();
    for mut child in node.get_children().iter_shared() { node.remove_child(&child); child.queue_free(); }
}

impl Game3D {
    pub(super) fn atlas_cell(&mut self, path: &str, columns: usize, rows: usize, index: usize) -> Option<Gd<AtlasTexture>> {
        let texture = self.cache.get(path)?;
        let size = texture.get_size() / Vector2::new(columns as f32, rows as f32);
        let mut atlas = AtlasTexture::new_gd();
        atlas.set_atlas(&texture);
        atlas.set_region(Rect2::new(Vector2::new((index%columns) as f32*size.x,(index/columns) as f32*size.y), size));
        if path.contains("items_atlas") {
            let padding=size*0.08;
            atlas.set_region(Rect2::new(Vector2::new((index%columns) as f32*size.x,(index/columns) as f32*size.y)+padding,size-padding*2.0));
        }
        if path.contains("classes_atlas") {
            atlas.set_region(Rect2::new(Vector2::new(index as f32*size.x+12.0,0.0),Vector2::new(size.x-24.0,500.0)));
        }
        Some(atlas)
    }

    pub(super) fn build_modern_panels(&mut self) {
        if let Some(mut panel) = self.inv_panel.clone() {
            for child in panel.get_children().iter_shared() {
                if let Ok(mut c) = child.try_cast::<Control>() { c.set_visible(false); }
            }
            place(&panel,0.5,0.5,420.0,160.0,1080.0,760.0);
            let mut title = Label::new_alloc();
            title.set_text(if self.settings.lang=="en" {"INVENTORY  /  FIELD KIT"} else {"ИНВЕНТАРЬ  /  СНАРЯЖЕНИЕ"});
            title.set_position(Vector2::new(28.0,20.0));
            title.add_theme_font_size_override("font_size",28);
            title.add_theme_color_override("font_color",C_PINK);
            panel.add_child(&title);
            let tabs = if self.settings.lang=="en" {["All","Supplies","Quest items"]} else {["Все","Расходники","Ключевые"]};
            for (index, text) in tabs.into_iter().enumerate() {
                let mut b = button(text,Vector2::new(184.0,44.0));
                b.set_position(Vector2::new(28.0+index as f32*192.0,72.0));
                b.connect("pressed", &self.to_gd().callable("ui_inventory_filter").bind(&[(index as i64).to_variant()]));
                panel.add_child(&b);
            }
            let mut scroll = ScrollContainer::new_alloc();
            scroll.set_position(Vector2::new(28.0,132.0));
            scroll.set_size(Vector2::new(584.0,540.0));
            let mut grid = GridContainer::new_alloc();
            grid.set_columns(4);
            grid.add_theme_constant_override("h_separation",10);
            grid.add_theme_constant_override("v_separation",10);
            scroll.add_child(&grid);
            panel.add_child(&scroll);
            self.inventory_grid = Some(grid);
            let mut detail = Label::new_alloc();
            detail.set_position(Vector2::new(642.0,132.0));
            detail.set_size(Vector2::new(410.0,412.0));
            detail.set_autowrap_mode(godot::classes::text_server::AutowrapMode::WORD_SMART);
            detail.add_theme_font_size_override("font_size",22);
            panel.add_child(&detail);
            self.inventory_detail = Some(detail);
            let mut use_button = button(if self.settings.lang=="en" {"USE SELECTED  [E]"} else {"ИСПОЛЬЗОВАТЬ  [E]"},Vector2::new(410.0,54.0));
            use_button.set_position(Vector2::new(642.0,562.0));
            use_button.connect("pressed", &self.to_gd().callable("ui_use_item"));
            panel.add_child(&use_button);
            self.inventory_use = Some(use_button);
            for branch in 1..=2 {
                let mut b = button(&format!("{} {branch}  [{branch}]",if self.settings.lang=="en" {"Weapon mod"} else {"Мод оружия"}),Vector2::new(280.0,46.0));
                b.set_position(Vector2::new(28.0+(branch-1) as f32*292.0,692.0));
                b.connect("pressed", &self.to_gd().callable("ui_weapon_mod").bind(&[(branch as i64).to_variant()]));
                panel.add_child(&b);
            }
            let mut close = button("[Esc]",Vector2::new(100.0,44.0));
            close.set_position(Vector2::new(952.0,20.0));
            close.connect("pressed", &self.to_gd().callable("ui_close_panel"));
            panel.add_child(&close);
        }
        if let Some(mut panel) = self.perk_panel.clone() {
            for child in panel.get_children().iter_shared() {
                if let Ok(mut c) = child.try_cast::<Control>() { c.set_visible(false); }
            }
            place(&panel,0.5,0.5,360.0,140.0,1200.0,800.0);
            let mut title = Label::new_alloc();
            title.set_position(Vector2::new(26.0,18.0));
            title.set_size(Vector2::new(1130.0,40.0));
            title.add_theme_font_size_override("font_size",24);
            panel.add_child(&title);
            self.perk_list = Some(title);
            let mut scroll = ScrollContainer::new_alloc();
            scroll.set_position(Vector2::new(26.0,76.0));
            scroll.set_size(Vector2::new(1148.0,640.0));
            let mut grid = GridContainer::new_alloc();
            grid.set_columns(3);
            grid.add_theme_constant_override("h_separation",12);
            grid.add_theme_constant_override("v_separation",12);
            scroll.add_child(&grid);
            panel.add_child(&scroll);
            self.perk_grid = Some(grid);
            let mut respec = button(if self.settings.lang=="en" {"RESPEC"} else {"СБРОС БИЛДА"},Vector2::new(300.0,48.0));
            respec.set_position(Vector2::new(26.0,730.0));
            respec.connect("pressed", &self.to_gd().callable("ui_respec_perks"));
            panel.add_child(&respec);
            self.perk_respec = Some(respec);
            let mut craft = button(if self.settings.lang=="en" {"CRAFT POINT"} else {"СКРАФТИТЬ ОЧКО"},Vector2::new(360.0,48.0));
            craft.set_position(Vector2::new(338.0,730.0));
            craft.connect("pressed", &self.to_gd().callable("ui_craft_perk"));
            panel.add_child(&craft);
            self.perk_craft = Some(craft);
            let mut close = button(if self.settings.lang=="en" {"BACK  [Esc]"} else {"НАЗАД  [Esc]"},Vector2::new(200.0,48.0));
            close.set_position(Vector2::new(974.0,730.0));
            close.connect("pressed", &self.to_gd().callable("ui_close_panel"));
            panel.add_child(&close);
        }
        if let Some(mut panel) = self.journal_panel.clone() {
            place(&panel,0.5,0.5,380.0,160.0,1160.0,760.0);
            if let Some(label) = self.journal_list.as_mut() {
                if let Some(mut parent) = label.get_parent().and_then(|p| p.try_cast::<ScrollContainer>().ok()) {
                    parent.set_position(Vector2::new(418.0,76.0));
                    parent.set_size(Vector2::new(714.0,600.0));
                }
                label.set_custom_minimum_size(Vector2::new(680.0,0.0));
                label.add_theme_font_size_override("font_size",20);
            }
            let mut scroll = ScrollContainer::new_alloc();
            scroll.set_position(Vector2::new(24.0,76.0));
            scroll.set_size(Vector2::new(374.0,540.0));
            let list = VBoxContainer::new_alloc();
            scroll.add_child(&list);
            panel.add_child(&scroll);
            self.journal_buttons=Some(list);
            let mut close = button("[Esc]",Vector2::new(100.0,44.0));
            close.set_position(Vector2::new(1032.0,20.0));
            close.connect("pressed", &self.to_gd().callable("ui_close_panel"));
            panel.add_child(&close);
        }
    }

    pub(super) fn refresh_inventory_cards(&mut self) {
        let Some(mut grid) = self.inventory_grid.clone() else { return; };
        clear_children(&mut grid);
        let items = self.state.as_ref().map(|s| s.inventory.items.clone()).unwrap_or_default();
        let lang = self.settings.lang.clone();
        let mut visible = Vec::new();
        for item in items {
            let cfg = self.cfg.as_ref().and_then(|c| c.items.iter().find(|i| i.id == item.id)).cloned();
            if self.inventory_filter==1 && !cfg.as_ref().is_some_and(|c| c.heal.unwrap_or(0.0)>0.0) { continue; }
            if self.inventory_filter==2 && cfg.as_ref().is_none_or(|c| c.category!="key") { continue; }
            visible.push(item.id.clone());
            let name = cfg.as_ref().map(|c| c.name(&lang)).unwrap_or(&item.name);
            let mut b = button(&format!("×{}",item.qty),Vector2::new(130.0,120.0));
            b.set_tooltip_text(name);
            if let Some(def) = cfg.quest(&q.id) {
                if ["ivo", "noel", "lucien", "ash", "emil", "yves"].contains(&def.giver.as_str()) {
                    let suffix = if def.stage > 1 { "_2" } else { "" };
                    if let Some(texture) = super::presentation::art_texture(&format!("res://assets/icons/neighbors/{}{suffix}.tres", def.giver)) {
                        b.set_button_icon(&texture);
                        b.set_expand_icon(true);
                        b.add_theme_constant_override("icon_max_width", 42);
                    }
                }
            }
            b.set_expand_icon(true);
            b.add_theme_constant_override("icon_max_width",80);
            if let Some(index) = cfg.as_ref().and_then(|c| c.icon) {
                if let Some(icon) = self.atlas_cell("res://assets/icons/items_atlas.png",5,5,index) { b.set_button_icon(&icon); }
            } else { b.set_text(&format!("{}\n×{}",name,item.qty)); }
            b.connect("pressed", &self.to_gd().callable("ui_select_item").bind(&[item.id.to_variant()]));
            grid.add_child(&b);
        }
        if !visible.contains(&self.selected_item) { self.selected_item = visible.first().cloned().unwrap_or_default(); }
        self.refresh_item_detail();
    }

    fn refresh_item_detail(&mut self) {
        let lang = &self.settings.lang;
        let item = self.cfg.as_ref().and_then(|c| c.items.iter().find(|i| i.id==self.selected_item));
        let gold = self.state.as_ref().map(|s| s.gold).unwrap_or(0);
        let cores = self.state.as_ref().map(|s| s.weapon_mod_cores).unwrap_or(0);
        let text = if let Some(item) = item {
            format!("{}\n\n{}\n\n{}: {}\n{}: {}\n\n{}",item.name(lang),
                if lang=="en" {&item.desc_en} else {&item.desc_ru},
                if lang=="en" {"Gold"} else {"Золото"},gold,
                if lang=="en" {"Boss cores"} else {"Ядра боссов"},cores,
                if lang=="en" {"Hover a slot for its name.\nWeapon modifications cost one boss core."} else {"Наведи курсор на слот, чтобы увидеть название.\nМодификация оружия стоит одно ядро босса."})
        } else if lang=="en" {"No items in this category.\nExplore the quarter and catacombs.".into()} else {"В этой категории пока нет предметов.\nИсследуй квартал и катакомбы.".into()};
        if let Some(label) = self.inventory_detail.as_mut() { label.set_text(&text); }
        if let Some(b) = self.inventory_use.as_mut() { b.set_disabled(!item.is_some_and(|i| i.heal.unwrap_or(0.0)>0.0)); }
    }

    pub(super) fn refresh_perk_cards(&mut self) {
        let Some(mut grid) = self.perk_grid.clone() else { return; };
        clear_children(&mut grid);
        let Some(state) = self.state.as_ref() else { return; };
        let lang = self.settings.lang.clone();
        if let Some(title) = self.perk_list.as_mut() {
            title.set_text(&format!("{}  ·  {} {}",if lang=="en" {"PERKS"} else {"ПЕРКИ"},state.perk_points,if lang=="en" {"points available"} else {"очков доступно"}));
        }
        let (respec_cost, has_perks, gold) = (state.respec_cost(), !state.perks.is_empty(), state.gold);
        let can_craft = state.can_craft_perk_point();
        if let Some(button) = self.perk_respec.as_mut() {
            let price = if respec_cost == 0 {
                if lang=="en" {"free".to_string()} else {"бесплатно".to_string()}
            } else {
                format!("{} {}", respec_cost, if lang=="en" {"gold"} else {"зол."})
            };
            button.set_text(&format!("{} ({})", if lang=="en" {"RESPEC"} else {"СБРОС БИЛДА"}, price));
            button.set_disabled(!has_perks || gold < respec_cost);
        }
        if let Some(button) = self.perk_craft.as_mut() {
            button.set_text(&format!(
                "{}\n{}×{} + {} {}",
                if lang=="en" {"CRAFT POINT"} else {"СКРАФТИТЬ ОЧКО"},
                if lang=="en" {"shards"} else {"осколки"}, crate::game_state::PERK_CRAFT_ITEM_QTY,
                crate::game_state::PERK_CRAFT_GOLD, if lang=="en" {"gold"} else {"зол."}
            ));
            button.set_disabled(!can_craft);
        }
        let branches = ["survival","offense","utility"];
        let lists: Vec<_> = branches.iter().map(|branch| crate::perk::perks().iter().filter(|p| p.branch==*branch).collect::<Vec<_>>()).collect();
        for row in 0..lists.iter().map(Vec::len).max().unwrap_or(0) {
            for list in &lists {
                let Some(p) = list.get(row) else { continue; };
                let rank = state.perks.get(&p.id).copied().unwrap_or(0);
                let met = crate::perk::reqs_met(&p.requires,&state.perks);
                let enabled = rank < p.max_ranks && met && state.perk_points >= p.cost;
                let mut b = button(&format!("{}  {}/{}\n{} {}",p.name(&lang),rank,p.max_ranks,
                    if lang=="en" {"Cost:"} else {"Цена:"},p.cost),Vector2::new(370.0,92.0));
                let requires = p.requires.iter().map(|id| {
                    let (id,rank) = id.split_once(':').unwrap_or((id.as_str(),"1"));
                    format!("{} {rank}",crate::perk::perk_by_id(id).map(|p| p.name(&lang)).unwrap_or(id))
                }).collect::<Vec<_>>().join(", ");
                b.set_tooltip_text(&format!("{}\n{}",p.description(&lang),if requires.is_empty() {String::new()} else {format!("{}: {}",if lang=="en" {"Requires"} else {"Требуется"},requires)}));
                b.set_disabled(!enabled);
                if let Some(texture) = super::presentation::art_texture(&format!("res://assets/icons/perks/{}.tres", p.id)) {
                    b.set_button_icon(&texture);
                    b.set_expand_icon(true);
                    b.add_theme_constant_override("icon_max_width", 58);
                }
                b.connect("pressed",&self.to_gd().callable("ui_buy_perk").bind(&[p.id.to_variant()]));
                grid.add_child(&b);
            }
        }
    }

    pub(super) fn refresh_journal_cards(&mut self) {
        let Some(mut list) = self.journal_buttons.clone() else { return; };
        clear_children(&mut list);
        let Some(state) = self.state.as_ref() else { return; };
        let Some(cfg) = self.cfg.as_ref() else { return; };
        let active: Vec<_> = state.quests.quests.iter().filter(|q| q.state==crate::quest::QuestState::Active).collect();
        for (index, q) in active.iter().enumerate() {
            let name = cfg.quests.iter().find(|c| c.id==q.id).map(|c| c.title(&self.settings.lang)).unwrap_or(&q.title);
            let progress = cfg.quest(&q.id).map(|def| {
                let current = self.quest_progress(def).min(def.count);
                if current >= def.count { if self.settings.lang == "en" {"Ready to turn in".to_string()} else {"Готово к сдаче".to_string()} }
                else { format!("{current}/{}", def.count) }
            }).unwrap_or_default();
            let mut b=button(&format!("{} {}\n{}",if index==self.tracked_quest_index {"◆"} else {"◇"},name,progress),Vector2::new(350.0,76.0));
            b.add_theme_font_size_override("font_size",18);
            b.set_text_overrun_behavior(godot::classes::text_server::OverrunBehavior::TRIM_ELLIPSIS);
            b.set_tooltip_text(name);
            b.connect("pressed",&self.to_gd().callable("ui_track_quest").bind(&[(index as i64).to_variant()]));
            list.add_child(&b);
        }
        if active.is_empty() {
            let mut label=Label::new_alloc();
            label.set_text(if self.settings.lang=="en" {"Talk to the quarter's residents\nto discover new quests."} else {"Поговори с жителями квартала,\nчтобы найти новые задания."});
            list.add_child(&label);
        }
    }
}


impl Game3D {
    pub(super) fn ui_track_quest_impl(&mut self, index: i64) {
        if index<0 { return; }
        self.track_journal_quest(index as usize);
        let Some(state)=self.state.as_ref() else {return;};
        let Some(q)=state.quests.quests.iter().filter(|q|q.state==crate::quest::QuestState::Active).nth(index as usize) else {return;};
        let Some(def)=self.cfg.as_ref().and_then(|cfg|cfg.quests.iter().find(|c|c.id==q.id)) else {return;};
        let lang=&self.settings.lang;
        let text=format!("{}\n\n{}\n\n{}\n\n{}/{}\n\n{} XP  ·  {} {}",def.chain(lang),def.title(lang),def.description(lang),self.quest_progress(def).min(def.count),def.count,def.reward_xp,def.reward_gold,if lang=="en" {"gold"} else {"золота"});
        if let Some(label)=self.journal_list.as_mut() {label.set_text(&text);}
    }

    pub(super) fn ui_select_item_impl(&mut self, id: GString) { self.selected_item=id.to_string(); self.refresh_item_detail(); }

    pub(super) fn ui_inventory_filter_impl(&mut self, filter: i64) { self.inventory_filter=filter; self.refresh_inventory_cards(); }

    pub(super) fn ui_weapon_mod_impl(&mut self, branch: i64) { self.select_weapon_mod(branch as u8); }

    pub(super) fn ui_use_item_impl(&mut self) {
        let id = self.selected_item.clone();
        if self.use_consumable_id(&id) { self.refresh_inventory_ui(); }
    }

    pub(super) fn ui_buy_perk_impl(&mut self, id: GString) {
        let index = self.state.as_ref().and_then(|s| crate::perk::available(&s.perks,s.perk_points).iter().position(|p| p.id==id.to_string()));
        if let Some(index) = index { self.buy_perk_at(index); }
    }

    pub(super) fn ui_respec_perks_impl(&mut self) {
        let lang = self.settings.lang.clone();
        match self.state.as_mut().and_then(|s| s.respec_perks()) {
            Some(refunded) => {
                let (ci, si) = self.state.as_ref().map(|s| (s.class_idx.unwrap_or(0), s.spec_idx)).unwrap_or((0, 0));
                self.apply_loadout(ci, si, false);
                self.refresh_perk_ui();
                self.show_flash(&if lang=="en" {
                    format!("Build reset · {refunded} points refunded")
                } else {
                    format!("Билд сброшен · возвращено очков: {refunded}")
                });
            }
            None => self.show_flash(if lang=="en" {"Not enough gold to respec"} else {"Недостаточно золота для сброса"}),
        }
    }

    pub(super) fn ui_craft_perk_impl(&mut self) {
        let lang = self.settings.lang.clone();
        let ok = self.state.as_mut().is_some_and(|s| s.craft_perk_point());
        if ok {
            self.refresh_perk_ui();
            self.show_flash(if lang=="en" {"Crafted a perk point"} else {"Скрафтено очко перка"});
        } else {
            self.show_flash(if lang=="en" {"Need shards and gold"} else {"Нужны осколки и золото"});
        }
    }

    pub(super) fn ui_close_panel_impl(&mut self) {
        match self.mode { Mode::Inventory=>self.close_inventory(), Mode::Perks=>self.close_perks(), Mode::Journal=>self.close_journal(), _=>{} }
    }
}
