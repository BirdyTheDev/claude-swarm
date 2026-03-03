use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "claude-swarm")]
#[command(about = "Terminal multi-agent Claude orchestrator")]
#[command(version)]
pub struct CliArgs {
    /// Path to swarm configuration file
    #[arg(short, long, default_value = "swarm.toml")]
    pub config: PathBuf,

    /// TUI tick rate in milliseconds
    #[arg(long, default_value_t = 250)]
    pub tick_rate: u64,

    /// Only spawn specific agents (comma-separated names)
    #[arg(long, value_delimiter = ',')]
    pub agents: Option<Vec<String>>,

    /// Send an initial prompt to the lead agent
    #[arg(short, long)]
    pub prompt: Option<String>,

    /// Log file path
    #[arg(long, default_value = "claude-swarm.log")]
    pub log_file: PathBuf,

    /// Log level
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Open visible Terminal.app windows for each agent (macOS only)
    #[arg(long, default_value_t = false)]
    pub visible_terminals: bool,
}
