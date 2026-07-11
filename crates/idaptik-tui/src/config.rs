//! CLI → core run configuration.

use idaptik_core::RunConfig;
use idaptik_core::scenario::DifficultyId;

/// Parse a difficulty token (`story` / `standard` / `operator`).
pub fn parse_difficulty(s: &str) -> Result<DifficultyId, String> {
    s.parse()
}

/// Build a [`RunConfig`] from the resolved difficulty and reduced-motion flag.
pub fn run_config(difficulty: DifficultyId, reduced_motion: bool) -> RunConfig {
    RunConfig {
        difficulty,
        reduced_motion,
    }
}
