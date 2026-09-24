//! Экран крафта: список рецептов и их выполнение.
//!
//! Правила крафта — в ядре (`openheart_core::craft`), здесь только показ и ввод.
//! Клавиша `C` открывает экран, цифры 1–9 выполняют рецепт из списка.

use godot::classes::{Input, Label, Panel, ScrollContainer};
use godot::prelude::*;

use openheart_core::craft::{self, CraftContext};
use openheart_core::item::Inventory;

use crate::locale::t;

use super::{Game3D, Mode};

/// Сколько рецептов показываем и крафтим по цифрам 1–8 (как в панели перков).
const MAX_ROWS: usize = 8;

impl Game3D {
    /// Собрать панель крафта. Вызывается один раз при построении HUD.
    pub(super) fn build_craft_panel(&mut self, layer: &mut Gd<godot::classes::CanvasLayer>) {
        let (pw, ph) = (820.0, 560.0);
        let mut panel = Panel::new_alloc();
        panel.set_position(Vector2::new((super::HUD_W - pw) * 0.5, (super::HUD_H - ph) * 0.5));
        panel.set_size(Vector2::new(pw, ph));
        panel.add_theme_stylebox_override(
            "panel",
            &super::make_style(super::C_UI_BG, super::C_BORDER, 2),
        );
        panel.set_visible(false);

        let lang = self.settings.lang.clone();
        let mut title = Label::new_alloc();
        title.set_text(t("craft_title", &lang));
        title.set_position(Vector2::new(24.0, 18.0));
        title.set_size(Vector2::new(pw - 48.0, 32.0));
        title.add_theme_font_size_override("font_size", 22);
        title.add_theme_color_override("font_color", super::C_PINK);
        panel.add_child(&title);

        let mut scroll = ScrollContainer::new_alloc();
        scroll.set_position(Vector2::new(24.0, 60.0));
        scroll.set_size(Vector2::new(pw - 48.0, ph - 110.0));
        panel.add_child(&scroll);

        let mut list = Label::new_alloc();
        list.set_custom_minimum_size(Vector2::new(pw - 76.0, 0.0));
        list.add_theme_font_size_override("font_size", 15);
        list.add_theme_color_override("font_color", super::C_MAIN);
        list.set_autowrap_mode(godot::classes::text_server::AutowrapMode::WORD);
        scroll.add_child(&list);

        let mut hint = Label::new_alloc();
        hint.set_text(t("craft_close", &lang));
        hint.set_position(Vector2::new(24.0, ph - 42.0));
        hint.set_size(Vector2::new(pw - 48.0, 28.0));
        hint.add_theme_font_size_override("font_size", 13);
        hint.add_theme_color_override("font_color", super::C_DIM);
        panel.add_child(&hint);

        layer.add_child(&panel);
        self.craft_list = Some(list);
        self.craft_panel = Some(panel);
    }

    pub(super) fn open_craft(&mut self) {
        self.mode = Mode::Craft;
        self.freeze_player(true);
        self.refresh_craft_ui();
        if let Some(panel) = self.craft_panel.as_mut() {
            panel.set_visible(true);
        }
        if let Some(label) = self.hint_label.as_mut() {
            label.set_visible(false);
        }
        Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
    }

    pub(super) fn close_craft(&mut self) {
        if let Some(panel) = self.craft_panel.as_mut() {
            panel.set_visible(false);
        }
        self.set_mode_explore();
    }

    pub(super) fn process_craft(&mut self) {
        let input = Input::singleton();
        if input.is_action_just_pressed("craft") || input.is_action_just_pressed("escape") {
            self.close_craft();
            return;
        }
        // Цифры 1–8 крафтят рецепт из списка (те же клавиши, что в панели перков).
        for index in 0..MAX_ROWS {
            if input.is_action_just_pressed(&format!("weapon_{}", index + 1)) {
                self.try_craft(index);
                return;
            }
        }
    }

    /// Станции крафта рядом с игроком: физические верстаки из карты
    /// (`stations` в hub.json). Станочные рецепты (`station: "bench"`) доступны
    /// только когда игрок стоит в зоне такой станции; безстаночные — где угодно.
    fn nearby_stations(&self) -> Vec<String> {
        let Some(player) = self.player() else {
            return Vec::new();
        };
        let pos = player.get_global_position();
        let mut out: Vec<String> = self
            .station_zones
            .iter()
            .filter(|(_, center, radius)| pos.distance_to(*center) <= *radius)
            .map(|(kind, _, _)| kind.clone())
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// Выполнить рецепт под номером `index` из показанного списка.
    fn try_craft(&mut self, index: usize) {
        let Some(recipe_id) = self.craft_shown.get(index).cloned() else {
            return;
        };
        let lang = self.settings.lang.clone();
        let Some(cfg) = self.cfg.take() else {
            return;
        };

        let result = (|| {
            let recipe = cfg.recipe(&recipe_id)?;
            let state = self.state.as_mut()?;
            let stations = self.craft_stations.clone();
            let flags: Vec<String> = state.flags.iter().cloned().collect();
            let ctx = CraftContext { stations: &stations, flags: &flags };

            let item = cfg.items.iter().find(|item| item.id == recipe.output);
            let name = item.map(|item| item.name(&lang)).unwrap_or(&recipe.output);
            let description = item
                .map(|item| if lang == "en" { item.desc_en.as_str() } else { item.desc_ru.as_str() })
                .unwrap_or("");

            Some(craft::craft(&mut state.inventory, recipe, &ctx, name, description))
        })();

        self.cfg = Some(cfg);

        match result {
            Some(Ok(item)) => {
                self.show_flash(&format!("{} {}", t("craft_done", &lang), item.name));
                self.refresh_craft_ui();
            }
            Some(Err(error)) => self.show_flash(&error.message(&lang)),
            None => {}
        }
    }

    /// Перерисовать список рецептов.
    pub(super) fn refresh_craft_ui(&mut self) {
        let lang = self.settings.lang.clone();
        let stations = self.nearby_stations();
        self.craft_stations = stations;

        let Some(cfg) = self.cfg.take() else {
            return;
        };
        let empty = Inventory::default();
        let inventory = self
            .state
            .as_ref()
            .map(|state| &state.inventory)
            .unwrap_or(&empty);
        let flags: Vec<String> = self
            .state
            .as_ref()
            .map(|state| state.flags.iter().cloned().collect())
            .unwrap_or_default();
        let ctx = CraftContext { stations: &self.craft_stations, flags: &flags };

        let mut lines = Vec::new();
        let mut shown = Vec::new();

        if cfg.recipes.is_empty() {
            lines.push(t("craft_empty", &lang).to_string());
        }

        for recipe in cfg.recipes.iter().take(MAX_ROWS) {
            let doable = craft::check(inventory, recipe, &ctx);
            let mark = if doable.is_ok() { "◆" } else { "◇" };
            let number = shown.len() + 1;
            shown.push(recipe.id.clone());

            let inputs: Vec<String> = recipe
                .inputs
                .iter()
                .map(|input| {
                    let item_name = cfg
                        .items
                        .iter()
                        .find(|item| item.id == input.item)
                        .map(|item| item.name(&lang).to_string())
                        .unwrap_or_else(|| input.item.clone());
                    format!("{item_name} {}/{}", inventory.count(&input.item), input.qty)
                })
                .collect();

            lines.push(format!(
                "{mark} {number}. {}  ←  {}",
                recipe.name(&lang),
                inputs.join(", ")
            ));
            if let Err(error) = doable {
                lines.push(format!("      {}", error.message(&lang)));
            }
            lines.push(String::new());
        }

        self.cfg = Some(cfg);
        self.craft_shown = shown;
        if let Some(label) = self.craft_list.as_mut() {
            label.set_text(&lines.join("\n"));
        }
    }
}
