# Architecture documents

Public IDApTIK documents describe stable interfaces, ownership, determinism,
and integration boundaries. Detailed cognitive-model internals belong in the
private canonical UMS repository; this repository records only the contracts
that IDApTIK must consume and verify.

- [ESM contract and repository boundaries](esm-contract-and-repo-boundaries.md)
- [UMS package contract ownership](ums-package-contract.md)
- [Repository architecture](../ARCHITECTURE.md)

## Public work rule

Keep public documents implementation-neutral where possible. Describe the
observable contract, validation requirements, provenance, and replay behaviour;
do not publish private model weights, tuning tables, proprietary inference
policies, or detailed AI internals.
