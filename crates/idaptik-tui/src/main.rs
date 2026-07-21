//! `idaptik-tui`: the ratatui/crossterm evaluation frontend for the Ghost Lobby
//! scenario, plus TTY-free `--headless`, `--replay` and `--export` verifiers over
//! `idaptik-core`.

mod app;
mod export;
mod net;
mod replay;

use idaptik_tui::{config, headless};

use clap::Parser;
use export::ExportKind;
use std::path::PathBuf;
use std::process::ExitCode;

/// The mode of operation, chosen by subcommand.
#[derive(Debug, clap::Subcommand)]
enum Mode {
    /// Explore the grounded network from a chosen vantage.
    Net {
        /// Vantage: inside | van | base.
        #[arg(long, default_value = "inside")]
        vantage: String,
    },
}

/// Command-line interface.
#[derive(Debug, Parser)]
#[command(
    name = "idaptik-tui",
    about = "Interactive TUI + headless/replay/export verifier for the Ghost Lobby scenario"
)]
struct Cli {
    /// Subcommand selecting an alternative mode (defaults to the Ghost Lobby TUI).
    #[command(subcommand)]
    mode: Option<Mode>,
    /// Run a script headlessly (needs --script) and print the JSON result.
    #[arg(long)]
    headless: bool,
    /// A script/replay file.
    #[arg(long, value_name = "FILE")]
    script: Option<PathBuf>,
    /// Re-run a script and verify determinism (PASS/FAIL, exit code).
    #[arg(long, value_name = "FILE")]
    replay: Option<PathBuf>,
    /// Print a JSON export surface (optionally after running --script).
    #[arg(long, value_enum)]
    export: Option<ExportKind>,
    /// Difficulty: story | standard | operator.
    #[arg(long, default_value = "standard")]
    difficulty: String,
    /// Run seed.
    #[arg(long, default_value_t = 123456)]
    seed: u32,
    /// Shorten the lights-flicker window (the only reduced-motion sim effect).
    #[arg(long)]
    reduced_motion: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Some(Mode::Net { vantage }) = &cli.mode {
        let v = match vantage.as_str() {
            "van" => idaptik_core::netsim::VantageKind::Van,
            "base" => idaptik_core::netsim::VantageKind::Base,
            _ => idaptik_core::netsim::VantageKind::Inside,
        };
        return match crate::net::run(v) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => fail(&e.to_string()),
        };
    }

    if let Some(path) = &cli.replay {
        return match replay::run(path) {
            Ok(true) => ExitCode::SUCCESS,
            Ok(false) => ExitCode::FAILURE,
            Err(e) => fail(&e),
        };
    }

    if let Some(kind) = cli.export {
        return match resolve_cfg(&cli) {
            Ok((cfg, seed)) => match export::run(kind, cli.script.as_deref(), cfg, seed) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => fail(&e),
            },
            Err(e) => fail(&e),
        };
    }

    if cli.headless {
        let Some(path) = &cli.script else {
            return fail("--headless requires --script FILE");
        };
        return match headless::run(path) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => fail(&e),
        };
    }

    // Default: interactive TUI.
    match resolve_cfg(&cli) {
        Ok((cfg, seed)) => match app::run(cfg, seed) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => fail(&e.to_string()),
        },
        Err(e) => fail(&e),
    }
}

fn resolve_cfg(cli: &Cli) -> Result<(idaptik_core::RunConfig, u32), String> {
    let diff = config::parse_difficulty(&cli.difficulty)?;
    Ok((config::run_config(diff, cli.reduced_motion), cli.seed))
}

fn fail(msg: &str) -> ExitCode {
    eprintln!("error: {msg}");
    ExitCode::FAILURE
}
