#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Gate the Idris2 ABI model against proof escapes (issue #118).
#
# Idris2 offers several constructs that silently downgrade a proof to an
# assertion. `believe_me` will produce a value of any type from a value of any
# other; `postulate` asserts a proposition without evidence; `assert_total`
# switches off the totality checker for a subterm; a `partial` or `covering`
# annotation weakens the `%default total` the model relies on. A module that
# loses its `%default total` line loses every proof in it, and no typechecker
# error is emitted -- the code still compiles, it just stops proving anything.
# None of these are visible in a diff unless something is looking for them.
#
# So: scan the source, fail on any occurrence that is not explicitly waived,
# and make a waiver a reviewable object rather than a silent edit.
#
#   - a waived escape carries an inline `-- ESCAPE-WAIVER: <id>` marker on its
#     own line, and a row with that id in the ledger giving the term, the file
#     and the reason. Keying on an id rather than a line number means the
#     waiver travels with the code it excuses: inserting a line above it
#     neither invalidates the waiver nor silently transfers it to a new escape.
#   - an unwaived escape, a marker with no ledger row, a ledger row whose term
#     or file disagrees with the code, and an orphan ledger row with nothing
#     to excuse are all failures. The ledger cannot grow, shrink or drift
#     without a visible diff on this file.
#   - scanning zero files is a failure, not a pass, and the denominator is
#     printed on every run including the successful ones. A glob that matches
#     nothing is the commonest way a check like this goes quietly vacuous.
#   - `--self-test` runs the scanner against a synthetic tree in which each
#     failure mode is planted in turn and asserts that it is caught. A gate
#     that has only ever been observed to say "ok" is not known to be able to
#     say anything else.
#
# Comments are excluded (`--`, `|||` and `{- -}`), so prose *about* the escape
# hatches -- which the model's own module header contains -- does not trip the
# gate. String literals are deliberately NOT excluded: a false positive fails
# loudly and is fixed in one line, whereas stripping strings risks a false
# negative, and a gate's errors should point the safe way.
#
# Usage:  bash scripts/idris_escape_scan.sh [--root DIR] [--ledger FILE]
#         bash scripts/idris_escape_scan.sh --self-test

set -uo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$REPO_DIR/crates/idaptik-ffi/abi"
LEDGER=""
SELF_TEST=0
# Modules the package builds under `%default total`. The smoke harness is a
# separate ipkg and an IO main; it is scanned for escapes like everything
# else, but it is not part of the proof surface, so it is not required to
# carry the pragma.
IPKG="idaptik-abi.ipkg"

while [ $# -gt 0 ]; do
  case "$1" in
    --root)      ROOT="$2"; shift 2 ;;
    --ledger)    LEDGER="$2"; shift 2 ;;
    --self-test) SELF_TEST=1; shift ;;
    -h|--help)   sed -n '3,40p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) printf 'escape-scan: unknown argument: %s\n' "$1" >&2; exit 2 ;;
  esac
done
ROOT="${ROOT%/}"   # a trailing slash would double in every reported path
[ -n "$LEDGER" ] || LEDGER="$ROOT/ESCAPE-LEDGER.tsv"

fail() { printf 'escape-scan: FAIL %s\n' "$1" >&2; exit 1; }

# --- the scan --------------------------------------------------------------
# One awk pass over the ledger followed by every .idr file. Emits diagnostics
# on stderr, a "SCANNED <files> <codelines> <waivers>" line on stdout, and
# exits 1 on any violation.
scan() {
  local root="$1" ledger="$2" files
  files="$(find "$root" -name '*.idr' -type f | sort)"
  if [ -z "$files" ]; then
    printf 'escape-scan: FAIL no .idr files under %s -- the gate would pass vacuously\n' "$root" >&2
    return 1
  fi
  # An absent ledger means "no waivers", which is the healthy state; /dev/null
  # keeps the awk argument list uniform.
  [ -f "$ledger" ] || ledger=/dev/null

  # shellcheck disable=SC2086
  awk -v LEDGER="$ledger" -v ROOT="$root" -f /dev/stdin "$ledger" $files <<'AWK'
BEGIN {
  # Constructs that convert a proof into an assertion. Each is matched whole:
  # `partialOrder` and `coveringSet` are ordinary names, not escapes.
  nterms = split("postulate believe_me really_believe_me assert_total " \
                 "assert_smaller assert_linear unsafePerformIO idris_crash " \
                 "partial covering %unsafe", TERM, " ")
  BOUND = "[^A-Za-z0-9_']"
  violations = 0; files = 0; codelines = 0; waived = 0
}

# --- ledger ---------------------------------------------------------------
# id <TAB> term <TAB> path-relative-to-root <TAB> reason
FILENAME == LEDGER {
  if (LEDGER == "/dev/null") next
  line = $0
  sub(/\r$/, "", line)
  if (line ~ /^[ \t]*#/ || line ~ /^[ \t]*$/) next
  n = split(line, F, "\t")
  if (n < 4) {
    printf("escape-scan: FAIL ledger %s:%d is not 4 tab-separated fields (id, term, file, reason)\n",
           LEDGER, FNR) > "/dev/stderr"
    violations++; next
  }
  id = F[1]
  if (id in L_term) {
    printf("escape-scan: FAIL ledger %s:%d duplicates waiver id %s\n", LEDGER, FNR, id) > "/dev/stderr"
    violations++; next
  }
  if (id !~ /^[A-Za-z0-9_.-]+$/) {
    printf("escape-scan: FAIL ledger %s:%d waiver id %s is not [A-Za-z0-9_.-]+\n", LEDGER, FNR, id) > "/dev/stderr"
    violations++; next
  }
  if (F[4] ~ /^[ \t]*$/) {
    printf("escape-scan: FAIL ledger %s:%d waiver %s has an empty reason\n", LEDGER, FNR, id) > "/dev/stderr"
    violations++; next
  }
  L_term[id] = F[2]; L_file[id] = F[3]; L_row[id] = FNR; L_used[id] = 0
  next
}

# --- sources ---------------------------------------------------------------
FNR == 1 { inblk = 0; files++ }

{
  raw = $0
  sub(/\r$/, "", raw)

  # The waiver marker is read from the RAW line: it lives in a comment, which
  # the code extraction below is about to discard.
  marker = ""
  if (match(raw, /ESCAPE-WAIVER:[ \t]*[A-Za-z0-9_.-]+/)) {
    marker = substr(raw, RSTART, RLENGTH)
    sub(/^ESCAPE-WAIVER:[ \t]*/, "", marker)
  }

  # A doc comment owns the whole line.
  if (!inblk && raw ~ /^[ \t]*\|\|\|/) next

  # Walk the line, dropping `{- -}` spans and everything after a `--` that
  # begins a token. `-->` and friends stay code.
  code = ""; i = 1; n = length(raw)
  while (i <= n) {
    two = substr(raw, i, 2)
    if (inblk) {
      if (two == "-}") { inblk = 0; i += 2 } else { i++ }
      continue
    }
    if (two == "{-") { inblk = 1; i += 2; continue }
    if (two == "--") {
      prev = (i == 1) ? " " : substr(raw, i - 1, 1)
      nxt  = substr(raw, i + 2, 1)
      if ((prev == " " || prev == "\t") && (nxt == "" || nxt == " " || nxt == "\t" || nxt == "-")) break
    }
    code = code substr(raw, i, 1)
    i++
  }

  if (code ~ /[^ \t]/) codelines++
  if (code !~ /[^ \t]/) next

  # Paths are reported, and ledgered, relative to the scan root, so a waiver
  # row reads the same whoever checked the repository out and wherever.
  rel = FILENAME
  if (index(rel, ROOT "/") == 1) rel = substr(rel, length(ROOT) + 2)

  for (t = 1; t <= nterms; t++) {
    term = TERM[t]
    if (!match(code, "(^|" BOUND ")" term "($|" BOUND ")")) continue

    if (marker == "") {
      printf("escape-scan: FAIL %s:%d unwaived proof escape `%s`\n    %s\n",
             rel, FNR, term, raw) > "/dev/stderr"
      violations++
      continue
    }
    if (!(marker in L_term)) {
      printf("escape-scan: FAIL %s:%d `%s` claims waiver %s, which is not in the ledger\n",
             rel, FNR, term, marker) > "/dev/stderr"
      violations++
      continue
    }
    if (L_term[marker] != term) {
      printf("escape-scan: FAIL %s:%d waiver %s is for `%s`, not `%s`\n",
             rel, FNR, marker, L_term[marker], term) > "/dev/stderr"
      violations++
      continue
    }
    if (L_file[marker] != rel) {
      printf("escape-scan: FAIL %s:%d waiver %s is ledgered against %s, not this file\n",
             rel, FNR, marker, L_file[marker]) > "/dev/stderr"
      violations++
      continue
    }
    L_used[marker]++
    waived++
  }
}

END {
  # A ledger row with nothing left to excuse is a stale permission. Deleting
  # the escape must also delete its waiver, or the next one to land here
  # inherits an approval nobody granted.
  for (id in L_term) {
    if (L_used[id] == 0) {
      printf("escape-scan: FAIL ledger %s:%d waiver %s (`%s` in %s) matches no escape -- remove it\n",
             LEDGER, L_row[id], id, L_term[id], L_file[id]) > "/dev/stderr"
      violations++
    } else if (L_used[id] > 1) {
      # One waiver excuses one escape. Reusing an id would let a single
      # reviewed justification cover code nobody read, and it is what
      # locates a waiver to a line: `grep -n ESCAPE-WAIVER: <id>` has
      # exactly one answer.
      printf("escape-scan: FAIL ledger %s:%d waiver %s is claimed on %d lines -- one waiver excuses exactly one escape\n",
             LEDGER, L_row[id], id, L_used[id]) > "/dev/stderr"
      violations++
    }
  }
  printf("SCANNED %d %d %d\n", files, codelines, waived)
  exit (violations > 0) ? 1 : 0
}
AWK
}

# --- 1. self-test ----------------------------------------------------------
# Each mutant reintroduces exactly one defect and must be killed. A suite that
# has only ever been run against a clean tree proves the gate can say "ok",
# and nothing else.
if [ "$SELF_TEST" -eq 1 ]; then
  T="$(mktemp -d)"
  trap 'rm -rf "$T"' EXIT
  st_fails=0
  # $1 expected rc, $2 name, rest: setup already done in $T/case
  expect() {
    local want="$1" name="$2" out rc
    out="$(scan "$T/case" "$T/case/ledger.tsv" 2>&1)"; rc=$?
    if [ "$rc" -ne "$want" ]; then
      printf 'escape-scan: SELF-TEST FAIL [%s] expected rc=%d got rc=%d\n%s\n' "$name" "$want" "$rc" "$out" >&2
      st_fails=$((st_fails + 1))
    else
      printf 'escape-scan: self-test ok [%s] rc=%d\n' "$name" "$rc"
    fi
  }
  newcase() { rm -rf "$T/case"; mkdir -p "$T/case"; : > "$T/case/ledger.tsv"; }

  # (a) a clean tree passes -- without this the other four prove only that the
  #     scanner can fail, which a `exit 1` would also achieve.
  newcase
  printf '%s\n' 'module Clean' '%default total' 'ident : Nat -> Nat' 'ident x = x' > "$T/case/Clean.idr"
  expect 0 "clean tree passes"

  # (b) the planted believe_me of the acceptance criteria.
  newcase
  printf '%s\n' 'module Bad' 'bad : a -> b' 'bad x = believe_me x' > "$T/case/Bad.idr"
  expect 1 "planted believe_me is caught"

  # (c) prose about the escapes, in all three comment forms, is not an escape.
  newcase
  printf '%s\n' 'module Doc' '||| we never use believe_me here' '-- nor postulate' \
                '{- nor assert_total' 'nor partial -}' 'f : Nat' 'f = 0' > "$T/case/Doc.idr"
  expect 0 "comments are not escapes"

  # (d) word boundaries: `partialOrder` is a name, not the totality modifier.
  newcase
  printf '%s\n' 'module Names' 'partialOrder : Nat' 'partialOrder = 1' \
                'coveringSet : Nat' 'coveringSet = 2' > "$T/case/Names.idr"
  expect 0 "substring names are not escapes"

  # (e) an empty tree fails loudly rather than reporting success.
  newcase
  expect 1 "empty tree fails"

  # (f) a correctly ledgered waiver passes.
  newcase
  printf '%s\n' 'module W' 'w : a -> b' 'w x = believe_me x -- ESCAPE-WAIVER: w-1' > "$T/case/W.idr"
  printf 'w-1\tbelieve_me\tW.idr\tself-test fixture\n' > "$T/case/ledger.tsv"
  expect 0 "ledgered waiver passes"

  # (g) a marker with no ledger row is not a waiver.
  newcase
  printf '%s\n' 'module W' 'w : a -> b' 'w x = believe_me x -- ESCAPE-WAIVER: w-1' > "$T/case/W.idr"
  expect 1 "marker without a ledger row fails"

  # (h) a ledger row for the wrong term does not excuse this escape.
  newcase
  printf '%s\n' 'module W' 'w : a -> b' 'w x = believe_me x -- ESCAPE-WAIVER: w-1' > "$T/case/W.idr"
  printf 'w-1\tassert_total\tW.idr\twrong term\n' > "$T/case/ledger.tsv"
  expect 1 "waiver for a different term fails"

  # (i) a ledger row naming another file does not travel.
  newcase
  printf '%s\n' 'module W' 'w : a -> b' 'w x = believe_me x -- ESCAPE-WAIVER: w-1' > "$T/case/W.idr"
  printf 'w-1\tbelieve_me\tOther.idr\twrong file\n' > "$T/case/ledger.tsv"
  expect 1 "waiver naming a different file fails"

  # (j) an orphan ledger row -- the escape was deleted, the permission was not.
  newcase
  printf '%s\n' 'module Clean' 'f : Nat' 'f = 0' > "$T/case/Clean.idr"
  printf 'w-1\tbelieve_me\tClean.idr\tthe escape is gone\n' > "$T/case/ledger.tsv"
  expect 1 "orphan ledger row fails"

  # (k) a waiver with no stated reason is not a waiver.
  newcase
  printf '%s\n' 'module W' 'w : a -> b' 'w x = believe_me x -- ESCAPE-WAIVER: w-1' > "$T/case/W.idr"
  printf 'w-1\tbelieve_me\tW.idr\t\n' > "$T/case/ledger.tsv"
  expect 1 "waiver with an empty reason fails"

  # (l) one waiver may not excuse two escapes.
  newcase
  printf '%s\n' 'module W' 'a : x -> y' 'a v = believe_me v -- ESCAPE-WAIVER: w-1' \
                'b : x -> y' 'b v = believe_me v -- ESCAPE-WAIVER: w-1' > "$T/case/W.idr"
  printf 'w-1\tbelieve_me\tW.idr\tone id, two escapes\n' > "$T/case/ledger.tsv"
  expect 1 "a waiver id reused on a second line fails"

  if [ "$st_fails" -ne 0 ]; then
    fail "self-test: $st_fails of 12 controls did not behave as specified"
  fi
  echo "escape-scan: self-test ok (12 controls, 10 of them mutants that must be killed)"
  exit 0
fi

# --- 2. %default total pragma ----------------------------------------------
# The model's proofs are equations the typechecker verified only because every
# declaration is total. Deleting the pragma is the cheapest way to disarm the
# whole package without producing a single error, and it leaves no token for
# the scan above to find -- the escape here is an ABSENCE.
[ -d "$ROOT" ] || fail "scan root does not exist: $ROOT"
if [ -f "$ROOT/$IPKG" ]; then
  # The module list runs from `modules =` to the next top-level directive --
  # NOT to end of file. An ipkg may carry `opts`, `depends` or anything else
  # after the module block, and a range that ends at EOF swallows them: the
  # earlier `sed -n '/^modules/,$p'` read `opts = "--total"` as a module named
  # `opts` and failed looking for src/opts.idr. Directives are lower-case and
  # contain `=`; module names are capitalised and dotted, so the two are
  # unambiguous.
  modules="$(awk '
    /^modules[[:space:]]*=/ { inmod = 1; sub(/^modules[[:space:]]*=[[:space:]]*/, "") }
    inmod && /^[[:space:]]*[a-z][A-Za-z0-9_]*[[:space:]]*=/ { inmod = 0 }
    inmod { print }
  ' "$ROOT/$IPKG" \
    | tr ',' '\n' \
    | sed -E 's/^[[:space:]]+//; s/[[:space:]]+$//' | grep -E '^[A-Za-z]' || true)"
  [ -n "$modules" ] || fail "could not read the module list from $IPKG"
  # If the parse ever goes wrong again, say so in terms of the parse rather
  # than as a missing source file three lines later.
  for m in $modules; do
    printf '%s' "$m" | grep -qE "^[A-Z][A-Za-z0-9_.']*$" \
      || fail "parsed \`$m\` from the module list of $IPKG, which is not a module name -- the ipkg parse is wrong, not the source tree"
  done
  nmod=0; missing=""
  for m in $modules; do
    src="$ROOT/src/$(printf '%s' "$m" | tr '.' '/').idr"
    nmod=$((nmod + 1))
    [ -f "$src" ] || fail "$IPKG lists module $m but $src does not exist"
    grep -qE '^%default[[:space:]]+total[[:space:]]*$' "$src" \
      || missing="$missing $m"
    if grep -qE '^%default[[:space:]]+(partial|covering)' "$src"; then
      fail "$m weakens the package default: $(grep -m1 -E '^%default' "$src")"
    fi
  done
  [ -z "$missing" ] || fail "modules without \`%default total\`:$missing"
  [ "$nmod" -gt 0 ] || fail "$IPKG lists no modules -- the pragma check would pass vacuously"
  printf 'escape-scan: %%default total present in all %d packaged modules\n' "$nmod"
fi

# --- 3. the scan ------------------------------------------------------------
out="$(scan "$ROOT" "$LEDGER")" || fail "escape scan did not pass (see above)"
set -- $out   # SCANNED <files> <codelines> <waivers>
printf 'escape-scan: ok (%s .idr files, %s lines of code scanned, %s waived escapes)\n' "$2" "$3" "$4"
