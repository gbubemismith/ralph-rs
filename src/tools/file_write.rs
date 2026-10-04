use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use serde::Deserialize;
use tokio::fs;

use crate::{
    error::{ToolError, ToolResult},
    tools::{Tool, file_read::resolve_safe_path},
};

pub struct FileWriteTool {
    working_dir: Arc<Path>,
}

impl FileWriteTool {
    pub fn new(working_dir: Arc<Path>) -> Self {
        Self { working_dir }
    }
}

#[derive(Deserialize)]
struct Input {
    path: String,
    contents: String,
}

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &'static str {
        "file_write"
    }

    fn description(&self) -> &'static str {
        "Write text to a file in the working directory. Creates parent directories \
        if they don't exist. Overwrites existing files. Path must be relative."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "The relative path to the file to write" },
                "contents": { "type": "string", "description": "The contents to write to the file" }
            },
            "required": ["path", "contents"]
        })
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult<String> {
        let input: Input =
            serde_json::from_value(input).map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        let abs_path = resolve_safe_path(&self.working_dir, &input.path)?;

        if let Some(parent) = abs_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&abs_path, &input.contents).await?;

        Ok(format!(
            "wrote {} bytes to {}",
            input.contents.len(),
            input.path
        ))
    }
}
