//! Главное меню: New Game / Continue / Settings / Quit.
//! Кнопки — Label-узлы с ручным обнаружением клика через unhandled_input.

use godot::prelude::*;
use godot::classes::{
    Control, IControl, InputEvent, InputEventMouseButton, Label,
    Panel, StyleBoxFlat,
};
use godot::global::{HorizontalAlignment, MouseButton};
use crate::locale::t;
use crate::save;
use crate::settings::Settings;

const W: f32 = 1920.0;
const H: f32 = 1080.0;
const BTN_W: f32 = 340.0;
const BTN_H: f32 = 58.0;
const BTN_X: f32 = (W - BTN_W) * 0.5;

#[derive(GodotClass)]
#[class(base = Control)]
pub struct MainMenu {
    base: Base<Control>,
    settings:      Settings,
    show_settings: bool,

    // Ректы кнопок для hit-теста
    r_new:      Rect2,
    r_cont:     Rect2,
    r_preset:   Rect2,
    r_settings: Rect2,
    r_quit:     Rect2,
    r_lang:     Rect2,
    r_back:     Rect2,

    // Дочерние узлы которые нам нужно обновлять
    lbl_continue: Option<Gd<Label>>,
    lbl_preset:   Option<Gd<Label>>,
    lbl_preset_desc: Option<Gd<Label>>,
    panel_settings: Option<Gd<Panel>>,
    lbl_lang_val:   Option<Gd<Label>>,

    presets:     Vec<String>,
    preset_idx:  usize,
}

// ── Утилиты ───────────────────────────────────────────────────────────────────

fn btn_rect(y: f32) -> Rect2 {
    Rect2::new(Vector2::new(BTN_X, y), Vector2::new(BTN_W, BTN_H))
}

fn make_style(bg: Color, border: Color, w: i32) -> Gd<StyleBoxFlat> {
    let mut s = StyleBoxFlat::new_gd();
    s.set_bg_color(bg);
    s.set_border_color(border);
    s.set_border_width_all(w);
    s.set_corner_radius_all(6);
    s.set_content_margin_all(10.0);
    s
}

fn add_label(
    parent: &mut Gd<Control>,
    text: &str, pos: Vector2, size: Vector2,
    font_size: i32, color: Color, align: HorizontalAlignment,
) -> Gd<Label> {
    let mut lbl = Label::new_alloc();
    lbl.set_text(text);
    lbl.set_position(pos);
    lbl.set_size(size);
    lbl.set_horizontal_alignment(align);
    lbl.add_theme_font_size_override("font_size", font_size);
    lbl.add_theme_color_override("font_color", color);
    parent.add_child(&lbl);
    lbl
}

// ── GodotClass ────────────────────────────────────────────────────────────────

#[godot_api]
impl IControl for MainMenu {
    fn init(base: Base<Control>) -> Self {
        Self {
            base,
            settings: Settings::default(),
            show_settings: false,
            r_new:      btn_rect(0.0),
            r_cont:     btn_rect(0.0),
            r_preset:   btn_rect(0.0),
            r_settings: btn_rect(0.0),
            r_quit:     btn_rect(0.0),
            r_lang:     btn_rect(0.0),
            r_back:     btn_rect(0.0),
            lbl_continue:   None,
            lbl_preset:     None,
            lbl_preset_desc: None,
            panel_settings: None,
            lbl_lang_val:   None,
            presets:    Vec::new(),
            preset_idx: 0,
        }
    }

    fn ready(&mut self) {
        self.settings = Settings::load();
        self.presets = crate::content::discover_presets();
        self.preset_idx = self.presets.iter()
            .position(|p| *p == self.settings.preset)
            .unwrap_or(0);
        self.base_mut().set_anchors_preset(
            godot::classes::control::LayoutPreset::FULL_RECT
        );

        let lang = self.settings.lang.clone();
        self.build_background();
        self.build_main_panel(&lang);
        self.build_settings_panel(&lang);
        self.refresh_preset_label();
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        let Ok(mb) = event.try_cast::<InputEventMouseButton>() else { return };
        if !mb.is_pressed() || mb.get_button_index() != MouseButton::LEFT { return }
        let raw = mb.get_position();
        // Scale from actual viewport pixels to our 1920×1080 design space
        let sz = self.base().get_size();
        let pos = if sz.x > 0.0 && sz.y > 0.0 {
            Vector2::new(raw.x * W / sz.x, raw.y * H / sz.y)
        } else { raw };

        if self.show_settings {
            self.handle_settings_click(pos);
        } else {
            self.handle_main_click(pos);
        }
    }
}

impl MainMenu {
    // ── Построение UI ─────────────────────────────────────────────────────────

    fn build_background(&mut self) {
        let mut panel = Panel::new_alloc();
        panel.set_position(Vector2::ZERO);
        panel.set_size(Vector2::new(W, H));
        panel.add_theme_stylebox_override(
            "panel",
            &make_style(Color::from_rgba(0.03, 0.01, 0.05, 1.0),
                        Color::from_rgba(0.1, 0.05, 0.15, 1.0), 0),
        );
        self.base_mut().add_child(&panel);
    }

    fn build_main_panel(&mut self, lang: &str) {
        // Заголовок
        add_label(
            &mut self.base_mut(), t("menu_title", lang),
            Vector2::new(0.0, H * 0.18), Vector2::new(W, 80.0),
            64, Color::from_rgba(1.0, 0.55, 0.8, 1.0),
            HorizontalAlignment::CENTER,
        );

        // Подзаголовок
        add_label(
            &mut self.base_mut(), "DOOM-style Action-RPG — открытый мир и процедурные данжи",
            Vector2::new(0.0, H * 0.18 + 88.0), Vector2::new(W, 30.0),
            18, Color::from_rgba(0.55, 0.44, 0.66, 1.0),
            HorizontalAlignment::CENTER,
        );

        let btn_start_y = H * 0.42;
        let gap = BTN_H + 20.0;

        // Кнопка «Новая игра»
        self.r_new = btn_rect(btn_start_y);
        self.make_btn(t("menu_new", lang), self.r_new);

        // Кнопка «Продолжить»
        self.r_cont = btn_rect(btn_start_y + gap);
        let has_save = save::exists();
        let cont_color = if has_save {
            Color::from_rgba(0.7, 0.95, 0.7, 1.0)
        } else {
            Color::from_rgba(0.35, 0.35, 0.4, 1.0)
        };
        let lbl = self.make_btn_colored(t("menu_continue", lang), self.r_cont, cont_color);
        self.lbl_continue = Some(lbl);

        // Кнопка «Пресет» (циклическое переключение установленных игр-пресетов)
        self.r_preset = btn_rect(btn_start_y + gap * 2.0);
        let lblp = self.make_btn_colored("Пресет: …", self.r_preset,
                                         Color::from_rgba(1.0, 0.72, 0.9, 1.0));
        self.lbl_preset = Some(lblp);
        let mut desc = Label::new_alloc();
        desc.set_position(Vector2::new(0.0, btn_start_y + gap * 2.0 + BTN_H - 4.0));
        desc.set_size(Vector2::new(W, 24.0));
        desc.set_horizontal_alignment(HorizontalAlignment::CENTER);
        desc.add_theme_font_size_override("font_size", 13);
        desc.add_theme_color_override("font_color", Color::from_rgba(0.5, 0.42, 0.6, 1.0));
        self.base_mut().add_child(&desc);
        self.lbl_preset_desc = Some(desc);

        // Кнопка «Настройки»
        self.r_settings = btn_rect(btn_start_y + gap * 3.0);
        self.make_btn(t("menu_settings", lang), self.r_settings);

        // Кнопка «Выход»
        self.r_quit = btn_rect(btn_start_y + gap * 4.0);
        self.make_btn(t("menu_quit", lang), self.r_quit);

        // Подсказка внизу
        add_label(
            &mut self.base_mut(), "WASD — движение  |  ЛКМ — выстрел  |  E — взаимодействие  |  I — инвентарь",
            Vector2::new(0.0, H - 40.0), Vector2::new(W, 30.0),
            13, Color::from_rgba(0.38, 0.32, 0.48, 1.0),
            HorizontalAlignment::CENTER,
        );
    }

    fn build_settings_panel(&mut self, lang: &str) {
        let pw = 600.0; let ph = 400.0;
        let px = (W - pw) * 0.5; let py = (H - ph) * 0.5;

        let mut panel = Panel::new_alloc();
        panel.set_position(Vector2::new(px, py));
        panel.set_size(Vector2::new(pw, ph));
        panel.add_theme_stylebox_override(
            "panel",
            &make_style(Color::from_rgba(0.05, 0.02, 0.09, 0.97),
                        Color::from_rgba(0.65, 0.30, 0.52, 1.0), 2),
        );
        panel.set_visible(false);

        macro_rules! row_label {
            ($text:expr, $y:expr, $size:expr, $col:expr) => {{
                let mut lbl = Label::new_alloc();
                lbl.set_text($text);
                lbl.set_position(Vector2::new(30.0, $y));
                lbl.set_size(Vector2::new(pw - 60.0, 36.0));
                lbl.add_theme_font_size_override("font_size", $size);
                lbl.add_theme_color_override("font_color", $col);
                panel.add_child(&lbl);
                lbl
            }};
        }

        row_label!(t("set_title", lang), 24.0, 22,
                   Color::from_rgba(1.0, 0.55, 0.8, 1.0));

        // Язык
        row_label!(t("set_lang", lang), 90.0, 15,
                   Color::from_rgba(0.7, 0.65, 0.8, 1.0));
        let lang_val = t(if lang == "en" { "set_lang_en" } else { "set_lang_ru" }, lang);
        let mut lbl_lang = Label::new_alloc();
        lbl_lang.set_text(&format!("[ {} ]  ← →  чтобы переключить", lang_val));
        lbl_lang.set_position(Vector2::new(30.0, 116.0));
        lbl_lang.set_size(Vector2::new(pw - 60.0, 32.0));
        lbl_lang.add_theme_font_size_override("font_size", 16);
        lbl_lang.add_theme_color_override("font_color", Color::from_rgba(0.9, 0.85, 1.0, 1.0));
        panel.add_child(&lbl_lang);

        // Кнопка переключения языка (рект внутри панели, скорректируем позже)
        self.r_lang = Rect2::new(Vector2::new(px + 30.0, py + 108.0), Vector2::new(pw - 60.0, 40.0));

        // Громкость (заглушка)
        row_label!(t("set_volume", lang), 175.0, 15,
                   Color::from_rgba(0.7, 0.65, 0.8, 1.0));
        row_label!(&format!("{:.0}%", self.settings.master_vol * 100.0), 200.0, 16,
                   Color::from_rgba(0.9, 0.85, 1.0, 1.0));

        // Чувствительность (заглушка)
        row_label!(t("set_sens", lang), 255.0, 15,
                   Color::from_rgba(0.7, 0.65, 0.8, 1.0));
        row_label!(&format!("{:.4}", self.settings.mouse_sens), 280.0, 16,
                   Color::from_rgba(0.9, 0.85, 1.0, 1.0));

        // Кнопка «Назад»
        self.r_back = Rect2::new(
            Vector2::new(px + (pw - BTN_W) * 0.5, py + ph - BTN_H - 24.0),
            Vector2::new(BTN_W, BTN_H),
        );
        let mut btn_back = Label::new_alloc();
        btn_back.set_text(t("set_back", lang));
        btn_back.set_position(Vector2::new((pw - BTN_W) * 0.5, ph - BTN_H - 24.0));
        btn_back.set_size(Vector2::new(BTN_W, BTN_H));
        btn_back.set_horizontal_alignment(HorizontalAlignment::CENTER);
        btn_back.add_theme_font_size_override("font_size", 18);
        btn_back.add_theme_color_override("font_color", Color::from_rgba(0.8, 0.7, 0.95, 1.0));
        panel.add_child(&btn_back);

        self.base_mut().add_child(&panel);
        self.lbl_lang_val   = Some(lbl_lang);
        self.panel_settings = Some(panel);
    }

    // ── Обработка кликов ──────────────────────────────────────────────────────

    fn handle_main_click(&mut self, pos: Vector2) {
        if self.r_new.contains_point(pos) {
            save::delete();
            self.load_scene("res://main.tscn");
        } else if self.r_cont.contains_point(pos) {
            if save::exists() {
                self.load_scene("res://main.tscn");
            }
        } else if self.r_preset.contains_point(pos) {
            // цикл по установленным пресетам («разные игры»)
            if !self.presets.is_empty() {
                self.preset_idx = (self.preset_idx + 1) % self.presets.len();
                self.settings.preset = self.presets[self.preset_idx].clone();
                self.settings.save();
                self.refresh_preset_label();
            }
        } else if self.r_settings.contains_point(pos) {
            self.show_settings = true;
            if let Some(ref mut p) = self.panel_settings { p.set_visible(true); }
        } else if self.r_quit.contains_point(pos) {
            self.base().get_tree().quit();
        }
    }

    fn refresh_preset_label(&mut self) {
        let id = self.presets.get(self.preset_idx).cloned().unwrap_or_else(|| "core".into());
        let info = crate::content::preset_info(&id);
        let multi = self.presets.len() > 1;
        if let Some(ref mut l) = self.lbl_preset {
            let arrow = if multi { "  ▸" } else { "" };
            l.set_text(&format!("Пресет: {}{}", info.name_ru, arrow));
        }
        if let Some(ref mut d) = self.lbl_preset_desc {
            d.set_text(&info.desc_ru);
        }
    }

    fn handle_settings_click(&mut self, pos: Vector2) {
        if self.r_back.contains_point(pos) {
            self.settings.save();
            self.show_settings = false;
            if let Some(ref mut p) = self.panel_settings { p.set_visible(false); }
        } else if self.r_lang.contains_point(pos) {
            // Переключить язык
            self.settings.lang = if self.settings.lang == "ru" {
                "en".into()
            } else {
                "ru".into()
            };
            let lang = self.settings.lang.clone();
            let val  = t(if lang == "en" { "set_lang_en" } else { "set_lang_ru" }, &lang);
            if let Some(ref mut lbl) = self.lbl_lang_val {
                lbl.set_text(&format!("[ {} ]  ← →  чтобы переключить", val));
            }
        }
    }

    // ── Вспомогательные ───────────────────────────────────────────────────────

    fn make_btn(&mut self, text: &str, rect: Rect2) -> Gd<Label> {
        self.make_btn_colored(text, rect, Color::from_rgba(0.85, 0.78, 0.95, 1.0))
    }

    fn make_btn_colored(&mut self, text: &str, rect: Rect2, color: Color) -> Gd<Label> {
        let mut lbl = Label::new_alloc();
        lbl.set_text(text);
        lbl.set_position(rect.position);
        lbl.set_size(rect.size);
        lbl.set_horizontal_alignment(HorizontalAlignment::CENTER);
        lbl.add_theme_font_size_override("font_size", 22);
        lbl.add_theme_color_override("font_color", color);
        self.base_mut().add_child(&lbl);
        lbl
    }

    fn load_scene(&mut self, path: &str) {
        self.base().get_tree().change_scene_to_file(path);
    }
}
