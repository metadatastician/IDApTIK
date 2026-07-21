# ADR-0005: Session relay topology — relay-only lockstep over typed Command/Event

- Status: Accepted (vertical-slice milestone)
- Date: 2026-07-17

## Context

Issue #5 asks the Elixir session layer (ADR-0002) to carry the Ghost Lobby
scenario's typed `Command`/`Event` stream between the two asymmetric roles,
with `idaptik-core` remaining the sole gameplay authority. Two topologies were
on the table:

1. **Rust-authoritative host** — a headless Rust process runs `GhostLobbySim`
   per session; Elixir relays each role's `Command`s to it and broadcasts the
   resulting `Event`s. One true world; clients are dumb terminals.
2. **Relay-only** — Elixir fans `Command`s/`Event`s between the clients; each
   client runs its own deterministic sim in lockstep.

The issue frames this milestone as *vertical-slice de-risking*: prove the
homoiconic "interaction is data" design survives the network boundary.

## Decision

**Relay-only (option 2) for this slice.** `IdaptikServerWeb.SessionChannel`
relays the serde wire enums verbatim — `"command"` messages (tagged `"cmd"`)
and `"event"` messages (tagged `"event"`) pass through byte-preserving to the
other seat. Elixir reads exactly one key of each payload (the tag), for routing
and role enforcement; it interprets nothing else.

What the evidence in the codebase supports:

- **Lockstep is already guaranteed.** ADR-0004 made the sim deterministic and
  event-sourced by construction; `replay_determinism`, `snapshot_equivalence`
  and the RNG vector tests enforce it. Two sims fed the same seed and the same
  ordered `Command` stream *are* the same world — the relay only has to
  preserve order and bytes, which is what a Phoenix Channel topic does.
- **The seam was built for this.** `command::fold` documents itself as "the
  seam the Elixir session layer and the TUI frontend share": clients fold the
  merged command stream into `TickInput`s locally. No host process exists to
  reuse — `idaptik-ffi` is a C ABI for engine bindings, not a network daemon —
  so option 1 means building and *operating* a new surface (per-session OS
  processes or NIFs under Elixir supervision, stdin/stdout or socket framing,
  crash/restart semantics) before the slice proves anything.
- **The slice's question is the wire, not authority.** What needed de-risking
  is whether `Command`/`Event` survive the boundary unchanged. The shared
  fixture (`fixtures/session_relay/`, captured via `idaptik-tui --headless`)
  is asserted byte-identical on both sides: the Elixir channel test proves
  pass-through, the Rust test (`session_relay_fixture.rs`) proves each relayed
  value deserializes into the real types and re-serializes identically.

Role enforcement is a pure routing table over the `"cmd"` tag — body verbs
(`SetButton`/`Jump`/`Interact`/`ThrowUsb`) are infiltrator-side; uplink and
pivot verbs (`Uplink`/`Pivot`/`Unpivot` — the v2 additions) are hacker-side;
session/test immediates (`Pause`/`Restart`/`Force*`) are either seat. An
optional integer `seq` envelope key (stripped before relay, never part of the
Rust JSON) lets the channel acknowledge-and-drop duplicate or out-of-order
commands instead of relaying them.

## Trade-offs, honestly

Option 1 remains the better *end state*, and the issue is right to prefer it
for a shipped game:

- **Cheating.** In relay-only, each client computes its own truth; a modified
  client can lie about outcomes. Acceptable between two cooperating
  playtesters; unacceptable for open play.
- **Late join / reconnection.** With no server-held sim there is no snapshot to
  hand a reconnecting client; peers must exchange `snapshot()`/`restore()`
  blobs themselves.
- **Hidden information.** A lockstep peer receives the other role's commands
  and could derive state its role should not see. The Ghost Lobby's asymmetry
  is currently cooperative, so this is tolerable — but a competitive mode is
  not viable on this topology.
- **Drift detection.** Lockstep failures (a non-determinism bug, a version
  skew) surface as silent divergence. Mitigated, not solved, by the `"event"`
  cross-feed: clients can hash and compare event streams.

None of these block the vertical slice, and none are made worse later: the
wire protocol is topology-agnostic. Moving to a Rust-authoritative host changes
*who consumes* the `"command"` topic and *who publishes* `"event"` (a host
process joining the session as a third, privileged participant) — the payload
shapes, the role table, and every client stay as they are. The fixture tests
carry over unchanged. That migration gets its own ADR when it lands (expected
alongside matchmaking/presence, where a server-held world is needed anyway).

## Consequences

- Elixir stays logic-free: the diff greps clean of scoring, FSM, and tick math.
  The only game-adjacent knowledge in `server/` is the `Command` tag → seat
  table, which is protocol routing, not rules.
- Clients own simulation cost and pacing (tick cadence is theirs); the server
  scales by connection count, not by session CPU.
- The `seq` envelope is the seam where lockstep input-delay scheduling will
  attach when real clients arrive; today it only de-duplicates.
- `fixtures/session_relay/` is a cross-language contract: regenerate
  `events.json` with `idaptik-tui --headless --script
  fixtures/session_relay/capture_script.json` (take `event_log`) whenever the
  `Event` wire format changes, and keep `commands.json` covering the full
  `Command` alphabet (a Rust test enforces this).

## Amendment (2026-07-21): the `at` scheduling envelope

The consequence above predicted the `seq` envelope is "the seam where lockstep
input-delay scheduling will attach when real clients arrive". The first real
client (`crates/idaptik-net`) landed scheduling client-side as a second
envelope key instead: `"at"` — the lockstep tick a command is scheduled for.
Clients author and consume it; the relay's behaviour is unchanged (it strips
exactly `"seq"` and relays the rest verbatim), so `"at"` passes through
untouched, and serde's internally-tagged `Command` decoding ignores it.
Neither envelope key is ever part of the Rust `Command` JSON itself. Clients
also exchange `"event"`-relayed control messages namespaced `"net:*"`
(handshake and event-log digest — the drift-detection mitigation above, made
real); the namespace cannot collide with the sim's `Event` tags, and the relay
treats them as ordinary events.
