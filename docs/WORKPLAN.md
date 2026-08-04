# IDApTIK workplan

Status: **organised first draft — 2026-07-26**

This is the work queue. Detailed cognitive-model design is documented here in
the open, alongside the code that implements it, under `docs/architecture/` and
`docs/dev-notes/` — see ADR-0011, which withdrew the earlier rule sending it to
a private repository.

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
- [x] Add an opt-in supervised headless runner path.

## Next: first playable proving slice

- [x] Model the first NPC operator-team type and attention trace event.
- [x] Connect the team allocation to actual Ghost Lobby world coverage.
      (Supervision is deterministic runtime state; `RunConfig::supervised`
      bends Billy's Assess patrol band toward the evidence-selected coverage
      target. The interactive TUI is always supervised.)
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

The supervised-coverage slice (2026-08-03) moved the supervisor inside the
deterministic runtime state: in a supervised run the sim folds its own event
stream into `RuntimeState::supervision` (snapshot format v3), the evidence
ledger picks the most plausible coverage target (`usb` vs `fridge_note`,
declaration-order tie-break), and Billy's Assess patrol band bends toward that
target in proportion to team attention. The allocation is visible in the
canonical log as announce-once `TeamAttentionAllocated` / `CoverageRetargeted`
events — the seed of the "developer telemetry" checkbox. The interactive TUI
always runs supervised; headless scripts and the CLI opt in via
`"supervised": true` / `--supervised`, and unsupervised runs are byte-identical
to before the slice.

Two facts the next session should know:

- Billy cannot badge through a closed door: the constraint bounce-back plus
  `door_wait` resetting on every unblocked tick form a limit cycle, so the
  badge timer never accumulates at any Billy speed. Every golden embeds this,
  so it is canonical behaviour for now — but it silently confines patrol
  coverage to one door region, and deserves its own decision (bug or feature).
- The next unchecked item is the bounded adaptive intervention; the
  `VsmDirector::observe`/`apply` policy envelope already exists and is tested,
  it just is not driven from sim state yet.
