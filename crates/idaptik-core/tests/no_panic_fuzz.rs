//! Stage B: random command streams never panic; invariants and phase
//! monotonicity hold across a long run.

mod common;
use idaptik_core::scenario::ActionKind;
use idaptik_core::scenario::command::{Button, Buttons, Command, fold};
use idaptik_core::scenario::common::Phase;
use idaptik_core::scenario::{GhostLobbySim, RunConfig, ghost_lobby};
use proptest::prelude::*;

fn cmds_for(code: u8) -> Vec<Command> {
    let mut out = Vec::new();
    match code % 12 {
        0 => out.push(Command::SetButton {
            button: Button::Left,
            down: true,
        }),
        1 => out.push(Command::SetButton {
            button: Button::Right,
            down: true,
        }),
        2 => out.push(Command::SetButton {
            button: Button::Left,
            down: false,
        }),
        3 => out.push(Command::SetButton {
            button: Button::Right,
            down: false,
        }),
        4 => out.push(Command::SetButton {
            button: Button::Crouch,
            down: true,
        }),
        5 => out.push(Command::SetButton {
            button: Button::Sprint,
            down: true,
        }),
        6 => out.push(Command::Jump),
        7 => out.push(Command::Interact),
        8 => out.push(Command::Uplink {
            kind: ActionKind::Camera,
        }),
        9 => out.push(Command::Uplink {
            kind: ActionKind::Lights,
        }),
        10 => out.push(Command::Uplink {
            kind: ActionKind::Door,
        }),
        _ => out.push(Command::Uplink {
            kind: ActionKind::Vacuum,
        }),
    }
    out
}

fn phase_rank(p: Phase) -> u8 {
    match p {
        Phase::Quiet => 0,
        Phase::Crisis => 1,
        Phase::Result => 2,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    #[test]
    fn random_streams_never_panic(seed in any::<u32>(), codes in proptest::collection::vec(any::<u8>(), 0..64)) {
        let cfg = RunConfig::standard();
        let mut sim = GhostLobbySim::new(ghost_lobby(), cfg, seed).expect("valid def");
        let _ = sim.drain_events();
        // Pivot in first, or every uplink in the stream is denied on route and the
        // fuzz never reaches the effects it exists to shake. The graph derives from
        // the definition alone, so this lands on every seed — through the canonical
        // command path, spending the run's first tick.
        let mut held = Buttons::default();
        {
            let input = fold(&[Command::Pivot { target: idaptik_core::scenario::command::PivotTarget::Bridge }], &mut held);
            let events = sim.tick(&input);
            prop_assert!(
                events.iter().any(|e| matches!(e, idaptik_core::scenario::event::Event::PivotOpened { .. })),
                "the van can reach the maintenance bridge"
            );
        }
        let mut last_phase = phase_rank(sim.state().phase);
        for i in 0..3600u64 {
            let code = codes.get((i as usize) % codes.len().max(1)).copied().unwrap_or(0);
            let cmds = if codes.is_empty() { Vec::new() } else { cmds_for(code.wrapping_add(i as u8)) };
            let input = fold(&cmds, &mut held);
            let _ = sim.tick(&input);

            let s = sim.state();
            // Meters stay in bounds.
            prop_assert!((0.0..=100.0).contains(&s.bandwidth));
            prop_assert!((0.0..=100.0).contains(&s.alert));
            prop_assert!(s.support >= 0.0 && s.support <= 1.0 + 1e-9);
            prop_assert!(s.isolation >= 0.0);
            // Phase is monotonic: Quiet -> Crisis -> Result.
            let rank = phase_rank(s.phase);
            prop_assert!(rank >= last_phase, "phase went backwards");
            last_phase = rank;
            // Ended is terminal.
            if s.ended {
                prop_assert_eq!(s.phase, Phase::Result);
            }
        }
    }
}
