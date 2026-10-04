use assert_cmd::Command;
use predicates::prelude::*;

/// A fresh invocation of the compiled `ralph-rs` binary.
fn ralph() -> Command {
    Command::cargo_bin("ralph-rs").unwrap()
}

#[test]
fn version_prints_version_and_succeeds() {
    ralph()
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("ralph-rs 0.1.0"));
}

#[test]
fn no_subcommand_fails_with_usage_error() {
    ralph()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn run_without_prompt_fails() {
    ralph()
        .arg("run")
        .env("ANTHROPIC_API_KEY", "test-key")
        .assert()
        .failure()
        .stderr(predicate::str::contains("<PROMPT>"));
}

#[test]
fn run_without_api_key_fails() {
    ralph()
        .args(["run", "do the thing"])
        .env_remove("ANTHROPIC_API_KEY")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--api-key"));
}

#[test]
fn run_with_invalid_timeout_fails() {
    ralph()
        .args(["run", "do the thing", "--api-key", "test-key", "--timeout", "bogus"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--timeout"));
}

#[test]
fn run_with_invalid_system_prompt_mode_fails() {
    ralph()
        .args([
            "run",
            "do the thing",
            "--api-key",
            "test-key",
            "--system-prompt-mode",
            "bogus",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("append").and(predicate::str::contains("replace")));
}
