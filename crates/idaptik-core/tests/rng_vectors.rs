//! The `mulberry32` port must reproduce the hand-verified seed-123456 vectors
//! bit-for-bit — the whole determinism story rests on it.

use idaptik_core::Mulberry32;

#[test]
fn seed_123456_u32_vector() {
    let mut r = Mulberry32::new(123456);
    let got = [r.next_u32(), r.next_u32(), r.next_u32(), r.next_u32()];
    assert_eq!(got, [1642107918, 3424218114, 4280064779, 687244953]);
}

#[test]
fn seed_123456_f64_vector() {
    let mut r = Mulberry32::new(123456);
    let got: Vec<f64> = (0..5).map(|_| r.next_f64()).collect();
    let want = [
        0.38233304349705577,
        0.7972629074938595,
        0.9965302373748273,
        0.16001168475486338,
        0.20857197884470224,
    ];
    assert_eq!(got.as_slice(), want.as_slice());
}

#[test]
fn edge_seeds_stay_in_unit_interval() {
    for seed in [0u32, 1, 2, u32::MAX] {
        let mut r = Mulberry32::new(seed);
        for _ in 0..10_000 {
            let v = r.next_f64();
            assert!((0.0..1.0).contains(&v), "seed {seed} produced {v}");
        }
    }
}

#[test]
fn state_serializes_and_resumes_the_sequence() {
    let mut r = Mulberry32::new(123456);
    let _ = r.next_u32();
    let _ = r.next_u32();
    let json = serde_json::to_string(&r).unwrap();
    let mut restored: Mulberry32 = serde_json::from_str(&json).unwrap();
    // The restored generator continues the identical sequence.
    let mut original = r.clone();
    for _ in 0..8 {
        assert_eq!(restored.next_u32(), original.next_u32());
    }
}
