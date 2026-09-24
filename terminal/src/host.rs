//! Own exactly the child we started; never kill a server found on a port.
use crate::options::Options;
use anyhow::{bail, Context, Result};
use std::{
    fs::{self, File},
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub struct Host {
    child: Child,
    status: PathBuf,
}

fn quiet(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let _ = command;
}

impl Host {
    pub fn start(options: &Options) -> Result<(Self, String, String)> {
        fs::create_dir_all(&options.data_dir)?;
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let exe_dir = std::env::current_exe()?
            .parent()
            .context("Нет каталога executable")?
            .to_path_buf();
        let name = if cfg!(windows) {
            "oh-server.exe"
        } else {
            "oh-server"
        };
        let binary = options.server_bin.clone().unwrap_or_else(|| {
            [
                exe_dir.join(name),
                exe_dir.join("server").join(name),
                root.join("server").join(name),
            ]
            .into_iter()
            .find(|p| p.is_file())
            .unwrap_or_else(|| root.join("server").join(name))
        });
        let core = options.core_wasm.clone().unwrap_or_else(|| {
            [
                exe_dir.join("core.wasm"),
                exe_dir.join("openheart_core.wasm"),
                binary.with_file_name("core.wasm"),
                root.join("target/wasm32-unknown-unknown/release/openheart_core.wasm"),
            ]
            .into_iter()
            .find(|p| p.is_file())
            .unwrap_or_else(|| {
                root.join("target/wasm32-unknown-unknown/release/openheart_core.wasm")
            })
        });
        if !binary.is_file() && options.server_bin.is_none() {
            anyhow::ensure!(
                root.join("server/go.mod").is_file(),
                "Положите oh-server рядом с клиентом или задайте --server-bin"
            );
            eprintln!("Собираю Go-сервер…");
            let mut build = Command::new("go");
            build
                .current_dir(root.join("server"))
                .args(["build", "-o"])
                .arg(&binary)
                .arg("./cmd/oh-server");
            quiet(&mut build);
            anyhow::ensure!(build.status()?.success(), "Не удалось собрать Go-сервер");
        }
        if !core.is_file() && options.core_wasm.is_none() {
            eprintln!("Собираю core.wasm (нужен rustup target add wasm32-unknown-unknown)…");
            let mut build = Command::new("cargo");
            build.current_dir(&root).args([
                "build",
                "-p",
                "openheart-core",
                "--target",
                "wasm32-unknown-unknown",
                "--features",
                "abi",
                "--release",
            ]);
            quiet(&mut build);
            anyhow::ensure!(build.status()?.success(), "Не удалось собрать core.wasm");
        }
        let binary = binary
            .canonicalize()
            .context("Не найден oh-server; укажите --server-bin")?;
        let core = core
            .canonicalize()
            .context("Не найден core.wasm; укажите --core-wasm")?;
        // SQLite's URI handling does not accept Rust's Windows extended path prefix.
        let data = portable_path(options.data_dir.canonicalize()?);
        let status = data.join(format!(
            "host-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        let log_path = data.join("server.log");
        let log = File::create(&log_path)?;
        let mut command = Command::new(binary);
        command
            .current_dir(&data)
            .args([
                "-embedded",
                "-listen",
                &options.listen,
                "-auth",
                if options.login.is_some() {
                    "local"
                } else {
                    "open"
                },
                "-preset",
                &options.preset,
                "-world",
                &options.world,
                "-depth",
                &options.depth.to_string(),
                "-public=false",
                "-relay=false",
            ])
            .arg("-presets")
            .arg(options.presets.canonicalize()?)
            .arg("-core")
            .arg(core)
            .arg("-db")
            .arg(data.join("server.db"))
            .arg("-status")
            .arg(&status)
            .arg("-idle-exit")
            .arg("2m")
            .stdin(Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log);
        if let Some(master) = &options.master {
            command.args(["-party", "-master", master]);
        }
        quiet(&mut command);
        let child = command.spawn().context("Не удалось запустить oh-server")?;
        let mut host = Self { child, status };
        let deadline = Instant::now() + Duration::from_secs(30);
        while Instant::now() < deadline {
            if let Some(exit) = host.child.try_wait()? {
                let detail = fs::read_to_string(&log_path).unwrap_or_default();
                bail!(
                    "oh-server завершился ({exit}); лог: {}\n{}",
                    log_path.display(),
                    detail
                        .lines()
                        .rev()
                        .take(6)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect::<Vec<_>>()
                        .join("\n")
                );
            }
            if let Ok(text) = fs::read_to_string(&host.status) {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                    if value["ready"].as_bool() == Some(true) {
                        let listen = value["listen"].as_str().unwrap_or(&options.listen);
                        let addr: std::net::SocketAddr =
                            listen.parse().context("Неверный адрес в status сервера")?;
                        let ip = if addr.ip().is_unspecified() {
                            std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)
                        } else {
                            addr.ip()
                        };
                        let url = format!("ws://{}/ws", std::net::SocketAddr::new(ip, addr.port()));
                        let code = value["code"].as_str().unwrap_or("").to_string();
                        return Ok((host, url, code));
                    }
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
        bail!("Сервер не готов за 30 секунд; лог: {}", log_path.display())
    }
}

fn portable_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let text = path.to_string_lossy();
        if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{rest}"));
        }
        if let Some(rest) = text.strip_prefix(r"\\?\") {
            return PathBuf::from(rest);
        }
    }
    path
}

impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_file(&self.status);
    }
}
