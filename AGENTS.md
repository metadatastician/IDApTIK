# Repository instructions

IDApTIK is an asymmetric two-player infiltration game: Rust owns gameplay
truth, Elixir/OTP owns multiplayer session life. It is a member repository of
The Metadatastician estate and adopts that estate's governance profile via
`docs/PROJECT-GOVERNANCE-BINDING.adoc`.

Read `.machine_readable/0-AI-MANIFEST.a2ml`, `README.md`, `GOVERNANCE.md`,
`.github/MAINTAINERS`, and `docs/PROJECT-GOVERNANCE-BINDING.adoc` before
editing. Canonical policy lives in the governance documents; machine-readable
descriptiles mirror declared state and do not outrank them.

Invariants (see `0-AI-MANIFEST.a2ml` for the full list):

- Licence layers are deliberate: engine/code is AGPL-3.0-or-later, game
  content is CC-BY-SA-4.0, names/marks (IDApTIK, Moletaire) are trademarked.
  Do not relicense the engine or flatten the layers.
- Gameplay truth lives in `crates/idaptik-core` and stays engine-agnostic and
  deterministic; `crates/idaptik-bevy` is the selected thin graphical
  frontend. Do not move game truth into Bevy.
- The session layer is Bandit + Phoenix Channels, not LiveView.
- State/metadata belongs under `.machine_readable/`, not the repository root.
- Contributions come in under DCO 1.1 — sign commits with `git commit -s`
  (see `CONTRIBUTING.md`).

Do not edit generated files directly; none are currently declared. Do not
change licences, upstream coined names, or evidence-status labels (in
`docs/PROJECT-ASSURANCE-PROFILE.adoc`) as a side effect of an unrelated change.

## The stack is Rust and Elixir — attempts 1–3 are dead

`README.md` documents four incarnations. Agents routinely read that lineage
table as current and propose work against a codebase that no longer exists:

| # | Codebase | Stack | Status |
|---|----------|-------|--------|
| 1 | IDApTIK | TypeScript / Excalibur | dead |
| 2 | IDApixiTIK | AffineScript / PixiJS | dead |
| 3 | idaptik | AffineScript / PixiJS | dead |
| 4 | **IDApTIK** | **Rust / Elixir** | **live — this repo** |

There is no JavaScript runtime here: no browser, no Canvas, no
`requestAnimationFrame`, no npm, no PixiJS, no Excalibur. A proposal written
against any of those cannot be applied.

Ruby and Python are also not implementation or project-tooling languages in
this repository. Do not introduce Ruby/Python source, runtime dependencies,
formatters, linters, package ecosystems, or workflow steps. A documentation
reference to a separate repository's implementation does not make that
language part of IDApTIK.

GitHub's hosted service actions are an infrastructure boundary, not part of
the project stack. Prefer direct commands when an action would add an avoidable
language runtime (checkout uses `git` directly). A hosted action whose service
protocol has no practical replacement, such as CodeQL analysis or Pages
deployment, may use its own internal runtime; keep it SHA-pinned and do not
extend that exception to repository code, scripts, dependencies, or general
workflow tooling.

Two further confusions worth pre-empting:

- **miniKanren is not in this repository.** It lives in
  `metadatastician/idaptik-ums` (the Unified Mission System, the level-design
  tool) and it is Python.
- **AffineScript is not the current language.** It is a separate language of the
  maintainer's, used in dead attempts 2 and 3 and a possible future target. It
  is not what this repo is written in.

## Already built — do not re-specify

- **Fixed 60 Hz timestep.** `crates/idaptik-bevy/src/driver.rs` sets
  `Time::<Fixed>::from_hz(60.0)` and steps the sim in `FixedUpdate`;
  `idaptik-core` defines `TICK_DT = 1.0/60.0` (`scenario/mathf.rs`) and
  accumulates time rather than multiplying it.
- **Determinism.** A run is a pure function of
  `(definition, config, seed, command stream)`, enforced by `replay_determinism`,
  `snapshot_equivalence`, `rng_vectors`, `golden_reset` and `no_panic_fuzz`.
- **The `Command`/`Event` wire API**, shared by the TUI and Bevy frontends;
  `crates/idaptik-bevy/tests/parity.rs` proves they stay in step.
- **Snapshots** (`scenario/snapshot.rs`) for save/restore and resync.
- **Netplay.** `crates/idaptik-net` holds a sans-IO delay-lockstep core behind a
  two-seat byte-identical loopback gate.

There is no garbage collector — this is Rust. "Avoid GC pauses" is not a design
goal here; it is free. Bevy is the selected graphical frontend (ADR-0008).
Fyrox was retired because its evaluation crate remained a bring-up stub while
Bevy acquired the renderer, parity tests and hosted-client design.

Nothing render-side may feed back into simulation state. The determinism tests
are load-bearing, not decorative.

## Checks must be able to fail

A check that cannot fail is not a check. Do not add a test that reports success
without running anything, and do not add a gate that exits 0 when its tool is
missing — an absent toolchain is a failure, not a skip.

## Building

Run `mise trust` first. An untrusted mise config is *silently ignored*, so the
global toolchain resolves instead (e.g. zig 0.16 rather than the pinned 0.14)
and the resulting stdlib errors point nowhere near the real cause.

```
just test-ghost              # cargo test (core/tui/ffi) + clippy -D warnings + fmt --check
just config-check            # nickel typecheck
just config-scenario-check   # Nickel export → Rust validator → every bad_*.ncl rejected
just loopback-check          # two-seat netplay parity gate
```
