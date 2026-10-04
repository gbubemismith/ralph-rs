use std::pin::Pin;

use async_trait::async_trait;
use futures_util::Stream;

use crate::{
    error::ProviderResult,
    provider::types::{ChatRequest, ChatResponse, StreamEvent},
};

pub mod anthropic;
pub mod types;

pub type EventStream = Pin<Box<dyn Stream<Item = ProviderResult<StreamEvent>> + Send>>;

#[async_trait]
pub trait Provider: Send + Sync {
    /// Send a chat request and return the full response.
    async fn chat(&self, request: ChatRequest) -> ProviderResult<ChatResponse>;

    /// Send a chat request and return a stream of events.
    async fn stream(&self, request: ChatRequest) -> ProviderResult<EventStream>;
}
