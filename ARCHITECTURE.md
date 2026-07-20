# Architecture

IDApTIK is a polyglot game monorepo. **Rust owns gameplay truth** (an
engine-agnostic, deterministic, event-sourced simulation); **Elixir owns
multiplayer** (a transport-agnostic session relay); frontends are thin drivers
over the same typed `Command`/`Event` wire.

For the reasoning behind each choice, see the ADRs in [`docs/adr/`](docs/adr/).

## Directory structure

```
.
├── crates/            # Rust Cargo workspace (edition 2024, AGPL-3.0-or-later)
│   ├── idaptik-core/  #   engine-agnostic sim: netsim + scenario + Moletaire companion
│   ├── idaptik-ffi/   #   C ABI surface (JSON in / JSON out) over the core
│   ├── idaptik-tui/   #   ratatui/crossterm frontend + --headless/--replay/--export verifier
│   ├── idaptik-bevy/  #   Bevy side-on 2.5D frontend (evaluation)
│   └── idaptik-fyrox/ #   Fyrox frontend (evaluation; ADR-0003)
├── server/            # Elixir/Phoenix session relay (Bandit + Phoenix Channels)
├── config/            # Nickel typed config: scenario-schema.ncl + fixtures (incl. bad_*.ncl)
├── docs/adr/          # Architecture Decision Records
├── fixtures/          # cross-language wire fixtures
├── scripts/           # bootstrap.sh, doctor.sh, install-idris2.sh
├── LICENSES/          # full SPDX license texts (REUSE)
└── justfile           # task runner (build/test/lint/config gates)
```

## The core (`crates/idaptik-core`)

The authoritative simulation. It is **engine-agnostic** (no rendering or I/O),
**deterministic**, and **event-sourced**: a run is a pure function of
`(definition, config, seed, command stream)` producing an `Event` log and a
`Debrief` (ADR-0004). `#![forbid(unsafe_code)]`.

- `netsim/` — the grounded network model (devices, addressing, DNS, routing,
  reachability, sessions) that the hack is played against.
- `scenario/` — definition-as-data scenarios (Ghost Lobby, the Exchange House
  building runtime as floors), the actor archetype + modifier registry, and the
  tick/command/event/snapshot machinery.
- `companion/` — Moletaire, the deterministic companion (Rust port).

Determinism is enforced by tests: `replay_determinism`, `snapshot_equivalence`,
`rng_vectors`, `no_panic_fuzz`, and the negative Nickel fixtures.

## The FFI boundary (`crates/idaptik-ffi`)

A C ABI that drives the core over a **JSON-in/JSON-out** surface
(`idap_ghost_lobby_new/tick_json/snapshot_json/free`, `idap_demo_network`, …).
Panics are contained with `catch_unwind`; errors surface as `{"error": …}`
sentinels. The generated header (`include/idaptik.h`) is committed and
regenerated with `just ffi-header`. An in-process-vs-ABI parity test guards
equivalence. Intended host bridge is Zig (ADR-0001).

## Frontends

`idaptik-tui` (the reference frontend and headless verifier), `idaptik-bevy`
(side-on 2.5D), and `idaptik-fyrox` are all thin drivers: they read input, push
`Command`s, and render the `Event`/snapshot stream. The core never depends on
them (ADR-0003, engine-agnostic strategy).

## Multiplayer (`server/`)

An Elixir/Phoenix app (Bandit + Phoenix Channels, no LiveView) that relays the
typed `Command`/`Event` stream between the two asymmetric roles over
`session:<id>` channels. The relay is **transport-agnostic** and holds no
gameplay authority — it forwards; the core (run on a client) decides (ADR-0005).

## Build & toolchain

`just` orchestrates `cargo` (Rust), `mix` (Elixir), and `nickel` (config).
Toolchains are pinned via `mise.toml` and `rust-toolchain.toml`. Key gates:

- `just test-ghost` — `cargo test` (core/tui/ffi) + `cargo clippy -D warnings` +
  `cargo fmt --check`.
- `just config-scenario-check` — exports the Nickel scenario, round-trips it
  through the Rust validator, and asserts every `bad_*.ncl` fixture is rejected.

## Design principles

- **One source of truth.** The core is authoritative; everything else observes.
- **Determinism first.** Same inputs ⇒ same `Event` log ⇒ replayable, testable.
- **Definition-as-data.** Scenarios, actors, and buildings are data, not code.
- **Transport-agnostic multiplayer.** Swapping the transport must not change the
  game (a plain socket substitutes for the real transport in tests).

---

*See the ADRs in `docs/adr/` for the decisions behind this structure.*
