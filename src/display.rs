use std::io::Write;

use crate::events::{Event, EventReceiver, RunOutcome};

/// Run the display loop. Returns when the channel closes.
pub async fn run_display(mut rx: EventReceiver) {
    while let Some(event) = rx.recv().await {
        render(event);
    }
}

fn render(event: Event) {
    match event {
        Event::IterationStart { iteration, max } => {
            println!("\n\x1b[1;36m=== Iteration {iteration}/{max} ===\x1b[0m");
        }
        Event::RunComplete {
            outcome,
            iterations,
        } => {
            let (label, color) = match outcome {
                RunOutcome::Complete => ("Complete", "32"),
                RunOutcome::MaxIterationsReached => ("Max iterations reached", "33"),
                RunOutcome::TimedOut => ("Timed out", "33"),
                RunOutcome::Cancelled => ("Cancelled", "33"),
                RunOutcome::Failed => ("Failed", "31"),
            };
            println!("\n\x1b[1;{color}m── {label} after {iterations} iteration(s) ──\x1b[0m");
        }
        Event::TurnStart { model } => {
            println!("\n\x1b[2m▶ {model}\x1b[0m");
        }
        Event::TextDelta { text } => {
            print!("{text}");
            let _ = std::io::stdout().flush();
        }
        Event::ToolCallStart { name, input, .. } => {
            // Compact one-liner version of input for the header.
            let input_summary = summarize_input(&input);
            println!("\n\x1b[33m🔧 {name}({input_summary})\x1b[0m");
        }
        Event::ToolCallComplete {
            name,
            output,
            is_error,
            ..
        } => {
            let preview = preview(&output, 200);
            if is_error {
                println!("\x1b[31m  ✗ {name}: {preview}\x1b[0m");
            } else {
                println!("\x1b[2m  ✓ {name}: {preview}\x1b[0m");
            }
        }
        Event::TurnComplete {
            stop_reason,
            input_tokens,
            output_tokens,
        } => {
            println!(
                "\n\x1b[2m  └ stop={stop_reason}  tokens={input_tokens}↓ {output_tokens}↑\x1b[0m"
            );
        }
        Event::Notice { message } => {
            println!("\x1b[2m• {message}\x1b[0m");
        }
        Event::Warning { message } => {
            eprintln!("\x1b[33m⚠ {message}\x1b[0m");
        }
        Event::Error { message } => {
            eprintln!("\x1b[31m✗ {message}\x1b[0m");
        }
    }
}

fn summarize_input(input: &serde_json::Value) -> String {
    let s = input.to_string();
    if s.len() > 80 {
        format!("{}…", &s[..80])
    } else {
        s
    }
}

fn preview(s: &str, max: usize) -> String {
    let s = s.trim();
    let single_line: String = s.chars().map(|c| if c == '\n' { ' ' } else { c }).collect();
    if single_line.chars().count() > max {
        let truncated: String = single_line.chars().take(max).collect();
        format!("{truncated}…")
    } else {
        single_line
    }
}
