# Architecture Decision Records

Decisions for the Rust era of IDApTIK are recorded here as ADRs, rather than
asserted in the README and quietly contradicted later.

| # | Decision | Status |
|---|----------|--------|
| [0001](0001-toolchain-and-runtime-management.md) | Toolchain & runtime management (mise + rustup; Idris2 via pack; opsm later) | Accepted |
| [0002](0002-multiplayer-transport.md) | Multiplayer transport: Bandit + Phoenix Channels, not LiveView | Accepted |
| [0003](0003-engine-strategy.md) | Engine strategy: engine-agnostic core, Bevy + Fyrox as frontends | Accepted (evaluation) |
| [0004](0004-deterministic-event-sourced-scenario-sim.md) | Deterministic, event-sourced, definition-as-data scenario simulation | Accepted (Envelope milestone) |
| [0005](0005-session-relay-topology.md) | Session relay topology: relay-only lockstep over typed Command/Event | Accepted (vertical-slice milestone) |
| [0006](0006-gossamer-host-and-burble-transport.md) | gossamer host window + burble transport: two-player windowed slice | Accepted (design; implementation deferred) |

New ADRs: copy the format of an existing one, take the next number, and add a row
above.
