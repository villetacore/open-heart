//! Встроенная локализация RU / EN.

pub fn t(key: &'static str, lang: &str) -> &'static str {
    if lang == "en" { en(key) } else { ru(key) }
}

fn ru(k: &'static str) -> &'static str {
    match k {
        // Главное меню
        "menu_title"       => "OpenHeart",
        "menu_new"         => "Новая игра",
        "menu_continue"    => "Продолжить",
        "menu_settings"    => "Настройки",
        "menu_quit"        => "Выход",
        // Настройки
        "set_title"        => "НАСТРОЙКИ",
        "set_lang"         => "Язык",
        "set_volume"       => "Громкость мастер",
        "set_sens"         => "Чувствительность мыши",
        "set_back"         => "← Назад",
        "set_lang_ru"      => "Русский",
        "set_lang_en"      => "English",
        // HUD
        "hud_hp"           => "HP",
        "hud_interact"     => "[ E ] — поговорить",
        "hud_shoot"        => "[ ЛКМ ] — выстрел",
        "hud_pickup"       => "[ E ] — подобрать",
        "hud_inv_empty"    => "Инвентарь пуст",
        "hud_gold"         => "Зол.",
        "hud_quests"       => "КВЕСТЫ",
        "hud_no_quests"    => "Нет активных квестов",
        // Инвентарь
        "inv_title"        => "ИНВЕНТАРЬ",
        "inv_use"          => "[ E ] использовать",
        "inv_close"        => "[ I ] закрыть",
        // Игровые сообщения
        "msg_hit"          => "Попал!",
        "msg_miss"         => "Промах",
        "msg_enemy_dead"   => "Враг убит",
        "msg_picked_up"    => "Подобрано",
        "msg_healed"       => "Восстановлено HP",
        "msg_saved"        => "Игра сохранена",
        "msg_no_save"      => "Нет сохранений",
        "msg_died"         => "Вы погибли",
        _ => k,
    }
}

fn en(k: &'static str) -> &'static str {
    match k {
        "menu_title"       => "OpenHeart",
        "menu_new"         => "New Game",
        "menu_continue"    => "Continue",
        "menu_settings"    => "Settings",
        "menu_quit"        => "Quit",
        "set_title"        => "SETTINGS",
        "set_lang"         => "Language",
        "set_volume"       => "Master Volume",
        "set_sens"         => "Mouse Sensitivity",
        "set_back"         => "← Back",
        "set_lang_ru"      => "Русский",
        "set_lang_en"      => "English",
        "hud_hp"           => "HP",
        "hud_interact"     => "[ E ] — talk",
        "hud_shoot"        => "[ LMB ] — shoot",
        "hud_pickup"       => "[ E ] — pick up",
        "hud_inv_empty"    => "Inventory empty",
        "hud_gold"         => "Gold",
        "hud_quests"       => "QUESTS",
        "hud_no_quests"    => "No active quests",
        "inv_title"        => "INVENTORY",
        "inv_use"          => "[ E ] use",
        "inv_close"        => "[ I ] close",
        "msg_hit"          => "Hit!",
        "msg_miss"         => "Miss",
        "msg_enemy_dead"   => "Enemy killed",
        "msg_picked_up"    => "Picked up",
        "msg_healed"       => "HP restored",
        "msg_saved"        => "Game saved",
        "msg_no_save"      => "No save found",
        "msg_died"         => "You died",
        _ => k,
    }
}
