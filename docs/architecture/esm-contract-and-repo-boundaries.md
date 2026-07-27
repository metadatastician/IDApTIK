# Shared epistemic state-machine contract and repository boundaries

Status: **draft v0.1**

This document proposes the first shared contract for epistemic state machines
(ESMs) across IDApTIK, the Enaction Engine, Universal Modding Studio (UMS),
and Chronicles of Slavia. It is deliberately small: it defines the seams and
invariants before any repository is asked to implement a complete theory of
mind.

## 1. Scope and vocabulary

An **epistemic state** is an agent's current, provenance-bearing model of what
is known, believed, suspected, or unknown about a world and about other agents.
It is not the world state itself. A **trace** is the ordered record from which
an epistemic state can be reconstructed. A **profile** supplies game- or
scenario-specific vocabulary and policy without changing the kernel semantics.

The first contract covers:

- observations and their provenance;
- beliefs, hypotheses, confidence, and explicit uncertainty;
- nested beliefs (theory of mind);
- contradictions and supersession;
- deterministic ordering and replay;
- optional affective and conative annotations.

It does not yet prescribe a particular cognitive architecture, machine-learning
model, planner, renderer, or multiplayer protocol.

## 2. Normative reduction contract

An implementation MUST expose a deterministic reduction equivalent to:

```text
reduce(
  prior_state: EpistemicState,
  event: EpistemicEvent,
  context: ReductionContext
) -> ReductionResult
```

where `ReductionResult` contains:

```text
{
  state: EpistemicState,
  derived: [Belief | Hypothesis | Intention | AffectAnnotation],
  trace_entry: TraceEntry,
  diagnostics: [Diagnostic]
}
```

For the same contract version, prior state, event, and context, the result MUST
be byte-for-byte or canonically equivalent. Wall-clock time, process identity,
mailbox arrival order, hash-map iteration order, and renderer state MUST NOT
alter the result.

The reduction is event-sourced. A snapshot is an optimisation and MUST be
reconstructible or verifiable against the ordered event trace.

## 3. Minimum data model

The names below are conceptual wire fields. Concrete Rust, Elixir, Idris2,
Nickel, or UMS representations may differ, but field meaning and required
invariants must remain compatible.

```text
EpistemicEvent {
  event_id: EventId,
  sequence: uint,
  tick: uint,
  observer: AgentId,
  subject: AgentId | World,
  kind: Observation | Communication | Inference | ActionOutcome | Correction,
  proposition: Proposition,
  confidence: Confidence,
  provenance: Provenance,
  affect: optional AffectAnnotation,
  conation: optional ConativeAnnotation
}

Belief {
  belief_id: BeliefId,
  holder: AgentId,
  proposition: Proposition,
  status: Known | Believed | Suspected | Rejected | Unknown,
  confidence: Confidence,
  source_events: [EventId],
  valid_from: Tick,
  supersedes: optional BeliefId
}
```

Required invariants:

1. `event_id` is globally unique within a trace.
2. `(tick, sequence)` gives a total order for reduction.
3. Every non-derived belief has at least one provenance event.
4. A contradiction is represented explicitly; it is not silently overwritten.
5. Nested theory of mind is represented as a belief whose holder is another
   agent, subject to a declared depth/resource policy.
6. Unknown is a first-class status and is distinct from rejected or false.
7. Confidence is typed and bounded by the contract; an implementation must not
   confuse confidence with truth.

## 4. Profiles and extensions

The kernel owns event ordering, provenance, status transitions, contradiction
handling, snapshots, and replay. A profile may add propositions, predicates,
actor roles, affect dimensions, conative drives, and transition policies.

Profiles MUST:

- declare the kernel and profile version they target;
- use namespaced extension identifiers;
- provide a validator and migration story for breaking changes;
- remain loadable without UMS being installed;
- avoid redefining kernel fields with different meanings.

The initial profile names are expected to be `idaptik/esm/v1` and
`slavia/esm/v1`. They are profiles, not separate ESM semantics.

## 5. Ownership and repository boundaries

| Concern | Authoritative home | Other repositories may do |
|---|---|---|
| Kernel event/state semantics, ordering, replay, snapshots | Enaction Engine | consume, test, propose versioned changes |
| Typed/proof-level laws and refinements | `epistemic-types`, `echo-types`, ACL2/Idris2 work | prove or model laws; publish evidence |
| Authoring UI, schema editing, validation and compilation | UMS | produce a versioned profile/package; never redefine kernel meaning |
| IDApTIK propositions, actors, scenarios and game rules | IDApTIK | publish `idaptik/esm/v1` and execute it in the game |
| Slavia's world ontology, factions, culture, and full affect/conation profile | Chronicles of Slavia | publish `slavia/esm/v1` against the shared kernel |
| Multiplayer/session transport | Elixir service in the consuming project | order and relay envelopes; never become semantic authority |
| Renderer, UI and presentation | each consuming project | observe commands, events, snapshots and traces |
| Experimental donor implementations | Project Ovine | provide experiments and fixtures; no authority over shared contracts |

The rule is **one semantic authority per layer**. UMS is the authoring surface,
not the owner of game truth. A multiplayer process may assign transport order,
but canonical ordering is carried in the event envelope and reduced by the
authoritative runtime.

## 6. Package and hand-off flow

The intended direction is:

```text
Enaction Engine kernel + proof laws
              ↓ versioned schema
UMS profile editor / validator / compiler
              ↓ signed or content-addressed package
IDApTIK or Slavia runtime
              ↓ ordered events, snapshots, traces
UMS inspection and replay tools
```

This is a bidirectional workflow but not shared mutable ownership: definitions
are authored and compiled in UMS, semantics are defined by the Enaction Engine,
and runtime truth belongs to the consuming game. A game MUST be able to load a
compiled package without UMS being present.

## 7. Validation, compatibility, and security

Every package MUST declare:

- `kernel_version`;
- `profile_id` and `profile_version`;
- schema/package format version;
- source provenance and content digest;
- supported migration range;
- deterministic seed or an explicit seed policy;
- declared event and snapshot guarantees.

Loaders MUST reject unknown incompatible major versions, invalid provenance,
duplicate event IDs, non-monotonic ordering, unbounded nested-belief expansion,
and packages whose digest does not match their contents. A diagnostic should
identify the stable field/path and compatibility rule that failed.

## 8. First implementation slice

The first interoperable slice should be intentionally narrow:

1. `Observation`, `Communication`, `Inference`, and `Correction` events.
2. `Known`, `Believed`, `Suspected`, `Rejected`, and `Unknown` statuses.
3. One level of nested belief, with a declared maximum depth.
4. Provenance, confidence, contradiction, replay, and snapshot verification.
5. A small IDApTIK Ghost Lobby profile and fixtures shared with UMS.
6. A transport-neutral JSON fixture set for Rust and Elixir consumers.

Affect and conation should attach as optional, typed annotations in this slice;
they should not force the first kernel to settle the complete Slavia model.

## 9. Open decisions

The following remain deliberately open and require an ADR when implementation
begins:

- canonical proposition language and identifier scheme;
- probability/confidence representation and calibration;
- merge semantics for concurrent observations;
- maximum theory-of-mind depth and resource accounting;
- signed package/envelope requirements;
- exact Idris2/ACL2 proof boundary;
- whether traces use JSON, CBOR, or another canonical encoding on the wire.

Until those decisions are accepted, this document is a design contract, not a
claim that the complete ESM already exists in any repository.
