# ADR-0004: Deterministic, event-sourced, definition-as-data scenario simulation

- Status: Accepted (Envelope milestone)
- Date: 2026-07-11

## Context

The canonical playable prototype for the Billy scenario ("Envelope 001 — Ghost
Lobby") exists as a ~2500-line HTML/JS toy: a variable-`dt` game loop, a
`mulberry32` RNG, a Billy finite-state machine, a hacker "support envelope"
model, and a scoring/debrief screen. It is the whole game, but it is not
gameplay *truth*: it renders while it simulates, its timing is frame-rate
dependent, and its state is trapped inside DOM and closures.

Per ADR-0003, gameplay truth must live in the engine-agnostic `idaptik-core`
crate so Bevy/Fyrox stay thin and the Elixir session layer (ADR-0002) can
coordinate it. The prototype also needs to slot, later, into the "Exchange
House / UMS building runtime" pattern: content declared as data, validated into
a model, simulated, and exported as JSON (definition, runtime snapshot,
after-action debrief).

## Decision

Port the prototype into `idaptik-core` as a **deterministic, event-sourced,
definition-as-data** scenario.

1. **Definition as data.** A scenario is a serde `ScenarioDefinition` (rooms,
   doors, cameras, hide spots, props, objectives, difficulty presets, a full
   tuning table, scoring weights). It round-trips through JSON unchanged and
   self-validates via `validate()` → an Exchange-House-style report of named
   checks, projected to typed `ValidationError`s by `ok()`.
2. **Constants dual-homed.** `constants.rs` holds every ported magic number as a
   `pub const` — the single source of truth and a compile-time audit.
   `ghost_lobby()` copies them into the definition so JSON round-trip *and*
   compile-time audit both exist; a test asserts the projection is faithful.
3. **Typed Commands in, typed Events out.** The sim consumes a `TickInput`
   (held `Buttons` bitset + edge actions + immediates) and emits a `Vec<Event>`.
   Every `logEvent` and state transition in the prototype is a typed `Event`;
   human log lines are a pure *view* via `log_view()`. The ordered event stream
   is the canonical artifact.
4. **Fixed 60 Hz tick.** `TICK_DT = 1/60`, commands sampled once per tick. The
   twelve systems run in a fixed, load-bearing order. Same
   `(definition, config, seed, command stream)` ⇒ byte-identical event log.
5. **Float / determinism policy.** `f64` with JavaScript operator semantics:
   `js_round(x) = floor(x + 0.5)` (not `f64::round`), `Math.sign(a || b)` →
   `sign_or`, RNG in wrapping `u32`. All transcendentals (`sin` for the camera
   sweep, `powf` for USB drag) go through the `libm` crate for byte-identical
   x86-64 / aarch64 / wasm results — the ephapax and typed-wasm builds target
   wasm and std intrinsics would drift the goldens. Time is **accumulated**
   (`t += TICK_DT`), never `tick as f64 / 60.0`. `serde_json` is built with
   `float_roundtrip` so snapshots and exports are bit-exact.
6. **`reduced_motion`** is the only `RunConfig` knob that changes sim math (the
   lights-flicker window, 1.45 s vs 0.70 s); it is captured in the snapshot and
   defaults to `false` for canonical/headless runs.
7. **String ids, resolved once.** Content ids are serde-transparent `String`
   newtypes (homoiconic, human-readable) resolved to `Vec` indices once at
   construction (`IdIndex`); the hot loop never hashes strings. Runtime enums
   (`Phase`, `BillyMode`, `ActionKind`, `DifficultyId`) stay hard enums for
   exhaustive matching; per-room quirks are `RoomDef`/`CameraDef` fields so no
   code branches on room strings.
8. **Three export surfaces + event log,** mirroring the Exchange House:
   `DefinitionExport`, `RuntimeSnapshot` (full state incl. RNG, restorable), and
   the after-action `Debrief`, plus the canonical `event_log`. `snapshot()` at
   any tick; `restore()` reconstructs an equivalent sim and continues identically.
9. **SPARK-equivalent rigor.** The crate is `#![forbid(unsafe_code)]`; the sim
   path is panic-free (no `unwrap`/`expect`/panicking index — `.get()` + match,
   clamped arithmetic, exhaustive matches); fallible construction returns
   `Result`; invariants are enforced by `validate()` at `new()`.

The playable frontend is a new `idaptik-tui` crate (ratatui + crossterm) with
the HTML key bindings, **plus** TTY-free `--headless --script`, `--replay`, and
`--export` modes — the headless path is how the scenario is verified in CI and
in constrained environments. Bevy/Fyrox are untouched.

## Consequences

- The scenario is reproducible byte-for-byte and inspectable as JSON at every
  layer, so replay, snapshot/restore, and after-action analysis are free.
- The `__IDAPTIK_TEST__` hooks (`start` at seed 123456, direct action injection,
  forced crisis/extract/fail) become the headless script API (`ForceCrisis`,
  `ForceExtract`, `ForceFail` + a tick-indexed hold/press timeline).
- Because content is data and the sim is engine-agnostic, Ghost Lobby can later
  become one floor of the UMS building runtime without touching the sim.
- HTML-string parity is explicitly **not** required: the goldens are our own
  committed event logs, formatted by our own deterministic formatter.
