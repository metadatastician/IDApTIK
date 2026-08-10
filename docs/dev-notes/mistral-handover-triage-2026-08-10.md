# Mistral handover pack — triage (2026-08-10)

The owner received an "IDApTIK — Ultimate Handover Package" produced in a
Mistral (Le Chat) session and asked for it to be incorporated **where it
fits**, with conflicts surfaced rather than silently resolved. This document
is that triage: what was adopted, what was rejected and why, and the open
questions that need an owner ruling. The pack described a JavaScript
"Phase 1 prototype" living in Mistral's own sandbox
(`/home/user/canvases/…`, `/home/user/uploads/…`) — those files are **not**
in this repository; only what is recorded here survives.

Verdict in one line: **the pack's roadmap is obsolete (this repo already ate
it), but its design content is genuinely valuable** — facility, wiring
topologies, tropes, scenarios, actor ecology — and most of it slots into
work this estate already has in flight.

---

## REJECTED (conflicts with ground truth)

### R1. The entire "Phase 2: Rust/Bevy/Elixir transpilation" plan
The pack's spine is a 13-day plan to port its JS prototype to Rust/Bevy +
Elixir, starting from `cargo init`. **This repository is that port's
descendant, already far past the plan's end state**: `idaptik-core/tui/net/
netplay/bevy` crates, a Phoenix relay with delay-lockstep netplay
(NET_PROTO 2, watermark commits, rejoin/resync), a Tailscale-aware launcher
at v0.4.2, session-loopback CI, and GUI netplay release-ready (#85–#90).
The plan's specifics would be regressions: Bevy 0.12 (repo is later),
edition 2021 (repo is 2024), a Rust `warp` server *alongside* Phoenix
(confused duplication; the relay exists), `web_sys` WebSockets (the
transport seam + Phoenix client already exist), `rand` in the AI arbitration
(the runtime is deterministic by doctrine — goldens are byte-identical).

### R2. The name expansion
Pack: "Invisible Door **Adaptive Puzzle Infiltration Kit**".
Canon: "Invisible Door: **Action Point Trauma Inc. Kickers**".
Canon stands unless the owner declares a rebrand (open question Q1).

### R3. CAC implemented as a client-side `cac.rs` port
The pack ports its JS CAC (beliefs/emotions/drives structs + update
pipeline) straight into the game client. Estate rulings route this
differently: **enaction-engine** owns the cognition substrate
(ADR-0019 contracts: the game supplies vocabulary, the engine supplies
ordering/causality/domain separation; C6 adoption = IDApTIK swaps its local
belief code for the crate), and IDApTIK-side state goes through the ESM/VSM
framework (#61) and the supervised-coverage runtime (snapshot v3). The CAC
*content* is adopted below (A5) as design input — the *implementation home*
in the pack is rejected. Two technical conflicts worth naming: the pack uses
floats everywhere (enaction uses integer milliunits — `value_milli: i32`)
and random tie-breaking (determinism doctrine).

### R4. Sandbox file paths as project state
`/home/user/canvases/*` and `/home/user/uploads/*` are Mistral's
environment. Anything there the owner wants preserved must be exported and
committed (open question Q3).

---

## ADOPTED (recorded here as design canon-candidates)

### A1. Relay House facility spec
Five floors — B // INFRASTRUCTURE, 1 // STREET & LOBBY, 2 // WORKSHOPS,
3 // TENEMENTS, R // ROOF PLANT — 20+ rooms, portals (doors/stairs/vents),
and five external connection points: GRID POWER, WATER MAIN, INTERNET,
STEAM PLANT, EXHAUST OUTLET. Rooms carry tags, UPS flags, redundant-link
flags, per-system sources, and device lists. This is the richest facility
spec the project has and complements `docs/level-design.md`.

### A2. Wiring topology system (the standout content)
Per-system physical topologies with gameplay consequences:

| System   | Topology     | Notes |
|----------|--------------|-------|
| Power    | Radial       | main breaker → all rooms; feeder routing A/B/C |
| Water    | Tree         | main → plumbing hub → branches |
| Network  | Star + Mesh  | router → rooms, redundant links survive cuts |
| Steam    | Loop         | boiler → rooms → return; supply vs return lines |
| Vent     | Mesh         | interconnected ducts + **hidden crawl-only bypass** |
| Security | Bus          | linear chain from alarm panel; UPS-backed |

Plus: critical-path markers, tamper warnings, UPS-backed devices staying
alive through power cuts, and hidden routes that fade in when tampered.
This maps directly onto the existing network-sim (game `DeviceKind`,
8-segment `Zone` enum, segment ladder) and gives the "what you cut has
consequences" layer the WORKPLAN's patrol-consequences item wants.

### A3. Trope system ("tropic forge")
Genre presets that re-skin *and* re-weight the same facility: Relay Age /
Cyberpunk / Biopunk shipped in the pack (Clockpunk/Steampunk/Dieselpunk
named as future), each with palette, per-system lexicon (power=CURRENT vs
GRID vs METABOLISM …), mechanics multipliers, and **CAC modifier tables**
(cognitive biases, affective multipliers, conative multipliers). The same
puzzle plays differently per trope. Estate fit: this is the long-missing
`relay-house-tropic-forge` owner-input, and trope-family thinking already
has a home in the estate's trope-theory thread. Natural implementation
route: **UMS game profiles** (trope preset = authored data pack, not code).

### A4. Actor ecology (Training Ground)
Archetypes with six-axis capabilities (observation/force/control/
information/mobility/resilience), tags, sensors, routines: Local ("Billy"),
Patrol Guard shipped; Technician, Remote Analyst, Commander named. Modifier
layer (Armoured, Wounded, Escapee, Remote, Commander) applying capability
deltas. Complements the ESM/VSM work; the pack's Billy matches the in-game
Billy.

### A5. CAC design detail → routed to enaction-engine
The pack's CAC pipeline is a concrete, tuned spec: perception → belief
update (memory freshness decaying at 0.14/s, object-importance growth),
appraisal → emotions (fear/suspicion/curiosity/boredom/trust/duty with
explicit rate constants), drives, and **utility arbitration with hysteresis
(commit timer) + PAD (pleasure/arousal/dominance)**. This is direct design
input for enaction-engine's next roadmap items — C4 conation (goal
priority, intention-as-pinned-goal: the hysteresis/commit mechanism) and
C1 valence (the PAD triple). To be translated into contracts vocabulary
(integer milliunits, TraceEvent causality, deterministic tie-breaking).

### A6. Scenario specs with asymmetric solutions
USB Decoy, Airgapped Server, Ventilation Bypass — each with per-role
solutions (infiltrator / hacker / drone) and per-trope variations. Fits the
existing script/scenario system (`config-scenario-check`, headless goldens).
Note: USB Decoy's "distract Billy" depends on how the open
**Billy-can't-badge-through-a-closed-door** ruling lands, since Billy's
investigate-goal pathing is the distraction mechanism.

### A7. Backstory and cast
Near-future adaptive-architecture premise; Jessica (infiltrator), Marek
(hacker), Moletaire (drone — see Q2); NPC cast as A4.

---

## OPEN QUESTIONS (owner rulings needed)

- **Q1 — Name**: keep canon "Action Point Trauma Inc. Kickers", or adopt the
  pack's "Adaptive Puzzle Infiltration Kit" as a rebrand?
- **Q2 — Moletaire (drone) as a third playable seat**: today the game is
  two-seat asymmetric. A drone seat touches netplay seats, the relay role
  table (already missing NetSsh/NetHack for the hacker), and the burble
  game-lane port. Adopt as a design goal, or keep two-seat?
- **Q3 — The JS Phase-1 prototypes**: export the HTML files from Mistral's
  sandbox into this repo (as `docs/prototypes/` reference material), or let
  this triage document stand as the record?
- **Q4 — Trope implementation route**: agree that trope presets are UMS
  profile data (authoring side) rather than engine enums? (Matches the
  two-taxonomies reading: UMS = what you place, game = what you hack.)

## Suggested sequencing (post-ruling)

1. Wiring topologies (A2) into the network-sim design — extends
   `docs/level-design.md`; pairs with the WORKPLAN patrol-consequences item.
2. CAC detail (A5) into enaction-engine's C4/C1 design docs before that
   implementation starts.
3. Trope presets (A3) as a UMS profile experiment (fast side of the
   two-repo doctrine).
4. Scenario specs (A6) after the Billy ruling.
