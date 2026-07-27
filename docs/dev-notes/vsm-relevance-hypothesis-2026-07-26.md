# Viable Systems Model relevance hypothesis — 2026-07-26

Status: **working hypothesis; not an accepted architecture decision**

Stafford Beer's Viable System Model (VSM) may provide a useful cybernetic
organisation for the Enaction/ESM stack. It should not be treated as a direct
five-box decomposition of affect, cognition, and conation: VSM is primarily a
model of viability, regulation, autonomy, identity, and recursion.

## Candidate mapping

| VSM function | Possible enaction interpretation |
|---|---|
| System 1 — operations | embodied action loops, perception/action subsystems, game actors |
| System 2 — coordination | synchronisation among modalities, agents, and concurrent processes |
| System 3 — present regulation | resource allocation, consistency, current-state regulation, safety constraints |
| System 3* — audit | independent inspection, replay checks, invariant and provenance audits |
| System 4 — intelligence | future/environment modelling, simulation, theory of mind, planning and learning |
| System 5 — policy/identity | values, identity, commitments, constitutional constraints, acceptable action policies |

This is a hypothesis about control roles, not a claim that each role must be a
separate process or repository.

## Particularly useful principles

- **Recursion:** an actor, team, faction, session, and whole game can each be
  analysed as a viable system, while preserving the same functional roles.
- **Variety management:** the system should preserve enough distinctions to
  respond to environmental variety, while attenuating irrelevant detail and
  amplifying weak but important signals.
- **Autonomy with coordination:** local agents can act locally while sharing
  constraints and negotiated state; the multiplayer relay must not become the
  semantic authority.
- **Algedonic signalling:** urgent pain/reward/threat signals can bypass normal
  coordination when delay would threaten viability. This is a promising place
  to study affective salience, but must be bounded and provenance-bearing.
- **Inside-and-now / outside-and-then:** present regulation and future-oriented
  modelling should be distinct but coupled, matching the ESM's current beliefs
  and hypothetical/ToM reasoning.

## Proposed relationship to the ESM

The ESM remains the canonical state-and-trace mechanism. VSM supplies a set of
questions about who regulates which transitions and how subsystems remain
viable:

```text
operations and embodied events (S1)
       ↕ coordination and synchronisation (S2)
current regulation and constraints (S3)
       ↕ audit / replay / invariant checks (S3*)
future models, ToM and planning (S4)
       ↕ identity, values and commitments (S5)
candidate action → verified transition → new event
```

The affective layer is not simply S4 or S5. Affective signals may originate in
S1, be coordinated through S2, regulate current behaviour through S3, and
provide algedonic escalation to S5 when identity or viability is threatened.

## Cautions

VSM was developed as a model of viable organisations and autonomous systems,
not as a validated theory of human emotion or consciousness. We should not
force every cognitive concept into a VSM box, nor confuse organisational
authority with truth. The mapping should earn its place by improving traces,
failure diagnosis, resource arbitration, or multi-agent coordination.

## Small experiment before adoption

Model one IDApTIK scenario with two recursive levels (agent and session):

1. identify S1 action loops and S2 coordination messages;
2. record S3 resource/safety decisions and S3* audit events;
3. give S4 a bounded future/ToM hypothesis buffer;
4. give S5 a small identity/commitment policy;
5. replay the trace and test whether the mapping explains a contradiction,
   delayed message, urgent threat, or rejected action better than the current
   ESM-only description.

Only then should VSM terminology be promoted into a shared contract or ADR.

## Concrete game applications

The most promising use is to treat NPCs and social groups as operational
systems at different recursive levels:

| Game concern | VSM-shaped interpretation |
|---|---|
| Individual NPC | S1 operator: perception, local memory, affective appraisal, action loop |
| NPC patrol/team | A viable unit composed of operators; S2 coordinates timing and communication, S3 allocates attention and resources |
| Faction or institution | Higher-level viable system supervising multiple teams while maintaining policy and identity |
| Director/adaptive-difficulty service | S4 models player tactics, forecasts likely game trajectories, and proposes interventions |
| Difficulty and fairness policy | S5 constrains what adaptations are acceptable and preserves the game's identity |
| Incident/replay analyser | S3* audits whether an NPC or group behaved according to its declared policy and evidence |

This makes “NPC operator” a useful term, but not every NPC needs to implement
all five functions independently. A guard can be an S1 operator inside a patrol
that supplies coordination and regulation; the patrol can in turn be an S1 unit
inside a faction. That is the practical value of recursion.

Adaptive difficulty belongs primarily to S4: it observes player tactics,
maintains forecasts, and proposes changes. S5 should provide the limits: no
adaptation that violates declared fairness, accessibility, narrative, or
project-identity constraints. S3 can then apply a bounded intervention in the
current scenario. The intervention should be an explicit, provenance-bearing
event, not an invisible change to NPC competence.

Possible events include:

```text
PlayerTacticObserved
TeamAssessmentUpdated
ThreatForecastChanged
DifficultyProposalGenerated
DifficultyPolicyChecked
AdaptiveInterventionApplied
```

This keeps learning and supervision inspectable in the same event-sourced trace
as ordinary NPC activity. It also leaves room for the player model to be
epistemic: the director can maintain hypotheses about what the player knows,
expects, or is likely to try without treating those hypotheses as facts.

## Deception and wrong hypotheses

False hypotheses should be a normal result of partial observability, not a
special-case deception script. Consider a player repeatedly creating evidence
that suggests a ventilation route, then entering through the front door and
knocking out a guard from behind near a vent. A guard may reasonably infer:

```text
player_used(vent-route) ≈ likely
```

even though the proposition is false. The resulting team intervention can move
attention to ventilation access points and leave the front door less protected.

To make this believable, a hypothesis needs more than a boolean:

```text
Hypothesis {
  proposition,
  confidence,
  supporting_events,
  contrary_events,
  source_quality,
  scope,
  expires_at,
  alternatives
}
```

Inference should rank possible explanations from the observing agent's limited
perspective. It must not consult the hidden ground-truth route unless the agent
has a legitimate information source. Later evidence can lower confidence,
promote an alternative, or mark the original attribution as disproven while
preserving the old trace.

The resulting interaction is not merely “NPCs make mistakes”: the player is
shaping the evidence distribution. The ESM records what the guards observed and
believed; the VSM layer decides how the team allocates attention; the world
simulation determines whether that allocation creates a real opening.
