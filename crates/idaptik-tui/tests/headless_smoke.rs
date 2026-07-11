//! Stage C: the headless / replay / export CLI paths need no TTY and exit 0.

use assert_cmd::Command;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures");
    p.push(name);
    p
}

#[test]
fn headless_clean_extract_prints_json_and_exits_zero() {
    let out = Command::cargo_bin("idaptik-tui")
        .unwrap()
        .args(["--headless", "--script"])
        .arg(fixture("clean_extract.json"))
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON on stdout");
    assert!(v.get("event_log").is_some(), "event_log present");
    assert!(v.get("debrief").is_some(), "debrief present");
    assert!(v.get("final_snapshot").is_some(), "final_snapshot present");
    // The scripted service-exit force-extract must succeed.
    assert_eq!(v["debrief"]["success"], serde_json::Value::Bool(true));
    assert_eq!(
        v["final_snapshot"]["format"],
        "idaptik-ghost-lobby-runtime-v1"
    );
}

#[test]
fn headless_caught_reports_failure() {
    let out = Command::cargo_bin("idaptik-tui")
        .unwrap()
        .args(["--headless", "--script"])
        .arg(fixture("caught.json"))
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["debrief"]["success"], serde_json::Value::Bool(false));
    assert_eq!(v["debrief"]["reason"], "Caught");
}

#[test]
fn replay_verifies_determinism() {
    let out = Command::cargo_bin("idaptik-tui")
        .unwrap()
        .arg("--replay")
        .arg(fixture("clean_extract.json"))
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("PASS"),
        "replay should report PASS: {stdout}"
    );
}

#[test]
fn export_definition_has_format_tag() {
    let out = Command::cargo_bin("idaptik-tui")
        .unwrap()
        .args(["--export", "definition"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["format"], "idaptik-ghost-lobby-v1");
    assert!(v.get("definition").is_some());
    assert!(v.get("validation").is_some());
}

#[test]
fn export_debrief_after_script() {
    let out = Command::cargo_bin("idaptik-tui")
        .unwrap()
        .args(["--export", "debrief", "--script"])
        .arg(fixture("clean_extract.json"))
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["format"], "idaptik-ghost-lobby-after-action-v1");
    assert_eq!(v["debrief"]["success"], serde_json::Value::Bool(true));
}
