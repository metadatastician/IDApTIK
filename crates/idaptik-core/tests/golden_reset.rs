//! The reset roll must reproduce the hand-verified init fixtures exactly, and
//! draw the RNG in the load-bearing order (with the Operator-only extra draw).

use idaptik_core::ghost_lobby;
use idaptik_core::scenario::rng::{Mulberry32, roll_init};
use idaptik_core::scenario::tuning::DifficultyId;

#[test]
fn standard_seed_123456_init_fixture() {
    let def = ghost_lobby();
    let mut rng = Mulberry32::new(123456);
    let roll = roll_init(&def, &mut rng, DifficultyId::Standard);

    assert_eq!(roll.note_x, 121.82197739277035);
    assert_eq!(roll.usb_x, 711.2934456393123);
    assert_eq!(roll.arrival, 25.97571166162379);
    assert_eq!(roll.stale_pulse, 3.060040896642022);
    assert_eq!(
        roll.door_delay,
        vec![
            0.3242859894223511,
            0.33641854885965583,
            0.45750674573238936,
            0.5239999126922339,
        ]
    );
    assert_eq!(roll.snack_x, 186.3118865666911);
}

#[test]
fn story_and_standard_draw_exactly_nine() {
    let def = ghost_lobby();
    for d in [DifficultyId::Story, DifficultyId::Standard] {
        let mut rng = Mulberry32::new(123456);
        let _ = roll_init(&def, &mut rng, d);
        let mut reference = Mulberry32::new(123456);
        for _ in 0..9 {
            reference.next_u32();
        }
        assert_eq!(rng, reference, "{d:?} should consume exactly 9 draws");
    }
}

#[test]
fn operator_draws_thirteen_via_short_circuit() {
    let def = ghost_lobby();
    let mut rng = Mulberry32::new(123456);
    let _ = roll_init(&def, &mut rng, DifficultyId::Operator);
    // 9 base draws + one extra per door (4) = 13.
    let mut reference = Mulberry32::new(123456);
    for _ in 0..13 {
        reference.next_u32();
    }
    assert_eq!(rng, reference);
}

#[test]
fn operator_penalty_matches_floor_pick() {
    // Independently reproduce the per-door penalty logic for seed 123456.
    let def = ghost_lobby();
    let sp = &def.spawn;
    let mut rng = Mulberry32::new(123456);
    // consume note, usb, arrival, stale.
    for _ in 0..4 {
        rng.next_f64();
    }
    let mut expected = Vec::new();
    for i in 0..def.doors.len() {
        let mut delay = sp.door_delay.0 + rng.next_f64() * sp.door_delay.1;
        let pick = (rng.next_f64() * 4.0).floor() as usize;
        if pick == i {
            delay += sp.operator_door_penalty;
        }
        expected.push(delay);
    }

    let mut rng2 = Mulberry32::new(123456);
    let roll = roll_init(&def, &mut rng2, DifficultyId::Operator);
    assert_eq!(roll.door_delay, expected);
}

#[test]
fn operator_snack_differs_from_standard() {
    let def = ghost_lobby();
    let std = {
        let mut r = Mulberry32::new(123456);
        roll_init(&def, &mut r, DifficultyId::Standard).snack_x
    };
    let op = {
        let mut r = Mulberry32::new(123456);
        roll_init(&def, &mut r, DifficultyId::Operator).snack_x
    };
    assert_ne!(std, op);
}
