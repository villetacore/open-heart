use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "oh-terminal",
    about = "OpenHeart — терминальный клиент",
    version
)]
pub struct Options {
    /// Connect directly to a Go server (ws:// or wss://).
    #[arg(long, conflicts_with_all = ["host", "offline"])]
    pub server: Option<String>,
    /// Start a managed Go server, then connect to it.
    #[arg(long, conflicts_with = "offline")]
    pub host: bool,
    /// Run the shared Rust simulation in this process.
    #[arg(long)]
    pub offline: bool,
    #[arg(long, default_value = "player")]
    pub nick: String,
    #[arg(long, default_value = "core")]
    pub preset: String,
    #[arg(long, default_value = "game/presets")]
    pub presets: PathBuf,
    #[arg(long, default_value = "hub", value_parser = ["hub", "delve"])]
    pub world: String,
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..=100))]
    pub depth: u32,
    #[arg(long, default_value_t = 42)]
    pub seed: u64,
    /// 1 Berserker, 2 Assault, 3 Operator.
    #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u8).range(1..=3))]
    pub class: u8,
    #[arg(long, default_value = "127.0.0.1:7788")]
    pub listen: String,
    #[arg(long)]
    pub server_bin: Option<PathBuf>,
    #[arg(long)]
    pub core_wasm: Option<PathBuf>,
    /// Master URL for hosting a party through the existing relay.
    #[arg(long)]
    pub master: Option<String>,
    #[arg(long, default_value = ".openheart-terminal")]
    pub data_dir: PathBuf,
    #[arg(long)]
    pub login: Option<String>,
    #[arg(long)]
    pub register: bool,
    /// Run a bounded protocol/simulation smoke check without a terminal.
    #[arg(long)]
    pub headless: bool,
    #[arg(long, default_value_t = 3, value_parser = clap::value_parser!(u64).range(1..=3600))]
    pub seconds: u64,
}

impl Options {
    pub fn preset_base(&self) -> PathBuf {
        self.presets.join(&self.preset)
    }
}
