<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
# idaptik-ffi Zig adapter — executable boundary, one-for-one with the Idris2 model

Issue #103: the adapter is the executable Zig counterpart of the C boundary
`crates/idaptik-ffi/include/idaptik.h`, kept in lockstep with the Idris2 model
(`../abi/src/Idaptik/Abi/Exports.idr`) by
`scripts/zig_adapter_surface_check.sh` and gated in CI by
`.github/workflows/abi-conformance.yml`.

## One-for-one correspondence (header order)

| # | C export (`idaptik.h`) | Idris2 model | Zig adapter |
|---|---|---|---|
| 1 | `idap_demo_network` | `idap_demo_network` | `Adapter.demoNetwork` |
| 2 | `idap_network_free` | `idap_network_free` (linear free-once) | `Adapter.networkFree` (null no-op) |
| 3 | `idap_network_device_count` | `idap_network_device_count` (null → 0) | `Adapter.networkDeviceCount` |
| 4 | `idap_ghost_lobby_new` | `idap_ghost_lobby_new` (null on bad difficulty, never panics) | `Adapter.ghostLobbyNew` (difficulty gate mirrors `parseDifficulty`) |
| 5 | `idap_ghost_lobby_tick_json` | `idap_ghost_lobby_tick_json` (LPair: handle handed back + owned wire) | `Adapter.ghostLobbyTickJson` → `Pair(*GhostLobbyHandle, OwnedWire)` |
| 6 | `idap_ghost_lobby_snapshot_json` | `idap_ghost_lobby_snapshot_json` | `Adapter.ghostLobbySnapshotJson` |
| 7 | `idap_ghost_lobby_free` | `idap_ghost_lobby_free` | `Adapter.ghostLobbyFree` |
| 8 | `idap_string_free` | `idap_string_free` (consumed exactly once) | `OwnedWire.deinit` (traps on double free in safe builds) |

Shape discrimination (`[` vs `{` for the tick wire) mirrors
`Idaptik.Abi.Json.renderTickIdentifiesShape`; snapshot honestly stays an
object on success and failure, as the model records.

## Conformance

`test/conformance_test.zig` drives `fixtures/abi/conformance-vectors.json`
(behaviour expectations from the Rust truth, structural expectations from the
model) through the live cdylib, and asserts every case in
`fixtures/abi/planted-failure.json` **fails** — the planted failing fixture
required by the issue's definition of done. If the harness ever reports a
planted case as passing, CI fails: the harness has gone blind.

Known model↔truth divergence recorded in the vectors file: the Idris model
idealises the demo network as 4 devices; the Rust truth exports 6. The
fixtures follow the truth; the model must be reconciled by its owners.

## Authority

The canonical unified-api-adapter surface is pinned in
`docs/abi/ABI-AUTHORITY.md` (`hyperpolymath/hypatia@2bbd3b2` digests, current
revision `bcb8bb37`); CI re-derives the digests and rejects drift.

## Run it

```bash
cargo build -p idaptik-ffi
cd crates/idaptik-ffi/zig-adapter
zig build test          # adapter unit tests
zig build conformance   # fixture conformance against the cdylib
```
