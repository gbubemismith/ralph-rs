use tokio::sync::mpsc;

/// Everything interesting that happens during a Ralph run.
/// The display task consumes these and renders them to the terminal.
#[derive(Debug, Clone)]
pub enum Event {
    /// Outer loop: a new iteration is starting.
    IterationStart { iteration: u32, max: u32 },
    /// Outer loop: the run is wrapping up.
    RunComplete {
        outcome: RunOutcome,
        iterations: u32,
    },

    /// Inner: model started generating a turn.
    TurnStart { model: String },
    /// Inner: a chunk of streamed text from the model.
    TextDelta { text: String },
    /// Inner: model is calling a tool. Fired once we have the full input parsed.
    ToolCallStart {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// Inner: tool finished executing.
    ToolCallComplete {
        id: String,
        name: String,
        output: String,
        is_error: bool,
    },
    /// Inner: the model's turn ended.
    TurnComplete {
        stop_reason: String,
        input_tokens: u32,
        output_tokens: u32,
    },

    /// Diagnostic / debug.
    Notice { message: String },
    /// Recoverable problem.
    Warning { message: String },
    /// Fatal error — the run is about to bail.
    Error { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    Complete,
    MaxIterationsReached,
    TimedOut,
    Cancelled,
    Failed,
}

pub type EventSender = mpsc::UnboundedSender<Event>;
pub type EventReceiver = mpsc::UnboundedReceiver<Event>;

pub fn channel() -> (EventSender, EventReceiver) {
    mpsc::unbounded_channel()
}
