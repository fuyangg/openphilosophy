use serde_json::{Value, json};
use std::{
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};

fn run(db: &Path, args: &[&str], input: Option<&[u8]>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ophi"))
        .arg("--db")
        .arg(db)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(input) = input {
        // Oversize input can be rejected before the entire pipe is consumed.
        let _ = child.stdin.take().unwrap().write_all(input);
    } else {
        drop(child.stdin.take());
    }
    child.wait_with_output().unwrap()
}
fn data(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["protocol_version"], 1);
    value["data"].clone()
}

#[test]
fn independent_processes_can_retrieve_original_history_and_references() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("session.db");
    let case = data(run(
        &db,
        &["case", "create", "--title", "Debug timeout"],
        None,
    ));
    let input =
        json!({"case_id":case["id"],"kind":"failure","content":"Pool exhausted","author":"human"});
    let entry = data(run(
        &db,
        &["entry", "append"],
        Some(input.to_string().as_bytes()),
    ));
    let input = json!({"case_id":case["id"],"kind":"discovery","content":"Increase pool size","author":"agent","references":[{"target_id":entry["id"],"relation":"supersedes"}]});
    let revision = data(run(
        &db,
        &["entry", "append"],
        Some(input.to_string().as_bytes()),
    ));
    assert_eq!(
        data(run(
            &db,
            &["entry", "get", entry["id"].as_str().unwrap()],
            None
        )),
        entry
    );
    let history = data(run(
        &db,
        &[
            "case",
            "history",
            case["id"].as_str().unwrap(),
            "--limit",
            "1",
        ],
        None,
    ));
    assert_eq!(history["entries"], json!([entry]));
    let next = data(run(
        &db,
        &[
            "case",
            "history",
            case["id"].as_str().unwrap(),
            "--after",
            history["next_cursor"].as_str().unwrap(),
        ],
        None,
    ));
    assert_eq!(next["entries"], json!([revision]));
    assert!(next["next_cursor"].is_null());
}

#[test]
fn malformed_and_oversized_input_produce_structured_errors_without_database_creation() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("invalid.db");
    let oversized = vec![b'x'; ophi_core::MAX_RECORD_BYTES + 1];
    for input in [
        b"not json".as_slice(),
        b"{}".as_slice(),
        oversized.as_slice(),
    ] {
        let output = run(&db, &["entry", "append"], Some(input));
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["code"], "invalid_input");
        assert!(!db.exists());
    }
}

#[test]
fn argument_errors_are_json_and_missing_records_are_not_null_successes() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.db");
    let output = run(&db, &["nonsense"], None);
    assert_eq!(output.status.code(), Some(2));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "invalid_arguments");
    let output = run(
        &db,
        &["entry", "get", "00000000-0000-4000-8000-000000000000"],
        None,
    );
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "not_found");
}

#[test]
fn case_creation_accepts_json_stdin() {
    let dir = tempfile::tempdir().unwrap();
    let value = data(run(
        &dir.path().join("test.db"),
        &["case", "create"],
        Some(br#"{"title":"Mathematics"}"#),
    ));
    assert_eq!(value["title"], "Mathematics");
}
