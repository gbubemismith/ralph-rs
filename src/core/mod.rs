use std::sync::Arc;

use anyhow::Result;
use futures_util::StreamExt;

use crate::{
    core::conversation::Conversation,
    events::{Event, EventSender},
    provider::{
        Provider,
        types::{
            ChatRequest, ContentBlock, ContentBlockStart, ContentDelta, StopReason, StreamEvent,
        },
    },
    tools::Registry,
};

pub mod conversation;
pub mod loop_engine;

/// Maximum number of turns to run in a single iteration of the agentic loop.
const MAX_TURNS_PER_ITERATION: u32 = 30;

/// Output of a single streamed turn.
struct TurnOutput {
    /// Assistant content blocks, fully assembled.
    blocks: Vec<ContentBlock>,
    stop_reason: StopReason,
    input_tokens: u32,
    output_tokens: u32,
}

/// In-progress assembly of one content block during streaming.
enum BlockBuilder {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        partial_json: String,
    },
}

impl BlockBuilder {
    /// Create a new BlockBuilder from the start of a content block.
    fn from_start(start: ContentBlockStart) -> Self {
        match start {
            ContentBlockStart::Text { text } => BlockBuilder::Text { text },
            ContentBlockStart::ToolUse { id, name, .. } => BlockBuilder::ToolUse {
                id,
                name,
                partial_json: String::new(),
            },
        }
    }

    /// Apply a delta to the in-progress block.
    fn apply_delta(&mut self, delta: ContentDelta) {
        match (self, delta) {
            (BlockBuilder::Text { text }, ContentDelta::TextDelta { text: chunk }) => {
                text.push_str(&chunk);
            }
            (
                BlockBuilder::ToolUse { partial_json, .. },
                ContentDelta::InputJsonDelta {
                    partial_json: chunk,
                },
            ) => {
                partial_json.push_str(&chunk);
            }
            // Mismatched delta type for the active block — Anthropic shouldn't send these,
            // so we'd just ignore. tracing::warn would be appropriate here.
            _ => {}
        }
    }

    /// Finish the block and return a ContentBlock.
    fn finish(self) -> Result<ContentBlock> {
        match self {
            BlockBuilder::Text { text } => Ok(ContentBlock::Text { text }),
            BlockBuilder::ToolUse {
                id,
                name,
                partial_json,
            } => {
                let input: serde_json::Value = if partial_json.is_empty() {
                    serde_json::json!({})
                } else {
                    serde_json::from_str(&partial_json)
                        .map_err(|e| anyhow::anyhow!("invalid tool input JSON: {e}"))?
                };
                Ok(ContentBlock::ToolUse { id, name, input })
            }
        }
    }
}

async fn run_turn(
    provider: &dyn Provider,
    request: ChatRequest,
    tx: &EventSender,
) -> Result<TurnOutput> {
    let _ = tx.send(Event::TurnStart {
        model: request.model.clone(),
    });

    let mut stream = provider.stream(request).await?;
    let mut blocks: Vec<Option<BlockBuilder>> = Vec::new();
    let mut stop_reason = StopReason::EndTurn;
    let mut input_tokens = 0u32;
    let mut output_tokens = 0u32;

    while let Some(event) = stream.next().await {
        match event? {
            StreamEvent::MessageStart { message } => {
                input_tokens = message.usage.input_tokens;
            }

            StreamEvent::ContentBlockStart {
                index,
                content_block,
            } => {
                let idx = index as usize;
                while blocks.len() <= idx {
                    blocks.push(None);
                }
                blocks[idx] = Some(BlockBuilder::from_start(content_block));
            }

            StreamEvent::ContentBlockDelta { index, delta } => {
                // Emit text deltas to the display in real time.
                if let ContentDelta::TextDelta { ref text } = delta {
                    let _ = tx.send(Event::TextDelta { text: text.clone() });
                }
                if let Some(builder) = blocks.get_mut(index as usize).and_then(|b| b.as_mut()) {
                    builder.apply_delta(delta);
                }
            }

            StreamEvent::ContentBlockStop { .. } => {
                // We finalize all builders together at message_stop.
            }

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

    let blocks: Vec<ContentBlock> = blocks
        .into_iter()
        .filter_map(|b| b.map(|b| b.finish()))
        .collect::<Result<_>>()?;

    let _ = tx.send(Event::TurnComplete {
        stop_reason: format!("{stop_reason:?}"),
        input_tokens,
        output_tokens,
    });

    Ok(TurnOutput {
        blocks,
        stop_reason,
        input_tokens,
        output_tokens,
    })
}

/// Run the inner agentic loop: keep turning until the model stops requesting tools.
pub async fn run_agentic_loop(
    provider: Arc<dyn Provider>,
    registry: Arc<Registry>,
    system_prompt: String,
    model: String,
    initial_user_prompt: String,
    tx: EventSender,
) -> Result<()> {
    let mut conversation = Conversation::default();
    conversation.user(initial_user_prompt);

    let tool_definitions = registry.definitions();

    for turn in 1..=MAX_TURNS_PER_ITERATION {
        tracing::debug!("inner turn {turn}");

        let request = ChatRequest {
            model: model.clone(),
            max_tokens: 4096,
            system: system_prompt.clone(),
            messages: conversation.messages().to_vec(),
            tools: tool_definitions.clone(),
            stream: true,
        };

        let output = run_turn(provider.as_ref(), request, &tx).await?;

        // The assistant's full reply (text + tool_use) goes into history as received
        conversation.assistant_blocks(output.blocks.clone());

        match output.stop_reason {
            StopReason::ToolUse => {
                // Execute every tool_use block, build the matching tool_result blocks.
                let mut result_blocks = Vec::new();
                for block in &output.blocks {
                    if let ContentBlock::ToolUse { id, name, input } = block {
                        let _ = tx.send(Event::ToolCallStart {
                            id: id.clone(),
                            name: name.clone(),
                            input: input.clone(),
                        });

                        let (output_str, is_error) = match registry.get(name) {
                            None => (format!("error: tool '{name}' not found"), true),
                            Some(tool) => match tool.execute(input.clone()).await {
                                Ok(s) => (s, false),
                                Err(e) => (format!("error: {e}"), true),
                            },
                        };

                        let _ = tx.send(Event::ToolCallComplete {
                            id: id.clone(),
                            name: name.clone(),
                            output: output_str.clone(),
                            is_error,
                        });

                        result_blocks.push(ContentBlock::ToolResult {
                            tool_use_id: id.clone(),
                            content: output_str,
                            is_error: if is_error { Some(true) } else { None },
                        });
                    }
                }
                conversation.user_blocks(result_blocks);
                // Loop again — the model now sees the results and decides what's next.
            }
            // Anything else → inner loop is done.
            _ => return Ok(()),
        }
    }

    let _ = tx.send(Event::Warning {
        message: format!("inner loop exceeded {MAX_TURNS_PER_ITERATION} turns; stopping"),
    });
    Ok(())
}
