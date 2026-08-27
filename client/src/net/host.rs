//! Игра с другом: клиент поднимает игровой сервер дочерним процессом.
//!
//! Второй реализации хоста нет — это тот же `oh-server`, что стоит на дедиках
//! (docs/MULTIPLAYER.md §7). Клиент только запускает его, ждёт файл состояния
//! и показывает пати-код.
//!
//! Всё, что можно проверить без движка — сборка аргументов, поиск бинарника,
//! разбор состояния — лежит здесь; движковая часть в самом низу.

use godot::classes::{file_access::ModeFlags, FileAccess, Os, ProjectSettings};
use godot::prelude::*;
use serde::Deserialize;

/// Порт, на котором слушает поднятый нами сервер.
///
/// Фиксированный: хозяин заходит на него напрямую (`127.0.0.1`), а друзья — по
/// коду через релей, и знать этот порт им не нужно.
pub const LOCAL_PORT: u16 = 7788;

/// Сколько ждём файла состояния, прежде чем признать, что сервер не поднялся.
pub const START_TIMEOUT: f64 = 25.0;

/// Состояние поднятого сервера — то, что он сам про себя написал.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct PartyStatus {
    #[serde(default)]
    pub ready: bool,
    #[serde(default)]
    pub listen: String,
    #[serde(default)]
    pub endpoint: String,
    /// Шестизначный код, которым зовут друга.
    #[serde(default)]
    pub code: String,
    /// Почему кода нет. Играть при этом можно — просто одному.
    #[serde(default)]
    pub error: String,
}

impl PartyStatus {
    /// Адрес, по которому подключается сам хозяин.
    ///
    /// Всегда локальный: гонять свой же трафик через релей мастера незачем.
    pub fn own_address(&self) -> String {
        let listen = if self.listen.is_empty() {
            format!("127.0.0.1:{LOCAL_PORT}")
        } else {
            self.listen.clone()
        };
        // Сервер слушает на 0.0.0.0 или 127.0.0.1 — подключаемся к себе.
        let port = listen.rsplit(':').next().unwrap_or("").to_string();
        format!("ws://127.0.0.1:{port}/ws")
    }
}

/// Разобрать файл состояния.
pub fn parse_status(text: &str) -> Result<PartyStatus, String> {
    serde_json::from_str(text).map_err(|error| error.to_string())
}

/// Аргументы запуска сервера для игры с другом.
pub fn host_args(master: &str, preset: &str, presets: &str, core: &str, status: &str) -> Vec<String> {
    let mut args = vec![
        "--embedded".to_string(),
        "-party".to_string(),
        "-listen".to_string(),
        format!("127.0.0.1:{LOCAL_PORT}"),
        "-preset".to_string(),
        preset.to_string(),
        "-presets".to_string(),
        presets.to_string(),
        "-auth".to_string(),
        "open".to_string(),
        "-status".to_string(),
        status.to_string(),
    ];
    if !master.is_empty() {
        args.push("-master".to_string());
        args.push(master.to_string());
    }
    if !core.is_empty() {
        args.push("-core".to_string());
        args.push(core.to_string());
    }
    args
}

/// Где искать `oh-server` рядом с игрой.
///
/// В собранной игре он лежит рядом с исполняемым файлом; при запуске из
/// исходников — в каталоге сервера. Порядок важен: своя сборка важнее чужой.
pub fn server_candidates(exe_dir: &str, project_dir: &str, windows: bool) -> Vec<String> {
    let name = if windows { "oh-server.exe" } else { "oh-server" };
    let exe_dir = exe_dir.trim_end_matches('/');
    let project_dir = project_dir.trim_end_matches('/');
    vec![
        format!("{exe_dir}/{name}"),
        format!("{exe_dir}/server/{name}"),
        format!("{project_dir}/../server/{name}"),
        format!("{project_dir}/../server/bin/{name}"),
    ]
}

// ── Движковая часть ──────────────────────────────────────────────────────────

/// Поднятый нами сервер.
pub struct PartyHost {
    pid: i32,
    status_path: String,
    waited: f64,
}

impl PartyHost {
    /// Запустить сервер. Ошибка — если бинарник не нашёлся или не стартовал.
    pub fn start(master: &str, preset: &str) -> Result<Self, String> {
        let binary = find_server().ok_or_else(|| "oh-server рядом с игрой не найден".to_string())?;

        let settings = ProjectSettings::singleton();
        let presets = settings.globalize_path("res://presets").to_string();
        let status_path = settings.globalize_path("user://party.json").to_string();
        // Файл прошлой игры не должен сойти за свежий.
        let _ = godot::classes::DirAccess::remove_absolute(&status_path);

        let core = core_next_to(&binary);
        let args = host_args(master, preset, &presets, &core, &status_path);

        let packed: PackedStringArray = args.iter().map(GString::from).collect();
        let pid = Os::singleton().create_process(&binary, &packed);
        if pid <= 0 {
            return Err(format!("не запустить {binary}"));
        }
        Ok(Self {
            pid,
            status_path,
            waited: 0.0,
        })
    }

    /// Проверить, не готов ли сервер. `Err` — время вышло.
    pub fn poll(&mut self, delta: f64) -> Result<Option<PartyStatus>, String> {
        if let Some(file) = FileAccess::open(&self.status_path, ModeFlags::READ) {
            let text = file.get_as_text().to_string();
            if !text.trim().is_empty() {
                let status = parse_status(&text)?;
                // Файл мог появиться раньше, чем сервер договорил: ждём флага.
                if status.ready {
                    return Ok(Some(status));
                }
            }
        }
        self.waited += delta;
        if self.waited > START_TIMEOUT {
            return Err("сервер не поднялся".to_string());
        }
        Ok(None)
    }

    /// Остановить сервер: игра с другом заканчивается вместе с игрой.
    pub fn stop(&self) {
        if self.pid > 0 {
            Os::singleton().kill(self.pid);
        }
    }
}

/// Первый существующий кандидат из списка.
fn find_server() -> Option<String> {
    let os = Os::singleton();
    let windows = os.get_name() == "Windows";
    let exe_dir = os
        .get_executable_path()
        .to_string()
        .rsplit_once(['/', '\\'])
        .map(|(dir, _)| dir.to_string())
        .unwrap_or_default();
    let project_dir = ProjectSettings::singleton()
        .globalize_path("res://")
        .to_string();

    server_candidates(&exe_dir, &project_dir, windows)
        .into_iter()
        .find(|path| FileAccess::file_exists(path))
}

/// `core.wasm` рядом с сервером: без него он играет на заглушке.
fn core_next_to(binary: &str) -> String {
    let dir = binary
        .rsplit_once(['/', '\\'])
        .map(|(dir, _)| dir.to_string())
        .unwrap_or_default();
    let candidate = format!("{dir}/core.wasm");
    if FileAccess::file_exists(&candidate) {
        candidate
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_gives_a_local_address() {
        let status = parse_status(r#"{"ready":true,"listen":"127.0.0.1:7788","code":"K7X2M9"}"#)
            .expect("разбор");
        assert!(status.ready);
        assert_eq!(status.code, "K7X2M9");
        assert_eq!(status.own_address(), "ws://127.0.0.1:7788/ws");
    }

    // Сервер мог слушать на всех интерфейсах — подключаться всё равно к себе.
    #[test]
    fn any_interface_still_means_loopback() {
        let status = parse_status(r#"{"ready":true,"listen":"0.0.0.0:9000"}"#).expect("разбор");
        assert_eq!(status.own_address(), "ws://127.0.0.1:9000/ws");
    }

    #[test]
    fn missing_listen_falls_back_to_the_default_port() {
        let status = PartyStatus::default();
        assert_eq!(
            status.own_address(),
            format!("ws://127.0.0.1:{LOCAL_PORT}/ws")
        );
    }

    // Без мастера кода не будет, но играть одному это не мешает: сервер
    // сообщает причину, а не молчит.
    #[test]
    fn error_is_carried_through() {
        let status = parse_status(r#"{"ready":true,"error":"мастер не отозвался"}"#).expect("разбор");
        assert!(status.code.is_empty());
        assert_eq!(status.error, "мастер не отозвался");
    }

    #[test]
    fn broken_status_is_an_error() {
        assert!(parse_status("не json").is_err());
    }

    #[test]
    fn args_carry_everything_the_server_needs() {
        let args = host_args("http://m", "core", "/p", "/c/core.wasm", "/s.json");
        let joined = args.join(" ");
        assert!(joined.contains("-party"), "{joined}");
        assert!(joined.contains("--embedded"), "{joined}");
        assert!(joined.contains("-master http://m"), "{joined}");
        assert!(joined.contains("-preset core"), "{joined}");
        assert!(joined.contains("-status /s.json"), "{joined}");
        assert!(joined.contains(&format!("127.0.0.1:{LOCAL_PORT}")), "{joined}");
    }

    // Без мастера сервер поднимается локально — просто без кода.
    #[test]
    fn args_skip_empty_options() {
        let args = host_args("", "core", "/p", "", "/s.json");
        let joined = args.join(" ");
        assert!(!joined.contains("-master"), "{joined}");
        assert!(!joined.contains("-core"), "{joined}");
    }

    #[test]
    fn candidates_look_next_to_the_game_first() {
        let list = server_candidates("/games/oh", "/src/game/", true);
        assert_eq!(list[0], "/games/oh/oh-server.exe");
        assert!(list.iter().any(|p| p.ends_with("/src/game/../server/oh-server.exe")));
        assert!(server_candidates("/games/oh", "/src/game", false)[0].ends_with("oh-server"));
    }
}
