//! Terminal presentation/actions over the existing character, dialogue and craft models.
use crate::app::App;
use anyhow::{anyhow, bail, ensure, Result};
use openheart_core::{
    classes,
    config::QuestCfg,
    craft::{self, CraftContext},
    dialogue::{self, Choice, Effect, Line, Scene},
    math::Vec3,
    perk, protocol, weapon,
};
use serde_json::json;

pub const HELP: &str = "WASD / стрелки — движение (удерживать)\nSpace — атака выбранной цели\nTab — следующая цель · R — перезарядка\n1–8 — оружие · Q/F — способности\nE — NPC / портал / помощь союзнику\nI — инвентарь · J — задания · P — перки\nC — крафт · N — NPC · F1 — помощь\n+ / - — масштаб · PgUp/PgDn — прокрутка\nEnter — следующая реплика · 1–9 — ответ\nEsc — закрыть панель · Ctrl+C — выход\n\n: — командная строка (Enter выполнить):\nhelp · inventory · quests · perks · craft\nclass 1..3 · spec 1..3 · perk ID\ntalk NPC_ID · quest QUEST_ID · choose N\nbuy ITEM_ID · sell ITEM_ID · use ITEM_ID\ncraft RECIPE_ID · weapon ID\ndelve [DEPTH] [OWNER_PEER] · hub\nsay ТЕКСТ · save · quit\n\n@ вы · & союзник · e враг · E элита\n* цель · $ лут · N NPC · > портал\n# стена · . пол\n\nСетевые меню не останавливают сервер.\nЛут подбирается автоматически рядом.";

impl App {
    pub fn execute(&mut self, line: &str) -> Result<()> {
        let (command, rest) = line.trim().split_once(' ').unwrap_or((line.trim(), ""));
        let arg = rest.trim();
        if matches!(command, "quit" | "exit") {
            self.quit = true;
            return Ok(());
        }
        if matches!(
            command,
            "help" | "inventory" | "quests" | "perks" | "npcs" | "shop"
        ) || command == "craft" && arg.is_empty()
        {
            self.panel = command.into();
            self.scroll = 0;
            return Ok(());
        }
        ensure!(self.ready, "Дождитесь подключения");
        match command {
            "respec" => {
                let refunded = self
                    .state
                    .respec_perks()
                    .ok_or_else(|| anyhow!("Недостаточно золота"))?;
                self.log(format!("Возвращено очков: {refunded}"));
            }
            "perkpoint" => {
                ensure!(
                    self.state.craft_perk_point(),
                    "Не хватает материалов или золота"
                );
                self.log("Создано очко перка");
            }
            "save" => {
                self.save()?;
                self.log("Сохранение отправлено / записано");
            }
            "say" => {
                ensure!(
                    !arg.is_empty() && arg.chars().count() <= 300,
                    "Сообщение: 1–300 символов"
                );
                if let Some(net) = &self.network {
                    net.raw(protocol::encode(
                        "chat",
                        &protocol::Chat { text: arg.into() },
                    )?)?;
                } else {
                    self.log(format!("Вы: {arg}"));
                }
            }
            "delve" => {
                ensure!(self.world.kind == "hub", "Вы уже в подземелье");
                let mut words = arg.split_whitespace();
                let depth: u32 = words.next().unwrap_or("1").parse()?;
                ensure!((1..=100).contains(&depth), "Глубина: 1–100");
                let owner: u16 = words.next().unwrap_or("0").parse()?;
                self.save()?;
                self.game_command("enter_delve", json!({"depth": depth, "owner": owner}))?;
            }
            "hub" => {
                ensure!(self.world.kind == "delve", "Вы уже в хабе");
                if self.downed {
                    let lost = self.state.gold / 4;
                    self.state.gold -= lost;
                    self.log(format!("Возрождение: потеряно {lost} золота"));
                }
                if !self.snapshot.enemies.is_empty() {
                    self.log("Возвращение без завершения забега");
                } else if !self.downed && self.snapshots_received > 0 {
                    self.state.dungeons_cleared = self.state.dungeons_cleared.max(self.depth);
                }
                self.save()?;
                self.game_command("leave_delve", json!({}))?;
            }
            "weapon" => {
                let id =
                    weapon::WeaponId::from_id(arg).ok_or_else(|| anyhow!("Неизвестное оружие"))?;
                ensure!(self.arsenal.has(id), "Этого оружия нет в арсенале");
                self.arsenal.current = id;
            }
            "class" => {
                ensure!(
                    self.state.class_idx.is_none(),
                    "Класс уже выбран. Для нового персонажа задайте --class при первом запуске."
                );
                let idx: usize = arg.parse()?;
                ensure!((1..=3).contains(&idx), "Класс: 1–3");
                self.state.class_idx = Some(idx - 1);
                self.game_command("choose_class", json!({"class":idx-1}))?;
            }
            "spec" => {
                let idx: usize = arg.parse()?;
                ensure!((1..=3).contains(&idx), "Специализация: 1–3");
                ensure!(
                    !self.state.flags.contains("terminal_spec_chosen"),
                    "Специализация уже выбрана"
                );
                self.state.spec_idx = idx - 1;
                let spec = &classes::classes()[self.state.class_idx.unwrap_or(1)].specs[idx - 1];
                if let Some(weapon) = spec.extra_weapon {
                    self.arsenal.give_weapon(weapon);
                }
                self.state.flags.insert("terminal_spec_chosen".into());
                self.log(format!("Специализация: {}", spec.name("ru")));
            }
            "perk" => {
                let def = perk::available(&self.state.perks, self.state.perk_points)
                    .into_iter()
                    .find(|p| p.id == arg)
                    .ok_or_else(|| {
                        anyhow!("Перк недоступен: требования / очки / максимальный ранг")
                    })?;
                self.state.perk_points -= def.cost;
                *self.state.perks.entry(arg.into()).or_default() += 1;
                self.log(format!("Изучен {}", def.name("ru")));
            }
            "craft" => {
                let recipe = self
                    .content
                    .cfg
                    .recipe(arg)
                    .ok_or_else(|| anyhow!("Нет рецепта {arg}"))?
                    .clone();
                let stations: Vec<_> = if self.world.kind == "hub" {
                    self.content
                        .map
                        .iter()
                        .flat_map(|m| &m.stations)
                        .filter(|s| {
                            (Vec3::new(s.pos[0], self.pos.y, s.pos[1]) - self.pos).length()
                                <= s.radius
                        })
                        .map(|s| s.kind.clone())
                        .collect()
                } else {
                    vec![]
                };
                let flags: Vec<_> = self.state.flags.iter().cloned().collect();
                let item = self
                    .content
                    .cfg
                    .item(&recipe.output)
                    .ok_or_else(|| anyhow!("Неизвестный результат рецепта"))?;
                craft::craft(
                    &mut self.state.inventory,
                    &recipe,
                    &CraftContext {
                        stations: &stations,
                        flags: &flags,
                    },
                    item.name("ru"),
                    &item.desc_ru,
                )
                .map_err(|e| anyhow!(e.message("ru")))?;
                self.log(format!("Создано: {}", recipe.name("ru")));
            }
            "buy" | "sell" => {
                ensure!(
                    self.near_npc("merchant"),
                    "Подойдите к торговцу (N на карте)"
                );
                let item = self
                    .content
                    .cfg
                    .item(arg)
                    .ok_or_else(|| anyhow!("Неизвестный предмет"))?;
                if command == "buy" {
                    ensure!(item.is_for_sale(), "Этот предмет не продаётся");
                    ensure!(
                        self.state.spend_gold(item.shop_price()),
                        "Недостаточно золота"
                    );
                    self.give_item(arg, 1);
                } else {
                    let price = item.sell_price();
                    ensure!(
                        price > 0 && self.state.inventory.remove_one(arg),
                        "Предмет нельзя продать"
                    );
                    self.state.gold += price;
                }
                self.log("Сделка завершена");
            }
            "use" => {
                ensure!(
                    self.network.is_none(),
                    "Сетевой протокол пока не поддерживает лечение предметом; HP задаёт сервер"
                );
                let heal = self
                    .content
                    .cfg
                    .item(arg)
                    .and_then(|i| i.heal)
                    .ok_or_else(|| anyhow!("Это не лечебный предмет"))?;
                ensure!(
                    self.hp < 100 && !self.downed,
                    "Лечение сейчас не нужно или персонаж повержен"
                );
                ensure!(self.state.inventory.remove_one(arg), "Нет предмета");
                if let Some(player) = self
                    .local
                    .as_mut()
                    .and_then(|s| s.players.get_mut(&self.peer))
                {
                    player.hp = (player.hp + heal).min(player.max_hp);
                }
            }
            "talk" => self.talk(arg)?,
            "quest" => self.quest(arg)?,
            "choose" => self.choose(arg.parse()?)?,
            _ => bail!("Неизвестная команда. F1 — помощь."),
        }
        Ok(())
    }

    pub fn near_npc(&self, id: &str) -> bool {
        self.world.kind == "hub"
            && self.content.cfg.npcs.iter().any(|n| {
                n.id == id && (Vec3::new(n.pos[0], self.pos.y, n.pos[1]) - self.pos).length() <= 4.0
            })
    }

    pub fn interact(&mut self) -> Result<()> {
        if self.scene.is_some() {
            self.advance_scene();
            return Ok(());
        }
        if self.world.kind == "delve"
            && (self.world.spawns.next_portal + self.world.offset - self.pos).length() < 3.0
        {
            ensure!(
                self.state.dungeons_cleared >= self.depth,
                "Сначала победите стража"
            );
            let depth = self.depth.saturating_add(1).min(100);
            self.execute("hub")?;
            self.next_depth = Some(depth);
            return Ok(());
        }
        if let Some(i) = self
            .points
            .iter()
            .position(|p| !p.used && (p.event.pos - self.pos).length() < 3.0)
        {
            self.use_point(i);
            return Ok(());
        }
        if self.world.kind == "delve"
            && (self.world.spawns.exit_portal + self.world.offset - self.pos).length() < 3.0
        {
            return self.execute("hub");
        }
        if self.world.kind == "hub" {
            if let Some(npc) = self.content.cfg.npcs.iter().find(|n| self.near_npc(&n.id)) {
                return self.talk(&npc.id.clone());
            }
            if let Some(gate) = self.content.map.as_ref().and_then(|m| m.gate) {
                if (Vec3::from(gate) - self.pos).length() < 5.0 {
                    return self.execute("delve 1");
                }
            }
        }
        if self.use_left <= 0.0 {
            self.log("Удерживайте E рядом с поверженным союзником для спасения");
        }
        self.use_left = 0.6;
        Ok(())
    }

    pub fn talk(&mut self, id: &str) -> Result<()> {
        ensure!(
            self.near_npc(id),
            "Подойдите к NPC (список и координаты: N)"
        );
        self.bump("interact", id);
        if id == "merchant" {
            self.panel = "shop".into();
            return Ok(());
        }
        let npc = self.content.cfg.npcs.iter().find(|n| n.id == id).unwrap();
        let mut scene_id = npc.scene.clone().unwrap_or_default();
        if scene_id == "story" {
            let aftermath = match id {
                "vale" => Some(("archive_sentinel", "hub_vale_archive_aftermath")),
                "victor" => Some(("crypt_warden", "hub_victor_warden_aftermath")),
                "elena" => Some(("blood_oracle", "hub_elena_oracle_aftermath")),
                "stranger" => Some(("heart_tyrant", "hub_stranger_tyrant_aftermath")),
                _ => None,
            };
            scene_id = aftermath
                .filter(|(boss, _)| {
                    self.state.has(&format!("boss_defeated_{boss}"))
                        && !self.state.has(&format!("discussed_{boss}"))
                })
                .map(|(_, scene)| scene.to_string())
                .unwrap_or_else(|| {
                    format!(
                        "hub_{id}_{}",
                        if self.state.has(&format!("met_{id}")) {
                            "repeat"
                        } else {
                            "intro"
                        }
                    )
                });
        }
        if let Some(scene) = self.resolve_scene(&scene_id) {
            self.open_scene(scene);
            return Ok(());
        }
        if let Some(q) = self.content.cfg.quests.iter().find(|q| {
            q.giver == id
                && !self.state.quests.is_completed(&q.id)
                && q.requires.iter().all(|r| self.state.quests.is_completed(r))
        }) {
            return self.quest(&q.id.clone());
        }
        self.log(format!("{}: Пока заданий нет.", npc.name("ru")));
        Ok(())
    }

    pub fn quest_progress(&self, q: &QuestCfg) -> u32 {
        match q.kind.as_str() {
            "clear_dungeon" => self.state.dungeons_cleared,
            "collect" => self
                .state
                .inventory
                .count(&q.target)
                .max(*self.state.quest_kills.get(&q.id).unwrap_or(&0)),
            _ => *self.state.quest_kills.get(&q.id).unwrap_or(&0),
        }
    }

    fn quest(&mut self, id: &str) -> Result<()> {
        let q = self
            .content
            .cfg
            .quest(id)
            .ok_or_else(|| anyhow!("Неизвестное задание"))?
            .clone();
        ensure!(self.near_npc(&q.giver), "Подойдите к NPC {}", q.giver);
        ensure!(!self.state.quests.is_completed(id), "Задание уже выполнено");
        ensure!(
            q.requires.iter().all(|r| self.state.quests.is_completed(r)),
            "Сначала завершите предыдущие задания"
        );
        let active = self.state.quests.is_active(id);
        let ready = self.quest_progress(&q) >= q.count;
        let custom = if !active {
            &q.offer_scene
        } else if ready {
            &q.complete_scene
        } else {
            &q.progress_scene
        };
        if let Some(mut scene) = custom.as_deref().and_then(|id| self.resolve_scene(id)) {
            if active && ready {
                for choice in &mut scene.choices {
                    if choice
                        .effects
                        .iter()
                        .any(|e| matches!(e,Effect::QuestDone(id) if id==&q.id))
                    {
                        for item in &q.reward_items {
                            choice.effects.push(Effect::Item {
                                id: item.id.clone(),
                                name: self
                                    .content
                                    .cfg
                                    .item(&item.id)
                                    .map(|i| i.name("ru"))
                                    .unwrap_or(&item.id)
                                    .into(),
                                qty: item.qty,
                            });
                        }
                    }
                }
            }
            self.open_scene(scene);
            return Ok(());
        }
        let effects = if !active {
            vec![Effect::Quest {
                id: q.id.clone(),
                title: q.title("ru").into(),
                desc: q.description("ru").into(),
            }]
        } else if ready {
            let mut effects = vec![
                Effect::QuestDone(q.id.clone()),
                Effect::Xp(q.reward_xp),
                Effect::Gold(q.reward_gold),
            ];
            for item in &q.reward_items {
                effects.push(Effect::Item {
                    id: item.id.clone(),
                    name: self
                        .content
                        .cfg
                        .item(&item.id)
                        .map(|i| i.name("ru"))
                        .unwrap_or(&item.id)
                        .into(),
                    qty: item.qty,
                });
            }
            effects
        } else {
            vec![]
        };
        self.open_scene(Scene {
            id: q.id.clone(),
            lines: vec![Line::new(
                &q.giver,
                "",
                &format!(
                    "{}\n{}\nПрогресс: {}/{} · Награда: {} XP / {} зол.",
                    q.title("ru"),
                    q.description("ru"),
                    self.quest_progress(&q),
                    q.count,
                    q.reward_xp,
                    q.reward_gold
                ),
            )],
            choices: vec![
                Choice::simple(
                    if !active {
                        "Взять задание"
                    } else if ready {
                        "Сдать задание"
                    } else {
                        "Вернусь позже"
                    },
                    effects,
                ),
                Choice::simple("До встречи", vec![]),
            ],
        });
        Ok(())
    }

    fn resolve_scene(&self, id: &str) -> Option<Scene> {
        self.content
            .cfg
            .dialogue(id)
            .cloned()
            .or_else(|| openheart_core::story::get_scene(id, &self.state))
    }
    fn open_scene(&mut self, scene: Scene) {
        self.scene = Some(scene);
        self.scene_line = 0;
        self.panel = "dialogue".into();
        self.scroll = 0;
        self.move_left = 0.0;
    }
    pub fn advance_scene(&mut self) {
        if let Some(scene) = &self.scene {
            if self.scene_line + 1 < scene.lines.len() {
                self.scene_line += 1;
                self.scroll = 0;
            } else if scene.choices.is_empty() {
                self.scene = None;
                self.panel.clear();
            }
        }
    }
    pub fn choose(&mut self, index: usize) -> Result<()> {
        let scene = self
            .scene
            .as_ref()
            .ok_or_else(|| anyhow!("Диалог не открыт"))?;
        ensure!(
            self.scene_line + 1 >= scene.lines.len(),
            "Сначала дочитайте диалог (Enter)"
        );
        let choice = scene
            .choices
            .get(index.wrapping_sub(1))
            .ok_or_else(|| anyhow!("Нет такого ответа"))?
            .clone();
        ensure!(
            choice
                .requires
                .as_ref()
                .is_none_or(|r| r.is_met(&self.state)),
            "Условия ответа не выполнены"
        );
        for message in self.state.apply(&choice.effects, "ru") {
            self.log(message);
        }
        self.scene = None;
        if let Some(next) = choice.next {
            if let Some(giver) = next.strip_prefix("giver:") {
                return self.talk(giver);
            }
            if let Some(scene) = self.resolve_scene(&next) {
                self.open_scene(scene);
                return Ok(());
            }
        }
        self.panel.clear();
        self.save()?;
        Ok(())
    }

    pub fn panel_text(&self) -> String {
        match self.panel.as_str() {
            "inventory" => {
                let mut text = format!("Золото: {}\n\n", self.state.gold);
                for i in &self.state.inventory.items {
                    text += &format!("{} ×{} [{}]\n", i.name, i.qty, i.id);
                }
                text += "\nОружие:\n";
                for w in weapon::weapons().iter().filter(|w| self.arsenal.has(w.id)) {
                    text += &format!(
                        "{} [{}] · {} в магазине\n",
                        w.name("ru"),
                        w.id.id(),
                        self.arsenal.clips[w.id.slot()]
                    );
                }
                text += "\n:use ID · :weapon ID";
                text
            }
            "quests" => self
                .content
                .cfg
                .quests
                .iter()
                .filter(|q| self.state.quests.has(&q.id))
                .map(|q| {
                    format!(
                        "{} [{}]\n{:?} · {}/{}\n{}\n",
                        q.title("ru"),
                        q.id,
                        self.state.quests.state_of(&q.id),
                        self.quest_progress(q),
                        q.count,
                        q.description("ru")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
            "perks" => format!(
                "Очки: {} · :perk ID\n\n{}",
                self.state.perk_points,
                perk::perks()
                    .iter()
                    .map(|p| format!(
                        "{} [{}] · {}/{} · цена {}\n{}",
                        p.name("ru"),
                        p.id,
                        self.state.perks.get(&p.id).unwrap_or(&0),
                        p.max_ranks,
                        p.cost,
                        p.description("ru")
                    ))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            ),
            "craft" => format!(
                ":craft ID · станция должна быть рядом\n\n{}",
                self.content
                    .cfg
                    .recipes
                    .iter()
                    .map(|r| format!(
                        "{} [{}]\n{} → {} ×{}\nСтанция: {}",
                        r.name("ru"),
                        r.id,
                        r.inputs
                            .iter()
                            .map(|i| format!("{} ×{}", i.item, i.qty))
                            .collect::<Vec<_>>()
                            .join(", "),
                        r.output,
                        r.output_qty,
                        r.station
                    ))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            ),
            "npcs" => self
                .content
                .cfg
                .npcs
                .iter()
                .map(|n| {
                    format!(
                        "{} [{}]\nx {:.0} z {:.0} · {:.0} м",
                        n.name("ru"),
                        n.id,
                        n.pos[0],
                        n.pos[1],
                        (Vec3::new(n.pos[0], self.pos.y, n.pos[1]) - self.pos).length()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n"),
            "shop" => format!(
                "Золото: {} · :buy ID / :sell ID\n\n{}",
                self.state.gold,
                self.content
                    .cfg
                    .items
                    .iter()
                    .filter(|i| i.is_for_sale())
                    .map(|i| format!(
                        "{} [{}]\nКупить {} / продать {}",
                        i.name("ru"),
                        i.id,
                        i.shop_price(),
                        i.sell_price()
                    ))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            ),
            "dialogue" => self
                .scene
                .as_ref()
                .map(|s| {
                    let mut text = s
                        .lines
                        .get(self.scene_line)
                        .map(|l| {
                            format!(
                                "{}\n\n{}\n\n",
                                l.speaker,
                                dialogue::localized(&l.text, "ru")
                            )
                        })
                        .unwrap_or_default();
                    if self.scene_line + 1 >= s.lines.len() {
                        for (i, c) in s.choices.iter().enumerate() {
                            text += &format!(
                                "{}: {}{}\n",
                                i + 1,
                                dialogue::localized(&c.text, "ru"),
                                if c.requires.as_ref().is_none_or(|r| r.is_met(&self.state)) {
                                    ""
                                } else {
                                    " [недоступно]"
                                }
                            );
                        }
                    }
                    text + "\nEnter — далее · Esc — закрыть"
                })
                .unwrap_or_default(),
            _ => HELP.into(),
        }
    }
}
