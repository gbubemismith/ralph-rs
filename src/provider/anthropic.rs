use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use reqwest::{Client, StatusCode};

use crate::{
    error::{ProviderError, ProviderResult},
    provider::{
        EventStream, Provider,
        types::{ChatRequest, ChatResponse, StreamEvent},
    },
};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

pub struct AnthrhopicProvider {
    api_key: String,
    client: Client,
}

impl AnthrhopicProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key: api_key,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl Provider for AnthrhopicProvider {
    async fn chat(&self, request: ChatRequest) -> ProviderResult<ChatResponse> {
        let request = request;

        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(match status {
                StatusCode::UNAUTHORIZED => ProviderError::AuthFailed,
                StatusCode::TOO_MANY_REQUESTS => ProviderError::RateLimited(None),
                code => ProviderError::ApiError {
                    status: code.as_u16(),
                    body,
                },
            });
        }

        let parsed: ChatResponse = response.json().await?;
        Ok(parsed)
    }

    async fn stream(&self, request: ChatRequest) -> ProviderResult<EventStream> {
        let request = request;

        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("Content-Type", "application/json")
            .header("accept", "text/event-stream")
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(match status {
                StatusCode::UNAUTHORIZED => ProviderError::AuthFailed,
                StatusCode::TOO_MANY_REQUESTS => ProviderError::RateLimited(None),
                code => ProviderError::ApiError {
                    status: code.as_u16(),
                    body,
                },
            });
        }

        let stream =
            response
                .bytes_stream()
                .eventsource()
                .map(|event| -> ProviderResult<StreamEvent> {
                    let event = event.map_err(|e| ProviderError::Stream(e.to_string()))?;

                    let parsed: StreamEvent = serde_json::from_str(&event.data)?;
                    Ok(parsed)
                });

        Ok(Box::pin(stream))
    }
}
