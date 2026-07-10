# ADR-0002: Multiplayer transport — Bandit + Phoenix Channels, not LiveView

- Status: Accepted
- Date: 2026-07-09

## Context

IDApTIK is an asymmetric two-player game: an infiltrator and a hacker share
one authoritative world. Rust owns gameplay truth; Elixir/OTP owns multiplayer
and session life — matchmaking, pairing the two roles, relaying intent, presence,
and reconnection. The open question was how the Elixir side should serve
real-time traffic: **Bandit**, **Phoenix LiveView**, or something else, and where
**hex** fits.

These are not alternatives at the same layer, which was the source of confusion:

- **Bandit** is a pure-Elixir HTTP/1.1 + HTTP/2 + WebSocket **server** (a Plug/
  WebSock adapter, the modern replacement for Cowboy). It is *what serves the
  sockets*.
- **Phoenix Channels** is a real-time **messaging layer** (pub/sub, presence,
  topics) that runs *over* a WebSocket served by an adapter like Bandit.
- **Phoenix LiveView** renders rich, server-driven **HTML UI** over a WebSocket.
  It is for web front-ends.
- **hex** is Elixir/Erlang's **package registry** — how `mix` fetches Phoenix,
  Bandit, etc. It is not a component you choose between; it is the plumbing that
  pulls the others.

## Decision

Serve the multiplayer backend with **Bandit** as the HTTP/WebSocket adapter and
**Phoenix Channels** as the real-time transport. **Do not use LiveView** for
gameplay.

Rationale:

- The game UI lives in Rust (Bevy/Fyrox), not in HTML. LiveView's whole value is
  server-rendered HTML DOM diffing — irrelevant to a native/Wasm game client, and
  it would be the wrong abstraction to push game state through.
- Phoenix Channels is the idiomatic BEAM primitive for exactly this: topic-based
  pub/sub, `Phoenix.Presence` for who-is-connected, and built-in reconnection —
  everything a two-player session needs — while leaving the wire payloads to us.
- Bandit is the modern default (it has been Phoenix's default adapter since
  1.7.11), gives us HTTP/2 + WebSockets with no extra work, and keeps the stack
  pure Elixir.
- Phoenix (Channels) buys presence/pub-sub/reconnect for free versus a raw
  `WebSockAdapter` handler on Bandit alone; the framework weight is worth it for
  multiplayer.

## What hex pulls (server/mix.exs, when scaffolded)

- `{:phoenix, "~> 1.7"}` — Channels, endpoint, socket (Bandit is its default
  adapter; nothing extra to wire for WebSockets)
- `{:bandit, "~> 1.11"}` — the HTTP/WebSocket server
- `{:phoenix_pubsub, "~> 2.1"}` — pub/sub backbone (clustered later if needed)
- `{:jason, "~> 1.4"}` — JSON, until/unless we adopt a binary wire format
- **Not** `{:phoenix_live_view, ...}`

A binary wire format (protobuf/flatbuffers/MessagePack) for the Rust↔Elixir
boundary is deferred to its own ADR once the protocol is designed; Channels are
payload-agnostic, so this decision does not block on it.

## Consequences

- The Elixir app is a headless real-time backend, not a web UI — smaller surface.
- Rust client speaks the Phoenix Channels socket protocol (a documented, stable
  framing) over Bandit's WebSocket.
- If we ever want a browser-based spectator/among-us-style lobby UI, LiveView can
  be added *alongside* for that specific surface without disturbing gameplay.
