//! Render interpolation: buffer semantics, endpoint exactness, and the
//! discontinuity rule.
//!
//! These tests guard properties that are invisible in a screenshot but obvious
//! in motion — an inexact endpoint pops once per tick, a mishandled restart
//! slides the actor across the level.

mod common;

use common::Runner;
use idaptik_core::interp::{
    Blend, Blending, CHANNELS, DoubleBuffer, MAX_DOORS, N_VISUAL_SLOTS, Pose, PoseBuffer,
    VisualSlot, door_opens_of, lerp, poses_of,
};
use idaptik_core::scenario::command::{Button, Command};

fn pose(x: f64, y: f64, facing: f64) -> Pose {
    Pose { x, y, facing }
}

/// A buffer whose two ticks differ, for sampling tests.
fn two_tick_buffer() -> PoseBuffer {
    let mut buf = PoseBuffer::new();
    let mut a = [Pose::default(); N_VISUAL_SLOTS];
    a[VisualSlot::Player.index()] = pose(0.1, 5.0, -1.0);
    let mut b = [Pose::default(); N_VISUAL_SLOTS];
    b[VisualSlot::Player.index()] = pose(0.3, 9.0, 1.0);
    buf.commit(&a);
    buf.commit(&b);
    buf
}

// ── endpoint exactness ──────────────────────────────────────────────────────

#[test]
fn alpha_endpoints_are_bit_exact() {
    let buf = two_tick_buffer();
    let slot = VisualSlot::Player.index();

    let at_zero = buf.sample(slot, 0.0);
    let at_one = buf.sample(slot, 1.0);

    // Bit-exact, not approximate. `assert_eq!` on f64 is deliberate here: the
    // naive lerp `prev + (curr - prev) * alpha` yields 0.30000000000000004 for
    // these inputs at alpha = 1.0, which pops visibly every tick boundary.
    assert_eq!(at_zero.x, 0.1, "alpha 0 must return prev exactly");
    assert_eq!(at_zero.y, 5.0);
    assert_eq!(at_one.x, 0.3, "alpha 1 must return curr exactly");
    assert_eq!(at_one.y, 9.0);
}

#[test]
fn computing_the_endpoint_would_be_inexact() {
    // Why `lerp` clamps the endpoints instead of computing them. Both of these
    // are real f64 results, not hypotheticals — measured before being written
    // down. If someone removes the clamp, this is what ships.
    for (prev, curr) in [(-5.5_f64, 3.3_f64), (1e16, 1.0)] {
        let computed = prev + (curr - prev) * 1.0;
        assert_ne!(
            computed, curr,
            "expected {prev} → {curr} to be inexact when computed at alpha 1"
        );
        assert_eq!(lerp(prev, curr, 1.0), curr, "but lerp must be exact");
    }
}

#[test]
fn a_stationary_value_does_not_jitter() {
    // Why the interior uses `prev + (curr - prev) * alpha` rather than the
    // symmetric `prev * (1 - alpha) + curr * alpha`: the latter returns
    // 42.00000000000001 here, so a motionless object shimmers every frame.
    for step in 1..10 {
        let alpha = f64::from(step) / 10.0;
        assert_eq!(lerp(42.0, 42.0, alpha), 42.0, "jitter at alpha {alpha}");
    }
}

#[test]
fn alpha_is_clamped_not_extrapolated() {
    let buf = two_tick_buffer();
    let slot = VisualSlot::Player.index();

    assert_eq!(
        buf.sample(slot, -0.5).x,
        0.1,
        "negative alpha clamps to prev"
    );
    assert_eq!(buf.sample(slot, 2.0).x, 0.3, "alpha > 1 clamps to curr");
    assert_eq!(
        buf.sample(slot, f64::NAN).x,
        0.3,
        "NaN must not extrapolate"
    );
}

#[test]
fn midpoint_is_between_the_ticks() {
    let buf = two_tick_buffer();
    let mid = buf.sample(VisualSlot::Player.index(), 0.5);
    assert!(
        (mid.y - 7.0).abs() < 1e-12,
        "5 → 9 at alpha 0.5 is 7, got {}",
        mid.y
    );
    assert!(mid.x > 0.1 && mid.x < 0.3);
}

// ── the channel contract ────────────────────────────────────────────────────

#[test]
fn facing_snaps_and_never_passes_through_zero() {
    let buf = two_tick_buffer();
    let slot = VisualSlot::Player.index();

    // prev.facing = -1.0, curr.facing = +1.0. Lerping crosses 0 at alpha 0.5,
    // which renders as the actor facing neither way mid-turn.
    for step in 0..=10 {
        let alpha = f64::from(step) / 10.0;
        let f = buf.sample(slot, alpha).facing;
        assert_eq!(f, 1.0, "facing must snap to curr at alpha {alpha}");
    }
}

#[test]
fn channel_table_matches_the_implemented_behaviour() {
    // The table is documentation only if nothing checks it against the code.
    let facing = CHANNELS.iter().find(|c| c.name == "pose.facing").unwrap();
    assert_eq!(facing.blending, Blending::Snap);

    let x = CHANNELS.iter().find(|c| c.name == "pose.x").unwrap();
    assert_eq!(x.blending, Blending::Lerp);

    let blended = Pose::blend(pose(0.0, 0.0, -1.0), pose(10.0, 0.0, 1.0), 0.5);
    assert_eq!(blended.facing, 1.0, "declared Snap, so it must snap");
    assert!(
        blended.x > 0.0 && blended.x < 10.0,
        "declared Lerp, so it must lerp"
    );
}

// ── buffer semantics ────────────────────────────────────────────────────────

#[test]
fn a_stationary_sim_renders_stationary() {
    let mut buf = PoseBuffer::new();
    let mut state = [Pose::default(); N_VISUAL_SLOTS];
    state[VisualSlot::Billy.index()] = pose(42.0, 7.0, 1.0);

    buf.commit(&state);
    buf.commit(&state);

    let slot = VisualSlot::Billy.index();
    for step in 0..=10 {
        let alpha = f64::from(step) / 10.0;
        assert_eq!(buf.sample(slot, alpha).x, 42.0, "no drift at alpha {alpha}");
    }
}

#[test]
fn commit_shifts_curr_into_prev() {
    let mut buf = PoseBuffer::new();
    let slot = VisualSlot::Usb.index();

    let mut a = [Pose::default(); N_VISUAL_SLOTS];
    a[slot] = pose(1.0, 0.0, 0.0);
    let mut b = [Pose::default(); N_VISUAL_SLOTS];
    b[slot] = pose(2.0, 0.0, 0.0);

    buf.commit(&a);
    buf.commit(&b);

    assert_eq!(buf.prev(slot).x, 1.0);
    assert_eq!(buf.curr(slot).x, 2.0);
}

#[test]
fn prime_seeds_both_ticks() {
    let mut buf = PoseBuffer::new();
    let mut state = [Pose::default(); N_VISUAL_SLOTS];
    state[VisualSlot::Player.index()] = pose(100.0, 50.0, 1.0);
    buf.prime(&state);

    let slot = VisualSlot::Player.index();
    assert_eq!(buf.prev(slot).x, 100.0);
    assert_eq!(buf.sample(slot, 0.0).x, 100.0, "no slide from the origin");
}

// ── the discontinuity rule ──────────────────────────────────────────────────

#[test]
fn snap_after_commit_leaves_nothing_to_interpolate() {
    // The correct restart sequence: commit the fresh state, THEN snap.
    let mut buf = PoseBuffer::new();
    let slot = VisualSlot::Player.index();

    let mut far = [Pose::default(); N_VISUAL_SLOTS];
    far[slot] = pose(900.0, 0.0, 1.0);
    let mut spawn = [Pose::default(); N_VISUAL_SLOTS];
    spawn[slot] = pose(10.0, 0.0, 1.0);

    buf.commit(&far);
    buf.commit(&spawn);
    buf.snap();

    for step in 0..=10 {
        let alpha = f64::from(step) / 10.0;
        assert_eq!(
            buf.sample(slot, alpha).x,
            10.0,
            "must sit at spawn for every alpha, got a slide at {alpha}"
        );
    }
}

#[test]
fn snap_alone_would_lose_the_fresh_state() {
    // This is the bug the rule exists to prevent, pinned so a future
    // "simplification" to `snap()` instead of `commit(); snap()` fails here
    // rather than shipping a visible jump.
    let mut buf = PoseBuffer::new();
    let slot = VisualSlot::Player.index();

    let mut far = [Pose::default(); N_VISUAL_SLOTS];
    far[slot] = pose(900.0, 0.0, 1.0);
    buf.commit(&far);

    // Wrong: snapping without committing the spawn state first.
    buf.snap();

    assert_eq!(
        buf.sample(slot, 1.0).x,
        900.0,
        "snap alone keeps the pre-restart position — hence commit-then-snap"
    );
}

// ── zero allocation ─────────────────────────────────────────────────────────

#[test]
fn buffers_are_inline_fixed_size_storage() {
    use std::mem::size_of;

    // Exactly two arrays of N, no pointer indirection, nothing heap-allocated.
    assert_eq!(
        size_of::<PoseBuffer>(),
        2 * N_VISUAL_SLOTS * size_of::<Pose>(),
        "PoseBuffer must be two inline arrays and nothing else"
    );
    assert_eq!(
        size_of::<DoubleBuffer<f64, MAX_DOORS>>(),
        2 * MAX_DOORS * size_of::<f64>()
    );
}

// ── the game-specific mapping, against a real sim ───────────────────────────

#[test]
fn poses_track_the_simulation() {
    let mut runner = Runner::standard();
    let before = poses_of(runner.sim.state());

    // Hold right long enough to move measurably.
    for _ in 0..30 {
        runner.step(&[Command::SetButton {
            button: Button::Right,
            down: true,
        }]);
    }
    let after = poses_of(runner.sim.state());

    let slot = VisualSlot::Player.index();
    assert!(
        after[slot].x > before[slot].x,
        "player should have moved right: {} → {}",
        before[slot].x,
        after[slot].x
    );
}

#[test]
fn interpolating_two_real_ticks_stays_between_them() {
    let mut runner = Runner::standard();
    for _ in 0..20 {
        runner.step(&[Command::SetButton {
            button: Button::Right,
            down: true,
        }]);
    }

    let mut buf = PoseBuffer::new();
    buf.prime(&poses_of(runner.sim.state()));
    let prev_x = runner.sim.state().player.x;

    runner.step(&[Command::SetButton {
        button: Button::Right,
        down: true,
    }]);
    buf.commit(&poses_of(runner.sim.state()));
    let curr_x = runner.sim.state().player.x;

    let slot = VisualSlot::Player.index();
    let mid = buf.sample(slot, 0.5).x;

    let (lo, hi) = if prev_x <= curr_x {
        (prev_x, curr_x)
    } else {
        (curr_x, prev_x)
    };
    assert!(
        (lo..=hi).contains(&mid),
        "interpolated x {mid} escaped the interval [{lo}, {hi}]"
    );
}

#[test]
fn door_openness_maps_by_index() {
    let runner = Runner::standard();
    let opens = door_opens_of(runner.sim.state());
    let doors = &runner.sim.state().doors;

    assert!(doors.len() <= MAX_DOORS, "scenario fits the fixed buffer");
    for (i, door) in doors.iter().enumerate() {
        assert_eq!(opens[i], door.open, "door {i} openness must map by index");
    }
    for slot in opens.iter().skip(doors.len()) {
        assert_eq!(*slot, 0.0, "unused slots stay zero");
    }
}

#[test]
fn interpolation_never_touches_simulation_state() {
    // The load-bearing invariant: sampling is a pure read. Two identical runs,
    // one of which is sampled at every tick, must end byte-identical.
    let mut plain = Runner::standard();
    let mut sampled = Runner::standard();
    let mut buf = PoseBuffer::new();
    buf.prime(&poses_of(sampled.sim.state()));

    for _ in 0..120 {
        plain.step(&[Command::SetButton {
            button: Button::Right,
            down: true,
        }]);

        sampled.step(&[Command::SetButton {
            button: Button::Right,
            down: true,
        }]);
        buf.commit(&poses_of(sampled.sim.state()));
        // Sample across the whole interval, as a renderer would.
        for step in 0..=4 {
            let _ = buf.sample(VisualSlot::Player.index(), f64::from(step) / 4.0);
        }
    }

    assert_eq!(
        plain.sim.snapshot(),
        sampled.sim.snapshot(),
        "sampling perturbed the simulation — determinism is broken"
    );
}
