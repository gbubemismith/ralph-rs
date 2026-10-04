use std::sync::Arc;

use ralph_rs::tools::{Tool, bash::BashTool, file_read::FileReadTool, file_write::FileWriteTool};
use serde_json::json;
use tempfile::tempdir;

#[tokio::test]
async fn file_write_then_read_roundtrip() {
    let dir = tempdir().unwrap();
    let writer = FileWriteTool::new(Arc::from(dir.path()));
    let reader = FileReadTool::new(Arc::from(dir.path()));

    let write_out = writer
        .execute(json!({ "path": "hello.txt", "contents": "world" }))
        .await
        .unwrap();
    assert!(write_out.contains("wrote 5 bytes"));

    let read_out = reader
        .execute(json!({ "path": "hello.txt" }))
        .await
        .unwrap();
    assert_eq!(read_out, "world");
}

#[tokio::test]
async fn file_read_rejects_traversal() {
    let dir = tempdir().unwrap();
    let reader = FileReadTool::new(Arc::from(dir.path()));

    let err = reader
        .execute(json!({ "path": "../etc/passwd" }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains(".."));
}

#[tokio::test]
async fn bash_runs_command() {
    let dir = tempdir().unwrap();
    let bash = BashTool::new(Arc::from(dir.path()));

    let out = bash
        .execute(json!({ "command": "echo hello" }))
        .await
        .unwrap();
    assert!(out.contains("hello"));
    assert!(out.contains("exit code: 0"));
}

#[tokio::test]
async fn bash_timeout_works() {
    let dir = tempdir().unwrap();
    let bash = BashTool::new(Arc::from(dir.path()));

    let out = bash
        .execute(json!({ "command": "sleep 5", "timeout_secs": 1 }))
        .await
        .unwrap();
    assert!(out.contains("timed out"));
}
