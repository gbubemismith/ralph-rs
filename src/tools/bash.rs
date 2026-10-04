use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use serde::Deserialize;
use tokio::{process::Command, time::Duration, time::timeout};

use crate::{
    error::{ToolError, ToolResult},
    tools::Tool,
};

pub struct BashTool {
    working_dir: Arc<Path>,
}

impl BashTool {
    pub fn new(working_dir: Arc<Path>) -> Self {
        Self { working_dir }
    }
}

#[derive(Deserialize)]
struct Input {
    command: String,
    #[serde(default)]
    timeout_secs: Option<u64>,
}

const DEFAULT_TIMEOUT_SECS: u64 = 60;
const MAX_OUTPUT_BYTES: usize = 100_000;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &'static str {
        "bash"
    }

    fn description(&self) -> &'static str {
        "Execute a shell command in the working directory. Returns combined stdout/stderr \
        and the exit code. The command runs through `sh -c`. Has a default 60s timeout."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute."
                },
                "timeout_secs": {
                    "type": "integer",
                    "description": "Optional timeout in seconds (default 60, max 300)."
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult<String> {
        let input: Input =
            serde_json::from_value(input).map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        let timeout_dur =
            Duration::from_secs(input.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS).min(300));

        let cmd_future = Command::new("sh")
            .arg("-c")
            .arg(&input.command)
            .current_dir(&self.working_dir)
            .output();

        let output = match timeout(timeout_dur, cmd_future).await {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => return Err(ToolError::Io(e)),
            Err(_) => {
                return Ok(format!(
                    "[command timed out after {}s]",
                    timeout_dur.as_secs()
                ));
            }
        };

        let mut combined = String::new();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !stdout.is_empty() {
            combined.push_str("--- stdout ---\n");
            combined.push_str(&stdout);
        }
        if !stderr.is_empty() {
            if !combined.is_empty() {
                combined.push('\n');
            }
            combined.push_str("--- stderr ---\n");
            combined.push_str(&stderr);
        }
        if combined.is_empty() {
            combined.push_str("(no output)");
        }
        combined.push_str(&format!(
            "\n--- exit code: {} ---",
            output
                .status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "killed".to_string())
        ));

        if combined.len() > MAX_OUTPUT_BYTES {
            combined.truncate(MAX_OUTPUT_BYTES);
            combined.push_str("\n[...output truncated]");
        }

        Ok(combined)
    }
}
