#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Keep the two Creusot ledgers honest (issue #121).
#
# Both ledgers are claims about what is on disk, and a claim that nobody
# re-checks decays into decoration. This gate re-derives each ledger's subject
# matter from the source tree and compares it against the rows in BOTH
# directions:
#
#   classification  every first-party .rs file has exactly one row, and every
#                   row names a file that exists. vendor/bevy_winit is excluded
#                   by EXPLICIT PATH, never a wildcard, so the exclusion is a
#                   line you can read rather than an accident of where find was
#                   pointed.
#   debt            every proof escape in the kernel -- a #[trusted], a derive
#                   compiled out under cfg(creusot), or a runtime fn with no
#                   functional contract -- has a row, and every row names an
#                   escape that still exists.
#
# Bidirectionality is the point. A one-way check lets the ledger grow silently
# (unledgered escapes) or rot quietly (rows for code that is gone).
#
# --self-test runs the controls. A gate that cannot fail is worse than no gate,
# so every check below has a mutant that must kill it, and every extraction has
# an emptiness guard: two empty sets compare equal, so a zero denominator is a
# FAILURE and never an "ok".

set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# The one vendored tree that is deliberately outside the proof scope. Named as
# a literal path: a wildcard would silently widen the exemption the first time
# somebody vendors something else.
VENDOR_EXCLUDE="vendor/bevy_winit"

# Fixture injection points, so --self-test can aim the checks at a mutated copy
# of the tree without touching the real one.
# Resolved inside each check, never at load time: --self-test sets these
# per probe, and resolving once at the top made every mutant read the REAL
# tree and report the real tree.s verdict. Measured: all six debt mutants
# "passed" while testing nothing, and the classification mutants went red for
# a genuine stale row rather than for the fault they planted -- a mutant that
# passes for the wrong reason.

say() { printf '[creusot-ledger] %s\n' "$*"; }
die() { printf '[creusot-ledger] FAIL: %s\n' "$*" >&2; exit 1; }

# ---------------------------------------------------------------- extraction

# Every first-party .rs file, vendor tree removed by explicit path.
disk_rs_files() {
  find "$SCAN_ROOT" -type f -name '*.rs' \
       -not -path "$SCAN_ROOT/target/*" \
       -not -path "$SCAN_ROOT/$VENDOR_EXCLUDE/*" \
    | sed "s|^$SCAN_ROOT/||" | sort
}

class_rows() { grep -v '^#' "$CLASS_TSV" | awk -F'\t' 'NF>=2{print $2}' | sort; }

# Proof escapes in the kernel, emitted as "path<TAB>item<TAB>mechanism".
#
# Each file is truncated at its #[cfg(test)] marker: test code is not proved,
# so an uncontracted test fn is not debt.
disk_escapes() {
  local f
  for f in "$KERNEL_SRC"/*.rs; do
    [ -e "$f" ] || continue
    local rel="crates/idaptik-kernel/src/$(basename "$f")"
    awk -v rel="$rel" '
      /^#\[cfg\(test\)\]/ { exit }

      # Inside an impl that cfg(creusot) compiles out -- see the impl rule
      # below. Its methods are covered by the one cfg-out row emitted there,
      # so they are consumed here rather than counted again as uncontracted
      # runtime fns.
      skip_depth > 0 {
        skip_depth += gsub(/\{/, "{") - gsub(/\}/, "}")
        next
      }

      # A multi-line attribute. Creusot contracts routinely wrap across lines,
      # and a continuation line looks like nothing in particular -- so the block
      # is held open on bracket balance rather than on how the line begins.
      # Getting this wrong is not a false alarm but a SILENT PASS in the other
      # direction: a wiped block makes a contracted fn look like debt, and a
      # fn whose contract is merely unparsed would otherwise need a ledger row
      # to quieten the gate. Measured: advance() has three ensures, the third
      # spanning three lines.
      in_attr > 0 {
        blk = blk "\n" $0
        attrs = attrs "\n" $0
        in_attr += gsub(/\[/, "[") - gsub(/\]/, "]")
        next
      }

      # A derive compiled out under cfg(creusot). Keyed by derive NAME, one
      # record each, because the escape belongs to the derive and not to the
      # type it happens to sit on.
      /cfg_attr\(not\(creusot\), *derive\(/ {
        line = $0
        sub(/.*derive\(/, "", line); sub(/\).*/, "", line)
        n = split(line, d, /, */)
        for (i = 1; i <= n; i++) {
          gsub(/[ \t]/, "", d[i])
          if (d[i] != "") print "crates/idaptik-kernel\t" d[i] "\tcfg-out"
        }
      }

      # Doc comments accumulate but do NOT participate in bracket balance.
      # Prose brackets are routinely unbalanced -- the doc line for next_f64
      # says "Next float in [0, 1)" -- and counting those held the block open
      # and swallowed the very attribute it was trying to read.
      /^[ \t]*\/\/\// { blk = blk "\n" $0; next }

      # A real attribute. Only these are balanced across lines.
      /^[ \t]*#\[/ {
        blk = blk "\n" $0
        attrs = attrs "\n" $0
        depth = gsub(/\[/, "[") - gsub(/\]/, "]")
        if (depth > 0) in_attr = depth
        next
      }

      # An impl compiled out under cfg(creusot): the SAME escape as a derive
      # compiled out the same way, in the form a hand-written impl takes.
      # Keyed by trait name, so replacing
      # `#[cfg_attr(not(creusot), derive(Hash))]` with an equivalent
      # hand-written impl keeps the ledger row it already had rather than
      # silently dropping out of the gate view -- which is exactly what
      # happened when clippy derived_hash_with_manual_eq forced that swap.
      #
      # Matched against the attributes ONLY, never the accumulated doc prose:
      # a doc comment that quotes an attribute must not be able to conjure an
      # escape out of nothing.
      /^[ \t]*impl[ \t<]/ {
        if (attrs ~ /#\[cfg\(not\(creusot\)\)\]/) {
          t = $0
          sub(/^[ \t]*impl[ \t]*(<[^>]*>)?[ \t]*/, "", t)
          sub(/[ \t]+for[ \t].*/, "", t)
          sub(/[ \t]*\{.*/, "", t)
          sub(/<.*/, "", t)
          nt = split(t, tp, /::/); t = tp[nt]
          gsub(/[ \t]/, "", t)
          if (t == "") {
            print "[creusot-ledger] PARSE FAILURE: cfg-out impl with no trait name at " rel ":" NR > "/dev/stderr"
            exit 2
          }
          print "crates/idaptik-kernel\t" t "\tcfg-out"
          # The impl body is not separately debt: the whole item sits outside
          # the proof, and the one row above says so. Consume it on balance.
          skip_depth = gsub(/\{/, "{") - gsub(/\}/, "}")
          if (skip_depth < 1) {
            print "[creusot-ledger] PARSE FAILURE: cfg-out impl body did not open at " rel ":" NR > "/dev/stderr"
            exit 2
          }
        }
        blk = ""; attrs = ""; next
      }

      /^[ \t]*(pub )?fn [A-Za-z_][A-Za-z0-9_]*/ {
        name = $0
        sub(/^[ \t]*(pub )?fn /, "", name)
        sub(/[^A-Za-z0-9_].*/, "", name)
        if (blk ~ /macros::trusted/)      print rel "\t" name "\ttrusted"
        else if (blk ~ /macros::logic/)   { }   # logic fns state the model; not runtime code
        else if (blk ~ /macros::(ensures|requires)/) { }
        else                              print rel "\t" name "\tno-contract"
        blk = ""; attrs = ""; next
      }

      { blk = ""; attrs = "" }

      # A skip region that never closes would swallow every later fn in the
      # file and turn this gate green by eating its own subject matter -- the
      # vacuous shape this repository keeps re-learning. So the balance is
      # asserted, not assumed.
      END {
        if (skip_depth > 0) {
          print "[creusot-ledger] PARSE FAILURE: unterminated cfg-out impl in " rel > "/dev/stderr"
          exit 2
        }
      }
    ' "$f"
  done | sort -u
}

debt_rows() {
  grep -v '^#' "$DEBT_TSV" | awk -F'\t' 'NF>=4{print $2 "\t" $3 "\t" $4}' | sort -u
}

# ------------------------------------------------------------------- compare

# Compare two sorted sets and report both directions. Refuses to pass when both
# are empty: that is the shape every vacuous gate in this estate has taken.
compare_sets() {
  local label="$1" disk_f="$2" ledger_f="$3" what="$4"
  local n_disk n_ledger missing extra
  n_disk=$(wc -l < "$disk_f"); n_ledger=$(wc -l < "$ledger_f")

  say "$label: $n_disk $what on disk / $n_ledger rows in the ledger"

  if [ "$n_disk" -eq 0 ] && [ "$n_ledger" -eq 0 ]; then
    die "$label: zero denominator -- 0 $what found AND 0 rows. Two empty sets compare equal, so this is a broken extractor, not a clean tree."
  fi
  [ "$n_disk" -eq 0 ] && die "$label: found 0 $what on disk but the ledger has $n_ledger rows. The extractor is broken or the tree moved."

  missing=$(comm -23 "$disk_f" "$ledger_f")
  extra=$(comm -13 "$disk_f" "$ledger_f")

  if [ -n "$missing" ]; then
    printf '[creusot-ledger] UNLEDGERED -- on disk, no row:\n%s\n' "$missing" >&2
  fi
  if [ -n "$extra" ]; then
    printf '[creusot-ledger] ORPHAN ROWS -- in the ledger, not on disk:\n%s\n' "$extra" >&2
  fi
  if [ -n "$missing" ] || [ -n "$extra" ]; then
    die "$label: ledger and disk disagree (see above)."
  fi
  say "$label: exact bijection over $n_disk $what. ok"
}

check_classification() {
  local CLASS_TSV="${IDAPTIK_CLASS_TSV:-$REPO_DIR/crates/CREUSOT-CLASSIFICATION.tsv}"
  local SCAN_ROOT="${IDAPTIK_SCAN_ROOT:-$REPO_DIR}"
  local d="$(mktemp)" l="$(mktemp)"
  trap 'rm -f "$d" "$l"' RETURN
  disk_rs_files > "$d"; class_rows > "$l"
  say "vendor exclusion: $VENDOR_EXCLUDE (explicit path; $(find "$SCAN_ROOT/$VENDOR_EXCLUDE" -name '*.rs' 2>/dev/null | wc -l) .rs files held out)"
  compare_sets "classification" "$d" "$l" "first-party .rs files"
}

check_debt() {
  local KERNEL_SRC="${IDAPTIK_KERNEL_SRC:-$REPO_DIR/crates/idaptik-kernel/src}"
  local DEBT_TSV="${IDAPTIK_DEBT_TSV:-$REPO_DIR/crates/CREUSOT-PROOF-DEBT.tsv}"
  local d="$(mktemp)" l="$(mktemp)"
  trap 'rm -f "$d" "$l"' RETURN
  disk_escapes > "$d"; debt_rows > "$l"
  compare_sets "debt" "$d" "$l" "proof escapes"
}

# ----------------------------------------------------------------- self-test

# Each control names the mutant it plants and the failure it must provoke. A
# control that cannot say both YES and NO is worthless, so the positive control
# runs in the same shape as the mutants.
self_test() {
  local work rc pass=0 fail=0
  work="$(mktemp -d)"
  trap 'rm -rf "$work"' RETURN

  probe() { # probe <description> <expect: pass|fail> <arm>
    local desc="$1" expect="$2" arm="$3"
    set +e; ( check_"$arm" ) >/dev/null 2>&1; rc=$?; set -e
    local got; [ $rc -eq 0 ] && got=pass || got=fail
    if [ "$got" = "$expect" ]; then
      printf '  ok   %-58s (%s)\n' "$desc" "$got"; pass=$((pass+1))
    else
      printf '  FAIL %-58s (expected %s, got %s)\n' "$desc" "$expect" "$got"; fail=$((fail+1))
    fi
  }

  local REAL_SRC="$REPO_DIR/crates/idaptik-kernel/src"
  local REAL_DEBT="$REPO_DIR/crates/CREUSOT-PROOF-DEBT.tsv"
  cp -r "$REAL_SRC" "$work/src"
  cp "$REAL_DEBT" "$work/debt.tsv"

  say "self-test: debt arm"
  IDAPTIK_KERNEL_SRC="$work/src" IDAPTIK_DEBT_TSV="$work/debt.tsv" \
    probe "positive control: unmodified tree is green" pass debt

  # Mutant 1: an escape appears with no row. This is the failure the gate
  # exists for -- debt growing silently.
  cp "$work/src/rng.rs" "$work/rng.bak"
  sed -i 's|^    pub fn new(seed: u32)|    #[cfg_attr(creusot, creusot_std::macros::trusted)]\n    pub fn new_unledgered(seed: u32) -> Self { Self { state: seed } }\n    pub fn new(seed: u32)|' "$work/src/rng.rs"
  IDAPTIK_KERNEL_SRC="$work/src" IDAPTIK_DEBT_TSV="$work/debt.tsv" \
    probe "mutant: new #[trusted] fn with no ledger row" fail debt
  cp "$work/rng.bak" "$work/src/rng.rs"

  # Mutant 2: a row survives the code it described.
  printf 'PD-99\tcrates/idaptik-kernel/src/rng.rs\tghost_fn\ttrusted\tdescribes nothing\tnone\tnever\n' >> "$work/debt.tsv"
  IDAPTIK_KERNEL_SRC="$work/src" IDAPTIK_DEBT_TSV="$work/debt.tsv" \
    probe "mutant: orphan row naming a fn that does not exist" fail debt
  cp "$REAL_DEBT" "$work/debt.tsv"

  # Mutant 3: a row is deleted while its escape remains.
  grep -v '^PD-06' "$REAL_DEBT" > "$work/debt.tsv"
  IDAPTIK_KERNEL_SRC="$work/src" IDAPTIK_DEBT_TSV="$work/debt.tsv" \
    probe "mutant: PD-06 deleted, next_u32 still uncontracted" fail debt
  cp "$REAL_DEBT" "$work/debt.tsv"

  # Mutant 4: the emptiness guard. An extractor aimed at nothing must not pass
  # merely because the ledger it is compared against is also nothing.
  mkdir -p "$work/empty"
  : > "$work/empty.tsv"
  IDAPTIK_KERNEL_SRC="$work/empty" IDAPTIK_DEBT_TSV="$work/empty.tsv" \
    probe "mutant: empty tree + empty ledger -> zero denominator" fail debt

  # Mutant 5: a contract is stripped. The fn becomes silent debt.
  sed -i '/macros::ensures(result@ == seed@)/d; /pub fn new(seed: u32)/i\    // contract stripped by self-test mutant' "$work/src/rng.rs" 2>/dev/null || true
  sed -i '0,/^    #\[cfg_attr(creusot, creusot_std::macros::ensures/{/^    #\[cfg_attr(creusot, creusot_std::macros::ensures/d}' "$work/src/rng.rs"
  IDAPTIK_KERNEL_SRC="$work/src" IDAPTIK_DEBT_TSV="$work/debt.tsv" \
    probe "mutant: a contract stripped -> fn becomes unledgered debt" fail debt
  cp "$work/rng.bak" "$work/src/rng.rs"


  # Mutants 6 and 7 guard the cfg-out IMPL rule. Clippy forced Alert::Hash from
  # a cfg-out derive to a hand-written cfg-out impl, and before this rule
  # existed the gate could not see the impl form at all -- the escape changed
  # shape and would have walked out of the ledger unnoticed.
  cp "$work/src/trace.rs" "$work/trace.bak"

  # Mutant 6: the cfg attribute is stripped, so the impl is no longer outside
  # the proof. The Hash row must orphan AND the method must surface as debt.
  # Only the attribute directly above an impl is removed, so the serde `use`
  # gating higher in the file is left alone and cannot mask the result.
  awk '{a[NR]=$0} END{for(i=1;i<=NR;i++){ if (a[i]=="#[cfg(not(creusot))]" && a[i+1] ~ /^impl /) continue; print a[i]}}' \
      "$work/trace.bak" > "$work/src/trace.rs"
  IDAPTIK_KERNEL_SRC="$work/src" IDAPTIK_DEBT_TSV="$work/debt.tsv" \
    probe "mutant: cfg attr stripped from the Hash impl -> row orphaned" fail debt

  # Mutant 7: the sharp one. The impl body is consumed on brace balance, and a
  # skip region that fails to close would swallow every later item and turn
  # this gate green by eating its own subject matter. An uncontracted fn
  # planted AFTER the impl must still be seen.
  sed 's|^#\[cfg(test)\]|pub fn leaked_past_the_skip_region(x: u32) -> u32 { x }\n\n#[cfg(test)]|' \
      "$work/trace.bak" > "$work/src/trace.rs"
  IDAPTIK_KERNEL_SRC="$work/src" IDAPTIK_DEBT_TSV="$work/debt.tsv" \
    probe "mutant: uncontracted fn after the cfg-out impl is still seen" fail debt
  cp "$work/trace.bak" "$work/src/trace.rs"
  say "self-test: classification arm"
  mkdir -p "$work/scan/crates" "$work/scan/$VENDOR_EXCLUDE"
  printf 'fn a() {}\n' > "$work/scan/crates/one.rs"
  printf 'fn v() {}\n' > "$work/scan/$VENDOR_EXCLUDE/vendored.rs"
  printf '# header\nS\tcrates/one.rs\tonly file\n' > "$work/class.tsv"
  IDAPTIK_SCAN_ROOT="$work/scan" IDAPTIK_CLASS_TSV="$work/class.tsv" \
    probe "positive control: 1 file, 1 row, vendor held out" pass classification

  # Mutant 6: the vendor exclusion is what is being tested -- if it stopped
  # working, the vendored file would appear on disk and be unledgered.
  printf 'fn extra() {}\n' > "$work/scan/crates/two.rs"
  IDAPTIK_SCAN_ROOT="$work/scan" IDAPTIK_CLASS_TSV="$work/class.tsv" \
    probe "mutant: unclassified .rs file added" fail classification
  rm "$work/scan/crates/two.rs"

  printf '# header\nS\tcrates/one.rs\tonly file\nS\tcrates/gone.rs\tdeleted\n' > "$work/class.tsv"
  IDAPTIK_SCAN_ROOT="$work/scan" IDAPTIK_CLASS_TSV="$work/class.tsv" \
    probe "mutant: row naming a file that does not exist" fail classification

  printf '# header only\n' > "$work/class.tsv"
  mkdir -p "$work/scanempty"
  IDAPTIK_SCAN_ROOT="$work/scanempty" IDAPTIK_CLASS_TSV="$work/class.tsv" \
    probe "mutant: empty scan root -> zero denominator" fail classification

  say "self-test: $pass passed, $fail failed"
  [ "$fail" -eq 0 ] || die "self-test: $fail control(s) did not behave as specified"
}

case "${1:-}" in
  classification) check_classification ;;
  debt)           check_debt ;;
  all)            check_classification; check_debt ;;
  --self-test)    self_test ;;
  *) printf 'usage: %s classification|debt|all|--self-test\n' "$0" >&2; exit 2 ;;
esac
