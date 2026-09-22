//! Floating-point helpers with **JavaScript operator semantics**.
//!
//! The prototype is JS, so a faithful port must reproduce quirks the Rust std
//! library does not share by default (ADR-0004):
//!
//! * [`js_round`] is `floor(x + 0.5)` — half rounds toward `+Inf`, unlike
//!   [`f64::round`] which rounds half away from zero.
//! * [`sign_or`] models `Math.sign(a || b)`.
//! * [`sin`] / [`powf`] go through `libm` so the camera sweep and USB drag are
//!   byte-identical across x86-64 / aarch64 / wasm targets (the ephapax and
//!   typed-wasm builds target wasm; std intrinsics would drift the goldens).
//!
//! Time is **accumulated** (`t += TICK_DT` each tick), never computed as
//! `tick as f64 / 60.0`, so rounding does not drift over a long run.

/// The fixed simulation timestep: 60 Hz.
pub const TICK_DT: f64 = 1.0 / 60.0;

/// Clamp `v` into `[lo, hi]`, matching JS `Math.min(Math.max(v, lo), hi)`.
#[inline]
pub fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    v.max(lo).min(hi)
}

/// Linear interpolation `a + (b - a) * t`.
#[inline]
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Move `v` toward `target` by at most `amt`, never overshooting.
#[inline]
pub fn approach(v: f64, target: f64, amt: f64) -> f64 {
    if v < target {
        (v + amt).min(target)
    } else {
        (v - amt).max(target)
    }
}

/// One-dimensional distance — `Math.abs(a - b)`.
#[inline]
pub fn dist(a: f64, b: f64) -> f64 {
    (a - b).abs()
}

/// `Math.round` for non-negative *and* negative halves: `floor(x + 0.5)`.
///
/// This differs from [`f64::round`]: `js_round(-0.5) == 0.0` (toward `+Inf`)
/// whereas `(-0.5f64).round() == -1.0` (away from zero).
#[inline]
pub fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// `Math.sign`: `-1`, `0`, or `+1` (NaN maps to `0`).
#[inline]
pub fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// `Math.sign(a || b)`: the sign of `a`, or of `b` when `a` is zero.
#[inline]
pub fn sign_or(a: f64, b: f64) -> f64 {
    if a != 0.0 { sign(a) } else { sign(b) }
}

/// `Math.sin` via `libm` (cross-target byte-identical).
#[inline]
pub fn sin(x: f64) -> f64 {
    libm::sin(x)
}

/// `Math.pow` via `libm` (cross-target byte-identical).
#[inline]
pub fn powf(base: f64, exp: f64) -> f64 {
    libm::pow(base, exp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_round_is_half_to_plus_infinity() {
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-0.5), 0.0);
        assert_eq!(js_round(0.5), 1.0);
        assert_eq!(js_round(-1.5), -1.0);
        assert_eq!(js_round(1.4999999), 1.0);
    }

    #[test]
    fn sign_or_models_logical_or() {
        assert_eq!(sign_or(0.0, -1.0), -1.0);
        assert_eq!(sign_or(3.0, -1.0), 1.0);
        assert_eq!(sign_or(0.0, 0.0), 0.0);
    }

    #[test]
    fn approach_does_not_overshoot() {
        assert_eq!(approach(0.0, 1.0, 10.0), 1.0);
        assert_eq!(approach(5.0, 0.0, 2.0), 3.0);
        assert_eq!(approach(1.0, 1.0, 2.0), 1.0);
    }

    #[test]
    fn clamp_and_lerp() {
        assert_eq!(clamp(5.0, 0.0, 1.0), 1.0);
        assert_eq!(clamp(-5.0, 0.0, 1.0), 0.0);
        assert_eq!(lerp(19.0, 26.0, 0.5), 22.5);
    }
}
