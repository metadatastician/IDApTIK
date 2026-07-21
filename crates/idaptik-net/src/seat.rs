//! Split one headless script across the two seats, and rebuild the merged
//! per-tick input identically on both sides.
//!
//! The invariant this module owes the loopback gate: **splitting a script by
//! seat, carrying each seat's commands over the wire, and re-merging them must
//! produce the exact `TickInput` sequence `expand()` produces from the whole
//! script** — same buttons, same edge order, same immediate order. The unit
//! test at the bottom pins that equality against the real wire fixture.
//!
//! Two representational differences from `expand()` are load-bearing:
//!
//! - A script `hold` line *replaces* the held set; the wire speaks
//!   [`Command::SetButton`] *diffs*. The split converts each hold line into
//!   press/release diffs (in `Button` declaration order — any fixed order
//!   yields the same final set, and `SetButton` contributes nothing to edges or
//!   immediates, so `TickInput` equality is unaffected).
//! - Within one tick the merged order is **infiltrator's commands, then the
//!   hacker's**, each seat in its own send order. Either-seat commands are
//!   authored by the infiltrator ([`scripted_sender`]). A script line mixes
//!   seats only through its `press` list, so scripts whose per-tick press
//!   lists interleave hacker tokens *before* infiltrator ones would reorder
//!   immediates relative to `expand()` — the fixture does not, and the
//!   determinism test would catch a script that does.

use crate::envelope::{Role, scripted_sender};
use idaptik_core::scenario::command::{Button, Buttons, Command, TickInput, fold};
use idaptik_tui::script::{ScriptFile, button_from, press_command};

/// All held-button flags, in declaration order — the canonical diff order.
const ALL_BUTTONS: [Button; 5] = [
    Button::Left,
    Button::Right,
    Button::Crouch,
    Button::Sprint,
    Button::Interact,
];

/// One command scheduled for a lockstep tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScheduledCommand {
    pub at: u64,
    pub cmd: Command,
}

/// The commands `role` sends for `script`, in send order (ascending `at`,
/// line order within a tick, hold-diffs → presses → test within a line —
/// mirroring `expand()`'s per-line processing order).
pub fn seat_schedule(script: &ScriptFile, role: Role) -> Vec<ScheduledCommand> {
    let mut lines = script.commands.clone();
    lines.sort_by_key(|l| l.at);

    let mut out = Vec::new();
    let mut held = Buttons::default();

    for line in &lines {
        if line.at >= script.max_ticks {
            // expand() never reaches these lines either.
            break;
        }
        if let Some(hold) = &line.hold {
            let mut next = Buttons::default();
            for name in hold {
                if let Some(b) = button_from(name) {
                    next.set(b, true);
                }
            }
            for b in ALL_BUTTONS {
                if held.has(b) != next.has(b) {
                    let cmd = Command::SetButton {
                        button: b,
                        down: next.has(b),
                    };
                    if scripted_sender(&cmd) == role {
                        out.push(ScheduledCommand { at: line.at, cmd });
                    }
                }
            }
            held = next;
        }
        if let Some(press) = &line.press {
            for name in press {
                if let Some(cmd) = press_command(name)
                    && scripted_sender(&cmd) == role
                {
                    out.push(ScheduledCommand { at: line.at, cmd });
                }
            }
        }
        if let Some(test) = &line.test {
            let cmd = test.command();
            if scripted_sender(&cmd) == role {
                out.push(ScheduledCommand { at: line.at, cmd });
            }
        }
    }
    out
}

/// Merge the two seats' schedules into one command list per tick:
/// infiltrator's commands first, then the hacker's, each in send order.
/// Both seats run this over identical inputs, so both fold identical
/// `TickInput`s — that *is* lockstep.
pub fn merged_by_tick(
    max_ticks: u64,
    infiltrator: &[ScheduledCommand],
    hacker: &[ScheduledCommand],
) -> Vec<Vec<Command>> {
    let mut per_tick: Vec<Vec<Command>> = vec![Vec::new(); max_ticks as usize];
    for seat in [infiltrator, hacker] {
        for sc in seat {
            if let Some(slot) = per_tick.get_mut(sc.at as usize) {
                slot.push(sc.cmd);
            }
        }
    }
    per_tick
}

/// Fold the merged schedule tick by tick — the wire-side twin of
/// `idaptik_tui::script::expand`.
pub fn tick_inputs(per_tick: &[Vec<Command>]) -> Vec<TickInput> {
    let mut held = Buttons::default();
    per_tick.iter().map(|cmds| fold(cmds, &mut held)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use idaptik_tui::headless;
    use idaptik_tui::script::expand;
    use std::path::Path;

    fn fixture() -> ScriptFile {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/session_relay/capture_script.json");
        headless::load(&path).expect("wire fixture script loads")
    }

    /// The split-carry-merge pipeline reproduces `expand()` exactly.
    #[test]
    fn split_and_merge_reproduces_expand() {
        let script = fixture();
        let inf = seat_schedule(&script, Role::Infiltrator);
        let hac = seat_schedule(&script, Role::Hacker);
        assert!(!inf.is_empty(), "fixture exercises the infiltrator seat");
        assert!(!hac.is_empty(), "fixture exercises the hacker seat");

        let merged = merged_by_tick(script.max_ticks, &inf, &hac);
        assert_eq!(tick_inputs(&merged), expand(&script));
    }

    /// End to end: the lockstep pipeline's sim run is event-for-event the
    /// reference headless run.
    #[test]
    fn lockstep_run_matches_reference_run() {
        let script = fixture();
        let (ref_sim, ref_log) = headless::simulate(&script).expect("reference run");

        let merged = merged_by_tick(
            script.max_ticks,
            &seat_schedule(&script, Role::Infiltrator),
            &seat_schedule(&script, Role::Hacker),
        );
        let mut sim = headless::build(&script).expect("sim builds");
        let mut log = sim.drain_events();
        let mut held = Buttons::default();
        for cmds in &merged {
            if sim.is_ended() {
                break;
            }
            log.extend(sim.tick(&fold(cmds, &mut held)));
        }

        assert_eq!(
            serde_json::to_value(&log).unwrap(),
            serde_json::to_value(&ref_log).unwrap(),
            "event logs diverged"
        );
        assert_eq!(
            serde_json::to_value(sim.debrief()).unwrap(),
            serde_json::to_value(ref_sim.debrief()).unwrap(),
            "debriefs diverged"
        );
    }

    /// Every seat's schedule respects the relay's role table (the relay would
    /// reject a mis-routed command with an error reply, failing the run — this
    /// just fails earlier and closer to the cause).
    #[test]
    fn schedules_respect_the_role_table() {
        use crate::envelope::{Seat, seat_for};
        let script = fixture();
        for sc in seat_schedule(&script, Role::Infiltrator) {
            assert_ne!(seat_for(&sc.cmd), Seat::Hacker, "{:?}", sc.cmd);
        }
        for sc in seat_schedule(&script, Role::Hacker) {
            assert_eq!(seat_for(&sc.cmd), Seat::Hacker, "{:?}", sc.cmd);
        }
    }
}
