use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API returned status {status}: {body}")]
    ApiError { status: u16, body: String },

    #[error("authentication failed")]
    AuthFailed,

    #[error("rate limited; retry after {0:?}")]
    RateLimited(Option<std::time::Duration>),

    #[error("failed to parse response: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("stream error: {0}")]
    Stream(String),
}

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("tool not found: {0}")]
    NotFound(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("tool execution failed: {0}")]
    Execution(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type ProviderResult<T> = Result<T, ProviderError>;
pub type ToolResult<T> = Result<T, ToolError>;
