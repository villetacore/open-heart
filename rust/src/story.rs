//! Контент визуальной новеллы: сцены, диалоги, история.
//! Сеттинг: Riverside Academy — элитная закрытая школа с секретами.

use crate::dialogue::{Choice, Effect, Line, Scene};
use crate::character::StatKind::*;
use crate::game_state::GameState;

fn l(sp: &str, port: &str, tx: &str) -> Line { Line::new(sp, port, tx) }
fn n(tx: &str) -> Line { Line::narr(tx) }
fn c(tx: &str, eff: Vec<Effect>) -> Choice { Choice::simple(tx, eff) }
fn cn(tx: &str, eff: Vec<Effect>, next: &str) -> Choice {
    let mut ch = Choice::simple(tx, eff);
    ch.next = Some(next.to_string());
    ch
}
fn cr(tx: &str, st: crate::character::StatKind, min: i32, eff: Vec<Effect>) -> Choice {
    Choice::req(tx, st, min, eff)
}
#[allow(dead_code)]
fn crn(tx: &str, st: crate::character::StatKind, min: i32, eff: Vec<Effect>, next: &str) -> Choice {
    let mut ch = Choice::req(tx, st, min, eff);
    ch.next = Some(next.to_string());
    ch
}

pub fn get_scene(id: &str, state: &GameState) -> Option<Scene> {
    match id {
        // Виктор
        "intro_victor"      => Some(intro_victor(state)),
        "victor_chat_2"     => Some(victor_chat_2(state)),
        "victor_quest_check"=> Some(victor_quest_check(state)),
        "victor_chat_end"   => Some(victor_chat_end()),
        // Ms. Вейл
        "meet_vale"         => Some(meet_vale(state)),
        "vale_class_chat"   => Some(vale_class_chat()),
        "vale_office_1"     => Some(vale_office_1(state)),
        "vale_office_2"     => Some(vale_office_2(state)),
        "vale_office_deep"  => Some(vale_office_deep(state)),
        // Елена
        "first_elena"       => Some(first_elena()),
        "elena_library_1"   => Some(elena_library_1(state)),
        "elena_chat_2"      => Some(elena_chat_2(state)),
        "elena_quest_check" => Some(elena_quest_check(state)),
        "elena_chat_end"    => Some(elena_chat_end()),
        // София
        "meet_sofia"        => Some(meet_sofia(state)),
        "sofia_chat"        => Some(sofia_chat(state)),
        "sofia_chat_3"      => Some(sofia_chat_3(state)),
        // Охранник
        "meet_guard"         => Some(meet_guard(state)),
        "guard_quest_offer"  => Some(guard_quest_offer()),
        "guard_quest_check"  => Some(guard_quest_check(state)),
        "guard_quest_end"    => Some(guard_quest_end()),
        // Торговец
        "meet_merchant"  => Some(meet_merchant()),
        "merchant_shop"  => Some(merchant_shop()),
        "merchant_again" => Some(merchant_again()),
        // Учёный
        "meet_scientist"           => Some(meet_scientist()),
        "scientist_quest_offer"    => Some(scientist_quest_offer(state)),
        "scientist_quest_check"    => Some(scientist_quest_check(state)),
        "scientist_quest_end"      => Some(scientist_quest_end()),
        // Незнакомец
        "meet_stranger"  => Some(meet_stranger()),
        "stranger_again" => Some(stranger_again()),
        _ => None,
    }
}

// ═══════════════════════════════════════════════════════════
// ВИКТОР
// ═══════════════════════════════════════════════════════════

fn intro_victor(state: &GameState) -> Scene {
    let name = &state.stats.name;
    Scene {
        id: "intro_victor".into(),
        lines: vec![
            n("Раннее утро. Луч солнца режет шторы. Рядом с тобой, на соседней кровати, сидит парень с растрёпанными волосами и широкой ухмылкой."),
            l("Виктор", "victor", "Живой? Отлично. Я уже думал, что мне дали соседа-зомби."),
            l("Виктор", "victor", &format!("Виктор Лян. Второй год в Riverside. А ты — {name}, стипендиат. Видел твоё дело в списках.")),
            l("Виктор", "victor", "Это место красивое и правила строгие. Но всё интереснее, чем кажется. Поверь мне."),
        ],
        choices: vec![
            c("«Рад познакомиться, Виктор.»", vec![
                Effect::Flag("met_victor".into()),
                Effect::Rel("victor".into(), 10),
                Effect::Flash("Виктор тебе понравился.".into()),
            ]),
            c("«Расскажи про школу подробнее.»", vec![
                Effect::Flag("met_victor".into()),
                Effect::Flag("victor_info_requested".into()),
                Effect::Rel("victor".into(), 15),
                Effect::Flash("«О, с удовольствием» — его глаза оживились.".into()),
            ]),
            c("«Тебе не следовало читать моё дело.»", vec![
                Effect::Flag("met_victor".into()),
                Effect::Rel("victor".into(), 5),
                Effect::Stat(Willpower, 1),
                Effect::Flash("«Справедливо,» — он поднял руки. «Мир?»".into()),
            ]),
        ],
    }
}

fn victor_chat_2(_state: &GameState) -> Scene {
    Scene {
        id: "victor_chat_2".into(),
        lines: vec![
            n("Виктор выглядит серьёзнее обычного. Он проверяет, не слушает ли кто."),
            l("Виктор", "victor", "Слушай. Я кое-что заметил. По ночам в восточном крыле горит свет — там склад, он должен быть закрыт."),
            l("Виктор", "victor", "Я пытался разведать — но меня слишком хорошо знают. Тебя — нет."),
            l("Виктор", "victor", "Не прошу лезть в опасность. Просто узнай — что там хранят. За это у меня есть кое-что ценное."),
        ],
        choices: vec![
            cn("«Договорились. Я разберусь.»", vec![
                Effect::Flag("victor_quest_given".into()),
                Effect::Quest {
                    id: "east_wing".into(),
                    title: "Тайна восточного крыла".into(),
                    desc: "Виктор просит выяснить, что происходит в закрытом складе. Найди ключ и загляни внутрь.".into(),
                },
                Effect::Rel("victor".into(), 10),
                Effect::Flash("Новый квест: «Тайна восточного крыла».".into()),
            ], "victor_quest_check"),
            c("«Пока не готов влезать в это.»", vec![
                Effect::Rel("victor".into(), -5),
                Effect::Flash("«Понимаю. Подумай, я не тороплю.»".into()),
            ]),
            cr("«Что именно там, как думаешь?» [INT 7+]", Intelligence, 7, vec![
                Effect::Flag("victor_quest_given".into()),
                Effect::Quest {
                    id: "east_wing".into(),
                    title: "Тайна восточного крыла".into(),
                    desc: "Виктор подозревает незаконные исследования. Найди ключ — ответы могут изменить всё.".into(),
                },
                Effect::Rel("victor".into(), 15),
                Effect::Stat(Intelligence, 1),
                Effect::Flash("Виктор: «Незаконные исследования... или хуже.» Квест получен.".into()),
            ]),
        ],
    }
}

fn victor_quest_check(state: &GameState) -> Scene {
    if state.inventory.has("key") {
        Scene {
            id: "victor_quest_check".into(),
            lines: vec![
                l("Виктор", "victor", "Подожди — у тебя есть ключ? Где ты его нашёл?!"),
                l("Виктор", "victor", "Это и есть ключ от восточного крыла. Я искал его три недели. Ты... невероятный человек."),
            ],
            choices: vec![
                c("Отдать ключ Виктору.", vec![
                    Effect::Flag("victor_quest_done".into()),
                    Effect::QuestDone("east_wing".into()),
                    Effect::Rel("victor".into(), 25),
                    Effect::Gold(50),
                    Effect::Stat(Reputation, 2),
                    Effect::Flash("Квест выполнен! +50 золота, +25 к отношениям с Виктором.".into()),
                ]),
                c("«Сам использую. Проверю что там.»", vec![
                    Effect::Stat(Intelligence, 2),
                    Effect::Flash("Виктор кивнул, хотя и выглядел разочарованным.".into()),
                ]),
            ],
        }
    } else {
        Scene {
            id: "victor_quest_check".into(),
            lines: vec![
                l("Виктор", "victor", "Ну как? Нашёл что-нибудь?"),
                l("Виктор", "victor", "Мне говорили, что ключ где-то на этаже. Осмотрись внимательнее."),
            ],
            choices: vec![
                c("«Продолжаю искать.»", vec![
                    Effect::Flash("Виктор ждёт. Найди ключ.".into()),
                ]),
                c("«Опасно там?»", vec![
                    Effect::Stat(Intelligence, 1),
                    Effect::Flash("«Не уверен,» — честно ответил он.".into()),
                ]),
            ],
        }
    }
}

fn victor_chat_end() -> Scene {
    Scene {
        id: "victor_chat_end".into(),
        lines: vec![
            n("Виктор выглядит одновременно взволнованным и довольным."),
            l("Виктор", "victor", "То, что ты сделал — это важнее, чем кажется. В Riverside скрывают много всего."),
            l("Виктор", "victor", "Я рад, что ты мой сосед. Серьёзно."),
        ],
        choices: vec![
            c("«Взаимно.»", vec![
                Effect::Stat(Reputation, 1),
                Effect::Flash("Виктор улыбается по-настоящему.".into()),
            ]),
        ],
    }
}

// ═══════════════════════════════════════════════════════════
// MS. ВЕЙЛ
// ═══════════════════════════════════════════════════════════

fn meet_vale(_state: &GameState) -> Scene {
    Scene {
        id: "meet_vale".into(),
        lines: vec![
            n("Перед первым уроком психологии ты задержался у доски. Молодая женщина — лет двадцати семи, в строгом сером пиджаке — обернулась раньше, чем ты успел что-то сказать."),
            l("Ms. Вейл", "vale", "Новенький. Стипендиат, верно? Я Мс. Вейл — ваш преподаватель психологии и консультант."),
            l("Ms. Вейл", "vale", "В Riverside принято знать людей вокруг себя. Это не просто школа — это среда. *Долгая пауза.* Что вас интересует в психологии?"),
        ],
        choices: vec![
            c("«Меня интересует, как люди скрывают своё истинное лицо.»", vec![
                Effect::Flag("met_vale".into()),
                Effect::Rel("vale".into(), 15),
                Effect::Stat(Charm, 1),
                Effect::Flash("Ms. Вейл чуть прищурилась. Ей интересно.".into()),
            ]),
            c("«Честно говоря, это обязательный предмет.»", vec![
                Effect::Flag("met_vale".into()),
                Effect::Rel("vale".into(), 5),
                Effect::Flash("Ms. Вейл улыбнулась уголком рта.".into()),
            ]),
            cr("«Меня интересуют скрытые механизмы влияния в закрытых сообществах.» [CHR 7+]", Charm, 7, vec![
                Effect::Flag("met_vale".into()),
                Effect::Flag("impressed_vale_first".into()),
                Effect::Rel("vale".into(), 22),
                Effect::Stat(Charm, 1),
                Effect::Flash("Что-то изменилось в её глазах. «Именно здесь — именно это.» +22 к отношениям.".into()),
            ]),
        ],
    }
}

fn vale_class_chat() -> Scene {
    Scene {
        id: "vale_class_chat".into(),
        lines: vec![
            n("Все вышли из класса. Ms. Вейл собирает бумаги. Ты остался."),
            l("Ms. Вейл", "vale", "Ты всегда задерживаешься после урока или это специально?"),
            l("Ms. Вейл", "vale", "Я заметила, как ты работаешь с материалом. Нестандартно. Это хорошо... или опасно. Зависит от того, куда ты это направишь."),
            l("Ms. Вейл", "vale", "Если хочешь разобраться глубже — мой кабинет открыт. Запись не обязательна."),
        ],
        choices: vec![
            c("«Куда вы думаете я это направляю?»", vec![
                Effect::Flag("vale_chat_1_done".into()),
                Effect::Rel("vale".into(), 10),
                Effect::Stat(Charm, 1),
                Effect::Flash("Ms. Вейл промолчала — но взгляд сказал больше.".into()),
            ]),
            c("«Приду. Интересно.»", vec![
                Effect::Flag("vale_chat_1_done".into()),
                Effect::Rel("vale".into(), 12),
                Effect::Flash("«Жду,» — сказала она без лишних слов.".into()),
            ]),
            c("«Просто стараюсь учиться.» — скромно.", vec![
                Effect::Flag("vale_chat_1_done".into()),
                Effect::Rel("vale".into(), 6),
                Effect::Flash("«Конечно,» — сказала она. Тон неоднозначный.".into()),
            ]),
        ],
    }
}

fn vale_office_1(_state: &GameState) -> Scene {
    Scene {
        id: "vale_office_1".into(),
        lines: vec![
            n("Кабинет небольшой. Полки с книгами по поведенческой психологии. Запах кофе. Ms. Вейл сидит напротив, нога на ногу, с блокнотом на коленях."),
            l("Ms. Вейл", "vale", "Расположитесь. Первая сессия — просто разговор. Никаких правильных ответов."),
            l("Ms. Вейл", "vale", "Расскажите мне: что вас привело именно в Riverside? Не официальный ответ. Настоящий."),
        ],
        choices: vec![
            c("«Я хотел сбежать от привычного окружения.»", vec![
                Effect::Rel("vale".into(), 10),
                Effect::Flash("Ms. Вейл что-то записала. «Честно» — произнесла она тихо.".into()),
            ]),
            c("«Мне нужна была новая среда для роста.»", vec![
                Effect::Rel("vale".into(), 7),
                Effect::Stat(Reputation, 1),
                Effect::Flash("Она кивнула. Профессиональный ответ её не удивил.".into()),
            ]),
            cr("«Я слышал о вашей репутации как исследователя.» [CHR 8+]", Charm, 8, vec![
                Effect::Rel("vale".into(), 15),
                Effect::Flag("intrigued_vale_office".into()),
                Effect::Flash("Она медленно подняла взгляд от блокнота. «Интересно. Продолжайте.»".into()),
            ]),
        ],
    }
}

fn vale_office_2(state: &GameState) -> Scene {
    let extra = if state.has("impressed_vale_first") {
        " Ты тот, кто удивляет её с первой встречи."
    } else { "" };
    Scene {
        id: "vale_office_2".into(),
        lines: vec![
            n(&format!("Второй раз в её кабинете уже по-другому.{extra} Ms. Вейл встречает тебя без формальности — просто кивает на кресло.")),
            l("Ms. Вейл", "vale", "Мне кажется, ты привыкаешь к этому месту быстрее, чем большинство."),
            l("Ms. Вейл", "vale", "Riverside любит тех, кто адаптируется. *Короткая пауза.* А тебе самому здесь комфортно?"),
            l("Ms. Вейл", "vale", "Не отвечай как на экзамене. Настоящий ответ."),
        ],
        choices: vec![
            c("«Некоторые вещи стали неожиданно... интересными.»", vec![
                Effect::Rel("vale".into(), 12),
                Effect::Stat(Charm, 1),
                Effect::Flash("Она чуть улыбнулась. «Хороший ответ.»".into()),
            ]),
            c("«Привыкаю. Не всегда легко.»", vec![
                Effect::Rel("vale".into(), 10),
                Effect::Stat(Willpower, 1),
                Effect::Flash("«Это честно,» — сказала она.".into()),
            ]),
            cr("«Интереснее всего здесь — вы.» [CHR 9+]", Charm, 9, vec![
                Effect::Rel("vale".into(), 18),
                Effect::Flag("vale_bold_move".into()),
                Effect::Stat(Charm, 1),
                Effect::Flash("Долгая пауза. Она сделала пометку и ничего не сказала. Но не отвела взгляд.".into()),
            ]),
        ],
    }
}

fn vale_office_deep(_state: &GameState) -> Scene {
    Scene {
        id: "vale_office_deep".into(),
        lines: vec![
            n("Вы оба знаете, что это уже не просто консультации. Атмосфера в кабинете другая — тише, напряжённее. Она сидит ближе, чем обычно."),
            l("Ms. Вейл", "vale", "Я должна сказать тебе кое-что. Не как консультант."),
            l("Ms. Вейл", "vale", "Я пришла в Riverside не только преподавать. Здесь идёт исследование, которое администрация официально отрицает. И ты... попадаешь в него."),
            l("Ms. Вейл", "vale", "*Тихо.* Это нарушает несколько правил сразу. Но ты же сам говорил, что тебя интересуют скрытые механизмы. *Пауза.* Что ты собираешься делать с этим?"),
        ],
        choices: vec![
            c("«Я думаю... мы оба знаем ответ.»", vec![
                Effect::Rel("vale".into(), 20),
                Effect::Flag("vale_deep_moment".into()),
                Effect::Stat(Charm, 2),
                Effect::Quest {
                    id: "vale_research".into(),
                    title: "Исследование Вейл".into(),
                    desc: "Ms. Вейл занимается тайным исследованием в Riverside. Выясни подробности.".into(),
                },
                Effect::Flash("Долгая пауза. Она не отвела взгляд. Новый квест получен.".into()),
            ]),
            cr("«Вы умнее меня. Что вы хотите, чтобы я сделал?» [WIL 10+]", Willpower, 10, vec![
                Effect::Rel("vale".into(), 25),
                Effect::Flag("vale_deep_moment".into()),
                Effect::Flag("player_bold".into()),
                Effect::Stat(Charm, 2),
                Effect::Stat(Willpower, 1),
                Effect::Quest {
                    id: "vale_research".into(),
                    title: "Исследование Вейл".into(),
                    desc: "Ms. Вейл занимается тайным исследованием. Она доверяет тебе. Разберись что происходит.".into(),
                },
                Effect::Flash("Её глаза расширились. Потом улыбка. «Хорошо.» Квест получен.".into()),
            ]),
            c("«Мне нужно подумать об этом.» — отступить.", vec![
                Effect::Stat(Willpower, 2),
                Effect::Flash("Она кивнула. «Мудрое решение. Двери открыты.»".into()),
            ]),
        ],
    }
}

// ═══════════════════════════════════════════════════════════
// ЕЛЕНА
// ═══════════════════════════════════════════════════════════

fn first_elena() -> Scene {
    Scene {
        id: "first_elena".into(),
        lines: vec![
            n("У дальнего окна стоит девушка — стопка учебников, выражение «не подходи». Потом она замечает, что ты смотришь."),
            l("Елена", "elena", "Ты что-то ищешь?"),
            l("Елена", "elena", "Новенький. Понятно. *Пауза.* Елена. Лучший результат на потоке. Можешь не запоминать — я не собираюсь быть дружелюбной."),
        ],
        choices: vec![
            c("«Зря. Я мог бы быть полезен.»", vec![
                Effect::Flag("met_elena".into()),
                Effect::Rel("elena".into(), 8),
                Effect::Stat(Charm, 1),
                Effect::Flash("Елена на секунду удивилась. Потом отвернулась.".into()),
            ]),
            cr("«Топ потока? Тогда нам есть о чём поговорить.» [INT 7+]", Intelligence, 7, vec![
                Effect::Flag("met_elena".into()),
                Effect::Rel("elena".into(), 14),
                Effect::Flash("Что-то в её взгляде изменилось. «Может быть.»".into()),
            ]),
            c("«Как скажешь.» — уйти.", vec![
                Effect::Flag("met_elena".into()),
                Effect::Flash("Ты ушёл. Она смотрела тебе вслед чуть дольше, чем нужно.".into()),
            ]),
        ],
    }
}

fn elena_library_1(state: &GameState) -> Scene {
    let opener = if state.rel("elena") >= 12 {
        "Елена поднимает взгляд раньше, чем ты подошёл. Как будто ждала."
    } else {
        "Елена делает вид, что не замечает. Но страницу не перевернула уже пять минут."
    };
    Scene {
        id: "elena_library_1".into(),
        lines: vec![
            n(opener),
            l("Елена", "elena", "Снова ты. *Вздыхает, но книгу закрывает.* Садись, раз уж пришёл."),
            l("Елена", "elena", "Я не привыкла к тому, что кто-то... обращает на меня внимание. Не из-за рейтинга."),
            l("Елена", "elena", "Это раздражает. *Тихо.* Но не так сильно, как должно бы."),
        ],
        choices: vec![
            c("«Может, это потому что ты больше, чем твой рейтинг?»", vec![
                Effect::Flag("elena_lib_1".into()),
                Effect::Rel("elena".into(), 14),
                Effect::Stat(Charm, 1),
                Effect::Flash("Она долго молчала. «Ты странный.»".into()),
            ]),
            c("«Давай поработаем вместе. Мне правда нужна помощь с материалом.»", vec![
                Effect::Flag("elena_lib_1".into()),
                Effect::Rel("elena".into(), 10),
                Effect::Stat(Intelligence, 1),
                Effect::Flash("Елена открыла книгу снова. «Хорошо. Смотри сюда.»".into()),
            ]),
        ],
    }
}

fn elena_chat_2(state: &GameState) -> Scene {
    let quest_note = if state.inventory.has("key") {
        " Кстати — у тебя ключ? Я потеряла его неделю назад."
    } else {
        ""
    };
    Scene {
        id: "elena_chat_2".into(),
        lines: vec![
            n("Елена ждала тебя. Или делает вид, что не ждала."),
            l("Елена", "elena", "Я думала о том, что ты сказал."),
            l("Елена", "elena", &format!("Мне нужна помощь с чем-то важным.{quest_note} Ты умеешь хранить тайны?")),
        ],
        choices: vec![
            cn("«Да. Что случилось?»", vec![
                Effect::Flag("elena_quest_given".into()),
                Effect::Quest {
                    id: "elena_notes".into(),
                    title: "Потерянные записи".into(),
                    desc: "Елена потеряла важные записи. Найди ключ — он откроет архив где они хранятся.".into(),
                },
                Effect::Rel("elena".into(), 12),
                Effect::Flash("Новый квест: «Потерянные записи».".into()),
            ], "elena_quest_check"),
            c("«Смотря что за тайна.»", vec![
                Effect::Stat(Willpower, 1),
                Effect::Flash("«Логично,» — она поджала губы. «Тогда пока не скажу.»".into()),
            ]),
        ],
    }
}

fn elena_quest_check(state: &GameState) -> Scene {
    if state.inventory.has("key") {
        Scene {
            id: "elena_quest_check".into(),
            lines: vec![
                l("Елена", "elena", "Это мой ключ. *Голос дрогнул.* Где ты его нашёл?"),
                l("Елена", "elena", "В этом ключе — всё. Мои исследования, три года работы. Если бы он попал не в те руки..."),
                l("Елена", "elena", "*Тихо.* Спасибо. Серьёзно."),
            ],
            choices: vec![
                c("Отдать ключ Елене.", vec![
                    Effect::Flag("elena_quest_done".into()),
                    Effect::QuestDone("elena_notes".into()),
                    Effect::Rel("elena".into(), 30),
                    Effect::Stat(Reputation, 2),
                    Effect::Stat(Intelligence, 1),
                    Effect::Flash("Квест выполнен! +30 к отношениям с Еленой.".into()),
                ]),
                cr("«Что в этих записях?» — сначала узнать. [INT 9+]", Intelligence, 9, vec![
                    Effect::Flag("elena_quest_done".into()),
                    Effect::QuestDone("elena_notes".into()),
                    Effect::Rel("elena".into(), 25),
                    Effect::Stat(Intelligence, 2),
                    Effect::Flash("Елена рассказала. Это изменило твоё понимание Riverside.".into()),
                ]),
            ],
        }
    } else {
        Scene {
            id: "elena_quest_check".into(),
            lines: vec![
                l("Елена", "elena", "Нашёл ключ?"),
                l("Елена", "elena", "Маленький, серебристый. Я теряю голову без него."),
            ],
            choices: vec![
                c("«Ищу. Найду.»", vec![
                    Effect::Rel("elena".into(), 3),
                    Effect::Flash("Елена кивнула. Ей спокойнее.".into()),
                ]),
            ],
        }
    }
}

fn elena_chat_end() -> Scene {
    Scene {
        id: "elena_chat_end".into(),
        lines: vec![
            n("Елена выглядит иначе — мягче. Граница, которую она строила, всё ещё есть, но стала тоньше."),
            l("Елена", "elena", "Знаешь, я думала, что в Riverside невозможно кому-то доверять. Ты доказал, что я ошибалась."),
            l("Елена", "elena", "Это... хорошо. Немного пугает. Но хорошо."),
        ],
        choices: vec![
            c("«Ты заслуживаешь большего, чем стена вокруг себя.»", vec![
                Effect::Rel("elena".into(), 10),
                Effect::Stat(Charm, 1),
                Effect::Flash("Она отвернулась — но ты видел улыбку.".into()),
            ]),
            c("«Взаимно. Ты лучше, чем кажешься снаружи.»", vec![
                Effect::Rel("elena".into(), 8),
                Effect::Flash("«Замолчи,» — сказала она. Но без злости.".into()),
            ]),
        ],
    }
}

// ═══════════════════════════════════════════════════════════
// СОФИЯ
// ═══════════════════════════════════════════════════════════

fn meet_sofia(_state: &GameState) -> Scene {
    Scene {
        id: "meet_sofia".into(),
        lines: vec![
            n("За центральным столом сидит группа — смех, телефоны, безупречный внешний вид. В центре — она. Блондинка, взгляд изучающий."),
            l("София", "sofia", "О. Стипендиат. *Обращается к компании, не к тебе.* Говорила же — нас разбавят."),
            l("София", "sofia", "*К тебе, наконец.* Ладно. Как тебя зовут? Шансы есть, если умеешь себя вести."),
        ],
        choices: vec![
            c("Представиться вежливо.", vec![
                Effect::Flag("met_sofia".into()),
                Effect::Rel("sofia".into(), 6),
                Effect::Stat(Reputation, 1),
                Effect::Flash("«Посмотрим,» — сказала она. Ни да ни нет.".into()),
            ]),
            cr("«Шансы на что именно?» — с усмешкой. [CHR 8+]", Charm, 8, vec![
                Effect::Flag("met_sofia".into()),
                Effect::Rel("sofia".into(), 15),
                Effect::Stat(Reputation, 2),
                Effect::Stat(Charm, 1),
                Effect::Flash("Она засмеялась первой. «Мне нравится. Садись.»".into()),
            ]),
            c("Проигнорировать высокомерие, спокойно ответить.", vec![
                Effect::Flag("met_sofia".into()),
                Effect::Rel("sofia".into(), 4),
                Effect::Stat(Willpower, 1),
                Effect::Flash("Она чуть подняла бровь. Ты её удивил.".into()),
            ]),
        ],
    }
}

fn sofia_chat(state: &GameState) -> Scene {
    let intimate = state.rel("sofia") >= 30;
    Scene {
        id: "sofia_chat".into(),
        lines: vec![
            n(if intimate { "За стойкой, подальше от толпы. София говорит тише, чем обычно." }
              else { "Она нашла тебя первой. Редкость." }),
            l("София", "sofia", if intimate {
                "Знаешь, ты единственный, кто не пытается что-то от меня получить. Это... неожиданно."
            } else {
                "Ты держишься лучше, чем я думала. Riverside ломает таких, как ты."
            }),
            l("София", "sofia", "Вся эта репутация, компания, имидж — это работа. Настоящий вопрос: зачем?"),
        ],
        choices: vec![
            cn("«Почему ты вообще так живёшь?»", vec![
                Effect::Rel("sofia".into(), 10),
                Effect::Stat(Intelligence, 1),
                Effect::Flash("«Хороший вопрос. Ответ позже.»".into()),
            ], "sofia_chat_3"),
            c("«Все что-то хотят. Я просто честен об этом.»", vec![
                Effect::Rel("sofia".into(), 12),
                Effect::Stat(Charm, 1),
                Effect::Flag("sofia_deep_done".into()),
                Effect::Flash("Она посмотрела на тебя иначе. «Редкость.»".into()),
            ]),
            c("«Ты не такая, какой кажешься снаружи.»", vec![
                Effect::Rel("sofia".into(), 8),
                Effect::Flag("sofia_deep_done".into()),
                Effect::Flash("«Никто не такой,» — ответила она. Тихо.".into()),
            ]),
        ],
    }
}

// ═══════════════════════════════════════════════════════════
// ОХРАННИК
// ═══════════════════════════════════════════════════════════

fn meet_guard(state: &GameState) -> Scene {
    let name = &state.stats.name;
    Scene {
        id: "meet_guard".into(),
        lines: vec![
            n("У восточного прохода стоит крупный мужчина в форме. Взгляд острый, рука на поясе."),
            l("Охранник", "guard", &format!("Стой. Это закрытая зона, {name}. Видел тебя раньше — ты стипендиат.")),
            l("Охранник", "guard", "Я Дрейк. Охрана Riverside уже восемь лет. Правила просты: не суйся куда не нужно и не создавай проблем."),
            l("Охранник", "guard", "...Хотя есть одно дело, с которым мне нужна помощь. Неофициально."),
        ],
        choices: vec![
            c("«Слушаю внимательно.»", vec![
                Effect::Flag("met_guard".into()),
                Effect::Rel("guard".into(), 10),
                Effect::Flash("Дрейк слегка расслабился.".into()),
            ]),
            c("«Проблем создавать не планирую.»", vec![
                Effect::Flag("met_guard".into()),
                Effect::Rel("guard".into(), 6),
                Effect::Stat(crate::character::StatKind::Reputation, 1),
                Effect::Flash("«Слова. Посмотрим на дела.» Он кивнул.".into()),
            ]),
            c("«Что за дело?» — сразу к сути.", vec![
                Effect::Flag("met_guard".into()),
                Effect::Rel("guard".into(), 8),
                Effect::Stat(crate::character::StatKind::Intelligence, 1),
                Effect::Flash("Охраннику понравилась прямолинейность.".into()),
            ]),
        ],
    }
}

fn guard_quest_offer() -> Scene {
    Scene {
        id: "guard_quest_offer".into(),
        lines: vec![
            n("Дрейк оглядывается — убедиться, что никто не слышит."),
            l("Охранник", "guard", "Кто-то ворует из хранилища на южной арене. Небольшие суммы — расходники, еда, золото."),
            l("Охранник", "guard", "Я знаю, кто это делает, но у меня нет доказательств. А ты... можешь появляться где угодно, не вызывая подозрений."),
            l("Охранник", "guard", "Принеси мне три золотые монеты из тайника под алтарём — это докажет, что ход открыт. Я не прошу воровать. Просто проверить."),
        ],
        choices: vec![
            c("«Договорились.»", vec![
                Effect::Flag("guard_quest_given".into()),
                Effect::Quest {
                    id: "guard_theft".into(),
                    title: "Кражи на арене".into(),
                    desc: "Дрейк просит найти тайник под алтарём на южной арене и принести монеты как доказательство.".into(),
                },
                Effect::Rel("guard".into(), 12),
                Effect::Flash("Новый квест: «Кражи на арене».".into()),
            ]),
            c("«Это звучит как подставить кого-то.»", vec![
                Effect::Rel("guard".into(), -5),
                Effect::Stat(crate::character::StatKind::Willpower, 1),
                Effect::Flash("«Нет. Но понимаю подозрение,» — Дрейк насупился.".into()),
            ]),
        ],
    }
}

fn guard_quest_check(state: &GameState) -> Scene {
    if state.gold >= 3 {
        Scene {
            id: "guard_quest_check".into(),
            lines: vec![
                l("Охранник", "guard", "У тебя монеты. Значит, тайник реальный."),
                l("Охранник", "guard", "Теперь у меня есть всё, что нужно. Спасибо, парень."),
            ],
            choices: vec![
                c("Передать три монеты Дрейку.", vec![
                    Effect::Flag("guard_quest_done".into()),
                    Effect::QuestDone("guard_theft".into()),
                    Effect::Gold(-3),
                    Effect::Gold(80),
                    Effect::Rel("guard".into(), 30),
                    Effect::Stat(crate::character::StatKind::Reputation, 2),
                    Effect::Flash("Квест выполнен! +77 золота (сдача), +30 к отношениям с Дрейком.".into()),
                ]),
            ],
        }
    } else {
        Scene {
            id: "guard_quest_check".into(),
            lines: vec![
                l("Охранник", "guard", "Нашёл тайник?"),
                l("Охранник", "guard", "Три монеты под алтарём арены. Если тайника нет — значит кто-то уже почистил."),
            ],
            choices: vec![
                c("«Ещё ищу.»", vec![
                    Effect::Flash("Дрейк кивнул. Продолжай.".into()),
                ]),
            ],
        }
    }
}

fn guard_quest_end() -> Scene {
    Scene {
        id: "guard_quest_end".into(),
        lines: vec![
            n("Дрейк выглядит спокойнее. Задание закрыто."),
            l("Охранник", "guard", "Виновника поймали. Без шума, как я и хотел."),
            l("Охранник", "guard", "Ты толковый человек. Если понадоблюсь — я здесь."),
        ],
        choices: vec![
            c("«Рад был помочь.»", vec![
                Effect::Rel("guard".into(), 8),
                Effect::Flash("Дрейк пожал тебе руку. По-настоящему.".into()),
            ]),
        ],
    }
}

// ═══════════════════════════════════════════════════════════
// ТОРГОВЕЦ
// ═══════════════════════════════════════════════════════════

fn meet_merchant() -> Scene {
    Scene {
        id: "meet_merchant".into(),
        lines: vec![
            n("За прилавком в восточном рынке сидит пухлый человек с хитрыми глазами. На прилавке — всё подряд: ключи, склянки, монеты, свёртки."),
            l("Торговец", "merchant", "О, новый покупатель! Гаспар к вашим услугам. Торгую всем, что нужно — и кое-чем, чего нет нигде."),
            l("Торговец", "merchant", "В Riverside умеют закрывать глаза. А я умею открывать нужные двери. Взаимовыгодное сотрудничество, понимаете?"),
            l("Торговец", "merchant", "Чем могу быть полезен, юный авантюрист?"),
        ],
        choices: vec![
            c("«Что у вас есть?»", vec![
                Effect::Flag("met_merchant".into()),
                Effect::Rel("merchant".into(), 8),
                Effect::Flash("Гаспар широко улыбнулся. «Отличный вопрос!»".into()),
            ]),
            c("«Вы торгуете незаконными вещами?»", vec![
                Effect::Flag("met_merchant".into()),
                Effect::Rel("merchant".into(), 4),
                Effect::Stat(crate::character::StatKind::Willpower, 1),
                Effect::Flash("«Законными, законными! Просто... специфическими.»".into()),
            ]),
            c("«Мне нужны расходники.»", vec![
                Effect::Flag("met_merchant".into()),
                Effect::Rel("merchant".into(), 10),
                Effect::Flash("«О, деловой подход! Уважаю.»".into()),
            ]),
        ],
    }
}

fn merchant_shop() -> Scene {
    Scene {
        id: "merchant_shop".into(),
        lines: vec![
            l("Торговец", "merchant", "Специально для вас сегодня: энергетические напитки — 15 HP, всего 20 золотых за штуку."),
            l("Торговец", "merchant", "Также есть информация. Например — где найти кое-что ценное в архиве. Это бесплатно для хорошего клиента."),
            l("Торговец", "merchant", "*Тихо.* В архиве, под третьим стеллажом слева — там спрятан рубин. Без понятия, чей. Но стоит он хорошо."),
        ],
        choices: vec![
            c("«Куплю энергетик. 20 золота.»", vec![
                Effect::Flag("merchant_bought".into()),
                Effect::Gold(-20),
                Effect::Quest {
                    id: "merchant_trade".into(),
                    title: "Торговый путь".into(),
                    desc: "Гаспар намекнул о рубине в архиве. Стоит проверить третий стеллаж слева.".into(),
                },
                Effect::Rel("merchant".into(), 15),
                Effect::Flash("-20 золота. Энергетик в кармане + наводка на рубин в архиве.".into()),
            ]),
            c("«Пока только слушаю.»", vec![
                Effect::Flag("merchant_bought".into()),
                Effect::Rel("merchant".into(), 5),
                Effect::Flash("«Ну, приходите когда нужда прижмёт!»".into()),
            ]),
        ],
    }
}

fn merchant_again() -> Scene {
    Scene {
        id: "merchant_again".into(),
        lines: vec![
            l("Торговец", "merchant", "Снова вы! Всегда рад постоянным клиентам."),
            l("Торговец", "merchant", "Новинок пока нет, но если принесёте что-то интересное — обменяю по-честному."),
        ],
        choices: vec![
            c("«Договорились.»", vec![
                Effect::Rel("merchant".into(), 4),
                Effect::Flash("Гаспар кивнул с довольной миной.".into()),
            ]),
        ],
    }
}

// ═══════════════════════════════════════════════════════════
// УЧЁНЫЙ
// ═══════════════════════════════════════════════════════════

fn meet_scientist() -> Scene {
    Scene {
        id: "meet_scientist".into(),
        lines: vec![
            n("В западной лаборатории, среди мигающих приборов и рассыпанных бумаг, стоит взволнованный мужчина в белом халате."),
            l("Учёный", "scientist", "А, посетитель! Подождите... ах. *Снимает очки, протирает.* Меня зовут Профессор Кейн."),
            l("Учёный", "scientist", "Я занимаюсь изучением странных явлений в Riverside. Аномальные показатели, необъяснимые события... Вы, случайно, не замечали ничего необычного?"),
            l("Учёный", "scientist", "Мне нужен свежий взгляд. Вы новенький — идеальный наблюдатель."),
        ],
        choices: vec![
            c("«Расскажите подробнее.»", vec![
                Effect::Flag("met_scientist".into()),
                Effect::Rel("scientist".into(), 12),
                Effect::Stat(crate::character::StatKind::Intelligence, 1),
                Effect::Flash("Профессор Кейн оживился. «Наконец-то кто-то слушает!»".into()),
            ]),
            c("«Ничего необычного.» — осторожно.", vec![
                Effect::Flag("met_scientist".into()),
                Effect::Rel("scientist".into(), 6),
                Effect::Flash("«Конечно, конечно. Поначалу так у всех.»".into()),
            ]),
        ],
    }
}

fn scientist_quest_offer(state: &GameState) -> Scene {
    let extra = if state.rel("scientist") >= 15 {
        " Я уже немного вам доверяю, так что скажу прямо."
    } else { "" };
    Scene {
        id: "scientist_quest_offer".into(),
        lines: vec![
            l("Учёный", "scientist", &format!("Вот в чём дело.{extra} В северном крыле есть некто — зовут его просто Незнакомцем.")),
            l("Учёный", "scientist", "Он знает об аномалиях больше меня. Но он не говорит с исследователями. Только с теми, кто пришёл сам."),
            l("Учёный", "scientist", "Вы доберётесь до тронного зала и поговорите с ним? Мне нужно знать что он скажет."),
        ],
        choices: vec![
            c("«Попробую найти его.»", vec![
                Effect::Flag("scientist_quest_given".into()),
                Effect::Quest {
                    id: "find_stranger".into(),
                    title: "Незнакомец".into(),
                    desc: "Профессор Кейн просит найти Незнакомца в тронном зале и поговорить с ним.".into(),
                },
                Effect::Rel("scientist".into(), 14),
                Effect::Flash("Новый квест: «Незнакомец».".into()),
            ]),
            c("«Это звучит опасно.»", vec![
                Effect::Rel("scientist".into(), -3),
                Effect::Flash("«Всё интересное немного опасно,» — пробурчал он.".into()),
            ]),
        ],
    }
}

fn scientist_quest_check(state: &GameState) -> Scene {
    if state.has("stranger_talked") {
        Scene {
            id: "scientist_quest_check".into(),
            lines: vec![
                l("Учёный", "scientist", "Вы нашли его! Что он сказал?"),
                l("Учёный", "scientist", "*Слушает, кивает.* Да. Да, это подтверждает мою теорию. Riverside стоит на чём-то очень древнем."),
                l("Учёный", "scientist", "Вы... очень помогли. Возьмите это. Я считал, что мне понадобится, но вам нужнее."),
            ],
            choices: vec![
                c("Получить награду.", vec![
                    Effect::Flag("scientist_quest_done".into()),
                    Effect::QuestDone("find_stranger".into()),
                    Effect::Gold(100),
                    Effect::Rel("scientist".into(), 35),
                    Effect::Stat(crate::character::StatKind::Intelligence, 2),
                    Effect::Flash("Квест выполнен! +100 золота, +2 интеллект, +35 к отношениям.".into()),
                ]),
            ],
        }
    } else {
        Scene {
            id: "scientist_quest_check".into(),
            lines: vec![
                l("Учёный", "scientist", "Нашли Незнакомца?"),
                l("Учёный", "scientist", "Он в тронном зале на севере. Будьте осторожны — там патрулируют враги."),
            ],
            choices: vec![
                c("«Ещё ищу.»", vec![
                    Effect::Flash("Профессор Кейн нетерпеливо кивнул.".into()),
                ]),
            ],
        }
    }
}

fn scientist_quest_end() -> Scene {
    Scene {
        id: "scientist_quest_end".into(),
        lines: vec![
            n("Профессор Кейн выглядит помолодевшим. Что-то щёлкнуло в его теории."),
            l("Учёный", "scientist", "Мне нужно написать три статьи немедленно. Вы спасли несколько лет работы."),
            l("Учёный", "scientist", "Если вам когда-нибудь понадобится что-то знать о Riverside — приходите."),
        ],
        choices: vec![
            c("«Обязательно приду.»", vec![
                Effect::Rel("scientist".into(), 10),
                Effect::Flash("Учёный уже уткнулся в записи. Это его лучшая похвала.".into()),
            ]),
        ],
    }
}

// ═══════════════════════════════════════════════════════════
// НЕЗНАКОМЕЦ
// ═══════════════════════════════════════════════════════════

fn meet_stranger() -> Scene {
    Scene {
        id: "meet_stranger".into(),
        lines: vec![
            n("В дальнем конце тронного зала, почти в тени, стоит фигура в тёмном капюшоне. Когда ты подходишь — он оборачивается. Лицо наполовину скрыто, глаза светлые."),
            l("Незнакомец", "stranger", "Ты дошёл. Большинство не доходят."),
            l("Незнакомец", "stranger", "Riverside не просто школа. Она построена на месте, где сходятся линии. Не энергии — страха. Тысячи решений приняты здесь под давлением."),
            l("Незнакомец", "stranger", "Ты один из тех, кто чувствует это. Поэтому ты здесь, а не на поверхности."),
            l("Незнакомец", "stranger", "*Протягивает старый медальон.* Возьми. Тебе понадобится."),
        ],
        choices: vec![
            c("«Кто вы?»", vec![
                Effect::Flag("met_stranger".into()),
                Effect::Flag("stranger_talked".into()),
                Effect::Rel("stranger".into(), 15),
                Effect::Stat(crate::character::StatKind::Willpower, 2),
                Effect::Quest {
                    id: "stranger_secret".into(),
                    title: "Тайна Незнакомца".into(),
                    desc: "Незнакомец знает о Riverside больше всех. Выясни кто он.".into(),
                },
                Effect::Flash("Незнакомец улыбнулся. «Вопрос правильный.» Квест получен.".into()),
            ]),
            c("«Принять медальон молча.»", vec![
                Effect::Flag("met_stranger".into()),
                Effect::Flag("stranger_talked".into()),
                Effect::Rel("stranger".into(), 20),
                Effect::Stat(crate::character::StatKind::Charm, 2),
                Effect::Flash("«Молчание — тоже ответ,» — сказал он с одобрением.".into()),
            ]),
            c("«Что значит «линии»?»", vec![
                Effect::Flag("met_stranger".into()),
                Effect::Flag("stranger_talked".into()),
                Effect::Rel("stranger".into(), 12),
                Effect::Stat(crate::character::StatKind::Intelligence, 2),
                Effect::Quest {
                    id: "stranger_secret".into(),
                    title: "Тайна Незнакомца".into(),
                    desc: "Незнакомец говорил о «линиях страха». Разберись что это значит.".into(),
                },
                Effect::Flash("«Хороший вопрос. Лучший из трёх.» Новый квест.".into()),
            ]),
        ],
    }
}

fn stranger_again() -> Scene {
    Scene {
        id: "stranger_again".into(),
        lines: vec![
            n("Незнакомец стоит на том же месте. Как будто никуда и не уходил."),
            l("Незнакомец", "stranger", "Ты снова здесь. Значит, вопросы ещё есть."),
            l("Незнакомец", "stranger", "Хорошо. Ответы появятся сами — если смотреть в правильную сторону."),
        ],
        choices: vec![
            c("«Я найду ответы.»", vec![
                Effect::Rel("stranger".into(), 8),
                Effect::Stat(crate::character::StatKind::Willpower, 1),
                Effect::Flash("Незнакомец кивнул. Почти с уважением.".into()),
            ]),
            c("«Что такое Riverside на самом деле?»", vec![
                Effect::Rel("stranger".into(), 12),
                Effect::Stat(crate::character::StatKind::Intelligence, 1),
                Effect::Flash("«Место, где страх становится силой — или слабостью. Зависит от тебя.»".into()),
            ]),
        ],
    }
}

fn sofia_chat_3(state: &GameState) -> Scene {
    Scene {
        id: "sofia_chat_3".into(),
        lines: vec![
            n(if state.has("sofia_deep_done") {
                "Она нашла тебя сама. Впервые без компании."
            } else {
                "Немного позже. Она одна — и это редкость."
            }),
            l("София", "sofia", "Я отвечу на твой вопрос. Честно. Обещай, что не расскажешь."),
            l("София", "sofia", "Мой отец — спонсор Riverside. Я здесь не потому что хочу. Я здесь потому что должна. Всё это — роль."),
            l("София", "sofia", "*Тихо, смотря в окно.* Хочешь знать что страшно? Я уже не помню, где роль, а где я."),
        ],
        choices: vec![
            c("«Я вижу тебя. Не роль.»", vec![
                Effect::Flag("sofia_deep_done".into()),
                Effect::Rel("sofia".into(), 20),
                Effect::Stat(Charm, 2),
                Effect::Flash("Она не ответила. Но через минуту: «Спасибо.»".into()),
            ]),
            cr("«Что ты хочешь на самом деле?» [WIL 8+]", Willpower, 8, vec![
                Effect::Flag("sofia_deep_done".into()),
                Effect::Rel("sofia".into(), 25),
                Effect::Stat(Willpower, 1),
                Effect::Quest {
                    id: "sofia_freedom".into(),
                    title: "Путь Софии".into(),
                    desc: "София в ловушке семейных ожиданий. Помоги ей найти выход.".into(),
                },
                Effect::Flash("Долгая пауза. «Я хочу... выбрать сама.» Новый квест.".into()),
            ]),
            c("«Это не так страшно, как кажется.»", vec![
                Effect::Flag("sofia_deep_done".into()),
                Effect::Rel("sofia".into(), 12),
                Effect::Flash("«Легко говорить,» — но она улыбнулась.".into()),
            ]),
        ],
    }
}
