#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Proves the OwnedWire double-free trap fires: compiles a tiny probe that
# frees one wire twice and asserts the process DIES (panic => nonzero exit).
# Used by .github/workflows/abi-conformance.yml (issue #103).

set -euo pipefail
cd "$(dirname "$0")/../crates/idaptik-ffi/zig-adapter"

LIBDIR="$(cd ../../../target/debug 2>/dev/null && pwd || true)"
[ -n "$LIBDIR" ] && [ -f "$LIBDIR/libidaptik_ffi.so" ] || {
  echo "probe: build the cdylib first (cargo build -p idaptik-ffi)"; exit 1;
}

cat > .double-free-probe.zig <<'Z'
const std = @import("std");
const adapter = @import("idaptik_ffi_adapter");
const c = adapter.c;

pub fn main() !void {
    const dbuf = try std.heap.page_allocator.allocSentinel(u8, 8, 0);
    @memcpy(dbuf[0..8], "standard");
    const h = c.idap_ghost_lobby_new(1, dbuf.ptr) orelse return error.Setup;
    var pair = adapter.Adapter.ghostLobbyTickJson(h, "[]");
    if (pair.wire.isNull()) return error.NoWire;
    pair.wire.deinit(); // first free: fine
    pair.wire.deinit(); // second free: MUST trap
    std.debug.print("TRAP DID NOT FIRE — double free went through\n", .{});
    return error.TrapDidNotFire;
}
Z
trap 'rm -f .double-free-probe.zig' EXIT

ZIG="${ZIG:-zig}"
if "$ZIG" build-exe \
    --dep idaptik_ffi_adapter -Mroot=.double-free-probe.zig \
    -Midaptik_ffi_adapter=src/idaptik_ffi_adapter.zig \
    -lc -L"$LIBDIR" -lidaptik_ffi -femit-bin=.double-free-probe; then
  :
else
  echo "probe: compile failed (this is a harness bug, not a pass)"; exit 1
fi
if ./.double-free-probe >/dev/null 2>&1; then
  echo "FAIL: double-free did not trap"; exit 1
fi
echo "OK: OwnedWire double-free traps (process aborted)"
