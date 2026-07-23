//! The live two-player seat (issue #7, interactive-client slice).
//!
//! Interactive (`--interactive`): the real client — the `idaptik-tui` face
//! over delay-lockstep netplay, playing one role of a shared deterministic
//! run against the peer seat.
//!
//! Scripted (default): the same live pipeline — real-time pacing, watermarks,
//! loss, resync — fed from a headless script instead of a keyboard, so the
//! loopback gate can drive it in CI and byte-compare both seats' artifacts
//! against `idaptik-tui --headless`. Stdout is one line of metadata JSON;
//! artifacts go only to `--out`.

use clap::Parser;
use idaptik_core::scenario::GhostLobbySim;
use idaptik_core::scenario::command::Command;
use idaptik_core::scenario::event::Event;
use idaptik_net::envelope::Role;
use idaptik_net::interactive::TerminalFrontend;
use idaptik_net::live::{LiveConfig, LiveEnd, LiveFrontend, LiveStatus, run_live_seat};
use idaptik_net::lockstep::{InputFeed, ScriptFeed};
use idaptik_net::phoenix::PhoenixClient;
use idaptik_net::ws::PlainWebSocketTransport;
use idaptik_tui::headless::{self, HEADLESS_FORMAT, HeadlessOutput};
use serde_json::json;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

#[derive(Debug, Parser)]
#[command(
    name = "idaptik-netplay",
    about = "One live seat of the two-player slice: delay-lockstep over the relay (ADR-0006)"
)]
struct Cli {
    /// The relay's Phoenix socket endpoint.
    #[arg(long, default_value = "ws://127.0.0.1:4000/socket/websocket")]
    url: String,
    /// Session id: both seats must pass the same one.
    #[arg(long)]
    session: String,
    /// Seat role: infiltrator | hacker.
    #[arg(long)]
    role: String,
    /// The run config carrier — and, in scripted mode, the input source
    /// (both seats pass the same file).
    #[arg(long)]
    script: PathBuf,
    /// Where to write the determinism artifact (HeadlessOutput JSON) on a
    /// completed run.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Play interactively in the terminal instead of from the script.
    #[arg(long)]
    interactive: bool,
    /// Pacing interval in milliseconds (16 ≈ the sim's 60 Hz).
    #[arg(long, default_value_t = 16)]
    tick_ms: u64,
    /// Input delay in ticks: input sampled at step T executes at T + delay.
    #[arg(long, default_value_t = 3)]
    input_delay: u64,
    /// Fresh seat: wait this long for a peer. Rejoining seat: for the resync.
    #[arg(long, default_value_t = 15_000)]
    join_timeout_ms: u64,
    /// Silence threshold while blocked on the peer (loss detection).
    #[arg(long, default_value_t = 5_000)]
    grace_ms: u64,
    /// After a loss, hold the run open this long for a rejoin.
    #[arg(long, default_value_t = 15_000)]
    rejoin_window_ms: u64,
    /// Rejoin a session this seat lost: expect a net:resync instead of tick 0.
    #[arg(long)]
    rejoin: bool,
    /// Test hook: die abruptly once this many steps have executed (exit 3, no
    /// close — exercises the peer's loss + resync path for real).
    #[arg(long, hide = true)]
    die_at_step: Option<u64>,
}

/// The scripted frontend: a [`ScriptFeed`] with no face — frames pass, status
/// changes go to stderr for the orchestrator's logs.
struct ScriptFrontend {
    feed: ScriptFeed,
}

impl InputFeed for ScriptFrontend {
    fn commands_for(&mut self, at: u64) -> Vec<Command> {
        self.feed.commands_for(at)
    }
}

impl LiveFrontend for ScriptFrontend {
    fn frame(&mut self, _sim: &GhostLobbySim, _fresh: &[Event]) -> bool {
        true
    }

    fn status(&mut self, status: LiveStatus) {
        eprintln!("status: {status:?}");
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let role: Role = match cli.role.parse() {
        Ok(r) => r,
        Err(e) => return fail(&e),
    };
    let script = match headless::load(&cli.script) {
        Ok(s) => s,
        Err(e) => return fail(&e),
    };

    let transport = match PlainWebSocketTransport::connect(&cli.url).await {
        Ok(t) => t,
        Err(e) => return fail(&e.to_string()),
    };
    let mut client = PhoenixClient::new(transport);
    let cfg = LiveConfig {
        session_id: cli.session,
        role,
        join_timeout: Duration::from_millis(cli.join_timeout_ms),
        grace: Duration::from_millis(cli.grace_ms),
        rejoin_window: Duration::from_millis(cli.rejoin_window_ms),
        tick: Duration::from_millis(cli.tick_ms),
        input_delay: cli.input_delay,
        die_at_step: cli.die_at_step,
        rejoin: cli.rejoin,
    };

    let end = if cli.interactive {
        let mut fe = match TerminalFrontend::new(role) {
            Ok(fe) => fe,
            Err(e) => return fail(&format!("terminal: {e}")),
        };
        run_live_seat(&mut client, &cfg, &script, &mut fe).await
    } else {
        let mut fe = ScriptFrontend {
            feed: ScriptFeed::new(&script, role),
        };
        run_live_seat(&mut client, &cfg, &script, &mut fe).await
    };

    match end {
        Ok(LiveEnd::DiedOnPurpose) => {
            // Abrupt on purpose: no leave, no close frame — the process just
            // stops existing, exactly like a crash the peer must survive.
            eprintln!("dying on purpose (--die-at-step)");
            std::process::exit(3);
        }
        Ok(LiveEnd::Completed(run)) => {
            if let Some(out) = &cli.out {
                let artifact = HeadlessOutput {
                    format: HEADLESS_FORMAT,
                    event_log: run.event_log,
                    debrief: run.debrief,
                    final_snapshot: run.final_snapshot,
                };
                let mut text = match serde_json::to_string_pretty(&artifact) {
                    Ok(t) => t,
                    Err(e) => return fail(&format!("serialize artifact: {e}")),
                };
                // Match `idaptik-tui --headless`'s println newline exactly.
                text.push('\n');
                if let Err(e) = std::fs::write(out, text) {
                    return fail(&format!("write {}: {e}", out.display()));
                }
            }
            emit(
                &role,
                &json!({ "status": "completed", "rejoined": cli.rejoin,
                "peer_digest_match": run.peer_digest_match }),
            );
            ExitCode::SUCCESS
        }
        Ok(LiveEnd::PeerGone { step }) => {
            emit(&role, &json!({ "status": "ended_peer_lost", "step": step }));
            ExitCode::SUCCESS
        }
        Ok(LiveEnd::Quit { step }) => {
            emit(&role, &json!({ "status": "quit", "step": step }));
            ExitCode::SUCCESS
        }
        Ok(LiveEnd::NoPeer) => {
            emit(&role, &json!({ "status": "ended_no_peer" }));
            ExitCode::SUCCESS
        }
        Err(e) => fail(&e.to_string()),
    }
}

fn emit(role: &Role, extra: &serde_json::Value) {
    let mut meta = json!({ "role": role.as_str() });
    meta.as_object_mut()
        .expect("object literal")
        .extend(extra.as_object().expect("object literal").clone());
    println!("{meta}");
}

fn fail(msg: &str) -> ExitCode {
    eprintln!("error: {msg}");
    ExitCode::FAILURE
}
