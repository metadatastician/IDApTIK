//! Stage B: snapshot at tick N, restore, continue M ticks == uninterrupted N+M.

mod common;
use common::Runner;
use idaptik_core::scenario::command::{Button, Command, TickInput, fold};
use idaptik_core::scenario::event::Event;
use idaptik_core::scenario::{GhostLobbySim, ghost_lobby};

fn step_stream(t: u64) -> Vec<Command> {
    let mut cmds = vec![Command::SetButton {
        button: Button::Right,
        down: true,
    }];
    if t == 30 {
        cmds.push(Command::ForceCrisis);
    }
    if t == 80 {
        cmds.push(Command::Uplink {
            kind: idaptik_core::scenario::ActionKind::Camera,
        });
    }
    cmds
}

#[test]
fn snapshot_restore_continues_identically() {
    // Uninterrupted reference run of 300 ticks.
    let mut reference = Runner::standard();
    for t in 0..300u64 {
        reference.step(&step_stream(t));
    }

    // Interrupted run: 150 ticks, snapshot, restore, 150 more.
    let mut a = Runner::standard();
    for t in 0..150u64 {
        a.step(&step_stream(t));
    }
    let snap = a.sim.snapshot();
    let held = a.held;
    let mut restored = GhostLobbySim::restore(ghost_lobby(), snap).expect("restore");

    let mut log: Vec<Event> = a.log.clone();
    let mut held = held;
    for t in 150..300u64 {
        let cmds = step_stream(t);
        let input: TickInput = fold(&cmds, &mut held);
        log.extend(restored.tick(&input));
    }

    let full = serde_json::to_string(&reference.log).unwrap();
    let split = serde_json::to_string(&log).unwrap();
    assert_eq!(full, split, "snapshot/restore must not perturb the stream");
    assert_eq!(reference.sim.current_tick(), restored.current_tick());
}

#[test]
fn paused_snapshot_restores_paused() {
    // Pause mid-run, snapshot, restore: the restored sim must still be paused so
    // subsequent non-pause commands are ignored (matching the paused original).
    let mut a = Runner::standard();
    for t in 0..40u64 {
        a.step(&step_stream(t));
    }
    a.step(&[Command::Pause { on: true }]);
    assert!(a.sim.is_paused());
    let snap = a.sim.snapshot();
    assert!(snap.paused, "snapshot captures the paused flag");

    let mut restored = GhostLobbySim::restore(ghost_lobby(), snap).expect("restore");
    assert!(restored.is_paused(), "restore round-trips paused");

    // A held-right tick is a no-op while paused: tick unchanged, no events.
    let before_tick = restored.current_tick();
    let mut held = a.held;
    let input = fold(
        &[Command::SetButton {
            button: Button::Right,
            down: true,
        }],
        &mut held,
    );
    let ev = restored.tick(&input);
    assert!(ev.is_empty(), "paused restore ignores the command stream");
    assert_eq!(restored.current_tick(), before_tick);
}

#[test]
fn snapshot_round_trips_through_json() {
    let mut r = Runner::standard();
    for t in 0..90u64 {
        r.step(&step_stream(t));
    }
    let snap = r.sim.snapshot();
    let json = serde_json::to_string(&snap).unwrap();
    let back: idaptik_core::scenario::RuntimeSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(snap, back);
    assert_eq!(
        back.format,
        idaptik_core::scenario::snapshot::SNAPSHOT_FORMAT
    );
}
