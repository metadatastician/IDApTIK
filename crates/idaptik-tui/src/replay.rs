//! Replay verifier: re-run a script and assert the event stream is reproducible
//! (and matches any recorded `expected_event_log`). Prints PASS/FAIL; the caller
//! turns the boolean into an exit code.

use crate::headless::{load, simulate};
use std::path::Path;

/// Re-run `path` twice and compare. Returns `Ok(true)` on a byte-identical
/// replay (and a match against any recorded log), `Ok(false)` on a mismatch.
pub fn run(path: &Path) -> Result<bool, String> {
    let script = load(path)?;
    let (_, first) = simulate(&script)?;
    let (_, second) = simulate(&script)?;

    let a = serde_json::to_value(&first).map_err(|e| format!("serialize: {e}"))?;
    let b = serde_json::to_value(&second).map_err(|e| format!("serialize: {e}"))?;
    if a != b {
        println!("FAIL: replay diverged between two runs of the same script");
        return Ok(false);
    }

    if let Some(expected) = &script.expected_event_log
        && *expected != a
    {
        println!("FAIL: replay does not match the recorded event log");
        return Ok(false);
    }

    println!("PASS: {} events reproduced deterministically", first.len());
    Ok(true)
}
