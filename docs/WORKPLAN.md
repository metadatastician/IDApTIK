# IDApTIK workplan

Status: **organised first draft — 2026-07-26**

This is the public work queue. It deliberately describes AI work at the level
of contracts and observable behaviour. Detailed model design belongs in the
private canonical UMS repository.

## Now: deterministic foundations

- [x] Clean `main`, Pages deployment, domain, HTTPS, security baseline.
- [x] Publish the shared ESM and repository-boundary contract.
- [x] Add the first VSM-shaped deterministic supervisor prototype.
- [x] Add bounded, provenance-bearing scalar hypothesis tests.
- [x] Add a deterministic DST-style evidence ledger using fixed-point masses.
- [x] Add USB/fridge-note and front-door/vent conformance fixtures.
- [x] Preserve conflict, ignorance, fixed-point replay, and focal-set queries.
- [x] Connect evidence plausibility to bounded patrol attention allocation.
- [x] Translate selected Ghost Lobby events into observer-relative evidence.

## Next: first playable proving slice

- [x] Model the first NPC operator-team type and attention trace event.
- [ ] Connect the team allocation to actual Ghost Lobby world coverage.
- [ ] Add a bounded adaptive intervention with an explicit policy check.
- [ ] Expose developer telemetry in TUI/replay output.
- [ ] Verify that the player can create a rational but false NPC hypothesis.

## Later: presentation and wider integration

- [ ] Connect the same state to the Bevy frontend.
- [ ] Add player-visible patrol consequences without exposing omniscient state.
- [ ] Export/import versioned UMS profile packages.
- [ ] Add typed/proof conformance checks in the Enaction Engine and formal
      repositories.
- [ ] Generalise the profile for Chronicles of Slavia's cognitive, affective,
      and conative scope.

## Boundary rule

IDApTIK owns gameplay meaning and runtime validation. UMS owns authoring and
profile compilation. The Enaction Engine owns shared semantics. Multiplayer
relays events but does not become semantic authority. Private AI details remain
in the canonical private UMS repository unless a public interface requires
their disclosure.

## Current stopping point

The public branch `docs/vsm-esm-framework` contains the first deterministic
evidence/VSM prototype and is open as PR #61. The core tests cover scalar
hypotheses, fixed-point focal masses, conflict, USB/fridge-note revision,
vent/front-door deception, and evidence-driven patrol attention. Nothing yet
changes the live Ghost Lobby simulation or Bevy presentation. The event adapter
now exists, but it is still opt-in; start the next session by invoking it from
the real simulation event stream and applying the resulting team allocation to
world coverage.
