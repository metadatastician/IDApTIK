#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Kill mutants in the Idris2 ABI model's proofs (issue #117).
#
# `scripts/abi_model_check.sh` establishes that the model compiles under
# `--total` and agrees with the C header. Neither fact says the PROOFS have
# teeth. A theorem stated about an uninhabited type is true and worthless; a
# lemma the elaborator discharges for a reason unrelated to its claim compiles
# exactly as happily as one that earns it. The only evidence that a proof
# constrains anything is that falsifying it breaks the build.
#
# So each mutant below negates exactly one lemma, and must be
#
#   1. rejected -- the build fails, and
#   2. rejected BY NAME -- the compiler output mentions the mutated lemma.
#
# (2) is the part that matters. A non-zero exit code proves only that
# something went wrong: a typo, a missing import, a toolchain fault and a
# genuinely dead mutant are indistinguishable by rc alone, and a mutant
# "killed" by an unrelated error is a false pass that looks exactly like a
# real one.
#
# Every mutation is applied to a COPY. The working tree is never modified, so
# an interrupted run cannot leave a falsified lemma behind.
#
# Usage:  bash scripts/abi_proof_mutants.sh          (IDRIS2 from env or PATH)

set -uo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ABI_DIR="$REPO_DIR/crates/idaptik-ffi/abi"
IDRIS2="${IDRIS2:-$(command -v idris2 2>/dev/null)}"

fail() { printf 'proof-mutants: FAIL %s\n' "$1" >&2; exit 1; }

[ -n "$IDRIS2" ] && [ -x "$IDRIS2" ] || fail "no idris2 toolchain (set \$IDRIS2)"
[ -d "$ABI_DIR" ] || fail "no abi package at $ABI_DIR"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
CASE="$WORK/case"

# Build the package as it stands in $CASE. Echoes the compiler output; the
# caller reads the exit status.
build_case() {
  ( cd "$CASE" && rm -rf build && "$IDRIS2" --build idaptik-abi.ipkg ) 2>&1
}

reset_case() {
  rm -rf "$CASE"
  cp -r "$ABI_DIR" "$CASE"
  rm -rf "$CASE/build" "$CASE/smoke/build"
}

# --- positive control ------------------------------------------------------
# Without this, a tree that fails to build for an unrelated reason would
# report every mutant as killed, and the suite would pass while proving
# nothing whatever.
reset_case
if ! control_out="$(build_case)"; then
  printf '%s\n' "$control_out" >&2
  fail "the unmutated model does not build -- every mutant below would 'die' for the wrong reason"
fi
echo "proof-mutants: control ok (unmutated model builds)"

# --- the mutants -----------------------------------------------------------
# id | file relative to the abi package | sed expression | lemma that must be named
#
# Between them these cover all four obligations of #117: ownership and
# lifetime (O1), string pairing (O2), wire shape and NUL-freedom (O3), and
# the deterministic clock (O4), plus the protocol/header tie-in.
mutants=$(cat <<'TABLE'
o1-free-count	src/Idaptik/Abi/Protocol.idr	s/typicalFreesOnce : freeCount Protocol.typicalSession = 1/typicalFreesOnce : freeCount Protocol.typicalSession = 2/	typicalFreesOnce
o1-double-free	src/Idaptik/Abi/Protocol.idr	s/:: FreeNull :: Free :: Done/:: FreeNull :: Free :: Free :: Done/	typicalSession
o2-string-count	src/Idaptik/Abi/Protocol.idr	s/typicalFreesTwoStrings : freeStrCount Protocol.typicalSession = 2/typicalFreesTwoStrings : freeStrCount Protocol.typicalSession = 3/	typicalFreesTwoStrings
abi-wrong-export	src/Idaptik/Abi/Protocol.idr	s/entryPoint (Tick _) = "idap_ghost_lobby_tick_json"/entryPoint (Tick _) = "idap_network_free"/	entryPointIsExported
o3-wrong-digits	src/Idaptik/Abi/Json.idr	s/natCharsIsDecimal : natChars 123 = \['1', '2', '3'\]/natCharsIsDecimal : natChars 123 = ['1', '2', '4']/	natCharsIsDecimal
o3-wrong-opener	src/Idaptik/Abi/Json.idr	s/headChar (renderSnapshotChars r) = Just '{'/headChar (renderSnapshotChars r) = Just '['/	renderSnapshotIsObject
o4-clock-off-by-one	src/Idaptik/Abi/Run.idr	s/tick (runState s css) = tick s + length css/tick (runState s css) = tick s + S (length css)/	runAdvancesByFrameCount
TABLE
)

total=0
survived=0
while IFS="$(printf '\t')" read -r id rel expr lemma; do
  [ -n "$id" ] || continue
  total=$((total + 1))
  reset_case
  [ -f "$CASE/$rel" ] || { printf 'proof-mutants: FAIL [%s] %s does not exist\n' "$id" "$rel" >&2; survived=$((survived + 1)); continue; }

  # A sed expression that no longer matches would silently mutate nothing.
  # The build would then pass and the mutant would look like a survivor --
  # correct in outcome, misleading in diagnosis. Say which it is.
  before="$(cksum < "$CASE/$rel")"
  sed -i "$expr" "$CASE/$rel"
  after="$(cksum < "$CASE/$rel")"
  if [ "$before" = "$after" ]; then
    printf 'proof-mutants: FAIL [%s] the mutation matched nothing in %s -- the lemma was renamed or reworded, so this mutant has stopped testing anything\n' "$id" "$rel" >&2
    survived=$((survived + 1))
    continue
  fi

  if out="$(build_case)"; then
    printf 'proof-mutants: FAIL [%s] SURVIVED -- the model still builds with `%s` falsified\n' "$id" "$lemma" >&2
    survived=$((survived + 1))
    continue
  fi
  if ! printf '%s\n' "$out" | grep -q -- "$lemma"; then
    printf 'proof-mutants: FAIL [%s] the build failed but never mentions `%s`, so the mutant died of something else:\n%s\n' \
      "$id" "$lemma" "$out" >&2
    survived=$((survived + 1))
    continue
  fi
  printf 'proof-mutants: killed [%s] (%s rejected)\n' "$id" "$lemma"
done <<EOF
$mutants
EOF

[ "$total" -gt 0 ] || fail "no mutants defined -- this gate would pass vacuously"
[ "$survived" -eq 0 ] || fail "$survived of $total mutants were not killed by name"
printf 'proof-mutants: ok (%d of %d mutants killed, each naming its lemma)\n' "$total" "$total"
