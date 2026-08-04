# Multiplayer troubleshooting

Known, reproduced failure modes of `idaptik-multiplayer-launcher.sh` and the
`idaptik-netplay` seat it drives, and what to do about each. Add to this file
when a new one is confirmed; don't speculate ones that haven't been hit.

## `ended_no_peer` even though the relay is up

Two independent causes produce the same symptom — the seat reports
`{"status":"ended_no_peer"}` after waiting for a peer that never arrives.
Check both.

### 1. The shared address is unreachable (WSL2 NAT)

**Status:** fixed in the launcher's share banner as of PR #78 (commit
`e01e3f5`) — this section documents the underlying cause for anyone still
hitting it via an older checkout, a hand-typed address, or a similar NAT
setup outside WSL2.

`share_addresses()` in `idaptik-multiplayer-launcher.sh` used to print
whatever `ip -4 route get 1.1.1.1 | grep -oP 'src \K[0-9.]+'` returned and
label it `LAN:`. Under WSL2's default (NAT) networking, that command reports
the **WSL2 virtual adapter's own address** (a `172.16.0.0/12` address — e.g.
`172.28.101.65`) — not the Windows host's real LAN address. That address is
NAT'd behind Windows and cannot be reached from any other machine, even one
on the same physical network. Forwarding the relay's TCP port on the home
router does not fix this either: the router can see the Windows host, but
not past it into WSL2.

A user hit this directly: shared the printed "LAN" address with a remote
player, who got `ended_no_peer` because the address was never reachable in
the first place — the relay itself was working fine.

The launcher now detects this case (`is_wsl` / `wsl_natted` in
`idaptik-multiplayer-launcher.sh`) and prints a warning plus the two routes
that actually work — a Tailscale/WireGuard tunnel (recommended), or a
router port-forward *combined with* a Windows `netsh interface portproxy`
hop into WSL2. WSL2 **mirrored** networking (Windows 11 22H2+) is unaffected
since it shares the host's real LAN address.

### 2. The 15-second join timeout is easy to miss when coordinating by hand

`idaptik-netplay`'s `--join-timeout-ms` defaults to `15000` (15 seconds;
see `join_timeout_ms` in `crates/idaptik-net/src/bin/netplay.rs`). Each seat
starts its own 15-second countdown independently, from the moment *that
seat* starts waiting — not from when the other seat starts.

Two humans coordinating an address over a separate channel (voice, chat)
routinely burn more than 15 seconds typing and pasting the join command, so
both seats give up and report `ended_no_peer` even though the relay and the
network path are both fine.

Workarounds:
- Have both players pre-type their `host`/`join` commands before either
  presses Enter, so both seats start waiting within a couple of seconds of
  each other.
- Or run the seat binary directly (bypassing the launcher, which does not
  currently expose this flag) with more slack:
  ```sh
  target/release/idaptik-netplay --interactive \
    --url ws://<host>:4000/socket/websocket --session ghost-lobby --role hacker \
    --script fixtures/session_relay/versus_script.json \
    --join-timeout-ms 60000
  ```

## Terminal left blank/black after a session ends

`TerminalFrontend` (`crates/idaptik-net/src/interactive.rs`) restores the
terminal — leaves raw mode, leaves the alternate screen, shows the cursor —
in its `Drop` impl, so it runs on every normal exit path. That cleanup uses
`let _ = …` throughout (see the `Drop for TerminalFrontend` block), so if
any one of those calls fails, the failure is silently swallowed and the
terminal is left in whatever state it was in — which can look like a blank
or black screen, even after a session that ended successfully
(`LiveEnd::Completed`).

If this happens:
- Type `reset` at the shell and press Enter, even blind (the terminal isn't
  reading your keystrokes, but the shell is).
- If that doesn't recover it, open a new terminal and run
  `pkill -f idaptik-netplay`. The launcher's own `--stop` only stops the
  relay (Elixir/Phoenix) — it does not touch the seat process, which is
  `exec`'d directly into your terminal by `run_seat()` in
  `idaptik-multiplayer-launcher.sh` and has no other way to be told to quit
  from outside.
