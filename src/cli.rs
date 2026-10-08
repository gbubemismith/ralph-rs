use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::{Ok, Result};
use clap::{Parser, Subcommand};
use tokio::task::JoinHandle;

use crate::{
    config::Config, core::run_agentic_loop, display::run_display, events::channel,
    provider::anthropic::AnthrhopicProvider, tools::Registry,
};

#[derive(Debug, Parser)]
#[command(name = "ralph-rs", version = "0.1.0", author = "Gbubemi Smith")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run an AI development loop
    Run(RunArgs),
    /// Print version information
    Version,
}

#[derive(Debug, Parser)]
pub struct RunArgs {
    /// The task prompt
    pub prompt: String,

    /// Maximum loop iterations before stopping
    #[arg(short = 'm', long, default_value_t = 10)]
    pub max_iterations: u32,

    /// Maximum total runtime e.g 30m or 1hr
    #[arg(short = 't', long, default_value = "30m", value_parser = parse_duration)]
    pub timeout: Duration,

    /// LLM provider model, for now only claude models allowed
    #[arg(long, default_value = "claude-haiku-4-5")]
    pub model: String,

    ///Working directory for tool execution
    #[arg(long)]
    pub working_dir: Option<PathBuf>,

    ///Custom system prompt. Appended to Ralph's default unless `--system-prompt-mode replace`
    #[arg(long)]
    pub system_prompt: Option<String>,

    ///How to combine the custom system prompt with Ralph's default
    #[arg(long, default_value = "append", value_parser = ["append", "replace"])]
    pub system_prompt_mode: String,
    #[arg(long)]
    pub dry_run: bool,

    /// Anthropic API key (also reads ANTHROPIC_API_KEY env var).
    #[arg(long, env = "ANTHROPIC_API_KEY")]
    pub api_key: String,
}

impl Cli {
    pub async fn start_execution() -> Result<()> {
        let args = Self::parse();

        match args.command {
            Command::Version => {
                println!("ralph-rs {}", env!("CARGO_PKG_VERSION"));
            }
            Command::Run(args) => {
                let config = Config::try_from(args)?;
                Self::run(config).await?;
            }
        }

        Ok(())
    }

    pub async fn run(config: Config) -> Result<()> {
        let provider = Arc::new(AnthrhopicProvider::new(config.api_key.clone()));
        let registry = Arc::new(Registry::standard(Arc::from(config.working_dir.as_path())));
        let (tx, rx) = channel();
        let display_task: JoinHandle<()> = tokio::spawn(run_display(rx));

        let result = run_agentic_loop(
            provider,
            registry,
            config.system_prompt.clone(),
            config.model.clone(),
            config.prompt.clone(),
            tx,
        )
        .await;

        display_task.await?;

        result
    }
}

fn parse_duration(s: &str) -> Result<Duration, String> {
    humantime::parse_duration(s).map_err(|e| e.to_string())
}
