//! Разбор ответов мастера: список серверов и пати-коды.
//!
//! Здесь нет движка — только URL, JSON и решение «пустят нас сюда или нет».
//! Сетевые запросы делает узел меню через `HTTPRequest`, а всё, что можно
//! проверить тестами, живёт тут.
//!
//! Формат ответов — `server/internal/registry` и `docs/MULTIPLAYER.md §8`.

use serde::Deserialize;

/// Запись сервера из `GET /v1/servers`.
#[derive(Clone, Debug, Deserialize)]
pub struct ServerEntry {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub motd: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub players: i64,
    #[serde(default)]
    pub max_players: i64,
    #[serde(default)]
    pub preset_id: String,
    #[serde(default)]
    pub content_hash: String,
    #[serde(default)]
    pub game_version: String,
    #[serde(default)]
    pub protocol: i32,
    #[serde(default)]
    pub passworded: bool,
    #[serde(default)]
    pub auth_mode: String,
    #[serde(default)]
    pub endpoint: String,
    /// Сервер за релеем: адрес выдал мастер, а не сам сервер.
    #[serde(default)]
    pub relayed: bool,
}

/// Почему на сервер нельзя зайти этой сборкой.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Blocker {
    /// Другая версия протокола: сообщения просто не сойдутся.
    Protocol,
    /// Другой набор данных пресета — сервер сам не пустит (`reject: content`).
    Content,
    /// Другой пресет: даже при совпадении хеша это другая игра.
    Preset,
    /// Мест нет.
    Full,
}

impl Blocker {
    pub fn text(self, lang: &str) -> &'static str {
        let en = lang == "en";
        match self {
            Blocker::Protocol => {
                if en {
                    "other protocol"
                } else {
                    "другой протокол"
                }
            }
            Blocker::Content => {
                if en {
                    "other data"
                } else {
                    "другие данные"
                }
            }
            Blocker::Preset => {
                if en {
                    "other preset"
                } else {
                    "другой пресет"
                }
            }
            Blocker::Full => {
                if en {
                    "no slots"
                } else {
                    "мест нет"
                }
            }
        }
    }
}

impl ServerEntry {
    /// Что мешает зайти. `None` — можно играть.
    ///
    /// Проверка нужна до подключения: иначе игрок жмёт на строку, ловит
    /// `reject` и не понимает, почему («сервер сломан»).
    pub fn blocker(&self, preset: &str, content_hash: &str) -> Option<Blocker> {
        if self.protocol != openheart_core::protocol::VERSION {
            return Some(Blocker::Protocol);
        }
        if !self.preset_id.is_empty() && self.preset_id != preset {
            return Some(Blocker::Preset);
        }
        // Пустой хеш у сервера означает «пресет не найден, сверка отключена»;
        // такому серверу верим на слово.
        if !self.content_hash.is_empty()
            && !content_hash.is_empty()
            && self.content_hash != content_hash
        {
            return Some(Blocker::Content);
        }
        if self.max_players > 0 && self.players >= self.max_players {
            return Some(Blocker::Full);
        }
        None
    }

    /// Строка списка: имя, заполненность и метки.
    ///
    /// `game_version` — наша версия: чужую показываем, только если она другая.
    pub fn row(&self, preset: &str, content_hash: &str, game_version: &str, lang: &str) -> String {
        let name = if self.name.is_empty() {
            "???"
        } else {
            &self.name
        };
        let mut row = format!("{name}   {}/{}", self.players, self.max_players);
        if !self.region.is_empty() {
            row.push_str(&format!("   [{}]", self.region));
        }
        if self.passworded {
            row.push_str("   \u{1f512}");
        }
        // Вход по аккаунту мастера: без него на сервер не попасть, и знать об
        // этом надо до клика.
        if self.auth_mode == "master" {
            row.push_str(if lang == "en" { "   account" } else { "   аккаунт" });
        }
        // Волна — сервер за релеем: пинг будет выше, чем у прямого.
        if self.relayed {
            row.push_str("   ~");
        }
        if !self.game_version.is_empty() && self.game_version != game_version {
            row.push_str(&format!("   v{}", self.game_version));
        }
        if let Some(blocker) = self.blocker(preset, content_hash) {
            row.push_str(&format!("   \u{2014} {}", blocker.text(lang)));
        } else if !self.motd.is_empty() {
            row.push_str(&format!("   \u{b7} {}", trim_motd(&self.motd)));
        }
        row
    }
}

/// Строка приветствия сервера — чужой текст: режем по длине, чтобы она не
/// выдавила из строки всё остальное.
fn trim_motd(motd: &str) -> String {
    const LIMIT: usize = 48;
    let clean: String = motd
        .chars()
        .filter(|c| !c.is_control())
        .take(LIMIT + 1)
        .collect();
    if clean.chars().count() > LIMIT {
        let short: String = clean.chars().take(LIMIT).collect();
        format!("{short}\u{2026}")
    } else {
        clean
    }
}

/// Разобрать ответ `GET /v1/servers` и разложить по-человечески.
///
/// Совместимые сверху и по заполненности: живой сервер интереснее пустого,
/// но забитый под завязку — уже нет.
pub fn parse_list(body: &str, preset: &str, content_hash: &str) -> Result<Vec<ServerEntry>, String> {
    let mut list: Vec<ServerEntry> =
        serde_json::from_str(body).map_err(|error| error.to_string())?;
    list.retain(|entry| !entry.endpoint.is_empty());
    list.sort_by(|a, b| {
        let key = |entry: &ServerEntry| {
            let blocked = entry.blocker(preset, content_hash).is_some();
            let full = entry.max_players > 0 && entry.players >= entry.max_players;
            (blocked, full, -entry.players)
        };
        key(a).cmp(&key(b))
    });
    Ok(list)
}

/// Адрес списка серверов. Фильтры отдаём мастеру: он умеет их применять сам,
/// и по сети приезжает меньше лишнего.
pub fn list_url(master: &str, preset: &str, content_hash: &str, only_compatible: bool) -> String {
    let mut url = format!(
        "{}/v1/servers?protocol={}",
        master.trim_end_matches('/'),
        openheart_core::protocol::VERSION
    );
    if only_compatible {
        if !preset.is_empty() {
            url.push_str(&format!("&preset={}", escape(preset)));
        }
        if !content_hash.is_empty() {
            url.push_str(&format!("&content_hash={}", escape(content_hash)));
        }
    }
    url
}

/// Адрес разбора пати-кода.
pub fn party_url(master: &str, code: &str) -> String {
    format!(
        "{}/v1/party/{}",
        master.trim_end_matches('/'),
        escape(&normalize_code(code))
    )
}

/// Пати-код игрок вводит с голоса: регистр и пробелы значения не имеют.
pub fn normalize_code(code: &str) -> String {
    code.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// Ответ `GET /v1/party/{code}`.
#[derive(Deserialize)]
struct PartyReply {
    #[serde(default)]
    endpoint: String,
}

/// Достать адрес сервера из ответа на пати-код.
pub fn parse_party(body: &str) -> Result<String, String> {
    let reply: PartyReply = serde_json::from_str(body).map_err(|error| error.to_string())?;
    if reply.endpoint.is_empty() {
        return Err("мастер не вернул адрес".into());
    }
    Ok(reply.endpoint)
}

/// Экранирование того немногого, что может попасть в query: коды и хеши —
/// ascii, но чужой мастер мог прислать что угодно.
fn escape(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
                c.to_string()
            } else {
                format!("%{:02X}", c as u32 & 0xFF)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "abc123";

    fn entry(json: &str) -> ServerEntry {
        serde_json::from_str(json).unwrap()
    }

    fn ok_json() -> String {
        format!(
            r#"{{"server_id":"s1","name":"Дом","players":2,"max_players":8,
                 "preset_id":"core","content_hash":"{HASH}","protocol":{},"endpoint":"ws://a/ws"}}"#,
            openheart_core::protocol::VERSION
        )
    }

    #[test]
    fn compatible_server_has_no_blocker() {
        assert_eq!(entry(&ok_json()).blocker("core", HASH), None);
    }

    #[test]
    fn wrong_protocol_or_data_is_blocked() {
        let other_protocol = ok_json().replace(
            &format!("\"protocol\":{}", openheart_core::protocol::VERSION),
            "\"protocol\":999",
        );
        assert_eq!(
            entry(&other_protocol).blocker("core", HASH),
            Some(Blocker::Protocol)
        );

        let other_hash = ok_json().replace(HASH, "deadbeef");
        assert_eq!(
            entry(&other_hash).blocker("core", HASH),
            Some(Blocker::Content)
        );

        let other_preset = ok_json().replace("\"core\"", "\"arena\"");
        assert_eq!(
            entry(&other_preset).blocker("core", HASH),
            Some(Blocker::Preset)
        );
    }

    // Сервер без пресета в анонсе — это старый или чужой мастер: не повод
    // прятать сервер, игрок сам решит.
    #[test]
    fn missing_hash_is_not_a_blocker() {
        let no_hash = ok_json().replace(&format!("\"content_hash\":\"{HASH}\""), "\"x\":0");
        assert_eq!(entry(&no_hash).blocker("core", HASH), None);
    }

    #[test]
    fn full_server_is_blocked() {
        let full = ok_json().replace("\"players\":2", "\"players\":8");
        assert_eq!(entry(&full).blocker("core", HASH), Some(Blocker::Full));
    }

    #[test]
    fn list_puts_playable_servers_first() {
        let body = format!(
            "[{},{},{}]",
            ok_json()
                .replace("\"name\":\"Дом\"", "\"name\":\"Чужой\"")
                .replace(HASH, "deadbeef"),
            ok_json().replace("\"name\":\"Дом\"", "\"name\":\"Пустой\"").replace("\"players\":2", "\"players\":0"),
            ok_json(),
        );
        let list = parse_list(&body, "core", HASH).unwrap();
        let names: Vec<&str> = list.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["Дом", "Пустой", "Чужой"]);
    }

    // Без endpoint строка бесполезна: нажать на неё некуда.
    #[test]
    fn entries_without_endpoint_are_dropped() {
        let body = format!("[{}]", ok_json().replace("ws://a/ws", ""));
        assert!(parse_list(&body, "core", HASH).unwrap().is_empty());
    }

    #[test]
    fn broken_json_is_an_error_not_a_panic() {
        assert!(parse_list("{не json}", "core", HASH).is_err());
    }

    #[test]
    fn party_code_is_forgiving() {
        assert_eq!(normalize_code(" k7x2-m9 "), "K7X2M9");
        assert_eq!(
            party_url("https://m.example.com/", "k7x 2m9"),
            "https://m.example.com/v1/party/K7X2M9"
        );
    }

    #[test]
    fn party_reply_gives_endpoint() {
        assert_eq!(
            parse_party(r#"{"endpoint":"ws://m/j/tok","server_id":"s1"}"#).unwrap(),
            "ws://m/j/tok"
        );
        assert!(parse_party(r#"{"error":"нет такого кода"}"#).is_err());
    }

    #[test]
    fn list_url_carries_filters_only_when_asked() {
        let strict = list_url("http://m", "core", HASH, true);
        assert!(strict.contains("preset=core") && strict.contains(&format!("content_hash={HASH}")));
        let loose = list_url("http://m", "core", HASH, false);
        assert!(!loose.contains("preset=") && !loose.contains("content_hash="));
    }

    #[test]
    fn row_marks_relayed_and_blocked() {
        let relayed = ok_json().replace("\"players\":2", "\"players\":2,\"relayed\":true");
        let row = entry(&relayed).row("core", HASH, "0.1.0-dev", "ru");
        assert!(row.contains("Дом") && row.contains("2/8") && row.contains('~'));

        let stale = ok_json().replace(HASH, "deadbeef");
        assert!(entry(&stale)
            .row("core", HASH, "0.1.0-dev", "ru")
            .contains("другие данные"));
    }

    // Приветствие сервера пишет чужой человек: в строку меню оно должно
    // приходить обрезанным и без управляющих символов.
    #[test]
    fn motd_is_trimmed_and_cleaned() {
        let long = "x".repeat(200);
        let json = ok_json().replace("\"players\":2", &format!("\"motd\":\"{long}\",\"players\":2"));
        let row = entry(&json).row("core", HASH, "0.1.0-dev", "ru");
        assert!(row.chars().count() < 120, "строка слишком длинная: {row}");
        assert_eq!(trim_motd("a\nb"), "ab");
    }

    #[test]
    fn row_shows_foreign_version_and_account_requirement() {
        let json = ok_json().replace(
            "\"players\":2",
            "\"game_version\":\"9.9\",\"auth_mode\":\"master\",\"players\":2",
        );
        let row = entry(&json).row("core", HASH, "0.1.0-dev", "ru");
        assert!(row.contains("v9.9") && row.contains("аккаунт"), "{row}");
    }
}
