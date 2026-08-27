//! Встроенная локализация RU / EN.

pub fn t(key: &'static str, lang: &str) -> &'static str {
    if lang == "en" {
        en(key)
    } else {
        ru(key)
    }
}

fn ru(k: &'static str) -> &'static str {
    match k {
        // Главное меню
        "menu_title" => "OpenHeart",
        "menu_new" => "Новая игра",
        "menu_continue" => "Продолжить",
        "menu_subtitle" => "DOOM-style Action-RPG — открытый мир и процедурные данжи",
        "menu_preset" => "Пресет",
        "menu_controls" => {
            "WASD — движение  |  ЛКМ — выстрел  |  E — взаимодействие  |  I — инвентарь"
        }
        "menu_connect" => "Подключиться",
        // Крафт
        "craft_title" => "КРАФТ",
        "craft_close" => "C или Esc — закрыть  |  1–9 — сделать",
        "craft_empty" => "В этом пресете нет рецептов.",
        "craft_done" => "Готово:",
        "craft_bench" => "нужен верстак",
        // Кооператив
        "net_downed" => "Ты упал. Держись — товарищ поднимет.",
        "net_revived" => "Тебя подняли.",
        "net_out" => "Ты истёк кровью и выбыл из забега.",
        "net_wipe" => "Пати легла. Забег провален.",
        "loot_taken" => "Подобрано:",
        "menu_settings" => "Настройки",
        "menu_quit" => "Выход",
        // Подключение к серверу
        "net_title" => "ПОДКЛЮЧЕНИЕ К СЕРВЕРУ",
        "net_hint" => "Адрес сервера, например ws://127.0.0.1:7777/ws",
        "net_go" => "Играть на сервере",
        "net_master" => "Мастер-сервер: список игр и пати-коды",
        "net_refresh" => "Обновить список",
        "net_code" => "Код компании от друга",
        "net_code_go" => "Зайти по коду",
        "net_host" => "Создать игру с другом",
        "net_hosting" => "Поднимаю сервер…",
        "net_code_ready" => "Готово! Код для друга:",
        "net_code_failed" => "Сервер поднят, но кода нет",
        "net_host_local" => "Сервер поднят локально: друзья зайдут по адресу",
        "net_host_failed" => "Не поднять сервер",
        "net_loading" => "Спрашиваю мастера…",
        "net_master_empty" => "Укажи адрес мастера, чтобы видеть список игр",
        "net_master_down" => "Мастер не ответил",
        "net_empty" => "Открытых серверов нет",
        "net_found" => "Серверов",
        "net_no_code" => "Такого кода нет или он протух",
        "net_solo" => "Играть в одиночку",
        // Настройки
        "set_title" => "НАСТРОЙКИ",
        "set_lang" => "Язык",
        "set_diff" => "Сложность",
        "set_volume" => "Громкость мастер",
        "set_sens" => "Чувствительность мыши",
        "set_back" => "← Назад",
        "set_lang_ru" => "Русский",
        "set_lang_en" => "English",
        // HUD
        "hud_hp" => "HP",
        "hud_interact" => "[ E ] — поговорить",
        "hud_shoot" => "[ ЛКМ ] — выстрел",
        "hud_pickup" => "[ E ] — подобрать",
        "hud_inv_empty" => "Инвентарь пуст",
        "hud_gold" => "Зол.",
        "hud_quests" => "КВЕСТЫ",
        "hud_no_quests" => "Нет активных квестов",
        // Инвентарь
        "inv_title" => "ИНВЕНТАРЬ",
        "inv_use" => "[ E ] использовать",
        "inv_close" => "[ I ] закрыть",
        // Игровые сообщения
        "msg_hit" => "Попал!",
        "msg_miss" => "Промах",
        "msg_enemy_dead" => "Враг убит",
        "msg_picked_up" => "Подобрано",
        "msg_healed" => "Восстановлено HP",
        "msg_saved" => "Игра сохранена",
        "msg_no_save" => "Нет сохранений",
        "msg_died" => "Вы погибли",
        _ => k,
    }
}

fn en(k: &'static str) -> &'static str {
    match k {
        "menu_title" => "OpenHeart",
        "menu_new" => "New Game",
        "menu_continue" => "Continue",
        "menu_subtitle" => "DOOM-style Action-RPG — open world and procedural dungeons",
        "menu_preset" => "Preset",
        "menu_controls" => "WASD — move  |  LMB — fire  |  E — interact  |  I — inventory",
        "menu_connect" => "Connect",
        // Crafting
        "craft_title" => "CRAFTING",
        "craft_close" => "C or Esc — close  |  1–9 — craft",
        "craft_empty" => "This preset has no recipes.",
        "craft_done" => "Crafted:",
        "craft_bench" => "workbench required",
        // Co-op
        "net_downed" => "You are down. Hold on — a teammate can revive you.",
        "net_revived" => "You are back on your feet.",
        "net_out" => "You bled out and are out of the run.",
        "net_wipe" => "The party is down. Run failed.",
        "loot_taken" => "Picked up:",
        "menu_settings" => "Settings",
        "menu_quit" => "Quit",
        // Server connection
        "net_title" => "CONNECT TO SERVER",
        "net_hint" => "Server address, e.g. ws://127.0.0.1:7777/ws",
        "net_go" => "Play online",
        "net_master" => "Master server: game list and party codes",
        "net_refresh" => "Refresh list",
        "net_code" => "Party code from a friend",
        "net_code_go" => "Join by code",
        "net_host" => "Host a game",
        "net_hosting" => "Starting the server…",
        "net_code_ready" => "Ready! Party code:",
        "net_code_failed" => "Server is up, but there is no code",
        "net_host_local" => "Server is up locally: friends join by address",
        "net_host_failed" => "Could not start the server",
        "net_loading" => "Asking the master…",
        "net_master_empty" => "Set a master address to see the game list",
        "net_master_down" => "Master did not answer",
        "net_empty" => "No open servers",
        "net_found" => "Servers",
        "net_no_code" => "No such code, or it expired",
        "net_solo" => "Play solo",
        "set_title" => "SETTINGS",
        "set_lang" => "Language",
        "set_diff" => "Difficulty",
        "set_volume" => "Master Volume",
        "set_sens" => "Mouse Sensitivity",
        "set_back" => "← Back",
        "set_lang_ru" => "Русский",
        "set_lang_en" => "English",
        "hud_hp" => "HP",
        "hud_interact" => "[ E ] — talk",
        "hud_shoot" => "[ LMB ] — shoot",
        "hud_pickup" => "[ E ] — pick up",
        "hud_inv_empty" => "Inventory empty",
        "hud_gold" => "Gold",
        "hud_quests" => "QUESTS",
        "hud_no_quests" => "No active quests",
        "inv_title" => "INVENTORY",
        "inv_use" => "[ E ] use",
        "inv_close" => "[ I ] close",
        "msg_hit" => "Hit!",
        "msg_miss" => "Miss",
        "msg_enemy_dead" => "Enemy killed",
        "msg_picked_up" => "Picked up",
        "msg_healed" => "HP restored",
        "msg_saved" => "Game saved",
        "msg_no_save" => "No save found",
        "msg_died" => "You died",
        _ => k,
    }
}
