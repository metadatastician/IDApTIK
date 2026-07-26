# Cognitive–affective–conative model hypothesis — 2026-07-26

Status: **working hypothesis; not an accepted architecture decision**

This note records an early systems sketch and the discussion around it. The
purpose is to preserve useful thinking without accidentally turning a simple
diagram into a requirement.

## Starting sketch

The proposed layers were:

```text
Affective: neural inputs
        ↓
Affective: Kautz Type 3
        ↓
Affective: symbolic emotions
        ↓
Cognitive: epistemic state
        ↓
Cognitive: miniKanren logic
        ↓
Conative: ACL2 verifier
        ↓
Conative: action output
```

## Current interpretation

The sketch is useful as a first interface map, but the runtime should not be a
strict one-way pipeline. Affect, cognition, and conation form a loop:

```text
world/bodily/neural input
        ↓
affective appraisal and symbolic affect
        ↓
epistemic state and theory of mind
        ↓
inference, hypothesis and possibility generation
        ↓
goal/drive/commitment and candidate action
        ↓
typed or ACL2 verification
        ↓
world transition and new observation ───┘
```

The epistemic state machine is the persistent centre of the loop. miniKanren is
a reasoning mechanism over that state, not the state itself. A verifier checks
a proposed action or transition; it should not be treated as the component
that chooses the agent's goals.

## Kautz terminology

Kautz Type 3 describes a neural component producing a symbolic representation
that is then processed by a symbolic reasoner (`Neuro | Symbolic`). This is a
reasonable label for a neural-affective-input → symbolic-affect interface.
It describes the neural/symbolic coupling pattern, not the affective theory.
Type 4 concerns symbolic reasoning used to train or shape a neural system and
is therefore a different pattern.

## Candidate event seam

An initial affective event might expose:

```text
AffectiveAppraisal {
  event_id,
  observer,
  stimulus,
  valence,
  arousal,
  salience,
  appraisal_tags,
  provenance
}
```

The ESM may derive `BeliefUpdated`, `ThreatConsidered`,
`AgentIntentAttributed`, `GoalUrgencyChanged`, or `ActionCandidateGenerated`.
These names are examples, not yet a frozen vocabulary.

## Questions to keep open

- Which affect dimensions are symbolic, and which remain continuous signals?
- How should affect alter confidence, attention, salience, or action urgency?
- Where do drives, goals, commitments, and affordances live in the shared model?
- What exactly is verified: an action, a plan, a state transition, or an invariant?
- How are contradictory affective appraisals represented without overwriting
  provenance?
- Which parts belong in the Enaction Engine kernel, and which belong in the
  IDApTIK or Slavia profile?

This note should be promoted into an ADR only after a small executable fixture
demonstrates the affect → epistemic update → candidate action → verification
loop.
