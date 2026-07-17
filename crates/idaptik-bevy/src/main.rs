//! `idaptik-bevy`: the Bevy evaluation frontend for the Ghost Lobby scenario
//! (ADR-0003) — a side-on 2.5D cross-section of the building over
//! [`idaptik_core`]'s `GhostLobbySim`, driven from the same `Command`/`Event`
//! wire API as the TUI.
//!
//! Usage: `cargo run -p idaptik-bevy [-- --seed N --difficulty story|standard|operator --reduced-motion]`
//!
//! On Linux, Bevy needs system libraries — install them with
//! `just bevy-linux-deps`.

use bevy::prelude::*;
use idaptik_bevy::FrontendPlugin;
use idaptik_bevy::driver::SimDriverPlugin;
use idaptik_core::RunConfig;
use idaptik_core::scenario::DifficultyId;

fn main() -> AppExit {
    let (cfg, seed) = match parse_args(std::env::args().skip(1)) {
        Ok(parsed) => parsed,
        Err(msg) => {
            eprintln!("error: {msg}");
            return AppExit::error();
        }
    };
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "IDApTIK — Envelope 001: Ghost Lobby".to_owned(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugins(SimDriverPlugin { cfg, seed })
        .add_plugins(FrontendPlugin)
        .run()
}

/// Parse `--seed N`, `--difficulty story|standard|operator` and
/// `--reduced-motion` (kept dependency-free; the TUI's clap surface is the
/// full-featured one).
fn parse_args(args: impl Iterator<Item = String>) -> Result<(RunConfig, u32), String> {
    let mut cfg = RunConfig::standard();
    let mut seed = 123456u32;
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seed" => {
                let v = args.next().ok_or("--seed needs a value")?;
                seed = v.parse().map_err(|_| format!("bad seed: {v}"))?;
            }
            "--difficulty" => {
                let v = args.next().ok_or("--difficulty needs a value")?;
                cfg.difficulty = match v.as_str() {
                    "story" => DifficultyId::Story,
                    "standard" => DifficultyId::Standard,
                    "operator" => DifficultyId::Operator,
                    other => return Err(format!("unknown difficulty: {other}")),
                };
            }
            "--reduced-motion" => cfg.reduced_motion = true,
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok((cfg, seed))
}
