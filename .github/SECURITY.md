# Security Policy

## Reporting a vulnerability

If you believe you have found a security vulnerability in IDApTIK, please
report it privately by emailing
[developer@joshuajewell.dev](mailto:developer@joshuajewell.dev) rather than
opening a public issue. Include as much detail as you can: affected
component, reproduction steps, and potential impact.

## Areas of particular concern

* `crates/idaptik-ffi` — the C-ABI surface consumed by Zig/Idris2 frontends.
  Memory-safety issues here cross a language boundary.
* `server/` — the Elixir/OTP session relay (Bandit + Phoenix Channels), which
  is network-facing and handles multiplayer session state.

## Response

This project is currently maintained by a single maintainer (see
`MAINTAINERS`); response times are best-effort. Assurance status for the
project as a whole is tracked in `PROJECT-ASSURANCE-PROFILE.adoc` — no formal
security audit has been performed to date.
