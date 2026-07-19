//! Построение HUD и полноэкранная постобработка.

use super::*;

// ── HUD ───────────────────────────────────────────────────────────────────────

/// Полноэкранная постобработка: виньетка, хроматическая аберрация, сканлайны,
/// анимированное зерно, лёгкий цвето-грейд + РЕАКТИВНЫЕ импульсы (урон/убийство/
/// подбор) — они затухают со временем, значения шлёт Rust каждый кадр.
/// Canvas-шейдер — работает и в GL Compatibility.
const POST_FX_SHADER: &str = r#"
shader_type canvas_item;
uniform sampler2D screen_tex : hint_screen_texture, filter_linear;
uniform float vignette   : hint_range(0.0, 1.0) = 0.34;
uniform float aberration = 1.4;
uniform float scanline   = 0.045;
uniform float grain      = 0.04;
// реактивные импульсы 0..1 (затухают в Rust)
uniform float hit  = 0.0;   // урон  — красная пульсация + тряска аберрации
uniform float kill = 0.0;   // фраг  — короткий яркий панч
uniform float pick = 0.0;   // подбор — тёплое золотистое свечение

float hash(vec2 p) {
    return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453);
}

void fragment() {
    vec2 uv = SCREEN_UV;
    float d = length(uv - 0.5);

    // хроматическая аберрация усиливается при уроне/фраге
    float ab = aberration + hit * 6.0 + kill * 3.0;
    vec2 off = (uv - 0.5) * ab * 0.0018;
    float r = texture(screen_tex, uv + off).r;
    float g = texture(screen_tex, uv).g;
    float b = texture(screen_tex, uv - off).b;
    vec3 col = vec3(r, g, b);

    // лёгкий цвето-грейд: холодные тени, тёплые света
    float luma = dot(col, vec3(0.299, 0.587, 0.114));
    col = mix(col, col * vec3(0.92, 0.98, 1.10), 0.25 * (1.0 - luma));
    col = mix(col, col * vec3(1.10, 1.02, 0.92), 0.20 * luma);

    // убийство: короткий яркий панч + лёгкая десатурация к белому по краям
    col += kill * 0.12;
    col = mix(col, vec3(luma), kill * 0.18);

    // подбор: тёплое золотистое свечение от краёв
    col += pick * vec3(0.35, 0.25, 0.05) * smoothstep(0.2, 0.9, d);

    // виньетка (при уроне краснеет и сгущается)
    float vig = vignette + hit * 0.35;
    vec3 vig_col = mix(vec3(0.0), vec3(0.6, 0.0, 0.02), hit);
    col = mix(col, vig_col, vig * smoothstep(0.30, 0.90, d));

    // анимированные сканлайны + зерно
    col *= 1.0 - scanline * (0.5 + 0.5 * sin((uv.y + TIME * 0.02) * 620.0));
    float gr = (hash(uv * vec2(1920.0, 1080.0) + fract(TIME) * 100.0) - 0.5);
    col += gr * grain;

    COLOR = vec4(col, 1.0);
}
"#;

impl Game3D {
    /// Слой постобработки поверх 3D, но ПОД HUD (layer -1 < HUD default 1... нет:
    /// экранная текстура читается до HUD, поэтому вешаем на слой 0 ниже HUD-слоя 1).
    pub(super) fn build_post_fx(&mut self) {
        use godot::classes::{ColorRect, Shader, ShaderMaterial};
        use godot::classes::control::MouseFilter;

        // Пост-обработку можно полностью выключить в настройках.
        if !self.settings.post_fx { return; }
        let k = self.settings.post_intensity.clamp(0.0, 2.0);

        let mut shader = Shader::new_gd();
        shader.set_code(POST_FX_SHADER);
        let mut mat = ShaderMaterial::new_gd();
        mat.set_shader(&shader);
        // базовые параметры масштабируем интенсивностью из настроек
        mat.set_shader_parameter("vignette",   &(0.34_f32 * k).to_variant());
        mat.set_shader_parameter("aberration", &(1.4_f32 * k).to_variant());
        mat.set_shader_parameter("scanline",   &(0.045_f32 * k).to_variant());
        mat.set_shader_parameter("grain",      &(0.04_f32 * k).to_variant());

        let mut rect = ColorRect::new_alloc();
        rect.set_anchors_preset(godot::classes::control::LayoutPreset::FULL_RECT);
        rect.set_material(&mat);
        rect.set_mouse_filter(MouseFilter::IGNORE);

        let mut layer = CanvasLayer::new_alloc();
        layer.set_layer(0); // под HUD (1), поверх 3D-вьюпорта
        layer.add_child(&rect);
        self.base_mut().add_child(&layer);
        self.post_mat = Some(mat);   // хэндл для реактивных импульсов
    }

    /// Затухание реактивных импульсов пост-процесса и отправка их в шейдер.
    /// Вызывается каждый кадр. Спайки ставят combat/подбор/урон.
    pub(super) fn tick_post_fx(&mut self, dt: f32) {
        // экспоненциальное затухание (импульсы короткие, ~0.25–0.5 c)
        self.fx_hit  = (self.fx_hit  - dt * 3.5).max(0.0);
        self.fx_kill = (self.fx_kill - dt * 4.5).max(0.0);
        self.fx_pick = (self.fx_pick - dt * 3.0).max(0.0);
        if let Some(mat) = self.post_mat.as_mut() {
            mat.set_shader_parameter("hit",  &self.fx_hit.to_variant());
            mat.set_shader_parameter("kill", &self.fx_kill.to_variant());
            mat.set_shader_parameter("pick", &self.fx_pick.to_variant());
        }
    }

    /// Спайк импульса (значение 0..1). Берём максимум, чтобы частые события
    /// не гасили друг друга.
    pub(super) fn punch_hit(&mut self)  {
        if self.settings.screen_shake { self.fx_hit = self.fx_hit.max(1.0); }
    }
    pub(super) fn punch_kill(&mut self) {
        if self.settings.screen_shake { self.fx_kill = self.fx_kill.max(1.0); }
    }
    pub(super) fn punch_pick(&mut self) { self.fx_pick = self.fx_pick.max(0.85); }

    pub(super) fn build_hud(&mut self, lang: &str) {
        self.build_post_fx();

        let mut layer = CanvasLayer::new_alloc();
        self.base_mut().add_child(&layer);

        // урон-флэш
        let mut df = Panel::new_alloc();
        df.set_anchors_preset(godot::classes::control::LayoutPreset::FULL_RECT);
        df.add_theme_stylebox_override("panel",
            &make_style(Color::from_rgba(0.8, 0.0, 0.0, 0.0), Color::TRANSPARENT_BLACK, 0));
        df.set_visible(false);
        layer.add_child(&df);
        self.damage_flash = Some(df);

        // FP-оружие (низ по центру)
        {
            let mut wr = TextureRect::new_alloc();
            place(&wr, 0.5, 1.0, HUD_W * 0.5 - 260.0, HUD_H - 560.0, 520.0, 560.0);
            wr.set_expand_mode(godot::classes::texture_rect::ExpandMode::IGNORE_SIZE);
            wr.set_stretch_mode(godot::classes::texture_rect::StretchMode::SCALE);
            wr.set_texture_filter(godot::classes::canvas_item::TextureFilter::NEAREST);
            wr.set_visible(false);
            layer.add_child(&wr);
            self.weapon_rect = Some(wr);
        }

        // прицел
        let mut cross = Label::new_alloc();
        cross.set_text("+");
        place(&cross, 0.5, 0.5, HUD_W * 0.5 - 8.0, HUD_H * 0.5 - 12.0, 16.0, 24.0);
        cross.set_horizontal_alignment(HorizontalAlignment::CENTER);
        cross.add_theme_font_size_override("font_size", 20);
        cross.add_theme_color_override("font_color", C_MAIN);
        layer.add_child(&cross);
        self.crosshair = Some(cross);

        // таргетинг
        let mut tgt = Label::new_alloc();
        place(&tgt, 0.5, 0.5, HUD_W * 0.5 - 200.0, HUD_H * 0.5 - 46.0, 400.0, 24.0);
        tgt.set_horizontal_alignment(HorizontalAlignment::CENTER);
        tgt.add_theme_font_size_override("font_size", 13);
        tgt.add_theme_color_override("font_color", C_RED);
        tgt.set_visible(false);
        layer.add_child(&tgt);
        self.targeting_label = Some(tgt);

        // компас
        let mut cmp = Label::new_alloc();
        cmp.set_text("N");
        place(&cmp, 0.5, 0.0, HUD_W * 0.5 - 40.0, 10.0, 80.0, 30.0);
        cmp.set_horizontal_alignment(HorizontalAlignment::CENTER);
        cmp.add_theme_font_size_override("font_size", 18);
        cmp.add_theme_color_override("font_color", C_CYAN);
        layer.add_child(&cmp);
        self.compass_label = Some(cmp);

        // локация
        let mut ll = Label::new_alloc();
        place(&ll, 0.5, 0.0, HUD_W * 0.5 - 300.0, 40.0, 600.0, 26.0);
        ll.set_horizontal_alignment(HorizontalAlignment::CENTER);
        ll.add_theme_font_size_override("font_size", 14);
        ll.add_theme_color_override("font_color", C_DIM);
        layer.add_child(&ll);
        self.loc_label = Some(ll);

        // Иконка HP (сердце)
        if let Some(tex) = self.cache.get("res://assets/ui/ui_heart.png") {
            let mut ic = TextureRect::new_alloc();
            ic.set_texture(&tex);
            place(&ic, 0.0, 1.0, 24.0, HUD_H - 100.0, 34.0, 34.0);
            ic.set_expand_mode(godot::classes::texture_rect::ExpandMode::IGNORE_SIZE);
            ic.set_stretch_mode(godot::classes::texture_rect::StretchMode::SCALE);
            ic.set_texture_filter(godot::classes::canvas_item::TextureFilter::NEAREST);
            layer.add_child(&ic);
        }

        // Иконка боезапаса
        if let Some(tex) = self.cache.get("res://assets/ui/ui_ammo.png") {
            let mut ic = TextureRect::new_alloc();
            ic.set_texture(&tex);
            place(&ic, 1.0, 1.0, HUD_W - 404.0, HUD_H - 66.0, 36.0, 36.0);
            ic.set_expand_mode(godot::classes::texture_rect::ExpandMode::IGNORE_SIZE);
            ic.set_stretch_mode(godot::classes::texture_rect::StretchMode::SCALE);
            ic.set_texture_filter(godot::classes::canvas_item::TextureFilter::NEAREST);
            layer.add_child(&ic);
        }

        // HP бар
        let mut hp_bg = Panel::new_alloc();
        place(&hp_bg, 0.0, 1.0, 24.0, HUD_H - 58.0, 222.0, 26.0);
        hp_bg.add_theme_stylebox_override("panel", &make_style(
            Color::from_rgba(0.08, 0.01, 0.01, 0.92), Color::from_rgba(0.35, 0.08, 0.08, 1.0), 1));
        layer.add_child(&hp_bg);

        let mut hp_fg = Panel::new_alloc();
        place(&hp_fg, 0.0, 1.0, 26.0, HUD_H - 56.0, 218.0, 22.0);
        hp_fg.add_theme_stylebox_override("panel", &make_style(C_RED, Color::TRANSPARENT_BLACK, 0));
        layer.add_child(&hp_fg);
        self.hp_bar_fg = Some(hp_fg);

        let mut hp_lbl = Label::new_alloc();
        place(&hp_lbl, 0.0, 1.0, 24.0, HUD_H - 84.0, 220.0, 24.0);
        hp_lbl.add_theme_font_size_override("font_size", 14);
        hp_lbl.add_theme_color_override("font_color", C_RED);
        layer.add_child(&hp_lbl);
        self.hp_label = Some(hp_lbl);

        // статусы игрока (над HP-баром)
        let mut st_lbl = Label::new_alloc();
        place(&st_lbl, 0.0, 1.0, 24.0, HUD_H - 112.0, 360.0, 24.0);
        st_lbl.add_theme_font_size_override("font_size", 16);
        st_lbl.add_theme_color_override("font_color", C_CYAN);
        layer.add_child(&st_lbl);
        self.status_label = Some(st_lbl);

        // XP бар
        let mut xp_bg = Panel::new_alloc();
        place(&xp_bg, 0.0, 1.0, 24.0, HUD_H - 28.0, 222.0, 10.0);
        xp_bg.add_theme_stylebox_override("panel", &make_style(
            Color::from_rgba(0.05, 0.03, 0.10, 0.92), Color::from_rgba(0.25, 0.15, 0.4, 1.0), 1));
        layer.add_child(&xp_bg);

        let mut xp_fg = Panel::new_alloc();
        place(&xp_fg, 0.0, 1.0, 25.0, HUD_H - 27.0, 0.0, 8.0);
        xp_fg.add_theme_stylebox_override("panel", &make_style(C_XP, Color::TRANSPARENT_BLACK, 0));
        layer.add_child(&xp_fg);
        self.xp_bar_fg = Some(xp_fg);

        let mut xp_lbl = Label::new_alloc();
        place(&xp_lbl, 0.0, 1.0, 252.0, HUD_H - 34.0, 260.0, 22.0);
        xp_lbl.add_theme_font_size_override("font_size", 13);
        xp_lbl.add_theme_color_override("font_color", C_XP);
        layer.add_child(&xp_lbl);
        self.xp_label = Some(xp_lbl);

        // Патроны и оружие (низ справа)
        let mut am = Label::new_alloc();
        place(&am, 1.0, 1.0, HUD_W - 360.0, HUD_H - 64.0, 336.0, 34.0);
        am.set_horizontal_alignment(HorizontalAlignment::RIGHT);
        am.add_theme_font_size_override("font_size", 26);
        am.add_theme_color_override("font_color", C_GOLD);
        layer.add_child(&am);
        self.ammo_label = Some(am);

        let mut wn = Label::new_alloc();
        place(&wn, 1.0, 1.0, HUD_W - 360.0, HUD_H - 92.0, 336.0, 24.0);
        wn.set_horizontal_alignment(HorizontalAlignment::RIGHT);
        wn.add_theme_font_size_override("font_size", 14);
        wn.add_theme_color_override("font_color", C_DIM);
        layer.add_child(&wn);
        self.weapon_label = Some(wn);

        // подсказка
        let mut hint = Label::new_alloc();
        place(&hint, 0.5, 1.0, HUD_W * 0.5 - 280.0, HUD_H - 130.0, 560.0, 28.0);
        hint.set_horizontal_alignment(HorizontalAlignment::CENTER);
        hint.add_theme_font_size_override("font_size", 16);
        hint.add_theme_color_override("font_color", C_GOLD);
        hint.set_visible(false);
        layer.add_child(&hint);
        self.hint_label = Some(hint);

        // инвентарь (строка)
        let mut inv = Label::new_alloc();
        place(&inv, 1.0, 0.0, HUD_W - 460.0, 10.0, 448.0, 24.0);
        inv.set_horizontal_alignment(HorizontalAlignment::RIGHT);
        inv.add_theme_font_size_override("font_size", 13);
        inv.add_theme_color_override("font_color", C_DIM);
        layer.add_child(&inv);
        self.inv_label = Some(inv);

        // квесты
        let mut ql = Label::new_alloc();
        place(&ql, 0.0, 0.0, 24.0, 44.0, 360.0, 150.0);
        ql.add_theme_font_size_override("font_size", 13);
        ql.add_theme_color_override("font_color", C_DIM);
        ql.set_autowrap_mode(godot::classes::text_server::AutowrapMode::WORD);
        layer.add_child(&ql);
        self.quest_label = Some(ql);

        // флэш
        let mut flash = Label::new_alloc();
        place(&flash, 0.5, 0.5, HUD_W * 0.5 - 300.0, HUD_H * 0.5 - 110.0, 600.0, 34.0);
        flash.set_horizontal_alignment(HorizontalAlignment::CENTER);
        flash.add_theme_font_size_override("font_size", 18);
        flash.add_theme_color_override("font_color", C_GOLD);
        flash.set_visible(false);
        layer.add_child(&flash);
        self.flash_label = Some(flash);

        // экран инвентаря
        {
            let pw = 720.0;
            let ph = 520.0;
            let mut ip = Panel::new_alloc();
            place(&ip, 0.5, 0.5, (HUD_W - pw) * 0.5, (HUD_H - ph) * 0.5, pw, ph);
            ip.add_theme_stylebox_override("panel", &make_style(C_UI_BG, C_BORDER, 2));
            ip.set_visible(false);

            let mut title = Label::new_alloc();
            title.set_text(t("inv_title", lang));
            title.set_position(Vector2::new(24.0, 16.0));
            title.set_size(Vector2::new(pw - 48.0, 32.0));
            title.add_theme_font_size_override("font_size", 22);
            title.add_theme_color_override("font_color", C_PINK);
            ip.add_child(&title);

            let mut il = Label::new_alloc();
            il.set_position(Vector2::new(24.0, 60.0));
            il.set_size(Vector2::new(pw - 48.0, ph - 110.0));
            il.add_theme_font_size_override("font_size", 15);
            il.add_theme_color_override("font_color", C_MAIN);
            il.set_autowrap_mode(godot::classes::text_server::AutowrapMode::WORD);
            ip.add_child(&il);

            let mut hint_i = Label::new_alloc();
            hint_i.set_text(t("inv_close", lang));
            hint_i.set_position(Vector2::new(24.0, ph - 42.0));
            hint_i.set_size(Vector2::new(pw - 48.0, 28.0));
            hint_i.add_theme_font_size_override("font_size", 13);
            hint_i.add_theme_color_override("font_color", C_DIM);
            ip.add_child(&hint_i);

            layer.add_child(&ip);
            self.inv_list = Some(il);
            self.inv_panel = Some(ip);
        }

        // экран перков
        {
            let pw = 900.0;
            let ph = 640.0;
            let mut pp = Panel::new_alloc();
            place(&pp, 0.5, 0.5, (HUD_W - pw) * 0.5, (HUD_H - ph) * 0.5, pw, ph);
            pp.add_theme_stylebox_override("panel", &make_style(C_UI_BG, C_BORDER, 2));
            pp.set_visible(false);

            let mut title = Label::new_alloc();
            title.set_text("ДЕРЕВО ПЕРКОВ");
            title.set_position(Vector2::new(24.0, 14.0));
            title.set_size(Vector2::new(pw - 48.0, 32.0));
            title.add_theme_font_size_override("font_size", 22);
            title.add_theme_color_override("font_color", C_XP);
            pp.add_child(&title);

            let mut pl = Label::new_alloc();
            pl.set_position(Vector2::new(24.0, 54.0));
            pl.set_size(Vector2::new(pw - 48.0, ph - 100.0));
            pl.add_theme_font_size_override("font_size", 14);
            pl.add_theme_color_override("font_color", C_MAIN);
            pl.set_autowrap_mode(godot::classes::text_server::AutowrapMode::WORD);
            pp.add_child(&pl);

            let mut hint_p = Label::new_alloc();
            hint_p.set_text("[ 1–8 ] купить перк   ·   [ P / Esc ] закрыть");
            hint_p.set_position(Vector2::new(24.0, ph - 40.0));
            hint_p.set_size(Vector2::new(pw - 48.0, 28.0));
            hint_p.add_theme_font_size_override("font_size", 13);
            hint_p.add_theme_color_override("font_color", C_DIM);
            pp.add_child(&hint_p);

            layer.add_child(&pp);
            self.perk_list = Some(pl);
            self.perk_panel = Some(pp);
        }

        // диалоговая панель
        {
            let mut panel = Panel::new_alloc();
            {
                // низ экрана во всю ширину, нижние 40%
                use godot::builtin::Side;
                panel.set_anchor(Side::LEFT,   0.0);
                panel.set_anchor(Side::RIGHT,  1.0);
                panel.set_anchor(Side::TOP,    0.60);
                panel.set_anchor(Side::BOTTOM, 1.0);
                for s in [Side::LEFT, Side::RIGHT, Side::TOP, Side::BOTTOM] {
                    panel.set_offset(s, 0.0);
                }
            }
            panel.add_theme_stylebox_override("panel", &make_style(C_UI_BG, C_BORDER, 2));
            panel.set_visible(false);

            let mut speaker = Label::new_alloc();
            speaker.set_position(Vector2::new(24.0, 16.0));
            speaker.set_size(Vector2::new(500.0, 30.0));
            speaker.add_theme_font_size_override("font_size", 20);
            speaker.add_theme_color_override("font_color", C_PINK);
            panel.add_child(&speaker);

            let mut text = Label::new_alloc();
            text.set_position(Vector2::new(24.0, 54.0));
            text.set_size(Vector2::new(HUD_W - 48.0, 145.0));
            text.add_theme_font_size_override("font_size", 16);
            text.add_theme_color_override("font_color", C_MAIN);
            text.set_autowrap_mode(godot::classes::text_server::AutowrapMode::WORD);
            panel.add_child(&text);

            let mut vbox = VBoxContainer::new_alloc();
            vbox.set_position(Vector2::new(24.0, 205.0));
            vbox.set_size(Vector2::new(HUD_W - 48.0, 170.0));
            panel.add_child(&vbox);

            let choice_lbls: [_; 4] = std::array::from_fn(|i| {
                let mut lbl = Label::new_alloc();
                lbl.set_text(&format!("{}.", i + 1));
                lbl.add_theme_font_size_override("font_size", 15);
                lbl.add_theme_color_override("font_color", C_DIM);
                lbl.set_visible(false);
                vbox.add_child(&lbl);
                lbl
            });
            let [c0, c1, c2, c3] = choice_lbls;

            layer.add_child(&panel);
            self.dlg_panel = Some(panel);
            self.dlg_speaker = Some(speaker);
            self.dlg_text = Some(text);
            self.choice_box = Some(vbox);
            self.cl0 = Some(c0); self.cl1 = Some(c1);
            self.cl2 = Some(c2); self.cl3 = Some(c3);
        }

        // миникарта данжа (правый верхний угол)
        {
            const MAP_SZ: f32 = 176.0;
            const MAP_X: f32 = HUD_W - MAP_SZ - 16.0;
            const MAP_Y: f32 = 40.0;

            let mut bg = Panel::new_alloc();
            place(&bg, 1.0, 0.0, MAP_X - 4.0, MAP_Y - 4.0, MAP_SZ + 8.0, MAP_SZ + 8.0);
            bg.add_theme_stylebox_override("panel",
                &make_style(Color::from_rgba(0.02, 0.01, 0.05, 0.88), C_BORDER, 1));
            bg.set_visible(false);
            layer.add_child(&bg);
            self.minimap_bg = Some(bg);

            let mut mr = TextureRect::new_alloc();
            place(&mr, 1.0, 0.0, MAP_X, MAP_Y, MAP_SZ, MAP_SZ);
            mr.set_expand_mode(godot::classes::texture_rect::ExpandMode::IGNORE_SIZE);
            mr.set_stretch_mode(godot::classes::texture_rect::StretchMode::SCALE);
            mr.set_texture_filter(godot::classes::canvas_item::TextureFilter::NEAREST);
            mr.set_visible(false);
            layer.add_child(&mr);
            self.minimap_rect = Some(mr);

            // точка игрока
            let mut dot = Panel::new_alloc();
            dot.set_size(Vector2::new(6.0, 6.0));
            dot.add_theme_stylebox_override("panel",
                &make_style(C_PINK, Color::TRANSPARENT_BLACK, 0));
            dot.set_visible(false);
            layer.add_child(&dot);
            self.minimap_dot = Some(dot);
        }

        // экран смерти
        {
            let mut dp = Panel::new_alloc();
            dp.set_anchors_preset(godot::classes::control::LayoutPreset::FULL_RECT);
            dp.add_theme_stylebox_override("panel",
                &make_style(Color::from_rgba(0.3, 0.0, 0.0, 0.88), Color::TRANSPARENT_BLACK, 0));
            dp.set_visible(false);

            let mut lbl = Label::new_alloc();
            lbl.set_text(t("msg_died", lang));
            place(&lbl, 0.5, 0.5, 0.0, HUD_H * 0.4, HUD_W, 60.0);
            lbl.set_horizontal_alignment(HorizontalAlignment::CENTER);
            lbl.add_theme_font_size_override("font_size", 56);
            lbl.add_theme_color_override("font_color", C_RED);
            dp.add_child(&lbl);

            let mut sub = Label::new_alloc();
            sub.set_text("E — вернуться в хаб (−25% золота)");
            place(&sub, 0.5, 0.5, 0.0, HUD_H * 0.4 + 70.0, HUD_W, 30.0);
            sub.set_horizontal_alignment(HorizontalAlignment::CENTER);
            sub.add_theme_font_size_override("font_size", 18);
            sub.add_theme_color_override("font_color", C_DIM);
            dp.add_child(&sub);

            layer.add_child(&dp);
            self.dead_panel = Some(dp);
        }

        // экран паузы
        {
            let mut pp = Panel::new_alloc();
            pp.set_anchors_preset(godot::classes::control::LayoutPreset::FULL_RECT);
            pp.add_theme_stylebox_override("panel",
                &make_style(Color::from_rgba(0.02, 0.01, 0.05, 0.82), Color::TRANSPARENT_BLACK, 0));
            pp.set_visible(false);

            let mut title = Label::new_alloc();
            title.set_text("ПАУЗА");
            place(&title, 0.5, 0.5, 0.0, HUD_H * 0.36, HUD_W, 60.0);
            title.set_horizontal_alignment(HorizontalAlignment::CENTER);
            title.add_theme_font_size_override("font_size", 52);
            title.add_theme_color_override("font_color", C_PINK);
            pp.add_child(&title);

            let mut lines = Label::new_alloc();
            lines.set_text("[ 1 / Esc ]  Продолжить\n\n[ 2 ]  Выйти в главное меню");
            place(&lines, 0.5, 0.5, 0.0, HUD_H * 0.36 + 90.0, HUD_W, 120.0);
            lines.set_horizontal_alignment(HorizontalAlignment::CENTER);
            lines.add_theme_font_size_override("font_size", 22);
            lines.add_theme_color_override("font_color", C_MAIN);
            pp.add_child(&lines);

            let mut note = Label::new_alloc();
            note.set_text("Прогресс сохраняется автоматически");
            place(&note, 0.5, 0.5, 0.0, HUD_H * 0.36 + 230.0, HUD_W, 30.0);
            note.set_horizontal_alignment(HorizontalAlignment::CENTER);
            note.add_theme_font_size_override("font_size", 14);
            note.add_theme_color_override("font_color", C_DIM);
            pp.add_child(&note);

            layer.add_child(&pp);
            self.pause_panel = Some(pp);
        }
    }

    /// Обновить AtlasTexture под текущее оружие.
    pub(super) fn refresh_weapon_sheet(&mut self) {
        let def = weapon_def(self.arsenal.current);
        let Some(tex) = self.cache.get(&def.sheet) else { return };
        let mut at = AtlasTexture::new_gd();
        at.set_atlas(&tex);
        at.set_region(Rect2::new(Vector2::ZERO, Vector2::new(FRAME_W, def.frame_h)));
        if let Some(ref mut wr) = self.weapon_rect {
            wr.set_texture(&at);
            // масштаб: высота на экране пропорциональна высоте кадра
            let k = 6.0;
            let w = FRAME_W * k;
            let h = def.frame_h * k;
            place(wr, 0.5, 1.0, HUD_W * 0.5 - w * 0.5, HUD_H - h, w, h);
            wr.set_visible(true);
        }
        self.weapon_atlas = Some(at);
        self.weapon_anim = WeaponAnim::Switch(0.22);
        self.set_weapon_frame(def.idle_frames[0]);
        if let Some(ref mut wl) = self.weapon_label {
            wl.set_text(&format!("[{}] {}  ({})", def.id.slot() + 1, def.name_ru, def.dmg_type.name_ru()));
        }
    }

    pub(super) fn set_weapon_frame(&mut self, frame: usize) {
        let def = weapon_def(self.arsenal.current);
        if let Some(ref mut at) = self.weapon_atlas {
            at.set_region(Rect2::new(
                Vector2::new(frame as f32 * FRAME_W, 0.0),
                Vector2::new(FRAME_W, def.frame_h),
            ));
        }
    }

    pub(super) fn tick_weapon_anim(&mut self, dt: f32) {
        if self.mode != Mode::Explore && self.mode != Mode::Dialogue { return; }
        let def = weapon_def(self.arsenal.current);
        self.anim_timer += dt;

        match self.weapon_anim {
            WeaponAnim::Fire(i) => {
                let frame_time = 1.0 / def.fire_fps;
                if self.anim_timer >= frame_time {
                    self.anim_timer = 0.0;
                    let next = i + 1;
                    if next < def.fire_frames.len() {
                        self.weapon_anim = WeaponAnim::Fire(next);
                        self.set_weapon_frame(def.fire_frames[next]);
                    } else {
                        self.weapon_anim = WeaponAnim::Idle;
                        self.set_weapon_frame(def.idle_frames[0]);
                    }
                }
            }
            WeaponAnim::Switch(t) => {
                let t2 = t - dt;
                if t2 <= 0.0 {
                    self.weapon_anim = WeaponAnim::Idle;
                } else {
                    self.weapon_anim = WeaponAnim::Switch(t2);
                }
            }
            WeaponAnim::Idle => {
                if def.idle_frames.len() > 1 && self.anim_timer >= 0.16 {
                    self.anim_timer = 0.0;
                    self.idle_frame = (self.idle_frame + 1) % def.idle_frames.len();
                    self.set_weapon_frame(def.idle_frames[self.idle_frame]);
                }
            }
        }

        // позиция: бо́б при ходьбе + провал при смене
        let moving = self.player.as_ref()
            .and_then(|p| p.clone().try_cast::<Player>().ok())
            .map(|p| p.bind().moving)
            .unwrap_or(false);
        let k = 6.0;
        let h = def.frame_h * k;
        let w = FRAME_W * k;
        let bob = if moving { (self.game_time * 9.0).sin() * 10.0 } else { (self.game_time * 2.0).sin() * 3.0 };
        let dip = match self.weapon_anim {
            WeaponAnim::Switch(t) => (t / 0.22) * 240.0,
            _ => 0.0,
        };
        if let Some(ref mut wr) = self.weapon_rect {
            wr.set_position(Vector2::new(
                HUD_W * 0.5 - w * 0.5 + if moving { (self.game_time * 4.5).sin() * 14.0 } else { 0.0 },
                HUD_H - h + 10.0 + bob + dip,
            ));
        }
    }

    pub(super) fn update_loc_label(&mut self) {
        let text = match self.loc {
            Loc::World => self.world_name.clone(),
            Loc::Dungeon => format!("{} — глубина {}", self.dungeon_name, self.dungeon_depth),
        };
        if let Some(ref mut l) = self.loc_label { l.set_text(&text); }
    }
}
