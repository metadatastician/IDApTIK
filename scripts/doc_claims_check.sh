#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Gate documentation claims against machine-derived status (issue #119).
#
# This repository was bitten three times in one review by documents that were
# more confident than their implementations: a digest that was simply false, a
# CI step *named* "diff against docs/abi/ABI-AUTHORITY.md" whose script never
# opened that file, and prose claiming CI "rejects drift" when every check
# fetches at a fixed revision. Each was caught by a human reading carefully.
# That does not scale. Two checks here:
#
#   1. digest coverage - every 64-hex digest written into docs/ also appears
#      in a workflow or script, so no document can carry a load-bearing hash
#      that no gate knows about.
#   2. step-name claims - a workflow step whose name asserts something about
#      a named file must actually reference that file in its body.
#
# `--self-test` runs the controls. A gate that cannot fail is worse than no
# gate, so every check below has a mutant that must kill it, and every
# extraction has an emptiness guard: two empty sets compare equal.

set -uo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$REPO_DIR"

fail() { printf 'doc-claims: FAIL %s\n' "$1" >&2; exit 1; }

usage() {
  cat <<'USAGE'
usage: doc_claims_check.sh [--root DIR] [--self-test] [-h|--help]

  --root DIR    repository root to check (default: the script's parent)
  --self-test   run the built-in controls and exit
USAGE
}

# --- check 1: every digest in docs/ is known to some gate -------------------
#
# "Covered by a check that re-derives it" is not decidable from the text; what
# IS decidable is set inclusion -- every 64-hex under docs/ also appears in
# .github/workflows/ or scripts/. The re-derivation is the gate's own job
# (abi-conformance.yml does it with sha256sum against a pinned revision); this
# check asserts that no document carries a digest no gate has ever heard of.
#
# Known limit, stated rather than hidden: inclusion cannot tell a digest in a
# prose comment from one a gate actually consumes. It catches the case that
# bit us -- a hash written into a document and nowhere else.
check_digests() {
  local root="$1" docs wf_dirs
  docs="$root/docs"

  if [ ! -d "$docs" ]; then
    printf 'doc-claims: FAIL no docs/ directory under %s -- the check would pass vacuously\n' "$root" >&2
    return 1
  fi

  local doc_set gate_set ndocs
  doc_set="$(grep -rhoE '\b[0-9a-f]{64}\b' "$docs" 2>/dev/null | sort -u)"

  # Emptiness guard. An extraction that silently matches nothing is the
  # commonest way a gate like this goes vacuous: with doc_set empty every
  # digest is trivially covered and the check reports success forever.
  if [ -z "$doc_set" ]; then
    printf 'doc-claims: FAIL no 64-hex digests found under %s -- the extraction matched nothing, so "all covered" is meaningless\n' "$docs" >&2
    return 1
  fi

  wf_dirs=""
  [ -d "$root/.github/workflows" ] && wf_dirs="$wf_dirs $root/.github/workflows"
  [ -d "$root/scripts" ] && wf_dirs="$wf_dirs $root/scripts"
  if [ -z "$wf_dirs" ]; then
    printf 'doc-claims: FAIL neither .github/workflows/ nor scripts/ exists under %s -- nothing could cover a digest\n' "$root" >&2
    return 1
  fi

  # shellcheck disable=SC2086
  gate_set="$(grep -rhoE '\b[0-9a-f]{64}\b' $wf_dirs 2>/dev/null | sort -u)"
  if [ -z "$gate_set" ]; then
    printf 'doc-claims: FAIL no 64-hex digests found in%s -- every documented digest would read as uncovered\n' "$wf_dirs" >&2
    return 1
  fi

  local uncovered nunc ngate
  uncovered="$(comm -23 <(printf '%s\n' "$doc_set") <(printf '%s\n' "$gate_set"))"
  ndocs="$(printf '%s\n' "$doc_set" | grep -c .)"
  ngate="$(printf '%s\n' "$gate_set" | grep -c .)"
  nunc="$(printf '%s\n' "$uncovered" | grep -c . || true)"

  if [ "$nunc" -gt 0 ]; then
    local d
    while IFS= read -r d; do
      [ -n "$d" ] || continue
      local where
      where="$(grep -rn "$d" "$docs" 2>/dev/null | head -1 | cut -d: -f1,2)"
      printf 'doc-claims: FAIL %s digest %s is written into a document but appears in no workflow or script -- either a gate must re-derive it or it must not be recorded as fact\n' \
        "${where:-$docs}" "$d" >&2
    done <<< "$uncovered"
    return 1
  fi

  printf 'DIGESTS %s %s\n' "$ndocs" "$ngate"
  return 0
}

# --- check 2: a step name that asserts something about a file must do it ----
#
# The naive form of this rule -- "a step name containing a filename must
# reference it in the run block" -- was MEASURED on this repository before
# being written, and flags 12 steps of which 11 are legitimate: `rustup show`
# genuinely reads rust-toolchain.toml without naming it. A gate that is 91%
# false positives is switched off in a week, so the rule is scoped by grammar.
#
# The discriminator is the parenthesis, because it marks a different speech
# act. A filename in the main clause is a CLAIM about what the step does to
# that file -- "Re-derive pinned digests and diff against
# docs/abi/ABI-AUTHORITY.md" -- and the body must honour it. A filename in
# parentheses is a PROVENANCE note -- "Provision pinned Rust toolchain
# (rust-toolchain.toml)" -- which says where something came from, not what
# this step does to it, and is exempt.
#
# A verb allowlist was considered and rejected: in the main clause every name
# is already a claim so the verbs never fire, and in the parenthesised branch
# they create false positives -- "Check formatting (rustfmt.toml)" would match
# on "check" and fail an honest step.
check_step_claims() {
  local root="$1" wfdir
  wfdir="$root/.github/workflows"

  if [ ! -d "$wfdir" ]; then
    printf 'doc-claims: FAIL no .github/workflows/ under %s -- the check would pass vacuously\n' "$root" >&2
    return 1
  fi

  local files
  files="$(find "$wfdir" -maxdepth 1 \( -name '*.yml' -o -name '*.yaml' \) -type f | sort)"
  if [ -z "$files" ]; then
    printf 'doc-claims: FAIL no workflow files under %s -- the check would pass vacuously\n' "$wfdir" >&2
    return 1
  fi

  # shellcheck disable=SC2086
  awk -v ROOT="$root" -f /dev/stdin $files <<'AWK_EOF'
BEGIN {
  # Extensions are an explicit allowlist, not a generic "dot then letters".
  # A generic pattern reads `actions/checkout@v7.0.1` and `Idris 2 (0.8.x)` as
  # filenames, which would make the gate argue with version numbers.
  EXT = "(yml|yaml|toml|json|md|sh|rs|zig|idr|ipkg|lock|nix|tsv|adoc|ncl|cfg|ini|txt|h|c)"
  FILE_RE = "[.]?[A-Za-z0-9_][A-Za-z0-9_.@/-]*\\." EXT
  nclaim = 0; nsat = 0; nprov = 0; nbad = 0; nnamed = 0
}

# Reset per-file state. A step left open at EOF of one file must not absorb
# the first lines of the next.
FNR == 1 { flush(); in_step = 0 }

{
  line = $0
  # Indent of the "- " that opens a sequence item, or -1.
  if (match(line, /^[ \t]*-[ \t]+[A-Za-z_][A-Za-z0-9_-]*:/)) {
    ind = match(line, /-/) - 1
    # A new item at this indent or shallower closes the current step.
    if (in_step && ind <= step_ind) flush()
    if (!in_step) { in_step = 1; step_ind = ind; sname = ""; sbody = ""; sfile = FILENAME; sline = FNR }
    else { sbody = sbody "\n" line }
    grab_name(line)
    next
  }
  if (in_step) {
    # A non-blank line shallower than the step's "- " ends the block.
    if (line ~ /[^ \t]/) {
      nind = match(line, /[^ \t]/) - 1
      if (nind <= step_ind) { flush(); next }
    }
    sbody = sbody "\n" line
    grab_name(line)
  }
}

END { flush(); report() }

function grab_name(l,   m) {
  if (sname != "") return
  if (match(l, /(^|[ \t-])name:[ \t]*/)) {
    m = substr(l, RSTART + RLENGTH)
    sub(/[ \t]+$/, "", m)
    # Strip one layer of YAML quoting.
    if (m ~ /^".*"$/ || m ~ /^'.*'$/) m = substr(m, 2, length(m) - 2)
    sname = m
  }
}

# True when the character run before position p has an unclosed "(".
function in_parens(s, p,   pre, i, ch, depth) {
  pre = substr(s, 1, p - 1); depth = 0
  for (i = 1; i <= length(pre); i++) {
    ch = substr(pre, i, 1)
    if (ch == "(") depth++
    else if (ch == ")" && depth > 0) depth--
  }
  return depth > 0
}

function basename(p,   n) { n = p; sub(/^.*\//, "", n); return n }

function flush(   name, body, rest, off, fn, bare, claimed) {
  if (!in_step) { in_step = 0; return }
  in_step = 0
  name = sname; body = sbody
  if (name == "") return
  nnamed++
  rest = name; off = 0
  while (match(rest, FILE_RE)) {
    fn = substr(rest, RSTART, RLENGTH)
    # A trailing dot or backtick is punctuation, not part of the name.
    sub(/[.,;:`'"]+$/, "", fn)
    if (in_parens(name, off + RSTART)) {
      nprov++
    } else {
      nclaim++
      bare = basename(fn)
      claimed = (index(body, fn) > 0 || (bare != fn && index(body, bare) > 0))
      if (claimed) {
        nsat++
      } else {
        nbad++
        printf("doc-claims: FAIL %s:%d step name claims `%s` but nothing in the step references it\n", short(sfile), sline, fn) > "/dev/stderr"
        printf("            name: %s\n", name) > "/dev/stderr"
        printf("            a filename in the main clause is a claim the step must honour; if the step merely takes its\n") > "/dev/stderr"
        printf("            values FROM that file, put the filename in parentheses and it is read as provenance\n") > "/dev/stderr"
      }
    }
    off += RSTART + RLENGTH - 1
    rest = substr(rest, RSTART + RLENGTH)
  }
  sname = ""; sbody = ""
}

function short(p,   s) { s = p; if (index(s, ROOT "/") == 1) s = substr(s, length(ROOT) + 2); return s }

function report() {
  if (nbad > 0) exit 1
  printf("STEPS %d %d %d %d\n", nnamed, nclaim, nsat, nprov)
}
AWK_EOF
}

# --- self-test --------------------------------------------------------------
self_test() {
  # Built from two 32-hex halves on purpose: a 64-hex literal in this file
  # would be picked up by check 1 as a gate-side digest, so the gate would
  # count its own test fixtures.
  local D_GOOD D_ORPHAN
  D_GOOD="aa11bb22cc33dd44ee55ff6600778899""aa11bb22cc33dd44ee55ff6600778899"
  D_ORPHAN="deadbeef00112233445566778899aabb""ccddeeff00112233445566778899aabb"
  local T rc pass=0 total=0
  T="$(mktemp -d)"
  trap 'rm -rf "$T"' RETURN

  newcase() {
    rm -rf "$T/r"
    mkdir -p "$T/r/docs" "$T/r/.github/workflows" "$T/r/scripts"
    # A minimal consistent world: one digest, recorded in a doc and known to a
    # script, and one workflow whose only named step honours its own name.
    printf 'digest: %s\n' "$D_GOOD" > "$T/r/docs/pin.md"
    printf '#!/bin/sh\nsha256sum -c <<< "%s  x"\n' "$D_GOOD" > "$T/r/scripts/verify.sh"
    cat > "$T/r/.github/workflows/w.yml" <<'WF'
name: w
on: [push]
jobs:
  j:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v7.0.1
      - name: Verify digests against docs/pin.md
        run: |
          grep -oE '[0-9a-f]{64}' docs/pin.md
WF
  }

  expect() { # want-rc name
    local want="$1" nm="$2" got
    total=$((total + 1))
    check_digests "$T/r" >/dev/null 2>&1 && check_step_claims "$T/r" >/dev/null 2>&1
    got=$?
    if [ "$got" -eq "$want" ]; then
      pass=$((pass + 1))
      printf '  ok   %s\n' "$nm"
    else
      printf '  FAIL %s (wanted rc=%s, got rc=%s)\n' "$nm" "$want" "$got" >&2
    fi
  }

  # (a) the consistent world passes -- without this every "fails" below could
  #     be the harness failing for an unrelated reason.
  newcase
  expect 0 "a consistent tree passes"

  # --- check 1 controls ---
  # (b) a digest in a document that no gate knows about.
  newcase
  printf 'orphan: %s\n' "$D_ORPHAN" >> "$T/r/docs/pin.md"
  expect 1 "a documented digest with no gate counterpart fails"

  # (c) emptiness guard: no digests anywhere in docs/.
  newcase
  printf 'no digests here\n' > "$T/r/docs/pin.md"
  expect 1 "a docs/ tree with no digests fails rather than passing vacuously"

  # (d) emptiness guard: docs/ absent entirely.
  newcase
  rm -rf "$T/r/docs"
  expect 1 "a missing docs/ directory fails rather than passing vacuously"

  # (e) emptiness guard: nothing gate-side carries a digest.
  newcase
  printf '#!/bin/sh\necho hi\n' > "$T/r/scripts/verify.sh"
  sed -i 's/[0-9a-f]\{64\}/REDACTED/' "$T/r/.github/workflows/w.yml" 2>/dev/null || true
  expect 1 "a tree whose gates carry no digests fails"

  # --- check 2 controls ---
  # (f) the historical defect: a name promising a diff, a body that never opens it.
  newcase
  cat > "$T/r/.github/workflows/w.yml" <<'WF'
name: w
on: [push]
jobs:
  j:
    steps:
      - name: Re-derive digests and diff against docs/pin.md
        run: echo nothing to see here
WF
  expect 1 "a step name claiming a file the body never references fails"

  # (g) the same name, honoured.
  newcase
  cat > "$T/r/.github/workflows/w.yml" <<'WF'
name: w
on: [push]
jobs:
  j:
    steps:
      - name: Re-derive digests and diff against docs/pin.md
        run: grep -c . docs/pin.md
WF
  expect 0 "a step name whose body references the file passes"

  # (h) provenance: parenthesised, unreferenced, exempt. This is the case the
  #     naive rule got wrong 10 times on the real tree.
  newcase
  cat > "$T/r/.github/workflows/w.yml" <<'WF'
name: w
on: [push]
jobs:
  j:
    steps:
      - name: Provision pinned Rust toolchain (rust-toolchain.toml)
        run: rustup show
WF
  expect 0 "a parenthesised filename with no reference is provenance, not a claim"

  # (i) a multi-line run block satisfies the claim.
  newcase
  cat > "$T/r/.github/workflows/w.yml" <<'WF'
name: w
on: [push]
jobs:
  j:
    steps:
      - name: Validate docs/pin.md
        run: |
          set -eu
          wc -l docs/pin.md
WF
  expect 0 "a claim satisfied inside a multi-line run block passes"

  # (j) a claim satisfied by `with:` rather than `run:`.
  newcase
  cat > "$T/r/.github/workflows/w.yml" <<'WF'
name: w
on: [push]
jobs:
  j:
    steps:
      - name: Lint scripts/verify.sh
        uses: some/linter@v1
        with:
          path: scripts/verify.sh
WF
  expect 0 "a claim satisfied by a with: input passes"

  # (k) a version number is not a filename -- otherwise the gate argues with
  #     `actions/checkout@v7.0.1` on every step that pins an action.
  newcase
  cat > "$T/r/.github/workflows/w.yml" <<'WF'
name: w
on: [push]
jobs:
  j:
    steps:
      - name: Bootstrap Idris 2 v0.8.0 and Chez 10.4.1
        run: echo build
WF
  expect 0 "a version number in a step name is not read as a filename"

  # (l) emptiness guard: a workflows directory with no workflow files.
  newcase
  rm -f "$T/r/.github/workflows/"*.yml
  expect 1 "an empty workflows directory fails rather than passing vacuously"

  # (m) the claim is detected in the SECOND of two steps, so a passing first
  #     step cannot mask it.
  newcase
  cat > "$T/r/.github/workflows/w.yml" <<'WF'
name: w
on: [push]
jobs:
  j:
    steps:
      - name: Validate docs/pin.md
        run: wc -l docs/pin.md
      - name: Compare against scripts/verify.sh
        run: echo unrelated
WF
  expect 1 "a bad claim in a later step is not masked by an earlier good one"

  printf 'doc-claims: self-test %s/%s controls passed (%s of them mutants that must be killed)\n' \
    "$pass" "$total" "$((total - 6))"
  [ "$pass" -eq "$total" ] || return 1
  return 0
}

# --- main -------------------------------------------------------------------
DO_SELFTEST=0
while [ $# -gt 0 ]; do
  case "$1" in
    --root) ROOT="${2:-}"; shift 2 ;;
    --self-test) DO_SELFTEST=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'doc-claims: unknown argument: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done
ROOT="${ROOT%/}"   # a trailing slash would double in every reported path

if [ "$DO_SELFTEST" -eq 1 ]; then
  self_test || fail "self-test did not pass -- the gate cannot be trusted to fail"
  exit 0
fi

d_out="$(check_digests "$ROOT")" || fail "digest coverage check did not pass (see above)"
s_out="$(check_step_claims "$ROOT")" || fail "step-name claim check did not pass (see above)"

# shellcheck disable=SC2086
set -- $d_out
printf 'doc-claims: ok (%s digests in docs/, all known to one of %s digests across workflows and scripts)\n' "$2" "$3"
# shellcheck disable=SC2086
set -- $s_out
printf 'doc-claims: ok (%s named workflow steps: %s name a file as a claim, %s of those satisfied; %s provenance notes exempt)\n' "$2" "$3" "$4" "$5"
