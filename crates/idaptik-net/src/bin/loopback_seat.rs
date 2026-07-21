//! One seat of the ADR-0006 §4 loopback gate.
//!
//! Joins `session:<id>` on a running relay over `PlainWebSocketTransport`,
//! plays one role of a headless script against the peer seat, and writes the
//! determinism artifact — the exact `HeadlessOutput` blob `idaptik-tui
//! --headless` prints — to `--out`. The orchestrator
//! (`scripts/loopback_check.sh`) byte-compares both seats' artifacts and the
//! reference runner's.
//!
//! Stdout is one line of metadata JSON (`role`, `status`, counters); artifacts
//! go only to `--out` so "compare the files" stays exactly that.

use clap::Parser;
use idaptik_net::envelope::Role;
use idaptik_net::phoenix::PhoenixClient;
use idaptik_net::session::{RunStatus, SessionConfig, SessionEnd, run_scripted_seat};
use idaptik_net::ws::PlainWebSocketTransport;
use idaptik_tui::headless::{self, HEADLESS_FORMAT, HeadlessOutput};
use serde_json::json;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

#[derive(Debug, Parser)]
#[command(
    name = "idaptik-loopback-seat",
    about = "One scripted seat of the two-player loopback slice (ADR-0006)"
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
    /// The headless script (both seats pass the same file).
    #[arg(long)]
    script: PathBuf,
    /// Where to write the determinism artifact (HeadlessOutput JSON).
    #[arg(long)]
    out: PathBuf,
    /// How long to wait for the peer before a clean no-peer end.
    #[arg(long, default_value_t = 15_000)]
    join_timeout_ms: u64,
    /// The PeerLost grace period / collect silence threshold.
    #[arg(long, default_value_t = 5_000)]
    grace_ms: u64,
    /// Test hook: die abruptly after sending N commands (exercises the peer's
    /// loss path). Exits 3 without closing the socket politely.
    #[arg(long, hide = true)]
    fail_after_seq: Option<u64>,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let role: Role = match cli.role.parse() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let script = match headless::load(&cli.script) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let transport = match PlainWebSocketTransport::connect(&cli.url).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut client = PhoenixClient::new(transport);
    let cfg = SessionConfig {
        session_id: cli.session,
        role,
        join_timeout: Duration::from_millis(cli.join_timeout_ms),
        grace: Duration::from_millis(cli.grace_ms),
        fail_after: cli.fail_after_seq,
    };

    match run_scripted_seat(&mut client, &cfg, &script).await {
        Ok(SessionEnd::DiedOnPurpose) => {
            // Abrupt on purpose: no leave, no close frame — the process just
            // stops existing, exactly like a crash the peer must survive.
            eprintln!("dying on purpose (--fail-after-seq)");
            std::process::exit(3);
        }
        Ok(SessionEnd::Run(run)) => {
            let meta = match run.status {
                RunStatus::Completed { peer_digest_match } => {
                    let artifact = HeadlessOutput {
                        format: HEADLESS_FORMAT,
                        event_log: run.event_log,
                        debrief: run.debrief,
                        final_snapshot: run.final_snapshot.expect("completed run has a snapshot"),
                    };
                    let mut text = match serde_json::to_string_pretty(&artifact) {
                        Ok(t) => t,
                        Err(e) => {
                            eprintln!("error: serialize artifact: {e}");
                            return ExitCode::FAILURE;
                        }
                    };
                    // Match `idaptik-tui --headless`'s println newline exactly.
                    text.push('\n');
                    if let Err(e) = std::fs::write(&cli.out, text) {
                        eprintln!("error: write {}: {e}", cli.out.display());
                        return ExitCode::FAILURE;
                    }
                    json!({
                        "role": role.as_str(),
                        "status": "completed",
                        "peer_digest_match": peer_digest_match,
                        "out": cli.out.display().to_string(),
                    })
                }
                RunStatus::PeerLost {
                    received,
                    expected,
                    peer_rejoined_within_grace,
                } => json!({
                    "role": role.as_str(),
                    "status": "ended_peer_lost",
                    "received": received,
                    "expected": expected,
                    "peer_rejoined_within_grace": peer_rejoined_within_grace,
                }),
                RunStatus::NoPeer => json!({
                    "role": role.as_str(),
                    "status": "ended_no_peer",
                }),
            };
            println!("{meta}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
