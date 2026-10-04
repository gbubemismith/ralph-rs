use crate::provider::types::{ContentBlock, Message, Role};

/// Append-only history of a single conversation.
/// One conversation per outer-loop iteration.
#[derive(Default)]
pub struct Conversation {
    messages: Vec<Message>,
}

impl Conversation {
    pub fn user(&mut self, text: impl Into<String>) {
        self.messages.push(Message::user(text));
    }

    pub fn user_blocks(&mut self, blocks: Vec<ContentBlock>) {
        self.messages.push(Message {
            role: Role::User,
            content: blocks,
        });
    }

    pub fn assistant_blocks(&mut self, blocks: Vec<ContentBlock>) {
        self.messages.push(Message {
            role: Role::Assistant,
            content: blocks,
        });
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }
}
