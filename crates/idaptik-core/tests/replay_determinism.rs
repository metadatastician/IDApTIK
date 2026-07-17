//! Stage B: the same seed + command stream yields a byte-identical event log.

mod common;
use common::Runner;
use idaptik_core::scenario::command::{Button, Command, PivotTarget};
use idaptik_core::scenario::event::Event;

/// A moderately busy scripted stream exercising movement, uplinks and crisis.
fn scripted(r: &mut Runner) {
    // Walk right for a bit.
    for t in 0..600u64 {
        let mut cmds = vec![Command::SetButton {
            button: Button::Right,
            down: true,
        }];
        // The uplinks below are route-gated, so the stream opens with the pivot,
        // through the canonical command path so the recorded stream replays
        // whole; without it the run would still be deterministic, but
        // deterministically denied, and the landed actions would drop out of the
        // replay it means to pin.
        if t == 0 {
            cmds.push(Command::Pivot {
                target: PivotTarget::Bridge,
            });
        }
        if t == 10 {
            cmds.push(Command::Uplink {
                kind: idaptik_core::scenario::ActionKind::Camera,
            });
        }
        if t == 40 {
            cmds.push(Command::Uplink {
                kind: idaptik_core::scenario::ActionKind::Door,
            });
        }
        if t == 120 {
            cmds.push(Command::ForceCrisis);
        }
        if t == 200 {
            cmds.push(Command::Uplink {
                kind: idaptik_core::scenario::ActionKind::Lights,
            });
        }
        r.step(&cmds);
        if r.sim.is_ended() {
            break;
        }
    }
}

#[test]
fn two_runs_produce_identical_event_json() {
    let mut a = Runner::standard();
    let mut b = Runner::standard();
    scripted(&mut a);
    scripted(&mut b);
    let ja = serde_json::to_string(&a.log).unwrap();
    let jb = serde_json::to_string(&b.log).unwrap();
    assert_eq!(ja, jb, "identical inputs must produce identical event logs");
    assert!(a.log.len() > 5);
}

#[test]
fn event_log_is_stable_under_reserialisation() {
    let mut r = Runner::standard();
    scripted(&mut r);
    let json = serde_json::to_string(&r.log).unwrap();
    let back: Vec<Event> = serde_json::from_str(&json).unwrap();
    assert_eq!(r.log, back);
}
