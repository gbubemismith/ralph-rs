use std::{path::PathBuf, time::Duration};

use anyhow::{Ok, Result};
use clap::{Parser, Subcommand};
use futures_util::StreamExt;
use tokio::task::JoinHandle;

use crate::{
    config::Config,
    display::run_display,
    events::{Event, EventSender, channel},
    provider::{
        Provider,
        anthropic::AnthrhopicProvider,
        types::{ChatRequest, ContentDelta, Message, StopReason, StreamEvent},
    },
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
        let (tx, rx) = channel();
        let display_task: JoinHandle<()> = tokio::spawn(run_display(rx));

        let result = drive_one_turn(&config, tx).await;
        display_task.await?;

        result
    }
}

async fn drive_one_turn(config: &Config, tx: EventSender) -> Result<()> {
    let provider = AnthrhopicProvider::new(config.api_key.clone());

    let request = ChatRequest {
        model: config.model.clone(),
        max_tokens: 4096,
        system: config.system_prompt.clone(),
        messages: vec![Message::user(config.prompt.clone())],
        tools: vec![],
        stream: true,
    };

    let _ = tx.send(Event::TurnStart {
        model: config.model.clone(),
    });

    let mut stream = provider.stream(request).await?;

    let mut stop_reason = StopReason::EndTurn;
    let mut input_tokens = 0u32;
    let mut output_tokens = 0u32;

    while let Some(event) = stream.next().await {
        match event? {
            StreamEvent::MessageStart { message } => {
                input_tokens = message.usage.input_tokens;
            }
            StreamEvent::ContentBlockStart { .. } => {}
            StreamEvent::ContentBlockDelta { delta, .. } => {
                if let ContentDelta::TextDelta { text } = delta {
                    let _ = tx.send(Event::TextDelta { text });
                }
            }
            StreamEvent::ContentBlockStop { .. } => {}
            StreamEvent::MessageDelta { delta, usage } => {
                if let Some(reason) = delta.stop_reason {
                    stop_reason = reason;
                }
                if let Some(u) = usage {
                    output_tokens = u.output_tokens;
                }
            }
            StreamEvent::MessageStop => break,
            StreamEvent::Ping => {}
            StreamEvent::Error { error } => {
                let _ = tx.send(Event::Error {
                    message: format!("{}: {}", error.error_type, error.message),
                });
                anyhow::bail!("stream error: {}", error.message);
            }
        }
    }

    let _ = tx.send(Event::TurnComplete {
        stop_reason: format!("{stop_reason:?}"),
        input_tokens,
        output_tokens,
    });

    Ok(())
}

fn parse_duration(s: &str) -> Result<Duration, String> {
    humantime::parse_duration(s).map_err(|e| e.to_string())
}
