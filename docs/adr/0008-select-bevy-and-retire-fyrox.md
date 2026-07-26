# ADR-0008: Select Bevy and retire the Fyrox evaluation frontend

- Status: Accepted
- Date: 2026-07-25
- Supersedes: the dual-frontend evaluation portion of ADR-0003

## Context

ADR-0003 kept gameplay truth in `idaptik-core` and opened an evaluation between
Bevy and Fyrox as interchangeable Rust frontends. The evaluation has now
produced materially different evidence.

The Bevy frontend contains the working Ghost Lobby driver, scene, HUD, keymap,
fixed-step integration and render interpolation. Headless Bevy tests compare
its event stream and snapshots with the engine-neutral reference run. ADR-0006
also defines the Gossamer-hosted client around the Bevy build.

The Fyrox crate never progressed beyond bring-up: it linked a Fyrox type,
loaded the demo network and printed a message. It had no executor, plugin,
renderer, input path, parity test or hosted-client integration. Maintaining the
dependency and a second frontend namespace therefore provided no current
portability evidence.

Chronicles of Slavia independently selected Bevy for its production L3
renderer, so Bevy also provides the shared proving path across both games.

## Decision

Bevy is the selected graphical frontend for IDApTIK. Remove
`idaptik-fyrox`, the Fyrox workspace dependency, its task-runner recipe and
operational documentation.

The renderer-neutral boundary remains mandatory:

- `idaptik-core` owns deterministic game truth and has no Bevy dependency;
- Bevy translates input into typed commands and presents events/snapshots;
- headless, TUI, FFI and multiplayer paths remain usable without Bevy;
- render-derived and interpolated values never feed simulation state.

ADR-0003 remains authoritative for the engine-agnostic core boundary and as the
historical record of the evaluation. This ADR supersedes only its requirement
to retain two graphical frontend crates.

## Consequences

- New graphical work targets Bevy once rather than maintaining false parity.
- The working Bevy parity tests remain the guard against renderer-owned game
  logic.
- Removing Fyrox reduces dependency and build surface substantially.
- Reintroducing Fyrox or another renderer requires a concrete consumer,
  implemented behaviour, tests and a new ADR; an empty comparison stub is not
  sufficient.
