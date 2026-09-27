//! Главное меню: New Game / Continue / Settings / Quit.
//! Кнопки — Label-узлы с ручным обнаружением клика через unhandled_input.

use crate::locale::t;
use crate::net::host::PartyHost;
use crate::net::master;
use crate::save;
use crate::settings::Settings;
use godot::classes::{
    Control, HttpRequest, IControl, InputEvent, InputEventKey, InputEventMouseButton, Label, Panel,
    StyleBoxFlat,
};
use godot::global::{HorizontalAlignment, Key, MouseButton};
use godot::prelude::*;

const W: f32 = 1920.0;
const H: f32 = 1080.0;
const BTN_W: f32 = 340.0;
const BTN_H: f32 = 58.0;
const BTN_X: f32 = 120.0;
/// Сколько серверов помещается на экране. Прокрутки в этом меню нет, а фильтр
/// по данным пресета всё равно отсекает большую часть чужих.
const SERVER_ROWS: usize = 6;

#[derive(GodotClass)]
#[class(base = Control)]
pub struct MainMenu {
    base: Base<Control>,
    settings: Settings,
    show_settings: bool,

    // Ректы кнопок для hit-теста
    r_new: Rect2,
    r_creative: Rect2,
    r_arena: Rect2,
    r_cont: Rect2,
    r_preset: Rect2,
    r_connect: Rect2,
    r_settings: Rect2,
    r_quit: Rect2,
    r_back: Rect2,

    // Экран сетевой игры: список у мастера, пати-код и прямой адрес
    show_connect: bool,
    r_net_go: Rect2,
    r_net_solo: Rect2,
    r_net_refresh: Rect2,
    r_net_code_go: Rect2,
    panel_connect: Option<Gd<Panel>>,
    net_address: Option<Gd<godot::classes::LineEdit>>,
    net_master: Option<Gd<godot::classes::LineEdit>>,
    net_code: Option<Gd<godot::classes::LineEdit>>,
    lbl_net_status: Option<Gd<Label>>,
    net_labels: Vec<Gd<Label>>,
    server_rows: Vec<(Rect2, Gd<Label>)>,
    servers: Vec<master::ServerEntry>,
    /// Отпечаток данных активного пресета: по нему видно, пустит ли сервер.
    content_hash: String,
    http: Option<Gd<HttpRequest>>,
    pending: MasterRequest,
    /// Поднятый нами сервер для игры с другом.
    party: Option<PartyHost>,
    r_net_host: Rect2,

    // Строки настроек: кликабельный рект + что он меняет, и параллельно label-ы
    set_rows: Vec<(Rect2, SetKind)>,
    set_labels: Vec<Gd<Label>>,

    // Дочерние узлы которые нам нужно обновлять
    lbl_title: Option<Gd<Label>>,
    lbl_subtitle: Option<Gd<Label>>,
    lbl_new: Option<Gd<Label>>,
    lbl_creative: Option<Gd<Label>>,
    lbl_arena: Option<Gd<Label>>,
    lbl_continue: Option<Gd<Label>>,
    lbl_preset: Option<Gd<Label>>,
    lbl_preset_desc: Option<Gd<Label>>,
    lbl_connect: Option<Gd<Label>>,
    lbl_settings: Option<Gd<Label>>,
    lbl_quit: Option<Gd<Label>>,
    lbl_controls: Option<Gd<Label>>,
    panel_settings: Option<Gd<Panel>>,
    settings_title: Option<Gd<Label>>,
    settings_hint: Option<Gd<Label>>,
    settings_back: Option<Gd<Label>>,

    presets: Vec<String>,
    preset_idx: usize,
}

/// Что меняет строка настроек (клик — шаг/переключение).
#[derive(Clone, Copy, PartialEq)]
enum SetKind {
    Lang,
    Difficulty,
    Fullscreen,
    Vsync,
    Fov,
    PostFx,
    PostIntensity,
    Glow,
    Shadows,
    ScreenShake,
    MasterVol,
    MusicVol,
    SfxVol,
    Sens,
}

// ── Утилиты ───────────────────────────────────────────────────────────────────

fn btn_rect(y: f32) -> Rect2 {
    Rect2::new(Vector2::new(BTN_X, y), Vector2::new(BTN_W, BTN_H))
}

/// Название строки настройки.
fn set_name(kind: SetKind, lang: &str) -> &'static str {
    let en = lang == "en";
    match kind {
        SetKind::Lang => {
            if en {
                "Language"
            } else {
                "Язык"
            }
        }
        SetKind::Difficulty => {
            if en {
                "Difficulty"
            } else {
                "Сложность"
            }
        }
        SetKind::Fullscreen => {
            if en {
                "Fullscreen"
            } else {
                "Полный экран"
            }
        }
        SetKind::Vsync => {
            if en {
                "Vertical sync"
            } else {
                "Верт. синхронизация"
            }
        }
        SetKind::Fov => {
            if en {
                "Field of view (FOV)"
            } else {
                "Поле зрения (FOV)"
            }
        }
        SetKind::PostFx => {
            if en {
                "Post-processing"
            } else {
                "Пост-эффекты"
            }
        }
        SetKind::PostIntensity => {
            if en {
                "Effect intensity"
            } else {
                "Интенсивность эффектов"
            }
        }
        SetKind::Glow => {
            if en {
                "Glow (bloom)"
            } else {
                "Свечение (bloom)"
            }
        }
        SetKind::Shadows => {
            if en {
                "Shadows + SSAO"
            } else {
                "Тени + SSAO"
            }
        }
        SetKind::ScreenShake => {
            if en {
                "Screen shake"
            } else {
                "Тряска экрана"
            }
        }
        SetKind::MasterVol => {
            if en {
                "Master volume"
            } else {
                "Общая громкость"
            }
        }
        SetKind::MusicVol => {
            if en {
                "Music"
            } else {
                "Музыка"
            }
        }
        SetKind::SfxVol => {
            if en {
                "Sound effects"
            } else {
                "Звуки"
            }
        }
        SetKind::Sens => {
            if en {
                "Mouse sensitivity"
            } else {
                "Чувствительность мыши"
            }
        }
    }
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
    text: &str,
    pos: Vector2,
    size: Vector2,
    font_size: i32,
    color: Color,
    align: HorizontalAlignment,
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
            r_new: btn_rect(0.0),
            r_creative: btn_rect(0.0),
            r_arena: btn_rect(0.0),
            r_cont: btn_rect(0.0),
            r_preset: btn_rect(0.0),
            r_connect: btn_rect(0.0),
            r_settings: btn_rect(0.0),
            r_quit: btn_rect(0.0),
            r_back: btn_rect(0.0),
            show_connect: false,
            r_net_go: btn_rect(0.0),
            r_net_solo: btn_rect(0.0),
            r_net_refresh: btn_rect(0.0),
            r_net_code_go: btn_rect(0.0),
            panel_connect: None,
            net_address: None,
            net_master: None,
            net_code: None,
            lbl_net_status: None,
            net_labels: Vec::new(),
            server_rows: Vec::new(),
            servers: Vec::new(),
            content_hash: String::new(),
            http: None,
            pending: MasterRequest::Idle,
            party: None,
            r_net_host: btn_rect(0.0),
            set_rows: Vec::new(),
            set_labels: Vec::new(),
            lbl_title: None,
            lbl_subtitle: None,
            lbl_new: None,
            lbl_creative: None,
            lbl_arena: None,
            lbl_continue: None,
            lbl_preset: None,
            lbl_preset_desc: None,
            lbl_connect: None,
            lbl_settings: None,
            lbl_quit: None,
            lbl_controls: None,
            panel_settings: None,
            settings_title: None,
            settings_hint: None,
            settings_back: None,
            presets: Vec::new(),
            preset_idx: 0,
        }
    }

    fn ready(&mut self) {
        self.settings = Settings::load();
        self.settings.apply_global(); // окно/vsync/громкость из сохранённых настроек
        self.presets = crate::content::discover_presets();
        self.preset_idx = self
            .presets
            .iter()
            .position(|p| *p == self.settings.preset)
            .unwrap_or(0);
        self.base_mut()
            .set_anchors_preset(godot::classes::control::LayoutPreset::FULL_RECT);

        let lang = self.settings.lang.clone();
        self.build_background();
        self.build_main_panel(&lang);
        self.build_settings_panel(&lang);
        self.build_connect_panel(&lang);
        self.refresh_preset_label();

        // Запросы к мастеру идут через узел движка: он один умеет HTTP и в
        // нативной сборке, и в вебе.
        let mut http = HttpRequest::new_alloc();
        http.set_timeout(8.0);
        self.base_mut().add_child(&http);
        let callback = self.base().callable("on_master_reply");
        http.connect("request_completed", &callback);
        self.http = Some(http);
    }

    fn process(&mut self, delta: f64) {
        if self.show_connect {
            self.tick_party(delta);
        }
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        // F11 — переключить полный экран (в меню и на экране настроек)
        if let Ok(k) = event.clone().try_cast::<InputEventKey>() {
            if k.is_pressed() && !k.is_echo() && k.get_physical_keycode() == Key::F11 {
                self.settings.fullscreen = !self.settings.fullscreen;
                self.settings.apply_video();
                self.settings.save();
                if self.show_settings {
                    self.refresh_set_labels();
                }
            }
            return;
        }
        let Ok(mb) = event.try_cast::<InputEventMouseButton>() else {
            return;
        };
        if !mb.is_pressed() {
            return;
        }
        let btn = mb.get_button_index();
        if btn != MouseButton::LEFT && btn != MouseButton::RIGHT {
            return;
        }
        let dir = if btn == MouseButton::RIGHT { -1.0 } else { 1.0 };
        let raw = mb.get_position();
        // Scale from actual viewport pixels to our 1920×1080 design space
        let sz = self.base().get_size();
        let pos = if sz.x > 0.0 && sz.y > 0.0 {
            Vector2::new(raw.x * W / sz.x, raw.y * H / sz.y)
        } else {
            raw
        };

        if self.show_settings {
            self.handle_settings_click(pos, dir);
        } else if self.show_connect {
            if dir > 0.0 {
                self.handle_connect_click(pos);
            }
        } else if dir > 0.0 {
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
            &make_style(
                Color::from_rgba(0.03, 0.01, 0.05, 1.0),
                Color::from_rgba(0.1, 0.05, 0.15, 1.0),
                0,
            ),
        );
        self.base_mut().add_child(&panel);
        if let Some(texture) = godot::classes::ResourceLoader::singleton().load("res://assets/illustrations/menu_femboy_city.png").and_then(|r| r.try_cast::<godot::classes::Texture2D>().ok()) {
            let mut art = godot::classes::TextureRect::new_alloc();
            art.set_expand_mode(godot::classes::texture_rect::ExpandMode::IGNORE_SIZE);
            art.set_stretch_mode(godot::classes::texture_rect::StretchMode::KEEP_ASPECT_COVERED);
            art.set_texture(&texture);
            art.set_size(Vector2::new(W, H));
            art.set_mouse_filter(godot::classes::control::MouseFilter::IGNORE);
            self.base_mut().add_child(&art);
        }
    }

    fn build_main_panel(&mut self, lang: &str) {
        // Заголовок
        let title = add_label(
            &mut self.base_mut(),
            t("menu_title", lang),
            Vector2::new(0.0, H * 0.18),
            Vector2::new(W, 80.0),
            64,
            Color::from_rgba(1.0, 0.55, 0.8, 1.0),
            HorizontalAlignment::CENTER,
        );
        self.lbl_title = Some(title);

        // Подзаголовок
        let subtitle = add_label(
            &mut self.base_mut(),
            t("menu_subtitle", lang),
            Vector2::new(0.0, H * 0.18 + 88.0),
            Vector2::new(W, 30.0),
            18,
            Color::from_rgba(0.55, 0.44, 0.66, 1.0),
            HorizontalAlignment::CENTER,
        );
        self.lbl_subtitle = Some(subtitle);

        let btn_start_y = 340.0;
        let gap = BTN_H + 20.0;

        // Кнопка «Новая игра»
        self.r_new = btn_rect(btn_start_y);
        let new_label = self.make_btn(t("menu_new", lang), self.r_new);
        self.lbl_new = Some(new_label);

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
        let lblp = self.make_btn_colored(
            &format!("{}: …", t("menu_preset", lang)),
            self.r_preset,
            Color::from_rgba(1.0, 0.72, 0.9, 1.0),
        );
        self.lbl_preset = Some(lblp);
        let mut desc = Label::new_alloc();
        desc.set_position(Vector2::new(0.0, btn_start_y + gap * 2.0 + BTN_H - 4.0));
        desc.set_size(Vector2::new(W, 24.0));
        desc.set_horizontal_alignment(HorizontalAlignment::CENTER);
        desc.add_theme_font_size_override("font_size", 13);
        desc.add_theme_color_override("font_color", Color::from_rgba(0.5, 0.42, 0.6, 1.0));
        self.base_mut().add_child(&desc);
        self.lbl_preset_desc = Some(desc);

        // Кнопка «Подключиться» — игра на сервере
        self.r_connect = btn_rect(btn_start_y + gap * 3.0);
        let connect_label = self.make_btn_colored(
            t("menu_connect", lang),
            self.r_connect,
            Color::from_rgba(0.65, 0.9, 1.0, 1.0),
        );
        self.lbl_connect = Some(connect_label);

        // Кнопка «Настройки»
        self.r_settings = btn_rect(btn_start_y + gap * 4.0);
        let settings_label = self.make_btn(t("menu_settings", lang), self.r_settings);
        self.lbl_settings = Some(settings_label);

        // Кнопка «Выход»
        self.r_quit = btn_rect(btn_start_y + gap * 5.0);
        let quit_label = self.make_btn(t("menu_quit", lang), self.r_quit);
        self.lbl_quit = Some(quit_label);
        self.r_creative = btn_rect(btn_start_y + gap * 6.0);
        self.lbl_creative = Some(self.make_btn(if lang == "en" {"CREATIVE · all unlocked"} else {"КРЕАТИВ · всё доступно"}, self.r_creative));
        self.r_arena = btn_rect(btn_start_y + gap * 7.0);
        self.lbl_arena = Some(self.make_btn(if lang == "en" {"ARENA · combat test"} else {"АРЕНА · боевой тест"}, self.r_arena));
        for label in [self.lbl_title.as_mut(), self.lbl_subtitle.as_mut()].into_iter().flatten() {
            let height = label.get_size().y;
            label.set_position(Vector2::new(100.0, if height > 40.0 {150.0} else {246.0}));
            label.set_size(Vector2::new(580.0, height));
            label.set_horizontal_alignment(HorizontalAlignment::LEFT);
        }
        if let Some(desc) = self.lbl_preset_desc.as_mut() { desc.set_position(Vector2::new(BTN_X, btn_start_y + gap * 2.0 + BTN_H)); desc.set_size(Vector2::new(BTN_W, 24.0)); }

        // Подсказка внизу
        let controls = add_label(
            &mut self.base_mut(),
            t("menu_controls", lang),
            Vector2::new(0.0, H - 40.0),
            Vector2::new(W, 30.0),
            13,
            Color::from_rgba(0.38, 0.32, 0.48, 1.0),
            HorizontalAlignment::CENTER,
        );
        self.lbl_controls = Some(controls);
    }

    fn build_settings_panel(&mut self, lang: &str) {
        let pw = 760.0;
        let ph = 860.0;
        let px = (W - pw) * 0.5;
        let py = (H - ph) * 0.5;

        let mut panel = Panel::new_alloc();
        panel.set_position(Vector2::new(px, py));
        panel.set_size(Vector2::new(pw, ph));
        panel.add_theme_stylebox_override(
            "panel",
            &make_style(
                Color::from_rgba(0.05, 0.02, 0.09, 0.98),
                Color::from_rgba(0.65, 0.30, 0.52, 1.0),
                2,
            ),
        );
        panel.set_visible(false);

        let mut title = Label::new_alloc();
        title.set_text(t("set_title", lang));
        title.set_position(Vector2::new(0.0, 22.0));
        title.set_size(Vector2::new(pw, 42.0));
        title.set_horizontal_alignment(HorizontalAlignment::CENTER);
        title.add_theme_font_size_override("font_size", 26);
        title.add_theme_color_override("font_color", Color::from_rgba(1.0, 0.55, 0.8, 1.0));
        panel.add_child(&title);
        self.settings_title = Some(title);

        const ROWS: [SetKind; 14] = [
            SetKind::Lang,
            SetKind::Difficulty,
            SetKind::Fullscreen,
            SetKind::Vsync,
            SetKind::Fov,
            SetKind::PostFx,
            SetKind::PostIntensity,
            SetKind::Glow,
            SetKind::Shadows,
            SetKind::ScreenShake,
            SetKind::MasterVol,
            SetKind::MusicVol,
            SetKind::SfxVol,
            SetKind::Sens,
        ];
        let row_h = 46.0;
        let y0 = 80.0;
        self.set_rows.clear();
        self.set_labels.clear();
        for (i, kind) in ROWS.iter().enumerate() {
            let y = y0 + i as f32 * row_h;
            let mut lbl = Label::new_alloc();
            lbl.set_text(&format!(
                "{}:   {}",
                set_name(*kind, lang),
                self.set_value_str(*kind)
            ));
            lbl.set_position(Vector2::new(36.0, y));
            lbl.set_size(Vector2::new(pw - 72.0, row_h - 8.0));
            lbl.add_theme_font_size_override("font_size", 18);
            lbl.add_theme_color_override("font_color", Color::from_rgba(0.9, 0.85, 1.0, 1.0));
            panel.add_child(&lbl);
            let rect = Rect2::new(
                Vector2::new(px + 24.0, py + y - 4.0),
                Vector2::new(pw - 48.0, row_h - 2.0),
            );
            self.set_rows.push((rect, *kind));
            self.set_labels.push(lbl);
        }

        let mut hint = Label::new_alloc();
        hint.set_text(if lang == "en" {
            "click — change / toggle   ·   RMB — decrease"
        } else {
            "клик — изменить / переключить   ·   ПКМ — уменьшить"
        });
        hint.set_position(Vector2::new(0.0, ph - 96.0));
        hint.set_size(Vector2::new(pw, 26.0));
        hint.set_horizontal_alignment(HorizontalAlignment::CENTER);
        hint.add_theme_font_size_override("font_size", 13);
        hint.add_theme_color_override("font_color", Color::from_rgba(0.55, 0.5, 0.65, 1.0));
        panel.add_child(&hint);
        self.settings_hint = Some(hint);

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
        self.settings_back = Some(btn_back);

        self.base_mut().add_child(&panel);
        self.panel_settings = Some(panel);
    }

    /// Текущее значение настройки строкой.
    fn set_value_str(&self, kind: SetKind) -> String {
        let s = &self.settings;
        let on = |b: bool| match (b, s.lang == "en") {
            (true, true) => "on",
            (false, true) => "off",
            (true, false) => "вкл",
            (false, false) => "выкл",
        };
        match kind {
            SetKind::Lang => {
                if s.lang == "en" {
                    "English".into()
                } else {
                    "Русский".into()
                }
            }
            SetKind::Difficulty => s.difficulty_name(&s.lang).to_string(),
            SetKind::Fullscreen => on(s.fullscreen).into(),
            SetKind::Vsync => on(s.vsync).into(),
            SetKind::Fov => format!("{:.0}°", s.fov),
            SetKind::PostFx => on(s.post_fx).into(),
            SetKind::PostIntensity => format!("{:.0}%", s.post_intensity * 100.0),
            SetKind::Glow => on(s.glow).into(),
            SetKind::Shadows => on(s.shadows).into(),
            SetKind::ScreenShake => on(s.screen_shake).into(),
            SetKind::MasterVol => format!("{:.0}%", s.master_vol * 100.0),
            SetKind::MusicVol => format!("{:.0}%", s.music_vol * 100.0),
            SetKind::SfxVol => format!("{:.0}%", s.sfx_vol * 100.0),
            SetKind::Sens => format!("{:.4}", s.mouse_sens),
        }
    }

    /// Изменить настройку (dir: +1 клик ЛКМ, −1 ПКМ), сохранить и применить.
    fn step_setting(&mut self, kind: SetKind, dir: f32) {
        match kind {
            SetKind::Lang => {
                self.settings.lang = if self.settings.lang == "ru" {
                    "en".into()
                } else {
                    "ru".into()
                }
            }
            SetKind::Difficulty => self.settings.cycle_difficulty(),
            SetKind::Fullscreen => self.settings.fullscreen = !self.settings.fullscreen,
            SetKind::Vsync => self.settings.vsync = !self.settings.vsync,
            SetKind::Fov => self.settings.fov = (self.settings.fov + dir * 5.0).clamp(60.0, 110.0),
            SetKind::PostFx => self.settings.post_fx = !self.settings.post_fx,
            SetKind::PostIntensity => {
                self.settings.post_intensity =
                    (self.settings.post_intensity + dir * 0.1).clamp(0.0, 2.0)
            }
            SetKind::Glow => self.settings.glow = !self.settings.glow,
            SetKind::Shadows => self.settings.shadows = !self.settings.shadows,
            SetKind::ScreenShake => self.settings.screen_shake = !self.settings.screen_shake,
            SetKind::MasterVol => {
                self.settings.master_vol = (self.settings.master_vol + dir * 0.1).clamp(0.0, 1.0)
            }
            SetKind::MusicVol => {
                self.settings.music_vol = (self.settings.music_vol + dir * 0.1).clamp(0.0, 1.0)
            }
            SetKind::SfxVol => {
                self.settings.sfx_vol = (self.settings.sfx_vol + dir * 0.1).clamp(0.0, 1.0)
            }
            SetKind::Sens => {
                self.settings.mouse_sens =
                    (self.settings.mouse_sens + dir * 0.0005).clamp(0.0005, 0.01)
            }
        }
        self.settings.save();
        self.settings.apply_global(); // окно/vsync/громкость сразу
        self.refresh_set_labels();
    }

    /// Обновить тексты всех строк настроек.
    fn refresh_set_labels(&mut self) {
        for i in 0..self.set_labels.len() {
            let kind = self.set_rows[i].1;
            let text = format!(
                "{}:   {}",
                set_name(kind, &self.settings.lang),
                self.set_value_str(kind)
            );
            self.set_labels[i].set_text(&text);
        }
        let lang = self.settings.lang.as_str();
        if let Some(ref mut label) = self.settings_title {
            label.set_text(t("set_title", lang));
        }
        if let Some(ref mut label) = self.settings_hint {
            label.set_text(if lang == "en" {
                "click — change / toggle   ·   RMB — decrease"
            } else {
                "клик — изменить / переключить   ·   ПКМ — уменьшить"
            });
        }
        if let Some(ref mut label) = self.settings_back {
            label.set_text(t("set_back", lang));
        }
        self.refresh_main_labels();
        self.refresh_preset_label();
    }

    fn refresh_main_labels(&mut self) {
        let lang = self.settings.lang.as_str();
        if let Some(label) = self.lbl_creative.as_mut() { label.set_text(if lang == "en" {"CREATIVE · all unlocked"} else {"КРЕАТИВ · всё доступно"}); }
        if let Some(label) = self.lbl_subtitle.as_mut() {
            label.set_text(t("menu_subtitle", lang));
        }
        if let Some(label) = self.lbl_new.as_mut() {
            label.set_text(t("menu_new", lang));
        }
        if let Some(label) = self.lbl_continue.as_mut() {
            label.set_text(t("menu_continue", lang));
        }
        if let Some(label) = self.lbl_settings.as_mut() {
            label.set_text(t("menu_settings", lang));
        }
        if let Some(label) = self.lbl_quit.as_mut() {
            label.set_text(t("menu_quit", lang));
        }
        if let Some(label) = self.lbl_controls.as_mut() {
            label.set_text(t("menu_controls", lang));
        }
    }

    /// Спрятать или показать главное меню.
    ///
    /// Панели рисуются поверх фона, но подписи меню — это отдельные узлы, и
    /// добавлены они позже, поэтому просвечивают сквозь любую панель.
    fn set_main_visible(&mut self, visible: bool) {
        for label in [
            self.lbl_subtitle.as_mut(),
            self.lbl_new.as_mut(),
            self.lbl_creative.as_mut(),
            self.lbl_arena.as_mut(),
            self.lbl_continue.as_mut(),
            self.lbl_preset.as_mut(),
            self.lbl_preset_desc.as_mut(),
            self.lbl_connect.as_mut(),
            self.lbl_settings.as_mut(),
            self.lbl_quit.as_mut(),
            self.lbl_controls.as_mut(),
        ]
        .into_iter()
        .flatten()
        {
            label.set_visible(visible);
        }
        if let Some(ref mut title) = self.lbl_title {
            title.set_visible(visible);
        }
    }

    // ── Обработка кликов ──────────────────────────────────────────────────────

    fn handle_main_click(&mut self, pos: Vector2) {
        if self.r_creative.contains_point(pos) {
            crate::nodes::game::creative::request();
            self.load_scene("res://main.tscn");
            return;
        }
        if self.r_arena.contains_point(pos) {
            crate::nodes::game::creative::request_arena();
            self.load_scene("res://main.tscn");
            return;
        }
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
        } else if self.r_connect.contains_point(pos) {
            self.open_connect_panel();
        } else if self.r_settings.contains_point(pos) {
            self.show_settings = true;
            self.set_main_visible(false);
            if let Some(ref mut p) = self.panel_settings {
                p.set_visible(true);
            }
        } else if self.r_quit.contains_point(pos) {
            self.base().get_tree().quit();
        }
    }

    fn refresh_preset_label(&mut self) {
        let id = self
            .presets
            .get(self.preset_idx)
            .cloned()
            .unwrap_or_else(|| "core".into());
        let info = crate::content::preset_info(&id);
        let multi = self.presets.len() > 1;
        if let Some(ref mut l) = self.lbl_preset {
            let arrow = if multi { "  ▸" } else { "" };
            let name = if self.settings.lang == "en" && !info.name_en.is_empty() {
                &info.name_en
            } else {
                &info.name_ru
            };
            l.set_text(&format!(
                "{}: {}{}",
                t("menu_preset", &self.settings.lang),
                name,
                arrow
            ));
        }
        if let Some(ref mut d) = self.lbl_preset_desc {
            d.set_text(if self.settings.lang == "en" && !info.desc_en.is_empty() {
                &info.desc_en
            } else {
                &info.desc_ru
            });
        }
    }

    fn handle_settings_click(&mut self, pos: Vector2, dir: f32) {
        if self.r_back.contains_point(pos) {
            self.settings.save();
            self.show_settings = false;
            self.set_main_visible(true);
            if let Some(ref mut p) = self.panel_settings {
                p.set_visible(false);
            }
            return;
        }
        let hit = self
            .set_rows
            .iter()
            .find(|(r, _)| r.contains_point(pos))
            .map(|(_, k)| *k);
        if let Some(kind) = hit {
            self.step_setting(kind, dir);
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

    // ── Экран сетевой игры ────────────────────────────────────────────────────

    /// Панель «Сетевая игра»: список серверов у мастера, вход по пати-коду и
    /// прямой адрес — три способа попасть в одну и ту же игру.
    fn build_connect_panel(&mut self, lang: &str) {
        let (pw, ph) = (1120.0, 640.0);
        let (px, py) = ((W - pw) * 0.5, (H - ph) * 0.5);
        let inner = pw - 120.0;

        let mut panel = Panel::new_alloc();
        panel.set_position(Vector2::new(px, py));
        panel.set_size(Vector2::new(pw, ph));
        let mut style = StyleBoxFlat::new_gd();
        style.set_bg_color(Color::from_rgba(0.06, 0.03, 0.09, 0.96));
        style.set_border_width_all(2);
        style.set_border_color(Color::from_rgba(0.45, 0.75, 1.0, 0.8));
        panel.add_theme_stylebox_override("panel", &style);
        panel.set_visible(false);
        self.base_mut().add_child(&panel);

        let title = add_label(
            &mut self.base_mut(),
            t("net_title", lang),
            Vector2::new(px, py + 18.0),
            Vector2::new(pw, 40.0),
            26,
            Color::from_rgba(0.65, 0.9, 1.0, 1.0),
            HorizontalAlignment::CENTER,
        );
        self.net_labels.push(title);

        // ── Мастер и список серверов ──
        let mut y = py + 68.0;
        let head_master = add_label(
            &mut self.base_mut(),
            t("net_master", lang),
            Vector2::new(px + 60.0, y),
            Vector2::new(inner, 24.0),
            15,
            Color::from_rgba(0.55, 0.5, 0.65, 1.0),
            HorizontalAlignment::LEFT,
        );
        self.net_labels.push(head_master);
        y += 26.0;

        let mut master = godot::classes::LineEdit::new_alloc();
        master.set_position(Vector2::new(px + 60.0, y));
        master.set_size(Vector2::new(inner - 240.0, 44.0));
        master.set_placeholder("http://127.0.0.1:7780");
        master.add_theme_font_size_override("font_size", 18);
        master.set_visible(false);
        self.base_mut().add_child(&master);
        self.net_master = Some(master);

        self.r_net_refresh = Rect2::new(
            Vector2::new(px + 60.0 + inner - 224.0, y),
            Vector2::new(224.0, 44.0),
        );
        let refresh = self.make_btn_colored(
            t("net_refresh", lang),
            self.r_net_refresh,
            Color::from_rgba(0.7, 0.9, 1.0, 1.0),
        );
        self.net_labels.push(refresh);
        y += 54.0;

        let status = add_label(
            &mut self.base_mut(),
            "",
            Vector2::new(px + 60.0, y),
            Vector2::new(inner, 24.0),
            15,
            Color::from_rgba(0.75, 0.65, 0.5, 1.0),
            HorizontalAlignment::LEFT,
        );
        self.lbl_net_status = Some(status.clone());
        self.net_labels.push(status);
        y += 28.0;

        // Строки списка: сколько влезает, столько и показываем — прокрутки в
        // этом меню нет, а фильтр по данным всё равно отсекает лишнее.
        for _ in 0..SERVER_ROWS {
            let rect = Rect2::new(Vector2::new(px + 60.0, y), Vector2::new(inner, 34.0));
            let mut row = Label::new_alloc();
            row.set_position(rect.position);
            row.set_size(rect.size);
            row.add_theme_font_size_override("font_size", 19);
            row.add_theme_color_override("font_color", Color::from_rgba(0.85, 0.82, 0.95, 1.0));
            row.set_visible(false);
            self.base_mut().add_child(&row);
            self.server_rows.push((rect, row));
            y += 34.0;
        }
        y += 12.0;

        // ── Пати-код ──
        let head_code = add_label(
            &mut self.base_mut(),
            t("net_code", lang),
            Vector2::new(px + 60.0, y),
            Vector2::new(inner, 24.0),
            15,
            Color::from_rgba(0.55, 0.5, 0.65, 1.0),
            HorizontalAlignment::LEFT,
        );
        self.net_labels.push(head_code);
        y += 26.0;

        let mut code = godot::classes::LineEdit::new_alloc();
        code.set_position(Vector2::new(px + 60.0, y));
        code.set_size(Vector2::new(inner - 240.0, 44.0));
        code.set_placeholder("K7X2M9");
        code.set_max_length(12);
        code.add_theme_font_size_override("font_size", 18);
        code.set_visible(false);
        self.base_mut().add_child(&code);
        self.net_code = Some(code);

        self.r_net_code_go = Rect2::new(
            Vector2::new(px + 60.0 + inner - 224.0, y),
            Vector2::new(224.0, 44.0),
        );
        let code_go = self.make_btn_colored(
            t("net_code_go", lang),
            self.r_net_code_go,
            Color::from_rgba(0.75, 0.95, 0.8, 1.0),
        );
        self.net_labels.push(code_go);
        y += 56.0;

        // ── Прямой адрес ──
        let head_direct = add_label(
            &mut self.base_mut(),
            t("net_hint", lang),
            Vector2::new(px + 60.0, y),
            Vector2::new(inner, 24.0),
            15,
            Color::from_rgba(0.55, 0.5, 0.65, 1.0),
            HorizontalAlignment::LEFT,
        );
        self.net_labels.push(head_direct);
        y += 26.0;

        let mut address = godot::classes::LineEdit::new_alloc();
        address.set_position(Vector2::new(px + 60.0, y));
        address.set_size(Vector2::new(inner - 240.0, 44.0));
        address.set_placeholder("ws://127.0.0.1:7777/ws");
        address.add_theme_font_size_override("font_size", 18);
        address.set_visible(false);
        self.base_mut().add_child(&address);
        self.net_address = Some(address);

        self.r_net_go = Rect2::new(
            Vector2::new(px + 60.0 + inner - 224.0, y),
            Vector2::new(224.0, 44.0),
        );
        let go = self.make_btn_colored(
            t("net_go", lang),
            self.r_net_go,
            Color::from_rgba(0.7, 0.95, 0.8, 1.0),
        );
        self.net_labels.push(go);
        y += 58.0;

        self.r_net_host = Rect2::new(
            Vector2::new(px + 60.0, y),
            Vector2::new(inner * 0.5 - 8.0, BTN_H),
        );
        let host_btn = self.make_btn_colored(
            t("net_host", lang),
            self.r_net_host,
            Color::from_rgba(1.0, 0.75, 0.5, 1.0),
        );
        self.net_labels.push(host_btn);

        self.r_net_solo = Rect2::new(
            Vector2::new(px + 60.0 + inner * 0.5 + 8.0, y),
            Vector2::new(inner * 0.5 - 8.0, BTN_H),
        );
        let solo = self.make_btn(t("net_solo", lang), self.r_net_solo);
        self.net_labels.push(solo);

        for label in &mut self.net_labels {
            label.set_visible(false);
        }
        self.panel_connect = Some(panel);
    }

    fn open_connect_panel(&mut self) {
        self.show_connect = true;
        self.set_main_visible(false);
        // Хеш пресета считается по файлам, поэтому один раз на открытие экрана,
        // а не на каждую перерисовку списка.
        self.content_hash = crate::content::preset_content_hash(&self.settings.preset);

        let last = self.settings.server.clone();
        if let Some(ref mut address) = self.net_address {
            address.set_text(&last);
            address.set_visible(true);
        }
        let master = self.settings.master.clone();
        if let Some(ref mut field) = self.net_master {
            field.set_text(&master);
            field.set_visible(true);
        }
        if let Some(ref mut field) = self.net_code {
            field.set_visible(true);
        }
        if let Some(ref mut panel) = self.panel_connect {
            panel.set_visible(true);
        }
        for label in &mut self.net_labels {
            label.set_visible(true);
        }
        if !master.is_empty() {
            self.request_server_list();
        } else {
            self.set_net_status(t("net_master_empty", &self.settings.lang));
        }
    }

    fn close_connect_panel(&mut self) {
        self.show_connect = false;
        self.set_main_visible(true);
        for field in [self.net_address.as_mut(), self.net_master.as_mut(), self.net_code.as_mut()]
            .into_iter()
            .flatten()
        {
            field.set_visible(false);
        }
        if let Some(ref mut panel) = self.panel_connect {
            panel.set_visible(false);
        }
        for label in &mut self.net_labels {
            label.set_visible(false);
        }
        for (_, row) in &mut self.server_rows {
            row.set_visible(false);
        }
    }

    fn handle_connect_click(&mut self, pos: Vector2) {
        if self.r_net_refresh.contains_point(pos) {
            self.request_server_list();
            return;
        }
        if self.r_net_code_go.contains_point(pos) {
            self.request_party();
            return;
        }
        if self.r_net_host.contains_point(pos) {
            self.host_game();
            return;
        }
        if self.r_net_go.contains_point(pos) {
            let address = self
                .net_address
                .as_ref()
                .map(|field| field.get_text().to_string())
                .unwrap_or_default();
            self.join(address.trim());
            return;
        }
        if self.r_net_solo.contains_point(pos) {
            self.settings.server.clear();
            self.settings.save();
            self.close_connect_panel();
            return;
        }
        // Клик по строке списка: строка знает свой адрес.
        let hit = self
            .server_rows
            .iter()
            .position(|(rect, row)| row.is_visible() && rect.contains_point(pos));
        if let Some(index) = hit {
            if let Some(entry) = self.servers.get(index) {
                let lang = self.settings.lang.clone();
                if let Some(blocker) = entry.blocker(&self.settings.preset, &self.content_hash) {
                    // Не пускаем в заведомый отказ: сервер всё равно откажет,
                    // но игрок увидит это только пустым экраном.
                    self.set_net_status(blocker.text(&lang));
                    return;
                }
                let endpoint = entry.endpoint.clone();
                self.join(&endpoint);
            }
        }
    }

    /// Запомнить адрес и уйти в игру. Пусто — значит остаёмся в меню.
    fn join(&mut self, endpoint: &str) {
        if endpoint.is_empty() {
            return;
        }
        // Адрес запоминаем: играть обычно возвращаются на тот же сервер.
        self.settings.server = endpoint.to_string();
        self.settings.save();
        self.load_scene("res://main.tscn");
    }

    fn set_net_status(&mut self, text: &str) {
        if let Some(ref mut label) = self.lbl_net_status {
            label.set_text(text);
        }
    }

    fn master_url(&self) -> String {
        self.net_master
            .as_ref()
            .map(|field| field.get_text().to_string())
            .unwrap_or_default()
            .trim()
            .to_string()
    }

    // ── Запросы к мастеру ─────────────────────────────────────────────────────

    fn request_server_list(&mut self) {
        let master = self.master_url();
        if master.is_empty() {
            self.set_net_status(t("net_master_empty", &self.settings.lang));
            return;
        }
        // Адрес мастера меняют редко, но запоминать его надо: иначе список
        // приходится настраивать при каждом запуске.
        if self.settings.master != master {
            self.settings.master = master.clone();
            self.settings.save();
        }
        let url = master::list_url(
            &master,
            &self.settings.preset,
            &self.content_hash,
            self.settings.servers_compatible_only,
        );
        if self.send(MasterRequest::List, &url) {
            self.set_net_status(t("net_loading", &self.settings.lang));
        }
    }

    fn request_party(&mut self) {
        let code = self
            .net_code
            .as_ref()
            .map(|field| field.get_text().to_string())
            .unwrap_or_default();
        let code = master::normalize_code(&code);
        if code.is_empty() {
            return;
        }
        let master = self.master_url();
        if master.is_empty() {
            self.set_net_status(t("net_master_empty", &self.settings.lang));
            return;
        }
        let url = master::party_url(&master, &code);
        if self.send(MasterRequest::Party, &url) {
            self.set_net_status(t("net_loading", &self.settings.lang));
        }
    }

    /// Отправить запрос мастеру. Одновременно летит только один: меню всё
    /// равно ждёт ответа, а параллельные ответы пришлось бы различать.
    fn send(&mut self, kind: MasterRequest, url: &str) -> bool {
        let Some(http) = self.http.clone() else {
            return false;
        };
        let mut http = http;
        if self.pending != MasterRequest::Idle {
            http.cancel_request();
        }
        self.pending = kind;
        if http.request(url) != godot::global::Error::OK {
            self.pending = MasterRequest::Idle;
            self.set_net_status(t("net_master_down", &self.settings.lang));
            return false;
        }
        true
    }

    /// Разложить список серверов по строкам.
    fn show_servers(&mut self, body: &str) {
        let lang = self.settings.lang.clone();
        let preset = self.settings.preset.clone();
        let hash = self.content_hash.clone();

        match master::parse_list(body, &preset, &hash) {
            Ok(list) => self.servers = list,
            Err(_) => {
                self.servers.clear();
                self.set_net_status(t("net_master_down", &lang));
            }
        }

        for index in 0..self.server_rows.len() {
            let text = self
                .servers
                .get(index)
                .map(|entry| entry.row(&preset, &hash, env!("CARGO_PKG_VERSION"), &lang));
            let blocked = self
                .servers
                .get(index)
                .and_then(|entry| entry.blocker(&preset, &hash))
                .is_some();
            let (_, row) = &mut self.server_rows[index];
            match text {
                Some(text) => {
                    row.set_text(&text);
                    row.add_theme_color_override(
                        "font_color",
                        if blocked {
                            Color::from_rgba(0.5, 0.45, 0.55, 1.0)
                        } else {
                            Color::from_rgba(0.85, 0.82, 0.95, 1.0)
                        },
                    );
                    row.set_visible(true);
                }
                None => row.set_visible(false),
            }
        }

        if self.servers.is_empty() {
            self.set_net_status(t("net_empty", &lang));
        } else {
            let shown = self.servers.len().min(self.server_rows.len());
            self.set_net_status(&format!("{}: {}", t("net_found", &lang), shown));
        }
    }

    // ── Игра с другом ─────────────────────────────────────────────────────────

    /// Поднять свой сервер дочерним процессом.
    ///
    /// Это тот же `oh-server`, что стоит на дедиках: второй реализации хоста
    /// не существует (docs/MULTIPLAYER.md §7).
    fn host_game(&mut self) {
        if self.party.is_some() {
            return; // уже поднимаем
        }
        let master = self.master_url();
        if self.settings.master != master {
            self.settings.master = master.clone();
            self.settings.save();
        }
        let preset = self.settings.preset.clone();
        match PartyHost::start(&master, &preset) {
            Ok(party) => {
                self.party = Some(party);
                self.set_net_status(t("net_hosting", &self.settings.lang));
            }
            Err(error) => {
                let lang = self.settings.lang.clone();
                self.set_net_status(&format!("{}: {error}", t("net_host_failed", &lang)));
            }
        }
    }

    /// Дождаться, пока сервер напишет о себе, и показать код.
    fn tick_party(&mut self, delta: f64) {
        let Some(mut party) = self.party.take() else {
            return;
        };
        let lang = self.settings.lang.clone();
        match party.poll(delta) {
            Ok(None) => {
                self.party = Some(party);
            }
            Ok(Some(status)) => {
                // Хозяин заходит к себе напрямую: гонять свой же трафик через
                // релей мастера незачем.
                let address = status.own_address();
                if let Some(ref mut field) = self.net_address {
                    field.set_text(&address);
                }
                if !status.code.is_empty() {
                    if let Some(ref mut field) = self.net_code {
                        field.set_text(&status.code);
                    }
                    self.set_net_status(&format!("{} {}", t("net_code_ready", &lang), status.code));
                } else if !status.error.is_empty() {
                    self.set_net_status(&format!("{}: {}", t("net_code_failed", &lang), status.error));
                } else {
                    // Кода нет — значит зовём по адресу, какой сервер сумел
                    // получить: через релей или напрямую.
                    let shared = if status.endpoint.is_empty() {
                        address.clone()
                    } else {
                        status.endpoint.clone()
                    };
                    self.set_net_status(&format!("{} {shared}", t("net_host_local", &lang)));
                }
                // Сервер живёт дальше: игрок жмёт «Играть на сервере» и заходит.
                self.party = Some(party);
            }
            Err(error) => {
                party.stop();
                self.set_net_status(&format!("{}: {error}", t("net_host_failed", &lang)));
            }
        }
    }

    fn load_scene(&mut self, path: &str) {
        self.base().get_tree().change_scene_to_file(path);
    }
}

/// Что мы сейчас спрашиваем у мастера.
#[derive(Clone, Copy, PartialEq, Eq)]
enum MasterRequest {
    Idle,
    List,
    Party,
}

#[godot_api]
impl MainMenu {
    /// Ответ мастера. `HTTPRequest` умеет только сигнал, поэтому разбор
    /// приходит сюда, а не туда, откуда запрос уходил.
    #[func]
    fn on_master_reply(
        &mut self,
        _result: i64,
        code: i64,
        _headers: PackedStringArray,
        body: PackedByteArray,
    ) {
        let kind = std::mem::replace(&mut self.pending, MasterRequest::Idle);
        let lang = self.settings.lang.clone();
        if code != 200 {
            self.set_net_status(if kind == MasterRequest::Party {
                t("net_no_code", &lang)
            } else {
                t("net_master_down", &lang)
            });
            return;
        }
        let text = String::from_utf8_lossy(body.as_slice()).to_string();
        match kind {
            MasterRequest::List => self.show_servers(&text),
            MasterRequest::Party => match master::parse_party(&text) {
                Ok(endpoint) => self.join(&endpoint),
                Err(_) => self.set_net_status(t("net_no_code", &lang)),
            },
            MasterRequest::Idle => {}
        }
    }
}

#[cfg(debug_assertions)]
#[godot_api(secondary)]
impl MainMenu {
    #[func]
    fn runtime_smoke_localization(&mut self) -> VarDictionary {
        let original = self.settings.lang.clone();
        self.settings.lang = "ru".into();
        self.refresh_set_labels();
        let ru = self.main_labels_match("ru");
        let ru_subtitle = self
            .lbl_subtitle
            .as_ref()
            .map(|label| label.get_text().to_string())
            .unwrap_or_default();

        self.settings.lang = "en".into();
        self.refresh_set_labels();
        let en = self.main_labels_match("en");
        let en_subtitle = self
            .lbl_subtitle
            .as_ref()
            .map(|label| label.get_text().to_string())
            .unwrap_or_default();
        let settings_localized = self.set_labels.iter().enumerate().all(|(index, label)| {
            label
                .get_text()
                .to_string()
                .starts_with(set_name(self.set_rows[index].1, "en"))
        });

        self.settings.lang = original;
        self.refresh_set_labels();
        let mut snapshot = VarDictionary::new();
        snapshot.set("ru", ru);
        snapshot.set("en", en);
        snapshot.set("changed", ru_subtitle != en_subtitle);
        snapshot.set("settings", settings_localized);
        snapshot
    }

    fn main_labels_match(&self, lang: &str) -> bool {
        [
            self.lbl_subtitle
                .as_ref()
                .is_some_and(|label| label.get_text() == t("menu_subtitle", lang)),
            self.lbl_new
                .as_ref()
                .is_some_and(|label| label.get_text() == t("menu_new", lang)),
            self.lbl_continue
                .as_ref()
                .is_some_and(|label| label.get_text() == t("menu_continue", lang)),
            self.lbl_settings
                .as_ref()
                .is_some_and(|label| label.get_text() == t("menu_settings", lang)),
            self.lbl_quit
                .as_ref()
                .is_some_and(|label| label.get_text() == t("menu_quit", lang)),
            self.lbl_controls
                .as_ref()
                .is_some_and(|label| label.get_text() == t("menu_controls", lang)),
        ]
        .into_iter()
        .all(|matches| matches)
    }
}
