use std::{path::PathBuf, time::Duration};

use anyhow::{Context, Ok, Result};

use crate::cli::RunArgs;

const DEFAULT_SYSTEM_PROMPT: &str = "You are ralph, an autonomous coding assistant. \
You complete tasks iterately, reading files, writing files, and running commands as needed. \
Be concise. Prefer doing the work over describing what you would do. \
When the task is complete say so and stop.";

#[derive(Debug, Clone)]
pub struct Config {
    pub prompt: String,
    pub max_iterations: u32,
    pub timeout: Duration,
    pub model: String,
    pub working_dir: PathBuf,
    pub system_prompt: String,
    pub api_key: String,
}

impl TryFrom<RunArgs> for Config {
    type Error = anyhow::Error;

    fn try_from(args: RunArgs) -> Result<Self, Self::Error> {
        // If `prompt` looks like a file path that exists and ends in `.md` then read it.
        let prompt = resolve_prompt(&args.prompt)?;

        let working_dir = match args.working_dir {
            Some(w) => w,
            None => std::env::current_dir().context("getting current directory")?,
        };

        let system_prompt = match (args.system_prompt, args.system_prompt_mode.as_str()) {
            (None, _) => DEFAULT_SYSTEM_PROMPT.to_string(),
            (Some(custom), "replace") => custom,
            (Some(custom), "append") => format!("{DEFAULT_SYSTEM_PROMPT}\n\n{custom}"),
            (Some(_), other) => anyhow::bail!("unknown system prompt mode: {other}"),
        };

        Ok(Self {
            prompt,
            max_iterations: args.max_iterations,
            timeout: args.timeout,
            model: args.model,
            working_dir,
            system_prompt,
            api_key: args.api_key,
        })
    }
}

fn resolve_prompt(input: &str) -> Result<String> {
    let path = std::path::Path::new(input);

    if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
        std::fs::read_to_string(path)
            .with_context(|| format!("reading prompt from {}", path.display()))
    } else {
        Ok(input.to_string())
    }
}
