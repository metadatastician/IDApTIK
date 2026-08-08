# Multiplayer troubleshooting

Known, reproduced failure modes of the multiplayer relay and launch path.
`launcher.sh` and `idaptik-multiplayer-launcher.sh` now run the Bevy GUI;
`idaptik-netplay` remains a direct TUI/verifier binary rather than the
player-facing launcher target. Add to this file when a new failure is
confirmed; don't speculate about ones that haven't been hit.

## A peer cannot join even though the relay is up

### The shared address is unreachable (WSL2 NAT)

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

## The GUI does not open

Run the player-facing diagnostic before retrying:

```sh
./launcher.sh --doctor
```

The interactive menu also exposes **Diagnostics and safe repair**. Fatal
display faults stop before either the relay or GUI starts, so repeated retries
cannot accumulate invisible windows or misleading multiplayer sessions. Open
the friendly decision flow with `./launcher.sh --flowdiags`; it uses the
Windows browser under WSL and does not depend on WSLg working.

### WSLg shows `[WARN: COPY MODE]`

This title prefix comes from WSLg, not the game. It means WSLg has fallen back
from its shared-memory VAIL window transport to RAIL pixel copying. A confirmed
failure can leave a penguin taskbar item that cannot be restored/maximised and
can force Bevy onto the slow llvmpipe renderer.

The launcher checks both `/mnt/shared_memory` and the current WSLg Weston boot
log. If the mount is missing or Weston reports `rdp_allocate_shared_memory`
with an input/output error, launch is blocked before any process starts. From
Windows PowerShell as Administrator run:

```powershell
wsl --update
wsl --shutdown
```

Reopen the distribution and rerun `./launcher.sh --doctor`. Restart Windows if
Copy Mode remains. Do not manually create `/mnt/shared_memory`: that can hide
the symptom without restoring WSLg's host-backed transport.

Missing audio and software rendering are reported separately as degradations;
they do not block play. `./launcher.sh --repair` installs pinned repository
toolchains and can guide Tailscale bootstrap/authentication, but intentionally
cannot restart WSL or alter Windows GPU drivers from inside the session.

## Host/join readiness choreography

The host must not invite the second player until the launcher prints the green
`READY FOR JOINERS` result. It means diagnostics passed, the Bevy build
completed, the relay answered its health probe, a peer-reachable address was
found, and the host seat is launching. Under WSL2 NAT, an authenticated
Tailscale route is required.

The joiner should start only after receiving that message. Their launcher
performs the same local checks and probes the exact host/port. It prints
`READY — JOINING HOST NOW` only after the local build is complete and the relay
answers. A cross is blocking; a warning is a disclosed degradation.

The main launcher builds and starts Bevy in the background. It reports the
startup log location when launching; by default this is
`$XDG_STATE_HOME/idaptik/game.log` (or
`$HOME/.local/state/idaptik/game.log`). Check it with:

```sh
tail -50 "${XDG_STATE_HOME:-$HOME/.local/state}/idaptik/game.log"
```

Run `./launcher.sh --status` to distinguish a slow first build from a process
that has exited, and `./launcher.sh --stop` to stop both the GUI and a relay
started by the host path.

## Legacy direct TUI use

The `idaptik-netplay --interactive` binary still has a 15-second default join
timeout and uses terminal raw mode. Those behaviours apply only when invoking
that binary directly; neither launcher selects it now. If a direct TUI run
leaves the terminal in raw/alternate-screen state, type `reset` (even blind)
or open another terminal and stop `idaptik-netplay`.
