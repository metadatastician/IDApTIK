# ADR-0006: gossamer host window + burble transport — the two-player windowed slice

- Status: Accepted (design; implementation deferred — see unblock conditions)
- Date: 2026-07-17

## Context

Issue #7 asks for the last layer of the vertical slice: wrap the Bevy Ghost
Lobby frontend in the **gossamer** host window and carry the Phoenix session
socket over **burble** transport, so two players launch a windowed build, join
the same `session:<id>`, and play the two roles against one shared
authoritative run. The issue is explicit that this is integration glue, the
churniest layer, and must land **last**.

Everything under it is now real:

- **The relay** (ADR-0005, `server/lib/idaptik_server_web/channels/session_channel.ex`)
  relays the typed `Command`/`Event` JSON verbatim over a Phoenix Channel with
  role enforcement, `peer_joined`/`peer_left` notifications, and an optional
  `seq` de-duplication envelope. Wire fixtures live in `fixtures/session_relay/`.
- **The frontend** (`crates/idaptik-bevy`) is a real Ghost Lobby renderer: a
  render-free `SimDriverPlugin` (`src/driver.rs`) steps `GhostLobbySim` at a
  fixed 60 Hz from a queued `Command` stream — the same wire API the TUI and
  the relay speak — and `tests/parity.rs` already drives it headless.
- **Determinism** (ADR-0004) makes relay-only lockstep sound: two sims fed the
  same seed and ordered `Command` stream are the same world.

The two external components are estate projects, and honesty about their
maturity is part of this record:

- **gossamer** (`metadatastician/gossamer`) is the estate's desktop shell —
  "Tauri/Electron-like FUI alternative". **The host runtime is gossamer, not
  Tauri; Tauri references estate-wide are stale** (idaptik-ums Recovery PR 4).
  Gossamer is at v0.3.1: cross-platform (WebKitGTK / WKWebView / WebView2),
  192 integration tests, packaged installers, async IPC
  (`gossamer_channel_bind_async`), backend→frontend streaming events
  (`gossamer_emit`), CSP enforcement, and Rust bindings. Its model is a **web
  frontend inside the OS webview** with an Ephapax/Zig native layer whose
  linear types make handle leaks and permission bypasses compile errors. It
  does **not** today offer native child-surface hosting (embedding a
  winit/wgpu swapchain inside a gossamer window) — that gap shapes the
  decision below.
- **burble** (`metadatastician/burble`) is the estate's self-hostable
  voice-first communications platform, self-assessed **pre-production (CRG
  grade C)**. Its embeddable client module (a `Burble.join`-style drop-in) is
  the declared adoption lever, and burble's ADR-0013 names IDApTIK as the
  dogfood-first target — but that embed API is not yet a shipped, versioned
  surface, and burble's QUIC transport is an optional NIF **disabled in the
  default build**. Known structural constraint, recorded in burble's ADR-0005:
  STUN/ICE hole-punching (`proven-stun`) **cannot reach offline peers** —
  hole-punching requires both peers coordinating through a rendezvous
  simultaneously, so "cold" reachability of a peer who is not already in a
  session is impossible by construction, not by implementation shortfall.

Neither runtime is consumable in this repo's build today. This ADR therefore
fixes the design and the seams so the layers underneath stop churning against
it, and defers implementation behind explicit unblock conditions.

## Decision

### 1. Layering: gossamer is the host shell around the Bevy build

The windowed slice ships as a **gossamer window hosting the Bevy Ghost Lobby
build**, not as Bevy's own winit window.

What gossamer hosting buys:

- **Resource-safe handles.** The window, the webview, the IPC channels, and —
  once the socket rides through the shell — the session connection are linear
  (`let!`) resources: leaking or double-using them is a compile error in the
  shell layer, exactly the failure class integration glue is worst at.
- **The estate shell standard.** One shell across IDApTIK, PanLL and future
  estate apps; compiler-enforced permissions and CSP rather than a config
  file; ~1 MB shell binary. Standardising here is what makes the stale Tauri
  references finally removable.
- **Room to grow.** The lobby/matchmaking/companion UI that ADR-0002 kept out
  of the game client has a natural home in the shell's web layer, talking to
  the game over gossamer's typed IPC instead of growing inside Bevy.

How the Bevy build actually gets inside a webview shell — the interface
assumption this ADR makes explicit:

- **Route A (design of record): Bevy compiled to `wasm32`**, rendered via
  WebGL2/WebGPU inside gossamer's webview, assets served from the bundle.
  This matches gossamer's *current* v0.3.1 interface (load content, IPC,
  streaming events) with no gossamer feature work, but moves the cost onto
  the game build: a second compilation target, wasm threading and audio
  restrictions, and webview input-latency risk that must be measured against
  the 60 Hz fixed tick.
- **Route B (fallback within gossamer): native child-surface hosting** — a
  gossamer API to parent a native winit/wgpu surface inside its window. This
  keeps the existing native build untouched but is a **gossamer feature that
  does not exist yet**; if gossamer ships it before the wasm route is proven,
  it is preferred, and this ADR gets a status note rather than a rewrite.

What it costs either way: gossamer becomes a launch-path dependency, and the
slice inherits its platform matrix. The mitigation is §5's fallback — the
plain-winit build never stops working, because `idaptik-bevy` keeps its own
`main.rs` entry and gossamer wraps the *build*, not the code.

### 2. Transport seam: a client-side `SessionTransport`, relay untouched

The Phoenix session socket rides burble in production and a plain WebSocket in
tests. The seam that makes both true is **client-side, in a new
`crates/idaptik-net`** — explicitly *not* in the relay:

- `SessionTransport`: a reliable, ordered, bidirectional text/binary message
  pipe with an async connect, send, receive, and close — nothing
  Phoenix-specific, nothing burble-specific.
- One **Phoenix Channels protocol client** (join / push / reply / heartbeat
  framing per the documented Channels wire protocol, ADR-0002) implemented
  *once* over `SessionTransport`. The typed `Command`/`Event` payloads and the
  `seq` envelope from ADR-0005 sit above this unchanged.
- Implementations:
  - `PlainWebSocketTransport` — `tokio-tungstenite` natively, the browser
    `WebSocket` under `wasm32`/Route A. This is the test transport and the
    fallback transport, forever.
  - `BurbleTransport` — the burble embed's reliable data channel carrying the
    same Phoenix frames. Burble terminates at a gateway co-located with the
    relay deployment (the reachable rendezvous, §3) and bridges to the
    relay's ordinary WebSocket endpoint.

The relay stays transport-agnostic by construction: `SessionChannel` already
reads exactly one key of each payload and speaks standard Phoenix framing over
whatever socket Bandit accepted. **No burble knowledge ever enters `server/`.**
Swapping burble for a plain socket is a constructor choice in the client;
the loopback test (§4) runs the same scripted session over both transports to
keep that swap honest.

Voice is *why* burble is in the stack at all — the co-op game wants game
traffic and voice on one connection, one deployment, one join. But voice is
out of scope for this slice: the contract here is only that the data channel
carries the session frames.

### 3. Session bring-up: the relay is the rendezvous

**The Phoenix relay is the rendezvous point.** Both clients dial *out* to a
reachable server; no client ever needs an inbound path to the other. This is a
structural choice, not a hedge: burble's own ADR-0005 records that
hole-punching cannot serve peers who are not simultaneously coordinating
through a rendezvous — an offline or not-yet-launched peer is unreachable by
STUN/ICE *by definition*. Peer-to-peer hole punching is therefore **out of
scope for session bring-up**, and nothing in this design assumes it later:
if burble adds P2P upgrade for live sessions, it slots under
`SessionTransport` without touching the join flow.

Join flow:

1. Client launches (gossamer shell → Bevy build), constructs its
   `SessionTransport`, connects to the rendezvous.
2. Joins `session:<id>` with `role: "infiltrator" | "hacker"` (the channel
   rejects anything else) and receives its join reply.
3. On the other seat's `peer_joined`, the seats perform the run handshake over
   the relayed stream: agree seed + `RunConfig` (fixture-style, as
   `fixtures/session_relay/capture_script.json` does), construct
   `GhostLobbySim` locally, and enter lockstep — commands out with monotone
   `seq`, peer commands and cross-fed events in.

Connection-loss handling is a small state machine per client, driven entirely
by events the relay already emits (`peer_joined`, `peer_left`, plus the
transport's own connect/disconnect edges):

- `WaitingForPeer` — joined, no peer yet (or peer never arrived). Sim not
  started.
- `Live` — both seats present, lockstep running.
- `PeerLost` — on `peer_left` (or transport loss detected locally): pause the
  sim (`Command::Pause` is already either-seat) and start a grace timer.
- From `PeerLost`:
  - peer rejoins within grace (`peer_joined`) → **resync**: the surviving seat
    sends its `RuntimeSnapshot` (the `snapshot()`/`restore()` blobs ADR-0005
    anticipated — there is no server-held sim to ask) over the relayed
    stream; both unpause → `Live`.
  - grace expires → **clean end**: surface the `Debrief` (or an aborted-run
    variant), leave the channel → `Ended`. Never a crash, never a hang.
- A client's *own* drop is the mirror image: reconnect the transport, rejoin
  with the same role, and proceed as the rejoining peer above. `seq`
  monotonicity means duplicate sends during reconnect are acknowledged and
  dropped by the relay, not replayed.

### 4. Implementation contract (the issue's acceptance criteria, made concrete)

The eventual implementation is done when, against the components now on
`main`:

1. **Two clients, one run.** Two instances of the gossamer-hosted Bevy build
   (`SimDriverPlugin` + `FrontendPlugin`) join the same `session:<id>` — one
   as infiltrator, one as hacker — over `BurbleTransport`, and play their
   asymmetric roles against one shared deterministic run (same seed, same
   merged command stream, per ADR-0004/0005).
2. **Traffic on burble, relay agnostic.** All `command`/`event` traffic rides
   burble; `server/` diffs empty of transport knowledge; constructing the
   client with `PlainWebSocketTransport` instead passes the same suite.
3. **Loss is handled.** Killing either client mid-run drives the other
   through `PeerLost` to either a resynced `Live` (rejoin within grace) or a
   clean `Ended` with a debrief — asserted in tests, not just survived.

**Loopback test plan** (the slice's gate): two client processes on one host, a
local relay, burble transport in loopback; scripted inputs on both seats
(reusing the headless-script mechanism behind
`fixtures/session_relay/capture_script.json` and the headless-`App` harness
from `crates/idaptik-bevy/tests/parity.rs`); assert both processes observe the
byte-identical `Event` log and the same `Debrief`. Run the identical script
over `PlainWebSocketTransport` and assert the same artefacts — that equality
*is* criterion 2.

### 5. Unblock conditions and fallback

Implementation starts when, and not before:

- **gossamer** ships either (A) documented hosting of a wasm/WebGPU app —
  Bevy's `wasm32` build loads, renders, and delivers keyboard input at
  fixed-tick-compatible latency in the gossamer webview, with a versioned
  Rust-consumable shell API for window lifecycle + IPC — or (B) native
  child-surface hosting for a winit/wgpu swapchain. Either unblocks §1.
- **burble** ships its embeddable client module as a versioned surface with a
  reliable, ordered data channel usable for non-voice traffic (the
  `Burble.join`-style embed of burble ADR-0013), working in burble's
  **default build** (no opt-in NIFs), plus a deployable rendezvous story that
  can sit in front of the Phoenix relay. That unblocks §2's `BurbleTransport`.

Until both hold, the **fallback slice stays shippable**: the plain-winit Bevy
build (`idaptik-bevy`'s own window) talking `PlainWebSocketTransport` directly
to the Phoenix relay. That configuration is not a stopgap to be deleted — it
is the permanent test configuration (§2), so building it first is on the
critical path either way. If gossamer or burble stalls indefinitely, the
fallback *is* the two-player windowed slice, and this ADR's status gets
amended to say so.

## Consequences

- `crates/idaptik-net` becomes the fourth consumer of the `Command`/`Event`
  wire contract (after TUI, relay, Bevy) and the only place Phoenix framing
  lives client-side; `fixtures/session_relay/` gains a consumer that must stay
  byte-compatible.
- The relay's public surface is now load-bearing for reconnection:
  `peer_joined`/`peer_left`, either-seat `Pause`, and `seq` de-duplication are
  the reconnect state machine's inputs and can no longer change shape without
  touching this ADR.
- Gossamer and burble each get one explicit, testable unblock condition
  instead of a standing "integrate later" ambition; progress is legible from
  their release notes.
- The implementation order inside the issue is fixed: `idaptik-net` + plain
  WebSocket + loopback test first (useful immediately, fallback forever), then
  `BurbleTransport`, then the gossamer wrap — glue last, just as issue #7
  demands, even within its own layer.

## Status note (2026-07-21): slice 1 landed; unblock conditions re-verified

- The §5 fallback slice is now real: `crates/idaptik-net` (`SessionTransport`,
  one Phoenix Channels client over it, `PlainWebSocketTransport`), the
  two-seat scripted session client, and the §4 loopback gate
  (`scripts/loopback_check.sh`; CI job `session-loopback`). Determinism is
  asserted byte-for-byte — both seats' artifacts against each other *and*
  against the `idaptik-tui --headless` reference — and loss is asserted by
  killing a seat mid-stream and requiring the survivor to end cleanly through
  `PeerLost`. v1 is batch-scripted: both seats exchange their scripted
  commands through the relay, then run the deterministic sim; real-time
  pacing (input delay) and mid-run pause/resync belong to the
  interactive-client slice, on the same wire shapes (`at`, `seq`, `net:*`).
- Recon (2026-07-21) re-verified both §5 unblock conditions remain unmet:
  gossamer is webview-only (no native child-surface hosting, no versioned
  wasm-hosting surface; its local build depends on the unreleased Ephapax
  compiler), and burble ships no embeddable non-voice data channel (no Rust
  surface; no `Phoenix.Socket.Transport` adapter; QUIC NIFs disabled in the
  default build). `BurbleTransport` and the gossamer wrap stay deferred.
- One bring-up detail the implementation fixed: the §3 handshake is a
  *barrier* — a seat streams commands only after holding the peer's
  `net:hello`, re-sent on `peer_joined`, because a relay-only topic has no
  history and anything sent before both seats are joined is unobservable.

## Status note (2026-07-21, later): the interactive-client slice landed

The live seat exists (`idaptik-netplay`): delay-based lockstep on the same
wire shapes, with the `idaptik-tui` face reused for interactive play
(`--interactive`) and a scripted feed for verification. What v1 deferred is
now real, and the loss path gained its second half:

- **Real-time pacing over `at`** — input sampled at local step `T` executes
  at `T + input_delay`; pacing hides latency, nobody rolls back, and seats
  with *different* delays interoperate (every command carries its own `at`).
- **`net:commit` watermarks** — sparse command streams cannot distinguish
  "no command at tick T" from "command still in flight", so each seat follows
  its sends with a cumulative `net:commit { through }`; a tick executes only
  when both watermarks cover it. Protocol version in `net:hello` is now 2.
- **Pause is lockstep-safe by construction** — `Pause` rides the scheduled
  command path like any either-seat immediate (scripts spell it
  `pause`/`resume`).
- **Resync via `RuntimeSnapshot` (`net:resync`)** — on loss the survivor
  *holds the run open*; a returning peer (fresh process, `rejoin: true` in
  its hello) is handed the snapshot (RNG and pause state included), the event
  log so far, the client fold state, and both seats' committed pending
  commands with watermarks. The one protocol rule that makes this exact:
  **uncommitted commands never happened** — the survivor prunes anything at
  or above the peer's watermark, and the rejoiner resamples those ticks.
- The loopback gate now also asserts, over the real relay: two live seats
  (mid-run pause window included, mismatched input delays) byte-identical to
  the headless reference, and a mid-pause death + rejoin + resync after which
  **both** seats — the rejoined process included — still end byte-identical
  to the reference. The sans-IO core (`lockstep.rs`) proves the same
  properties in unit tests over an in-memory wire, staggered delivery and
  orphan-command pruning included.

The relay is untouched — still relay-only ADR-0005, no new events, no state;
everything above is client-side vocabulary on the existing `"event"`
pass-through. Slices 2–3 (`BurbleTransport`, gossamer wrap) remain gated on
the unchanged §5 unblock conditions.
