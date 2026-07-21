# IDApTIK programme roadmap — July 2026

Distilled from the Jonathan/Joshua planning discussion of 2026-07-20,
reorganised into workstreams, cross-checked against what actually
exists in `IDApTIK` and `idaptik-ums`, and extended with the things the
discussion didn't reach. This is a **living document**: edit it, strike
things out, argue in-line. When something here hardens into a real
decision, promote it to an ADR and link it.

Status legend: **[DONE]** built and gated · **[PARTIAL]** some of it
exists · **[DECIDED]** agreed, not built · **[OPEN]** needs a decision
· **[GATED]** blocked on something external · **[UNSTARTED]** not begun.

---

## 1. Where we actually are (measured, not remembered)

Worth restating because sequencing arguments change when you know what's
already real:

| Piece | State |
|---|---|
| Deterministic event-sourced sim (core), TUI client, headless/replay | **[DONE]** — the TUI is a full playable interface |
| **Multiplayer**: seat split, relay (Elixir/Phoenix), live delay-lockstep netplay with pause + drop/rejoin resync | **[DONE]** — PR #34 + PR #37; byte-identical to the headless reference under CI gates |
| Multiplayer remaining: gossamer window hosting, burble voice transport | **[GATED]** — ADR-0006 §5 unblock conditions unmet upstream (no wasm surface in gossamer; burble has no Rust client) |
| Ghost Lobby scenario (the "odd level or two") | **[DONE]** — our vertical-slice testbed |
| Bevy renderer crate | **[PARTIAL]** — real and tested, but not a player-facing GUI yet; fyrox crate is a stub |
| Nickel config schema + scenario fixtures | **[DONE]** |
| **UMS**: Idris2 ABI (17 modules) + Zig FFI, level JSON extractors for *every* section (devices, zones, guards, dogs, drones, assassins, items, wiring, defences) | **[DONE]** — extractors landed 2026-07-21 (UMS PR #14), 40/40 runtime checks |
| UMS ↔ game taxonomy map (device kinds 1:1, zone tier→segment ratified) | **[DONE]** |
| UMS ai_edit engine (miniKanren, Kautz-6 verbs, 49 tests) | **[DONE]** as prototype — **[OPEN]**: Python is undeclared in the UMS language plan (affinescript/idris2/zig); declare it or schedule an Idris2 rewrite |
| UMS AffineScript shell; dlc/vm reversible VM | **[UNSTARTED]** / port written but **never compiled** (toolchain not installable yet — honestly declared in-repo) |
| Items/inventory *data model* (cables, adapters, tools, keycards, consumables, condition, containers) | **[DONE]** in the ABI — note: "items" as a workstream below means gameplay + presentation, not the data model |

Implication the discussion already sensed: **multiplayer "before GUI"
largely happened.** The netplay core is in and modular; what's left of
multiplayer is gated integration glue, not design work.

---

## 2. Decisions from the discussion (so we don't relitigate)

1. **GUI before objects/items.** Content authored once, for the final
   interface, instead of twice (TUI then GUI). If Jonathan wants to
   prototype some objects/items in TUI first anyway, that's fine — but
   treat it as throwaway-tolerant prototyping, not content production.
2. **A playable prototype comes before the big three** (GUI,
   multiplayer polish, items-at-scale). Ghost Lobby is the seed of
   this.
3. **Settings menu = a config file editable in the TUI** at this stage.
   Scalable later. (Foundation note: the Nickel schema already *is* the
   config layer — the "settings menu" is a UI over it, not a new
   system.)
4. **Skills advancement comes after GUI** — and it touches many
   systems, so until then: **every place a future skills hook belongs
   gets a marked integration note at the point of contact** (see §6,
   "integration-note discipline"). Lesson learned from IDApTIK v1
   (hand-coded skills) vs IDApixiTIK (not): retrofitting is the
   expensive path.
5. **Dialogue comes later. Storyline storyboarding starts now-ish** —
   either immediately or once enough modules exist to assemble
   something beyond Ghost Lobby.
6. **Tropes — settled part:** *geographic/regional* tropic variance is
   agreed (Invisible-Inc-style: one region robot-heavy, another with
   wall-watching-trained guards; New York gets yellow cabs and porter
   architecture). Jonathan takes tropes/thematics as a focus area and
   will produce **per-region trope definitions, docs-first**, which
   directly feed Joshua's work.
7. **Tropes — open part:** *time-period/style switching* (high-tech /
   noir / industrial / dieselpunk as a toggle). Joshua's concern: it
   multiplies asset+script+tone work and risks looking like we couldn't
   pick an aesthetic. Jonathan's argument: Conway-style worlds get
   their coherence from deciding these things early. **[OPEN]** —
   see §5 for a proposed middle path (era ledger as data).
8. **Division of focus:** Jonathan → thematics/tropes/regional
   definitions (+ "will just do stuff" across systems); Joshua → GUI
   and the systems work that hangs off it. Neither blocks the other.

---

## 3. Workstreams

### A. Interfaces

- **GUI** **[DECIDED, next major]** — port what's portable from
  IDApixiTIK. Open sub-questions: which host (the existing Bevy crate
  is real and tested — is it the GUI substrate, or is IDApixiTIK's
  approach ported onto something else?); TUI feature-parity checklist;
  whether netplay's interactive client (render/keymap/input, already
  library-ified) shares widgets with the GUI.
- **Game screens** (title, lobby, loadout, results, credits)
  **[UNSTARTED]** — cheap to storyboard now, belongs with GUI.
- **Settings** **[DECIDED]** — cfg file + TUI editor now; GUI settings
  screen later reads the same file. Keep *all* tunables in the Nickel
  schema so the settings UI is generated from one source of truth.
- **Controls** **[PARTIAL]** — keymap exists in TUI; needs remapping
  UI, controller support decision for GUI. *(Not discussed: input
  remapping persistence, simultaneous keyboard+controller in shared-
  screen co-op.)*
- **Accessibility** **[UNSTARTED, not discussed]** — colourblind-safe
  palettes (network-map colour coding!), font scaling, screen-reader
  affinity of the TUI (it's actually an accessibility asset — keep it
  first-class rather than deprecating it post-GUI), key-only and
  mouse-only play, photosensitivity (alarm flashes).

### B. Multiplayer & networking

- **Core netplay** **[DONE]** — delay lockstep, pause/resync,
  drop/rejoin, digest cross-check; TUI-based interactive client.
- **Gossamer window hosting** **[GATED]** — re-check unblock conditions
  when gossamer grows a wasm-hosting surface; eases burble when it comes.
- **Burble voice** **[GATED]** — in-game co-op voice only (burble has
  **no role in UMS** — settled 2026-07-16).
- **Relay hosting** **[OPEN]** — `idaptik-relay-house-tropic-forge`
  exists as a name only; decide: who runs relays, is there a public
  one, session discovery/matchmaking vs. share-a-session-id (current).
- *(Not discussed:)* spectator mode (cheap over the event-log
  broadcast), reconnect UX in the GUI, relay abuse/security posture,
  NAT/firewall guidance for self-hosters.

### C. World & content

- **Environment objects** **[DECIDED: after GUI]** — largely agreed
  between us already on what exists in the world; data model for
  devices/containers is done, the *interactive object* layer
  (animations, interactions, placement authoring in UMS) is the work.
- **Items** **[DECIDED: after GUI]** — data model (kinds, condition,
  uses, containers, keycards) is **[DONE]** in the UMS ABI and the
  game's config layer; remaining: acquisition/use gameplay, GUI
  presentation, item art.
- **NPCs** **[PARTIAL]** — guards/dogs/drones/assassins exist as
  placement + behaviour data; "NPCs" as *characters* (civilians,
  named people, Billy the office worker) are unstarted and heavily
  entangled with local AI (E) and dialogue (D).
- **Sprites, movement, interactions** **[UNSTARTED]** — art pipeline
  needed first (see H).

### D. Systems

- **Character system** **[UNSTARTED]** — player identity, stats
  substrate that skills advancement will hang off. Design doc can
  precede GUI.
- **Skills advancement** **[DECIDED: after GUI]** — with
  integration-note discipline *now* (§6).
- **Puzzles** **[PARTIAL]** — wiring-challenge data model is done
  (patch panel / backplane / rack / fibre-splicing / PBX types);
  legacy TS puzzles preserved in UMS `dlc/`; the *play* layer and
  puzzle authoring in UMS remain.
- **Dialogue** **[DECIDED: later]** — but the *speech style per trope
  region* (H) constrains it; storyline work should tag lines-of-intent
  ("guard banter here") without writing dialogue.
- **Storyline** **[DECIDED: storyboard now]** — storyboard against
  Ghost Lobby + 2–3 aspirational levels; produce a story bible
  skeleton (who, where, what era, what world events are canon — feeds
  H directly).

### E. AI (the two-level architecture)

- **Local AI** **[UNSTARTED]** — occupies every *thing*: a named NPC
  ("Billy"), a robot vacuum cleaner, a guard, or an augmentation on a
  player character. Per-class slots/variables specialise it.
- **Global AI** **[UNSTARTED]** — the state machine that coordinates
  all local AIs across the game: shared definitions instantiated
  per-entity (like globals + per-class slot overrides) so each new
  entity type is a *specialisation, not a rewrite*, and total
  processing stays bounded.
- Sequencing: roughly half of {player stuff, skills, items, objects,
  local AI} needs the GUI to be meaningfully testable — take them as
  they come, prototype-first.
- *(Not discussed:)* determinism constraint — **all AI must run inside
  the deterministic sim** (fixed-seed RNG, no wall-clock) or netplay's
  byte-parity guarantee breaks. This is a hard architectural rule;
  write it into the AI design doc on day one. Also: AI debugging
  tooling (a "why did it do that" trace view) is cheap if built in
  from the start and miserable to retrofit.

### F. UMS (Unified Modding Studio)

- **Level data pipeline** **[DONE]** — full extraction for every
  section; taxonomy map ratified; Zig FFI and Idris2 ABI agree on the
  wire format.
- **Per-function authoring** **[UNSTARTED]** — rooms, buildings,
  regions, campaigns as first-class authoring functions (the
  discussion's "for each function like rooms, buildings, regions").
  Suggested order: rooms → buildings → regions, because the **region
  layer is where H's per-region tropes attach** — give regions a
  `trope` field from day one.
- **ai_edit engine** **[DONE as prototype / OPEN ruling]** — Python
  miniKanren engine works and is tested; the language ruling (declare
  Python vs Idris2 rewrite) is with Jonathan.
- **AffineScript shell + dlc/vm** **[GATED]** on an installable
  AffineScript toolchain.
- **UMS as the modding surface** *(not discussed)* — UMS is
  effectively the public modding story; at some point it needs
  user-facing docs, versioned level-format guarantees, and a
  compatibility policy (what happens to community levels when the
  schema moves).

### G. Foundational / platform

- **Coprocessor support** **[DECIDED, undefined]** — game/UMS should
  *offer* coprocessor support automatically when present at runtime.
  Needs a definition pass before anything else: which workloads (local
  AI inference? procedural gen?), which APIs, detection, and the
  fallback path. Write the one-pager before writing code.
- **Save/load & persistence** **[UNSTARTED, not discussed]** — the
  event-sourced sim makes saves nearly free (snapshot + log — the
  netplay resync payload *is* a save file in embryo). Decide: save
  format stability, versioning, save-compat policy across releases.
- **Determinism & replay** **[DONE]** — protect it; it is the
  foundation under multiplayer, saves, and AI-debugging.
- **Performance budgets** *(not discussed)* — set them when the GUI
  lands, per-frame and per-tick; the global AI's "reduce processing"
  goal needs a number to aim at.
- **Packaging & distribution** *(not discussed)* — target platforms,
  itch/Steam/flatpak, AGPL-3.0 engine + CC-BY-SA-4.0 content
  obligations at distribution time, crash reporting (opt-in only).

### H. Aesthetics & thematics (Jonathan's focus)

- **The two trope levels**, made precise:
  1. *Era/style axis* **[OPEN]** — high-tech / noir / industrial /
     dieselpunk as switchable global styles.
  2. *Geographic axis* **[DECIDED]** — per-region tropic variance
     within one chosen era.
- **The era ledger** *(proposed middle path for the [OPEN] axis)* —
  regardless of whether era ever becomes a *switch*, the game is set
  in **some** period, and coherence demands the period be decided
  early and enforced. So: a machine-readable ledger of era-sensitive
  facts — post-its (1974+), USB (mid-90s+), robot vacuums
  (near-futuristic), laptops vs mainframes, colour TV, furniture
  materials, car silhouettes, fashion, speech register, referenceable
  world events (a war? climate themes?). Each entry: item → earliest
  plausible era → substitution (post-it → paper memo on desk).
  **This makes the aesthetic-dilution argument concrete**: trope-max
  = ledger consulted per style; trope-min = ledger consulted once for
  the single canonical era. Either way the ledger is needed, so it's
  not wasted work while the decision stays open. It can live in UMS
  as data and be *linted* against level content (an anachronism
  checker fits the existing validation pipeline naturally).
- **Per-region trope definitions** **[DECIDED, Jonathan, docs-first]**
  — for each region: surveillance philosophy (robot-heavy vs
  human-guard wall-watchers), architecture, street furniture,
  transport (yellow vs black cabs), dress, speech style, ambient
  events. Deliverable: one page per region; feeds Joshua's asset and
  design work directly; becomes `trope` data on UMS regions later.
- **Mood, music, sprites** **[UNSTARTED]** — blocked on the era
  decision more than on tooling; commissioning/production pipeline
  needs its own plan (sources, palette, licence vetting for every
  asset — content is CC-BY-SA-4.0, so incoming assets must be
  compatible).
- *(Not discussed:)* sound design beyond music (UI feedback, alarm
  vocabulary, network-action sounds are gameplay-relevant in a hacking
  game); a one-page art bible before any sprite is commissioned.

### I. Outward-facing

**[UNSTARTED, deliberately later — but not last]**: social media,
marketing, PR plan, branding. Minimum early moves that cost little:
reserve names/handles now; keep a devlog habit (this file is nearly
one); decide what's public when (the engine is AGPL and public — the
*story* and *era* reveals are the marketing levers, so the story bible
should mark spoiler tiers). Press kit, trailer, store pages come after
the GUI prototype exists to screenshot.

---

## 4. Sequencing (proposal, edit me)

```
now ──────────────────────────────────────────────────────────▶
[P0] Prototype hardening: Ghost Lobby + TUI + netplay as the
     demoable vertical slice (mostly DONE — keep it green)
[P1] GUI bring-up (port from IDApixiTIK; decide Bevy-or-not)
     ∥ Storyline storyboard + story bible skeleton   (Jonathan)
     ∥ Per-region trope one-pagers + era ledger v0   (Jonathan)
     ∥ Character-system + coprocessor one-pagers     (design only)
[P2] Objects & items gameplay on the GUI · settings UI over cfg
     ∥ UMS authoring functions: rooms → buildings → regions(trope)
[P3] Skills advancement (integration notes cashed in) · local AI
     on named entities · NPC characters
[P4] Global AI coordination · dialogue · puzzles play-layer
[gated, any time they unblock] gossamer window → burble voice
[continuous] CI green, determinism protected, era ledger enforced
```

---

## 5. Open questions

| # | Question | Leaning / owner |
|---|---|---|
| 1 | Era/style: single canonical era or switchable tropes? | Build era ledger either way; decision can wait until P2 · both |
| 2 | GUI substrate: extend the Bevy crate or port IDApixiTIK's approach? | Needs a spike · Joshua |
| 3 | Which era *is* canon (if single)? Drives every asset. | Before any sprite is commissioned · both |
| 4 | Coprocessor support: define workloads/APIs | One-pager first · Jonathan |
| 5 | UMS ai_edit language: declare Python or rewrite in Idris2? | Declare now, rewrite later at will · Jonathan |
| 6 | Relay hosting: public relay? matchmaking? | After GUI, before any public playtest · both |
| 7 | Shared-screen co-op (one keyboard) vs netplay-only? | Not discussed — decide cheaply · both |
| 8 | Playtesting: who, when, what NDA-ish etiquette pre-reveal? | P2 boundary · both |

---

## 6. Working agreements (proposed)

- **Integration-note discipline**: wherever future skills / local-AI /
  dialogue hooks belong, leave a greppable marker comment at the exact
  point of contact — `INTEGRATION(skills):`, `INTEGRATION(local-ai):`,
  `INTEGRATION(dialogue):` — plus one line on what will attach there.
  A `just integration-map` recipe can then list the outstanding
  surface. Markers are cheap; archaeology is not.
- **Determinism is load-bearing**: nothing enters the sim that reads
  wall-clock, OS randomness, or floats-across-platforms without going
  through the seeded RNG / fixed-point conventions. New-system design
  docs must state their determinism story.
- **Docs-first for the expensive stuff** (tropes, coprocessor,
  character system): one page before code, in this directory.
- **This file is the agenda**: strike through what's done, add what's
  missing, and re-cut §4 when reality disagrees with it.

---

## 7. What this document deliberately does not do

It does not assign dates, and it does not pretend the [GATED] items
are schedulable. Both gates (gossamer surface, AffineScript toolchain)
are external; re-check them at each phase boundary rather than
polling continuously.
