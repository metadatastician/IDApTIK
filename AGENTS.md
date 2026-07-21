# Repository instructions

IDApTIK is an asymmetric two-player infiltration game: Rust owns gameplay
truth, Elixir/OTP owns multiplayer session life. It is a member repository of
The Metadatastician estate and adopts that estate's governance profile via
`PROJECT-GOVERNANCE-BINDING.adoc`.

Read `0-AI-MANIFEST.a2ml`, `README.md`, `GOVERNANCE.md`, `MAINTAINERS`, and
`PROJECT-GOVERNANCE-BINDING.adoc` before editing. Canonical policy for this
repo lives in the root governance documents; `.machine_readable/` descriptiles
mirror declared state and do not outrank them.

Invariants (see `0-AI-MANIFEST.a2ml` for the full list):

- Licence layers are deliberate: engine/code is AGPL-3.0-or-later, game
  content is CC-BY-SA-4.0, names/marks (IDApTIK, Moletaire) are trademarked.
  Do not relicense the engine or flatten the layers.
- Gameplay truth lives in `crates/idaptik-core` and stays engine-agnostic and
  deterministic; `crates/idaptik-bevy`/`crates/idaptik-fyrox` are thin,
  replaceable frontends.
- The session layer is Bandit + Phoenix Channels, not LiveView.
- State/metadata belongs under `.machine_readable/`, not the repository root.
- Contributions come in under DCO 1.1 — sign commits with `git commit -s`
  (see `CONTRIBUTING.adoc`).

Do not edit generated files directly; none are currently declared. Do not
change licences, upstream coined names, or evidence-status labels (in
`PROJECT-ASSURANCE-PROFILE.adoc`) as a side effect of an unrelated change.
