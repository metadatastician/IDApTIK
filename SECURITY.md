# Security Policy

## Reporting a vulnerability

Do not report vulnerabilities through public issues, pull requests,
discussions, or social media.

Use GitHub's private
[security-advisory form](https://github.com/metadatastician/IDApTIK/security/advisories/new).
If that is unavailable, email
[developer@joshuajewell.dev](mailto:developer@joshuajewell.dev) with the
affected component, reproduction steps, potential impact, and any suggested
remediation.

## Scope

Security reports may cover the repository's code, dependencies, build and
deployment configuration, and published releases. The most sensitive areas
are:

- `crates/idaptik-ffi`: the unsafe C-ABI boundary used by non-Rust consumers;
- `crates/idaptik-net`: the network-facing client over burble game-session fabric;
- `.github/workflows/`: privileged automation and supply-chain configuration;
- snapshot, package, and multiplayer wire validation at trust boundaries.

Note: the session relay itself now lives in the burble repository
(`metadatastician/burble/server/lib/burble_web/channels/game_channel.ex`).

Do not perform denial-of-service testing, social engineering, or testing
against infrastructure or accounts you do not own.

## Response and disclosure

IDApTIK is currently maintained by one maintainer, so response times are
best-effort. Reports will be acknowledged privately, triaged, remediated, and
disclosed through a coordinated GitHub Security Advisory where appropriate.
No independent security audit has yet been performed; current evidence and
limitations are recorded in `docs/PROJECT-ASSURANCE-PROFILE.adoc`.
