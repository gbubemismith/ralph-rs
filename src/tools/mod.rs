use std::{collections::HashMap, path::Path, sync::Arc};

use async_trait::async_trait;

use crate::{error::ToolResult, provider::types::ToolDefinition};

pub mod bash;
pub mod file_read;
pub mod file_write;

#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the name of the tool.
    fn name(&self) -> &'static str;

    /// Get the description of the tool.
    fn description(&self) -> &'static str;

    /// JSON Schema describing the tools input parameters.
    fn input_schema(&self) -> serde_json::Value;

    async fn execute(&self, input: serde_json::Value) -> ToolResult<String>;
}

/// A registry of tools that can be called by the model.
pub struct Registry {
    tools: HashMap<String, Box<dyn Tool>>,
    /// The working directory for tools that touch the filesystem.
    pub working_dir: Arc<Path>,
}

impl Registry {
    pub fn new(working_dir: Arc<Path>) -> Self {
        Self {
            tools: HashMap::new(),
            working_dir,
        }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn standard(working_dir: Arc<Path>) -> Self {
        let mut r = Self::new(Arc::clone(&working_dir));

        r.register(Box::new(file_read::FileReadTool::new(Arc::clone(
            &working_dir,
        ))));
        r.register(Box::new(file_write::FileWriteTool::new(Arc::clone(
            &working_dir,
        ))));
        r.register(Box::new(bash::BashTool::new(working_dir)));

        r
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    /// Convert all registered tools into the format the API wants
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .map(|t| ToolDefinition {
                name: t.name().to_string(),
                description: t.description().to_string(),
                input_schema: t.input_schema(),
            })
            .collect()
    }
}
