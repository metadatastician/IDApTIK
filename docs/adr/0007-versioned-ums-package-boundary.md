# ADR-0007: IDApTIK owns the versioned UMS package boundary

**Status:** accepted
**Date:** 2026-07-25

## Decision

IDApTIK publishes `contracts/idaptik/v1` and loads
`idaptik-package/v1` through `idaptik-core`. UMS consumes those artifacts and
adds profile-specific generation rules, but neither UMS Core nor its UI becomes
a game dependency.

The JSON Schema defines the stable envelope. Rust `ScenarioDefinition` remains
the authoritative game vocabulary and `ScenarioDefinition::validate` remains
the semantic authority. UMS must not re-declare that type or its enums.

## Consequences

- A compiled package can be tested at the real boundary.
- Game evolution requires a versioned contract or migration rather than a
  silent editor-side copy.
- The game can load packages without UMS being installed.
- v1 is intentionally Ghost-Lobby-shaped; it does not claim every future
  building, actor pack or scenario migration is solved.
