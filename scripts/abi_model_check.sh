#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Gate the Idris2 model of the idaptik-ffi C ABI (issue #103):
#
#   1. escape scan  - scripts/idris_escape_scan.sh: no proof escape
#                     (postulate, believe_me, assert_total, partial, ...)
#                     outside comments, every packaged module under
#                     %default total, and any escape kept must be waived
#                     by a reviewed row in abi/ESCAPE-LEDGER.tsv. Runs
#                     before the toolchain check, so it gates even where
#                     no Idris2 compiler exists.
#   2. typecheck    - idris2 --build under %default total: every property
#                     the model states is machine-checked
#   3. run          - the smoke session executes end-to-end under the Chez
#                     backend (typechecking proved it; this runs it)
#   4. header diff  - the model's exportedFunctions list is one-for-one with
#                     the idap_* exports of include/idaptik.h
#   5. format diff  - the model's snapshot format tag equals the Rust
#                     SNAPSHOT_FORMAT constant, so the two cannot drift
#
# Toolchain resolution: $IDRIS2, then the known bootstrap locations (the
# sandbox workspace keeps one under /home/user/idris-toolchain; CI builds its
# own under /tmp), then idris2 from PATH. A missing toolchain is a failure,
# never a skip — build one with the recipe in
# crates/idaptik-ffi/abi/README.md (also used by the CI abi-model job).

set -uo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ABI_DIR="$REPO_DIR/crates/idaptik-ffi/abi"
SMOKE_DIR="$ABI_DIR/smoke"
HEADER="$REPO_DIR/crates/idaptik-ffi/include/idaptik.h"

fail() { printf 'abi-model: FAIL %s\n' "$1" >&2; exit 1; }

# Build steps are silenced so a passing gate prints one line. But a step
# whose failure message says "see output above" while its output went to
# /dev/null is a diagnosis dead end -- the compiler error is the one thing
# a reader needs. Keep the output, and print it only when the step fails.
BUILD_LOG="$(mktemp)"
trap 'rm -f "$BUILD_LOG"' EXIT
dump_log() {
  printf 'abi-model: --- %s output ---\n' "$1" >&2
  cat "$BUILD_LOG" >&2
  printf 'abi-model: --- end %s output ---\n' "$1" >&2
}

# --- 1. escape scan --------------------------------------------------------
# Deliberately first: it needs no compiler, so a proof escape is caught on
# every run rather than only on runs where the Idris2 toolchain resolved.
# The old inline grep silently scanned nothing when the toolchain was
# missing, and matched `partialOrder` as if it were `partial`.
bash "$REPO_DIR/scripts/idris_escape_scan.sh" || fail "proof-escape scan failed (see above)"

# --- toolchain -------------------------------------------------------------
IDRIS2="${IDRIS2:-}"
if [ -z "$IDRIS2" ]; then
  for candidate in /home/user/idris-toolchain/bin/idris2 /tmp/idris-bootstrap/bin/idris2 "$(command -v idris2 2>/dev/null)"; do
    if [ -n "$candidate" ] && [ -x "$candidate" ]; then IDRIS2="$candidate"; break; fi
  done
fi
[ -n "$IDRIS2" ] && [ -x "$IDRIS2" ] || fail "no idris2 toolchain (set IDRIS2 or bootstrap per abi/README.md)"

# The Chez codegen shells out to `scheme`; the sandbox bootstrap keeps it
# next to the compiler.
for scheme_dir in /tmp/idris-bootstrap/chez-native/bin; do
  [ -d "$scheme_dir" ] && PATH="$scheme_dir:$PATH"
done
export PATH

# --- 2. typecheck ----------------------------------------------------------
if ! (cd "$ABI_DIR" && "$IDRIS2" --build idaptik-abi.ipkg) >"$BUILD_LOG" 2>&1; then
  dump_log typecheck
  fail "typecheck failed (the model is total and checked; compiler output above)"
fi

# --- 3. runtime smoke ------------------------------------------------------
if ! (cd "$ABI_DIR" && "$IDRIS2" --install idaptik-abi.ipkg) >"$BUILD_LOG" 2>&1; then
  dump_log "package install"
  fail "package install failed"
fi
if ! (cd "$SMOKE_DIR" && rm -rf build && "$IDRIS2" --build smoke.ipkg) >"$BUILD_LOG" 2>&1; then
  dump_log "smoke build"
  fail "smoke build failed"
fi
smoke_out="$("$SMOKE_DIR/build/exec/abi-smoke" 2>&1)" \
  || fail "smoke execution failed: $smoke_out"
printf '%s\n' "$smoke_out" | grep -q '^session: tick + snapshot + free complete$' \
  || fail "smoke session did not complete: $smoke_out"
printf '%s\n' "$smoke_out" | grep -q '^smoke: interior NUL rejected$' \
  || fail "interior-NUL rejection missing: $smoke_out"

# --- 4. header correspondence ---------------------------------------------
header_fns="$(grep -oE '\bidap_[a-z_]+\b' "$HEADER" | sort -u)"
model_fns="$(awk '/^exportedFunctions/,/^\]/' "$ABI_DIR/src/Idaptik/Abi/Exports.idr" \
  | grep -oE '"idap_[a-z_]+"' | tr -d '"' | sort -u)"
[ "$header_fns" = "$model_fns" ] \
  || fail "model/header drift:
header: $(printf '%s ' $header_fns)
model:  $(printf '%s ' $model_fns)"

# --- 5. snapshot format parity --------------------------------------------
rust_tag="$(grep -oE 'SNAPSHOT_FORMAT: &str = "[^"]+"' "$REPO_DIR/crates/idaptik-core/src/scenario/snapshot.rs" | grep -oE '"[^"]+"' | tr -d '"')"
idris_tag="$(grep -oE 'formatTag RuntimeV3 = "[^"]+"' "$ABI_DIR/src/Idaptik/Abi/Run.idr" | grep -oE '"[^"]+"' | tr -d '"')"
[ -n "$rust_tag" ] || fail "could not read SNAPSHOT_FORMAT from snapshot.rs"
[ "$rust_tag" = "$idris_tag" ] \
  || fail "snapshot format drift: rust=$rust_tag idris=$idris_tag"

echo "abi-model: ok (escapes scanned, typechecked, smoke ran, header + format parity)"
