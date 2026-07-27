# ADR-0011: Cognitive-model work is documented in the open

- Status: Accepted
- Date: 2026-07-27
- Supersedes: the public/private documentation split stated in
  `docs/architecture/README.md`

## Context

`docs/architecture/README.md` stated that detailed cognitive-model internals
belong in a private canonical UMS repository, and that IDApTIK should record
only the contracts it must consume and verify. The corresponding private
document, `idaptik-ums-canonical/docs/architecture/idaptik-ai-model-private.md`,
states the same rule from the other side.

The split produced three concrete problems.

One design ended up spread across two repositories with no index, so no single
place described the model. The most detailed cognition specification in the
estate — the evidence ledger design, covering frames of discernment,
fixed-point mass, provenance, independence groups, combination policies and
conformance fixtures — sat **untracked** in the less-maintained of the two
repositories, which carries roughly twenty branches and several modified
worktree entries. It was one `git clean` from being unrecoverable. And the
boundary the split was meant to protect became harder to check, not easier,
because the checkable half and the reasoning behind it were in different places.

The two UMS repositories are now being consolidated into one, which removes the
private side of the split as a matter of fact.

The engine is AGPL-3.0-or-later and the project is built to stay open. There are
no model weights, no proprietary inference policies and no third-party licensed
material in this work. The thing being protected did not exist.

## Decision

The public/private documentation split is withdrawn. Cognitive-model internals
are documented in the open, in the repository that holds the code implementing
them, under `docs/architecture/` and `docs/dev-notes/`.

The rule in `docs/architecture/README.md` is void and that file is rewritten.

The two rescued design documents — the working cognitive model and the evidence
ledger design — land under `docs/architecture/` as part of the UMS
consolidation, with their `-private` suffixes dropped.

This decision covers design documentation. It does not change anything about
story, era or narrative reveals, which are marketing levers with their own
spoiler tiers and are unaffected.

## Consequences

- One design, one location, reviewable.
- `docs/architecture/README.md` no longer instructs a reader to look in a
  repository that is being archived.
- The evidence ledger design is under version control (committed 2026-07-27,
  241 lines) rather than untracked.
- Anything genuinely unpublishable in future needs its own ADR stating what it
  is and why. "AI internals" as a category is not a reason.

**Real-game status:** documentation policy only. No code or contract changes.
