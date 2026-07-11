//! Cross-cutting runtime enums shared across the scenario.
//!
//! These stay hard `enum`s (not string ids) so the simulation matches on them
//! exhaustively — a totality property the SPARK-equivalent core relies on. All
//! derive serde so they round-trip through the JSON export surfaces.

use serde::{Deserialize, Serialize};

/// Mission phase. Monotonic: `Quiet -> Crisis -> Result`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
    Quiet,
    Crisis,
    Result,
}

/// Billy's finite-state-machine mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BillyMode {
    Offsite,
    Entering,
    Shock,
    Assess,
    Investigate,
    Secure,
    Guard,
    CallBoss,
    Pursue,
}

/// The two physical objectives Billy can fixate on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectKind {
    Note,
    Usb,
}

/// What Billy reports to his boss.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReportedTarget {
    Note,
    Usb,
    Intruder,
}

/// How the infiltrator got out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExtractMethod {
    ServiceExit,
    LaundryChute,
}

/// Why a mission failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FailReason {
    Caught,
    Partition,
    Lockdown,
}

/// What tipped the floor from quiet into crisis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CrisisReason {
    Timer,
    Usb,
    Test,
}

/// Terminal outcome of a run (the debrief's headline).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Outcome {
    Extracted,
    Caught,
    Partition,
    Lockdown,
}

/// Final grade band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Grade {
    S,
    A,
    B,
    C,
    D,
}

/// Debrief-tag tone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tone {
    Good,
    Warn,
    Bad,
}

/// Log-line severity (drives styling; `Billy` / `Hacker` are speaker styles).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Good,
    Warn,
    Bad,
    Billy,
    Hacker,
}

/// Which surface a log line belongs to. The canonical determinism diff compares
/// `Log` + `Telemetry`; `Tutorial` / `Prompt` are frontend-only and have zero
/// simulation effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Channel {
    Log,
    Tutorial,
    Prompt,
    Telemetry,
}

/// How the laundry chute was revealed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChuteMethod {
    Physical,
    Vacuum,
}

/// Why an uplink action was denied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DenyReason {
    Cooldown,
    Bandwidth,
    VacuumFallen,
}

/// Derived objective status for the ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectiveStatus {
    Open,
    Available,
    Done,
    Failed,
    Locked,
}
