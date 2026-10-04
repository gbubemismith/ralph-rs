use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use tokio::fs;

use async_trait::async_trait;
use serde::Deserialize;

use crate::{
    error::{ToolError, ToolResult},
    tools::Tool,
};

pub struct FileReadTool {
    working_dir: Arc<Path>,
}

impl FileReadTool {
    pub fn new(working_dir: Arc<Path>) -> Self {
        Self { working_dir }
    }
}

#[derive(Deserialize)]
struct Input {
    path: String,
}

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &'static str {
        "file_read"
    }

    fn description(&self) -> &'static str {
        "Read a file from the working directory and return its contents as text.\
        Path must be relative to the working directory"
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "The relative path to the file to read" }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult<String> {
        let input: Input =
            serde_json::from_value(input).map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        let abs_path = resolve_safe_path(&self.working_dir, &input.path)?;
        let contents = fs::read_to_string(&abs_path).await?;

        // Cap output to avoid blowing up the context window on accidental reads
        // of huge files.
        const MAX_BYTES: usize = 100_000;
        if contents.len() > MAX_BYTES {
            Ok(format!(
                "{}\n\n[...truncated, file is {} bytes total]",
                &contents[..MAX_BYTES],
                contents.len()
            ))
        } else {
            Ok(contents)
        }
    }
}

/// Reject absolute paths and any `..` traversal to keep tools sandboxed
/// to the working directory.
pub(super) fn resolve_safe_path(working_dir: &Path, rel: &str) -> ToolResult<PathBuf> {
    let path = std::path::Path::new(rel);
    if path.is_absolute() {
        return Err(ToolError::InvalidInput(format!(
            "absolute paths not allowed: {rel}"
        )));
    }
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(ToolError::InvalidInput(format!(
            "path must not contain '..': {rel}"
        )));
    }
    Ok(working_dir.join(path))
}
