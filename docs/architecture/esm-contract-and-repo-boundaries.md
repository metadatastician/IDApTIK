# Shared epistemic state-machine contract and repository boundaries

Status: **draft v0.2 — being split**

This document currently does three jobs, and only one of them belongs in
IDApTIK. Its own §5 states the rule *"one semantic authority per layer"* and
assigns kernel event and state semantics to the Enaction Engine — yet the
kernel semantics are written down here and nowhere else, which is the rule
being broken by the document that states it.

The split, tracked by ADR-0009 and ADR-0010:

| Section | Destination |
|---|---|
| §2 reduction contract, §3 data model and invariants, §4 profiles | Move to `enaction-engine/docs/architecture/CAC-KERNEL.adoc`, which becomes normative |
| §5 ownership, §6 hand-off flow, §7 validation and compatibility | **Stay here.** IDApTIK owns the versioned package boundary per ADR-0007 |
| §8 first implementation slice | It is a schedule, not a contract. Moves to `docs/dev-notes/` |
| §9 open decisions | Each is closed or given an owner below |

Until `CAC-KERNEL.adoc` exists, §2–§4 remain here and remain authoritative.

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

**This table is a destination, not a description of the present tense.** The
Enaction Engine holds no evidence ledger, no reduction kernel and no snapshot
machinery today; the working implementations of all three are in
`crates/idaptik-core/`, and ADR-0010 records the decision to keep them there
until the extraction trigger fires. Read the table as where each concern
*settles*, and ADR-0010 for where the code *is*.

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

Each of the original seven is now closed or has a named owner. Nothing here is
left open and unattributed.

| Decision | Disposition |
|---|---|
| Canonical proposition language and identifier scheme | **Open.** The hardest one, and it blocks a single ESM crate. Slavia has ground `Fact { predicate, args }`; IDApTIK has `proposition: String`; the ledger has a `HypothesisId` drawn from a declared frame. Sharp sub-question: *can a Dempster–Shafer focal set contain a non-ground proposition?* miniKanren's whole value is variables; DS frames are enumerated sets. If irreconcilable, "one kernel" weakens to "one trace, two reasoners". |
| Probability/confidence representation and calibration | **Closed** by Enaction ADR-0016. Fixed-point `Mass` in units of 1/10,000, typed as a justification budget rather than a probability. `Read` is a solution-set cardinality and has no numeric conversion. |
| Merge semantics for concurrent observations | Deferred to the evidence crate's combination policies. Moot under delay-lockstep, where total order comes from the envelope; not moot under the relay with several independent observers. |
| Maximum theory-of-mind depth and resource accounting | **Open.** No depth policy exists anywhere in the estate. Note the trap: a cap denominated in *milliseconds* fires at different points on two machines and breaks byte-parity. Any cap must be counted in deterministic units — inferences, unifications, stream nodes. |
| Signed package/envelope requirements | Already owned by IDApTIK ADR-0007. |
| Exact Idris2/ACL2 proof boundary | **Open.** Related: nobody has yet said what "verified" *means* here. Both the action-verification gate and the loop diagram place a verifier in the conative path, and both are mocks. Whatever the answer, a veto must be an event — a silent substitution that never appears in the trace violates the provenance invariants in §3. |
| JSON, CBOR or another canonical wire encoding | **Closed** by Enaction ADR-0015. Canonical CBOR for bytes and parity comparison, JSON for human-readable fixtures, with a round-trip equality test between them. |

Further open problems identified since v0.1, recorded here so they are not
silently assumed:

- **Frames do not compose.** DS requires a declared frame per question; Slavia's
  belief space is open-ended. What happens to evidence about a proposition
  outside every declared frame is undecided.
- **Affect's clock is undecided**, and it sets the scale ceiling. Per-tick,
  event-gated, or reduced-rate with algedonic interrupts?
- **Is affect state, or annotation?** This document permits only an optional
  annotation on events. A continuous appraisal layer is state with its own
  dynamics. The contract does not yet say which.
- **The 1:1 domain/stage correspondence is unvalidated.** Six trace domains map
  exactly onto six Ghost Lobby guard stages — for *one* scenario. Either a deep
  result about appraisal structure, or one camera-failure sequence frozen into a
  kernel enum. A second scenario with a different natural stage count settles it.
- **Nothing has been benchmarked.** No measurement exists anywhere in the estate
  for a solver query, a ledger combine, or a tick of affect. Every scale claim
  is unmeasured.

This document remains a design contract, not a claim that the complete ESM
exists in any repository.
