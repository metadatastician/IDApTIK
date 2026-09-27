#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Surface lockstep gate (issue #103): the C header, the Idris2 model, and the
# Zig adapter must enumerate exactly the same exported functions, in order.
# Fails on drift in any of the three.

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HEADER="$ROOT/crates/idaptik-ffi/include/idaptik.h"
IDRIS="$ROOT/crates/idaptik-ffi/abi/src/Idaptik/Abi/Exports.idr"
ZIG="$ROOT/crates/idaptik-ffi/zig-adapter/src/idaptik_ffi_adapter.zig"

fail() { echo "FAIL: $*" >&2; exit 1; }

[ -f "$HEADER" ] || fail "missing $HEADER"
[ -f "$IDRIS" ] || fail "missing $IDRIS"
[ -f "$ZIG" ] || fail "missing $ZIG"

# Header: exported function names in declaration order.
header_list() {
  grep -oE '\bidap_[a-z_0-9]+\s*\(' "$HEADER" | tr -d ' \t(' | awk '!seen[$0]++'
}
# Idris model: exportedFunctions list, in order.
idris_list() {
  sed -n '/exportedFunctions =/,/^]/p' "$IDRIS" | grep -oE '"idap_[a-z_0-9]+"' | tr -d '"'
}
# Zig adapter: exported_functions table, in order.
zig_list() {
  sed -n '/pub const exported_functions = /,/^};/p' "$ZIG" | grep -oE '"idap_[a-z_0-9]+"' | tr -d '"'
}

header_list > /tmp/surface_header.txt
idris_list  > /tmp/surface_idris.txt
zig_list    > /tmp/surface_zig.txt

for f in /tmp/surface_header.txt /tmp/surface_idris.txt /tmp/surface_zig.txt; do
  [ -s "$f" ] || fail "$f is empty — surface extraction failed"
done

diff /tmp/surface_header.txt /tmp/surface_idris.txt >/dev/null \
  || fail "header and Idris model disagree:$(diff /tmp/surface_header.txt /tmp/surface_idris.txt | head -10)"
diff /tmp/surface_header.txt /tmp/surface_zig.txt >/dev/null \
  || fail "header and Zig adapter disagree:$(diff /tmp/surface_header.txt /tmp/surface_zig.txt | head -10)"

echo "OK: header, Idris2 model, and Zig adapter agree on $(wc -l < /tmp/surface_header.txt | tr -d ' ') exports (in order)."
