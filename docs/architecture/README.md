# Architecture documents

IDApTIK documents describe stable interfaces, ownership, determinism, and
integration boundaries.

- [ESM contract and repository boundaries](esm-contract-and-repo-boundaries.md)
- [UMS package contract ownership](ums-package-contract.md)
- [Repository architecture](../ARCHITECTURE.md)

## Where cognitive-model work is documented

Cognitive-model internals are documented here, in the open, alongside the code
that implements them.

An earlier rule sent detailed internals to a private canonical UMS repository
and kept only the consumable contract here. That rule is withdrawn: it split
one design across two repositories, left the most detailed specification in the
estate untracked in the less-maintained of the two, and made the boundary
harder to check rather than easier. See ADR-0011.

The engine is AGPL-3.0-or-later and the project is built to stay open. A design
that cannot be published is a design that cannot be reviewed.
