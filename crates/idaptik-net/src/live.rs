//! The paced async driver for a live seat: [`crate::lockstep::LockstepCore`]
//! carried over the real relay.
//!
//! The frame loop is `pump → drain-until-deadline → advance → frame`, where
//! the drain *is* the pacing sleep — waiting for broadcasts and waiting for
//! the next tick boundary are the same wait. Determinism never depends on
//! pacing (the unit tests prove the core over arbitrary delivery timing);
//! `tick` only decides how the run *feels*, which is why the loopback gate can
//! run it at 2 ms without weakening what it proves.
//!
//! Loss handling (ADR-0006 §3, live form): `peer_left`, or grace-length
//! silence while blocked on the peer's watermark, moves the seat to
//! `PeerLost`. Unlike the batch seat, a live seat then *waits*: if the peer
//! rejoins within `rejoin_window` (a fresh process joining with `rejoin:
//! true` in its hello), the survivor prunes uncommitted history, ships a
//! [`Resync`] — snapshot, event log, fold state, both seats' committed
//! pending commands — and play resumes as if the death never happened. The
//! rejoin can even be noticed before the loss (the rejoiner's hello arriving
//! mid-`Live`); the handling is the same.

use crate::envelope::{
    self, COMMIT_TAG, Commit, DIGEST_TAG, Digest, HELLO_TAG, Hello, RESYNC_TAG, Role,
    decode_command, encode_command,
};
use crate::error::NetError;
use crate::lockstep::{InputFeed, LockstepCore, Outgoing, Resync};
use crate::phoenix::PhoenixClient;
use crate::session::{await_peer_digest, push_control};
use crate::transport::SessionTransport;
use idaptik_core::Debrief;
use idaptik_core::scenario::event::Event;
use idaptik_core::scenario::{GhostLobbySim, RuntimeSnapshot};
use idaptik_tui::script::ScriptFile;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time::Instant;

/// Live-session parameters.
pub struct LiveConfig {
    /// Session identifier: the seat joins `session:<id>`.
    pub session_id: String,
    pub role: Role,
    /// Fresh seat: how long to wait for the peer's hello. Rejoining seat: how
    /// long to wait for the survivor's `net:resync`.
    pub join_timeout: Duration,
    /// Silence threshold while blocked on the peer's watermark — the local
    /// read on loss (`peer_left` is best-effort).
    pub grace: Duration,
    /// After loss: how long the survivor holds the run open for a rejoin.
    pub rejoin_window: Duration,
    /// The pacing interval (16 ms ≈ the sim's 60 Hz; tests shrink it).
    pub tick: Duration,
    /// Input-delay in ticks: input sampled at step `T` executes at `T +
    /// input_delay`. Per-seat local; seats with different delays interoperate.
    pub input_delay: u64,
    /// Test hook: die abruptly (caller exits the process) once this many
    /// steps have executed. Exercises the peer's loss + resync path for real.
    pub die_at_step: Option<u64>,
    /// This seat is re-entering a session it lost and expects a `net:resync`.
    pub rejoin: bool,
}

/// What the driver tells the frontend about the session, for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveStatus {
    WaitingForPeer,
    Live,
    PeerLost,
    Resyncing,
}

/// A live frontend: the input source plus the render/quit surface. One trait
/// because one struct owns both ends (the terminal frontend samples the
/// keyboard *and* draws; the scripted frontend serves a schedule and draws
/// nothing).
pub trait LiveFrontend: InputFeed {
    /// A frame boundary: react to freshly-executed events (often empty) and
    /// draw. Returning `false` quits politely (leave, clean end).
    fn frame(&mut self, sim: &GhostLobbySim, fresh: &[Event]) -> bool;
    /// The session changed state in a way worth surfacing.
    fn status(&mut self, status: LiveStatus);
}

/// A completed run's artifacts — the same blob the batch seat leaves behind.
pub struct LiveArtifacts {
    pub event_log: Vec<Event>,
    pub debrief: Option<Debrief>,
    pub final_snapshot: RuntimeSnapshot,
    pub peer_digest_match: Option<bool>,
}

/// How the live session ended.
pub enum LiveEnd {
    /// Ran to the end of the tick budget (or the sim's own end).
    Completed(Box<LiveArtifacts>),
    /// The peer was lost and did not rejoin within the window; ended cleanly.
    PeerGone { step: u64 },
    /// The frontend asked to quit.
    Quit { step: u64 },
    /// No peer (fresh seat) or no resync (rejoining seat) within the timeout.
    NoPeer,
    /// The `die_at_step` hook fired: the caller must exit abruptly — no
    /// leave, no close.
    DiedOnPurpose,
}

/// Run one live seat over an already-connected client.
pub async fn run_live_seat<T: SessionTransport, F: LiveFrontend>(
    client: &mut PhoenixClient<T>,
    cfg: &LiveConfig,
    script: &ScriptFile,
    fe: &mut F,
) -> Result<LiveEnd, NetError> {
    let mut hello = Hello::new(cfg.role, script, 0);
    hello.rejoin = cfg.rejoin;

    client
        .join(
            &format!("session:{}", cfg.session_id),
            json!({ "role": cfg.role.as_str() }),
        )
        .await?;
    push_control(client, &hello).await?;
    fe.status(LiveStatus::WaitingForPeer);

    // -- Bring-up: fresh hello barrier, or adopt the survivor's resync -------
    let mut core = if cfg.rejoin {
        match await_resync(client, cfg, &hello).await? {
            Some(resync) => LockstepCore::adopt_resync(cfg.role, cfg.input_delay, resync)?,
            None => return Ok(LiveEnd::NoPeer),
        }
    } else {
        match wait_for_peer_hello(client, cfg, &hello).await? {
            Some(peer) => {
                if peer.rejoin {
                    return Err(NetError::Session(
                        "peer expects a resync, but this seat has no run to resume".into(),
                    ));
                }
                if !hello.compatible_with(&peer) {
                    return Err(NetError::Session(format!(
                        "seats configured for different runs: mine {hello:?}, peer {peer:?}"
                    )));
                }
                LockstepCore::new(cfg.role, cfg.input_delay, script).map_err(NetError::Session)?
            }
            None => return Ok(LiveEnd::NoPeer),
        }
    };
    fe.status(LiveStatus::Live);

    // -- The frame loop ------------------------------------------------------
    let mut seq: u64 = 0;
    let mut peer_digest: Option<Digest> = None;
    let mut stalled_since: Option<Instant> = None;
    let mut deadline = Instant::now() + cfg.tick;

    while !core.finished() {
        // 1. Pump: sample local input up to the delay horizon and send it.
        for o in core.pump_outgoing(fe) {
            match o {
                Outgoing::Command { at, cmd } => {
                    seq += 1;
                    let payload = encode_command(&cmd, seq, at)?;
                    let reply = client.push("command", payload).await?;
                    if reply.get("relayed").and_then(Value::as_bool) != Some(true) {
                        return Err(NetError::Session(format!(
                            "relay refused command seq {seq}: {reply}"
                        )));
                    }
                }
                Outgoing::Commit { through } => {
                    let commit = Commit {
                        role: cfg.role,
                        through,
                    };
                    client.push("event", commit.to_control()).await?;
                }
            }
        }

        // 2. Drain broadcasts until the frame deadline (this is the pacing
        //    sleep too).
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match client.next_broadcast(remaining).await? {
                None => break,
                Some(b) => {
                    stalled_since = None;
                    match b.event.as_str() {
                        "command" => {
                            let (at, cmd) = decode_command(&b.payload)?;
                            core.on_peer_command(at, cmd)?;
                        }
                        "event" => match envelope::event_tag(&b.payload) {
                            Some(COMMIT_TAG) => {
                                if let Some(c) = Commit::from_control(&b.payload)
                                    && c.role != cfg.role
                                {
                                    core.on_peer_commit(c.through)?;
                                }
                            }
                            Some(DIGEST_TAG) => {
                                if let Ok(d) = serde_json::from_value::<Digest>(b.payload.clone())
                                    && d.role != cfg.role
                                {
                                    peer_digest = Some(d);
                                }
                            }
                            Some(HELLO_TAG) => {
                                // A rejoin-hello arriving mid-Live means the
                                // peer died and returned before we noticed.
                                if let Ok(h) = serde_json::from_value::<Hello>(b.payload.clone())
                                    && h.role != cfg.role
                                    && h.rejoin
                                {
                                    send_resync(client, cfg, &mut core, fe).await?;
                                }
                            }
                            _ => {}
                        },
                        "peer_left" => {
                            if !hold_for_rejoin(client, cfg, &mut core, &hello, fe).await? {
                                return Ok(LiveEnd::PeerGone { step: core.step() });
                            }
                            stalled_since = None;
                        }
                        // A joiner mid-run is the rejoiner arriving: re-send
                        // our hello so it can compat-check (its own hello
                        // triggers the resync above / in hold_for_rejoin).
                        "peer_joined" => push_control(client, &hello).await?,
                        _ => {}
                    }
                }
            }
        }
        let now = Instant::now();
        deadline = (deadline + cfg.tick).max(now);

        // 3. Advance every ready step.
        let mut fresh: Vec<Event> = Vec::new();
        core.advance_with(|_, events| fresh.extend_from_slice(events));
        if let Some(n) = cfg.die_at_step
            && core.step() >= n
        {
            return Ok(LiveEnd::DiedOnPurpose);
        }

        // 4. Frame boundary.
        if !fe.frame(core.sim(), &fresh) {
            client.leave().await;
            return Ok(LiveEnd::Quit { step: core.step() });
        }

        // 5. Loss by silence: blocked on the peer's watermark, and quiet for
        //    a whole grace period.
        if core.blocked_on_peer() {
            let since = *stalled_since.get_or_insert(now);
            if since.elapsed() >= cfg.grace {
                if !hold_for_rejoin(client, cfg, &mut core, &hello, fe).await? {
                    return Ok(LiveEnd::PeerGone { step: core.step() });
                }
                stalled_since = None;
            }
        } else {
            stalled_since = None;
        }
    }

    // -- Digest cross-check, then a polite exit ------------------------------
    let log_json = serde_json::to_string(core.log())
        .map_err(|e| NetError::Protocol(format!("digest: {e}")))?;
    let digest = Digest::new(cfg.role, &log_json);
    push_control(client, &digest).await?;
    let peer = match peer_digest {
        Some(d) => Some(d),
        None => await_peer_digest(client, cfg.role).await?,
    };
    let peer_digest_match = peer.map(|d| d.fnv1a64 == digest.fnv1a64);
    client.leave().await;

    let (event_log, debrief, final_snapshot) = core.into_artifacts();
    Ok(LiveEnd::Completed(Box::new(LiveArtifacts {
        event_log,
        debrief,
        final_snapshot,
        peer_digest_match,
    })))
}

/// Prune uncommitted peer history and ship the snapshot hand-off.
async fn send_resync<T: SessionTransport, F: LiveFrontend>(
    client: &mut PhoenixClient<T>,
    _cfg: &LiveConfig,
    core: &mut LockstepCore,
    fe: &mut F,
) -> Result<(), NetError> {
    fe.status(LiveStatus::Resyncing);
    core.on_peer_lost();
    let resync = core.make_resync();
    push_control(client, &resync).await?;
    fe.status(LiveStatus::Live);
    Ok(())
}

/// The peer is lost: hold the run open for `rejoin_window`, resyncing a
/// returning peer. `Ok(true)` means play resumes; `Ok(false)` means nobody
/// came back.
async fn hold_for_rejoin<T: SessionTransport, F: LiveFrontend>(
    client: &mut PhoenixClient<T>,
    cfg: &LiveConfig,
    core: &mut LockstepCore,
    own_hello: &Hello,
    fe: &mut F,
) -> Result<bool, NetError> {
    fe.status(LiveStatus::PeerLost);
    core.on_peer_lost();
    let deadline = Instant::now() + cfg.rejoin_window;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(false);
        }
        match client.next_broadcast(remaining).await {
            Ok(Some(b)) => match b.event.as_str() {
                // The rejoiner needs our hello for its compat check.
                "peer_joined" => push_control(client, own_hello).await?,
                "event" => {
                    if envelope::event_tag(&b.payload) == Some(HELLO_TAG)
                        && let Ok(h) = serde_json::from_value::<Hello>(b.payload.clone())
                        && h.role != cfg.role
                        && h.rejoin
                    {
                        send_resync(client, cfg, core, fe).await?;
                        return Ok(true);
                    }
                }
                _ => {}
            },
            Ok(None) | Err(NetError::Closed) => return Ok(false),
            Err(e) => return Err(e),
        }
    }
}

/// Fresh-seat barrier: wait for the peer's hello, re-sending ours whenever a
/// peer joins (a hello sent into an empty session is lost — the relay has no
/// history).
async fn wait_for_peer_hello<T: SessionTransport>(
    client: &mut PhoenixClient<T>,
    cfg: &LiveConfig,
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

/// Rejoining-seat bring-up: compat-check the survivor's hello when it
/// arrives, and wait for the `net:resync`.
async fn await_resync<T: SessionTransport>(
    client: &mut PhoenixClient<T>,
    cfg: &LiveConfig,
    own_hello: &Hello,
) -> Result<Option<Resync>, NetError> {
    let deadline = Instant::now() + cfg.join_timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(None);
        }
        let Some(b) = client.next_broadcast(remaining).await? else {
            return Ok(None);
        };
        // Everything that is not control traffic — in particular any live
        // `"command"` broadcast the survivor pushes between our join and its
        // resync — is deliberately dropped: the resync's pending sets are the
        // authoritative copy of that window, and consuming a command both
        // live *and* from the payload would double-count it.
        if b.event != "event" {
            continue;
        }
        match envelope::event_tag(&b.payload) {
            Some(HELLO_TAG) => {
                if let Ok(h) = serde_json::from_value::<Hello>(b.payload.clone())
                    && h.role != cfg.role
                    && !own_hello.compatible_with(&h)
                {
                    return Err(NetError::Session(format!(
                        "seats configured for different runs: mine {own_hello:?}, peer {h:?}"
                    )));
                }
            }
            Some(RESYNC_TAG) => {
                if let Ok(r) = serde_json::from_value::<Resync>(b.payload.clone())
                    && r.role != cfg.role
                {
                    return Ok(Some(r));
                }
            }
            _ => {}
        }
    }
}
