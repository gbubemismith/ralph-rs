# ralph-rs

A small Rust CLI that runs an AI development loop: you give it a task prompt, and it repeatedly calls an LLM (Anthropic's Claude) that can use tools (bash, file read, file write) to work on the task until it finishes or hits a limit.

> **Note:** This project is for learning purposes. It's an experiment in building an agent loop in Rust (async, streaming APIs, tool use, CLI design) and is not intended for production use.

## Usage

```sh
export ANTHROPIC_API_KEY=...   # if required by your config
cargo run -- run "Add a hello world function" --max-iterations 10 --timeout 30m
```

Options include `--model`, `--working-dir`, and `--system-prompt`. Run `cargo run -- run --help` for the full list.

## Layout

- `src/cli.rs` – CLI arguments and the run loop
- `src/provider/` – Anthropic provider and streaming types
- `src/tools/` – bash, file read, and file write tools
- `src/core/` – conversation state
- `tests/` – CLI and tool tests

## Tests

```sh
cargo test
```
