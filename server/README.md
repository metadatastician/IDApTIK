# idaptik-server

The multiplayer/session layer for IDApTIK: Phoenix Channels over Bandit, with
no LiveView. Rust owns gameplay truth; this service pairs the two asymmetric
seats and relays their typed stream without interpreting gameplay.

## Run

```sh
mix deps.get
mix phx.server
```

The development endpoint listens on `127.0.0.1:4000`. From the repository root
use `just server-setup`, `just server-test`, or `just server`.

## Session shape

Two clients join `session:<id>` with
`{"role":"infiltrator"}` or `{"role":"hacker"}`.

| Client event | Payload | Relay behaviour |
|---|---|---|
| `command` | Rust `Command` JSON tagged with `cmd` | Enforces the command-to-seat routing table, strips optional relay `seq`, and sends the command to the peer |
| `event` | Rust `Event` JSON tagged with `event` | Relays verbatim to the peer; this also carries namespaced `net:*` control messages |
| `ping` | any JSON | Replies with `{"pong":true}` |
| `intent`, `hacker_action` | legacy free-form JSON | Compatibility relay for pre-typed clients |

An optional strictly increasing `seq` de-duplicates commands. The client-owned
`at` tick remains in the payload for delay-lockstep scheduling. Join and leave
produce `peer_joined` and best-effort `peer_left` notifications.

The relay contains no scoring, physics, FSM, tick, or snapshot decisions. The
shared fixtures under `fixtures/session_relay/` are decoded by Rust tests and
relayed by Elixir tests, and `just loopback-check` proves both seats remain
byte-identical to the headless reference through pause, peer loss, and resync.
See ADR-0005 and ADR-0006.
