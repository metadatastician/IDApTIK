# idaptik-server

The multiplayer / session layer for IDApTIK — a headless realtime backend.
**Phoenix Channels over Bandit, no LiveView** (see `../docs/adr/0002-multiplayer-transport.md`).
Rust owns gameplay truth; this pairs the two asymmetric roles and relays their
intent.

## Run

```bash
mix deps.get
mix phx.server        # listens on 127.0.0.1:4000 (dev)
```

Or from the repo root: `just server-setup && just server`.

## Shape

- `IdaptikServerWeb.Endpoint` — Bandit adapter, one `/socket`.
- `IdaptikServerWeb.UserSocket` — routes `session:*` topics.
- `IdaptikServerWeb.SessionChannel` — a single game session. Two clients join
  `session:<id>` with `%{"role" => "infiltrator" | "hacker"}`; the channel relays
  `"intent"` (infiltrator → hacker) and `"hacker_action"` (hacker → infiltrator),
  each tagged with its origin, plus a `"ping"`/pong liveness check.
- `IdaptikServerWeb.StatusPlug` — terminal plug answering plain HTTP with a
  small JSON status (this backend is sockets, not an HTTP API).

## Wire protocol (current)

| Direction | Event | Payload |
|-----------|-------|---------|
| client → server | join `session:<id>` | `{"role": "infiltrator"｜"hacker"}` |
| infiltrator → hacker | `intent` | game-defined, server adds `"from"` |
| hacker → infiltrator | `hacker_action` | game-defined, server adds `"from"` |
| server → peer | `peer_joined` | `{"role": ...}` |

A binary wire format for the Rust↔Elixir boundary is a later decision
(ADR-0002); Channels are payload-agnostic, so it does not block this skeleton.
