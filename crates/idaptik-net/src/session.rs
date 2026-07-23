//! The scripted-seat session state machine (ADR-0006 §3, batch form).
//!
//! States: `WaitingForPeer` → `Live` (stream + collect + run) → `Ended`, with
//! `PeerLost` as the loss path. Two properties are load-bearing:
//!
//! - **The hello exchange is a barrier.** A seat streams commands only after
//!   it holds the peer's `net:hello`. A hello sent into an empty session is
//!   lost (the relay has no history), so each seat re-sends its hello when
//!   `peer_joined` arrives; once both hellos have crossed, both seats are
//!   provably joined and nothing streamed after that point can be missed.
//! - **Loss detection is client-side.** The relay's `peer_left` is best-effort
//!   (an abrupt disconnect may skip `terminate/2`), so silence longer than the
//!   grace period while commands are still owed counts as loss too — that is
//!   ADR-0006 §3's "or transport loss detected locally", seen from the
//!   surviving side.
//!
//! v1 ends cleanly on loss (the issue's "reconnect or clean session end"):
//! a peer rejoining within grace is observed and recorded, but resync — the
//! surviving seat shipping its `RuntimeSnapshot` — belongs to the interactive
//! client slice, where there is a mid-run sim to resync.

use crate::envelope::{
    self, DIGEST_TAG, Digest, HELLO_TAG, Hello, Role, decode_command, encode_command,
};
use crate::error::NetError;
use crate::phoenix::PhoenixClient;
use crate::seat::{ScheduledCommand, merged_by_tick, seat_schedule, tick_inputs};
use crate::transport::SessionTransport;
use idaptik_core::Debrief;
use idaptik_core::scenario::RuntimeSnapshot;
use idaptik_core::scenario::event::Event;
use idaptik_tui::headless;
use idaptik_tui::script::ScriptFile;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time::Instant;

/// Session parameters. The transport is constructed by the caller and arrives
/// inside the [`PhoenixClient`] — swapping transports is a constructor choice
/// (ADR-0006 §2), invisible here.
pub struct SessionConfig {
    /// Session identifier: the seat joins `session:<id>`.
    pub session_id: String,
    pub role: Role,
    /// How long to wait in `WaitingForPeer` before a clean no-peer end.
    pub join_timeout: Duration,
    /// The `PeerLost` grace period, and the silence threshold while collecting.
    pub grace: Duration,
    /// Test hook: abruptly die (caller exits the process) after successfully
    /// pushing this many commands. Exercises the peer's loss path for real.
    pub fail_after: Option<u64>,
}

/// How the session ended.
#[derive(Debug)]
pub enum RunStatus {
    /// Full run completed. `peer_digest_match` is `None` when the peer's
    /// digest never arrived (it ended first) — the orchestrator's byte-diff
    /// of both artifacts remains the authoritative comparison.
    Completed { peer_digest_match: Option<bool> },
    /// The peer was lost mid-session; ended cleanly without a full run.
    PeerLost {
        received: u64,
        expected: u64,
        peer_rejoined_within_grace: bool,
    },
    /// No peer ever arrived.
    NoPeer,
}

/// The artifacts a seat leaves behind.
pub struct SessionRun {
    pub status: RunStatus,
    pub event_log: Vec<Event>,
    pub debrief: Option<Debrief>,
    pub final_snapshot: Option<RuntimeSnapshot>,
}

/// A finished call: either artifacts, or the instruction to die abruptly
/// (the `fail_after` hook must not unwind politely — no leave, no close).
pub enum SessionEnd {
    Run(Box<SessionRun>),
    DiedOnPurpose,
}

/// Run one scripted seat over an already-connected client.
pub async fn run_scripted_seat<T: SessionTransport>(
    client: &mut PhoenixClient<T>,
    cfg: &SessionConfig,
    script: &ScriptFile,
) -> Result<SessionEnd, NetError> {
    let schedule = seat_schedule(script, cfg.role);
    let hello = Hello::new(cfg.role, script, schedule.len() as u64);

    // -- Join, then the hello barrier (WaitingForPeer) -----------------------
    client
        .join(
            &format!("session:{}", cfg.session_id),
            json!({ "role": cfg.role.as_str() }),
        )
        .await?;
    push_control(client, &hello).await?;

    let peer_hello = match wait_for_peer_hello(client, cfg, &hello).await? {
        Some(h) => h,
        None => return Ok(SessionEnd::Run(Box::new(no_peer_run()))),
    };
    if !hello.compatible_with(&peer_hello) {
        return Err(NetError::Session(format!(
            "seats configured for different runs: mine {hello:?}, peer {peer_hello:?}"
        )));
    }

    // -- Live: stream our commands, then collect the peer's ------------------
    let mut sent: u64 = 0;
    for sc in &schedule {
        sent += 1;
        let payload = encode_command(&sc.cmd, sent, sc.at)?;
        let reply = client.push("command", payload).await?;
        if reply.get("relayed").and_then(Value::as_bool) != Some(true) {
            return Err(NetError::Session(format!(
                "relay refused command seq {sent}: {reply}"
            )));
        }
        if cfg.fail_after == Some(sent) {
            return Ok(SessionEnd::DiedOnPurpose);
        }
    }

    let peer_cmds = match collect_peer_commands(client, cfg, peer_hello.commands).await? {
        Collected::All(cmds) => cmds,
        Collected::Lost {
            got,
            peer_rejoined_within_grace,
        } => {
            let received = got.len() as u64;
            return Ok(SessionEnd::Run(Box::new(SessionRun {
                status: RunStatus::PeerLost {
                    received,
                    expected: peer_hello.commands,
                    peer_rejoined_within_grace,
                },
                event_log: Vec::new(),
                debrief: None,
                final_snapshot: None,
            })));
        }
    };

    // -- The deterministic run (both seats compute the same world) -----------
    let (infiltrator, hacker) = match cfg.role {
        Role::Infiltrator => (schedule, peer_cmds),
        Role::Hacker => (peer_cmds, schedule),
    };
    let merged = merged_by_tick(script.max_ticks, &infiltrator, &hacker);
    let mut sim = headless::build(script).map_err(NetError::Session)?;
    let mut log = sim.drain_events();
    for input in tick_inputs(&merged) {
        if sim.is_ended() {
            break;
        }
        log.extend(sim.tick(&input));
    }

    // -- Digest cross-check (drift tripwire), then a polite exit -------------
    let log_json =
        serde_json::to_string(&log).map_err(|e| NetError::Protocol(format!("digest: {e}")))?;
    let digest = Digest::new(cfg.role, &log_json);
    push_control(client, &digest).await?;
    let peer_digest_match = await_peer_digest(client, cfg.role)
        .await?
        .map(|d| d.fnv1a64 == digest.fnv1a64);

    client.leave().await;
    Ok(SessionEnd::Run(Box::new(SessionRun {
        status: RunStatus::Completed { peer_digest_match },
        debrief: sim.debrief().cloned(),
        final_snapshot: Some(sim.snapshot()),
        event_log: log,
    })))
}

/// Serialize a control message and push it on the event relay.
pub(crate) async fn push_control<T: SessionTransport, M: serde::Serialize>(
    client: &mut PhoenixClient<T>,
    msg: &M,
) -> Result<(), NetError> {
    let payload = serde_json::to_value(msg)
        .map_err(|e| NetError::Protocol(format!("encode control: {e}")))?;
    client.push("event", payload).await?;
    Ok(())
}

/// `WaitingForPeer`: until the peer's hello arrives or `join_timeout` elapses.
async fn wait_for_peer_hello<T: SessionTransport>(
    client: &mut PhoenixClient<T>,
    cfg: &SessionConfig,
    own_hello: &Hello,
) -> Result<Option<Hello>, NetError> {
    let deadline = Instant::now() + cfg.join_timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(None);
        }
        let Some(b) = client.next_broadcast(remaining).await? else {
            return Ok(None);
        };
        match b.event.as_str() {
            // The peer joined after us: it missed our join-time hello.
            "peer_joined" => push_control(client, own_hello).await?,
            "event" => {
                if envelope::event_tag(&b.payload) == Some(HELLO_TAG)
                    && let Ok(h) = serde_json::from_value::<Hello>(b.payload.clone())
                    && h.role != cfg.role
                {
                    return Ok(Some(h));
                }
            }
            _ => {}
        }
    }
}

enum Collected {
    All(Vec<ScheduledCommand>),
    Lost {
        got: Vec<ScheduledCommand>,
        peer_rejoined_within_grace: bool,
    },
}

/// Collect the peer's announced commands. `peer_left`, or silence longer than
/// the grace period while commands are still owed, is loss; after loss we
/// linger one grace period to *observe* a rejoin (v1 records it, ends cleanly).
async fn collect_peer_commands<T: SessionTransport>(
    client: &mut PhoenixClient<T>,
    cfg: &SessionConfig,
    expected: u64,
) -> Result<Collected, NetError> {
    let mut got = Vec::new();
    while (got.len() as u64) < expected {
        match client.next_broadcast(cfg.grace).await {
            Ok(Some(b)) => match b.event.as_str() {
                "command" => {
                    let (at, cmd) = decode_command(&b.payload)?;
                    got.push(ScheduledCommand { at, cmd });
                }
                "peer_left" => {
                    let rejoined = lingering_rejoin(client, cfg.grace).await?;
                    return Ok(Collected::Lost {
                        got,
                        peer_rejoined_within_grace: rejoined,
                    });
                }
                _ => {}
            },
            // Quiet for a whole grace period with commands still owed: the
            // local read on loss (peer_left is best-effort, ADR-0006 §3).
            Ok(None) => {
                return Ok(Collected::Lost {
                    got,
                    peer_rejoined_within_grace: false,
                });
            }
            // Our own pipe died while owed commands: also a clean loss end.
            Err(NetError::Closed) => {
                return Ok(Collected::Lost {
                    got,
                    peer_rejoined_within_grace: false,
                });
            }
            Err(e) => return Err(e),
        }
    }
    Ok(Collected::All(got))
}

/// After `peer_left`: watch one grace period for a rejoin.
async fn lingering_rejoin<T: SessionTransport>(
    client: &mut PhoenixClient<T>,
    grace: Duration,
) -> Result<bool, NetError> {
    let deadline = Instant::now() + grace;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(false);
        }
        match client.next_broadcast(remaining).await {
            Ok(Some(b)) if b.event == "peer_joined" => return Ok(true),
            Ok(Some(_)) => continue,
            Ok(None) | Err(NetError::Closed) => return Ok(false),
            Err(e) => return Err(e),
        }
    }
}

/// The peer's digest, if it arrives before the peer departs.
pub(crate) async fn await_peer_digest<T: SessionTransport>(
    client: &mut PhoenixClient<T>,
    own_role: Role,
) -> Result<Option<Digest>, NetError> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(None);
        }
        match client.next_broadcast(remaining).await {
            Ok(Some(b)) if b.event == "event" => {
                if envelope::event_tag(&b.payload) == Some(DIGEST_TAG)
                    && let Ok(d) = serde_json::from_value::<Digest>(b.payload.clone())
                    && d.role != own_role
                {
                    return Ok(Some(d));
                }
            }
            Ok(Some(_)) => continue,
            Ok(None) | Err(NetError::Closed) => return Ok(None),
            Err(e) => return Err(e),
        }
    }
}

fn no_peer_run() -> SessionRun {
    SessionRun {
        status: RunStatus::NoPeer,
        event_log: Vec::new(),
        debrief: None,
        final_snapshot: None,
    }
}
