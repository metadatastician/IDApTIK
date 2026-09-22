//! Moletaire's training-ground chiptune loop — as pure pattern data.
//!
//! Ported from the archive's `MoletaireMusic.res` with the Web Audio I/O
//! stripped: the 16-step square-wave melody and triangle-wave bass patterns
//! (~114 BPM, 16th-note steps) are data, and [`schedule_notes`] is the pure
//! look-ahead scheduler — it returns the [`NoteEvent`]s a frontend should
//! start inside the look-ahead window and advances the scheduler cursor.
//! No audio I/O, no wall clock: the caller supplies `current_time`.

use serde::{Deserialize, Serialize};

/// Loop tempo in beats per minute.
pub const BPM: f64 = 114.0;
/// Steps in the repeating pattern (16th notes, 4 beats).
pub const PATTERN_LENGTH: usize = 16;
/// Look-ahead window for scheduling, in seconds.
pub const SCHEDULE_AHEAD_TIME_SEC: f64 = 0.1;
/// Scheduler callback interval in milliseconds (frontend hint; the pure
/// scheduler itself is interval-agnostic).
pub const SCHEDULER_INTERVAL_MS: u32 = 25;
/// Melody note gain (archive square-wave voice).
pub const MELODY_GAIN: f64 = 0.08;
/// Bass note gain (archive triangle-wave voice).
pub const BASS_GAIN: f64 = 0.12;
/// Melody note duration, in steps (archive `secondsPerStep * 0.8`).
pub const MELODY_DURATION_STEPS: f64 = 0.8;
/// Bass note duration, in steps (archive `secondsPerStep * 1.5`).
pub const BASS_DURATION_STEPS: f64 = 1.5;

/// Seconds per beat (`60 / BPM`).
pub fn seconds_per_beat() -> f64 {
    60.0 / BPM
}

/// Seconds per 16th-note step.
pub fn seconds_per_step() -> f64 {
    seconds_per_beat() / 4.0
}

/// Melody pattern — note frequencies in Hz, `0.0` = rest. C major pentatonic
/// in a digging-themed rhythm, exactly the archive array.
pub const MELODY_NOTES: [f64; PATTERN_LENGTH] = [
    523.25, // C5
    0.0,    // rest
    659.26, // E5
    0.0,    // rest
    783.99, // G5
    698.46, // F5
    659.26, // E5
    0.0,    // rest
    523.25, // C5
    587.33, // D5
    659.26, // E5
    523.25, // C5
    440.00, // A4
    0.0,    // rest
    523.25, // C5
    0.0,    // rest
];

/// Bass pattern — octave-lower root notes, exactly the archive array.
pub const BASS_NOTES: [f64; PATTERN_LENGTH] = [
    130.81, // C3
    0.0, 0.0, 0.0, 164.81, // E3
    0.0, 0.0, 0.0, 130.81, // C3
    0.0, 146.83, // D3
    0.0, 110.00, // A2
    0.0, 0.0, 0.0,
];

/// Oscillator wave shape for a scheduled note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaveType {
    Square,
    Triangle,
}

/// One note the frontend should start (the pure analogue of the archive's
/// `playNote` call).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteEvent {
    /// Pattern step this note belongs to (`0..PATTERN_LENGTH`).
    pub step: usize,
    /// Absolute start time on the caller's clock, seconds.
    pub start_time: f64,
    /// Frequency in Hz.
    pub freq: f64,
    /// Note duration in seconds.
    pub duration: f64,
    /// Oscillator shape.
    pub wave: WaveType,
    /// Note gain.
    pub gain: f64,
}

/// Scheduler cursor: where the loop is and when the next step sounds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SchedulerState {
    /// Absolute time of the next unscheduled step, seconds.
    pub next_note_time: f64,
    /// Monotonic step counter (wrapped modulo [`PATTERN_LENGTH`] per step).
    pub current_step: u64,
}

impl SchedulerState {
    /// Start the loop at `start_time` on step 0 (the archive `start` reset).
    pub fn new(start_time: f64) -> SchedulerState {
        SchedulerState {
            next_note_time: start_time,
            current_step: 0,
        }
    }
}

/// The pure look-ahead scheduler (archive `scheduleNotes`): emits every note
/// whose step starts before `current_time + SCHEDULE_AHEAD_TIME_SEC`, advancing
/// the cursor one 16th-note step at a time. Melody notes are square wave at
/// 0.8-step duration; bass notes triangle at 1.5 steps. Rests emit nothing.
pub fn schedule_notes(state: &mut SchedulerState, current_time: f64) -> Vec<NoteEvent> {
    let mut out = Vec::new();
    let step_len = seconds_per_step();

    while state.next_note_time < current_time + SCHEDULE_AHEAD_TIME_SEC {
        let step = (state.current_step % PATTERN_LENGTH as u64) as usize;

        if let Some(&freq) = MELODY_NOTES.get(step)
            && freq > 0.0
        {
            out.push(NoteEvent {
                step,
                start_time: state.next_note_time,
                freq,
                duration: step_len * MELODY_DURATION_STEPS,
                wave: WaveType::Square,
                gain: MELODY_GAIN,
            });
        }

        if let Some(&freq) = BASS_NOTES.get(step)
            && freq > 0.0
        {
            out.push(NoteEvent {
                step,
                start_time: state.next_note_time,
                freq,
                duration: step_len * BASS_DURATION_STEPS,
                wave: WaveType::Triangle,
                gain: BASS_GAIN,
            });
        }

        state.next_note_time += step_len;
        state.current_step += 1;
    }

    out
}
