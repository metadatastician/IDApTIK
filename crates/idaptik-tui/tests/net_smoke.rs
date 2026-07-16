//! Net mode smoke tests: makes the manual vantage/pivot runs permanent.
//!
//! Task 14 adds the other half: the pivot verbs the Ghost Lobby keys emit, driven
//! through the real binary. The interactive TUI needs a TTY, so the keys are
//! exercised by the scripted headless path they share -- `press: ["pivot"]` folds
//! to the same `Command::Pivot` that `p` does, into the same tick, through the
//! same immediates block.

use assert_cmd::Command;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures");
    p.push(name);
    p
}

/// The `event` tags of a scripted run's event log, in order.
fn event_log(name: &str) -> Vec<serde_json::Value> {
    let out = Command::cargo_bin("idaptik-tui")
        .unwrap()
        .args(["--headless", "--script"])
        .arg(fixture(name))
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON on stdout");
    v["event_log"].as_array().expect("an event log").clone()
}

/// Every event in the log whose tag is `tag`.
fn events_named<'a>(log: &'a [serde_json::Value], tag: &str) -> Vec<&'a serde_json::Value> {
    log.iter().filter(|e| e["event"] == tag).collect()
}

#[test]
fn inside_vantage_hack_door_reports_effect() {
    let out = Command::cargo_bin("idaptik-tui")
        .unwrap()
        .args(["net", "--vantage", "inside"])
        .write_stdin("hack door-hall-office\nquit\n")
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("DoorHeld"), "stdout: {stdout}");
    assert!(stdout.contains("door-hall-office"), "stdout: {stdout}");
}

#[test]
fn van_vantage_ssh_pivots_to_bridge() {
    let out = Command::cargo_bin("idaptik-tui")
        .unwrap()
        .args(["net", "--vantage", "van"])
        .write_stdin("ssh bridge.local\nquit\n")
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    // The bridge host is listed as reachable before any command runs, so a bare
    // IP match would pass even if the ssh failed; require the success message.
    assert!(stdout.contains("pivoted to 10.20.5.2"), "stdout: {stdout}");
}

//## The Ghost Lobby pivot keys, through the binary

#[test]
fn the_pivot_key_opens_the_building_line_for_the_uplinks() {
    // The regression that matters most: before Task 14 no key emitted a pivot, so
    // this same door press answered `UplinkDenied { reason: NoRoute }` for ever.
    let log = event_log("pivot_building.json");

    let opened = events_named(&log, "PivotOpened");
    assert_eq!(
        opened.len(),
        1,
        "the pivot key landed exactly once: {log:?}"
    );
    assert_eq!(opened[0]["host"], "bridge.local");
    assert_eq!(opened[0]["hops"], 1);

    assert!(
        events_named(&log, "UplinkDenied").is_empty(),
        "nothing may be denied once the hacker is on the bridge: {log:?}"
    );
    assert_eq!(
        events_named(&log, "UplinkAction").len(),
        1,
        "the door uplink landed"
    );
    assert_eq!(
        events_named(&log, "DoorRouted").len(),
        1,
        "and it routed a door"
    );
    assert_eq!(
        events_named(&log, "DoorHoldActive").len(),
        1,
        "which then actually opened"
    );
}

#[test]
fn the_upstream_line_must_be_walked_one_hop_at_a_time() {
    // The whole reason the grid jump host needs a key of its own. Cold from the
    // van it is out of reach; the ISP ops host opens it; and only from there is
    // the hacker two deep. A keymap offering the ISP without the grid would strand
    // the player one hop short of the substation with no way to say the last word.
    let log = event_log("pivot_upstream.json");

    let denied = events_named(&log, "PivotDenied");
    assert_eq!(denied.len(), 1, "exactly the cold attempt was refused");
    assert_eq!(denied[0]["host"], "jump.grid.local");
    assert_eq!(
        denied[0]["reason"], "NoRoute",
        "and refused for want of a route, not for want of a host"
    );

    let opened = events_named(&log, "PivotOpened");
    assert_eq!(opened.len(), 2, "both hops landed: {log:?}");
    assert_eq!(opened[0]["host"], "ops.isp.net");
    assert_eq!(opened[0]["hops"], 1);
    assert_eq!(opened[1]["host"], "jump.grid.local");
    assert_eq!(
        opened[1]["hops"], 2,
        "the second hop is what the substation answers to"
    );

    let closed = events_named(&log, "PivotClosed");
    assert_eq!(closed.len(), 1, "the unpivot key popped one layer");
    assert_eq!(closed[0]["hops"], 1, "one layer, not the whole stack");
}
