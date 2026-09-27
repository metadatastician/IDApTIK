#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Offline fixture coverage for the `.well-known/` publish step in
# .github/workflows/pages.yml.
#
# ── WHY A TEST FOR EIGHT LINES OF `cp` ──────────────────────────────────────
#
# Because those eight lines broke the deployed website on main, and nothing in
# the repository could tell anyone why.
#
# PR #147 (2026-09-27) widened the step from publishing `security.txt` alone to
# publishing the whole directory, so that `ai.txt` and `humans.txt` — added in
# the same commit for RSR criterion 2.2.1 — would not 404 on the site. The
# widened step used `basename` and `head`. The previous version had needed
# neither. It failed on main at step 7 of 8 with "Process completed with exit
# code 1" and no retrievable log: the Actions results endpoint was unreachable,
# so the container's own output was gone and the only evidence was the exit code.
#
# The step runs under **dash** inside `ghcr.io/stefan-hoeck/idris2-pack`, a
# Nix-based image with no bash and no guaranteed coreutils beyond what the step
# itself has already proven present. A missing external command is not even
# reported cleanly there: under `set -e`, a failed command substitution inside
# an argument does NOT abort dash, so `cp "$f" "_site/.well-known/$(basename "$f")"`
# becomes `cp "$f" "_site/.well-known/"` and fails at `cp` with a message about a
# destination, pointing nowhere near the absent tool.
#
# So this suite does two things. It runs the REAL step text, extracted from the
# REAL workflow file, under dash against stub trees — and it runs it with a PATH
# containing only the four external commands the pre-#147 version proved exist.
# A revision that reaches for a fifth one fails here, locally, in about a
# second, instead of failing on main with no log.
#
# The step is extracted rather than duplicated. A copy of a gate is not the gate:
# the whole reason this defect shipped is that the thing running in CI and the
# thing anyone reasoned about were allowed to be different.

set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKFLOW="$REPO_DIR/.github/workflows/pages.yml"
FIXTURE="$(mktemp -d)"
failures=0
checks=0

trap 'rm -rf "$FIXTURE"' EXIT

# Resolved to an ABSOLUTE path, and that is load-bearing. run_step invokes dash
# through `env -i PATH="$MINBIN"`, and env resolves the command name against the
# NEW environment, not the caller's. With a bare "dash" and a minimal PATH that
# contains only mkdir, cp and cat, exec fails with "env: 'dash': No such file or
# directory" — every fixture then fails for a reason that has nothing to do with
# the step under test, while the two mutants that merely need a NONZERO exit
# appear to fire. A suite that reports 5 passes and 12 failures for the wrong
# reason is worse than one that reports nothing.
DASH="$(command -v "${DASH:-dash}" || true)"
if [ -z "$DASH" ] || [ ! -x "$DASH" ]; then
  printf 'pages-wellknown: FAIL — dash is required and was not found on PATH\n' >&2
  printf '  This suite tests a step that runs under dash in a container with no\n' >&2
  printf '  bash. Running it under bash instead would test the wrong shell and\n' >&2
  printf '  pass for reasons that do not hold in production. An absent tool is a\n' >&2
  printf '  failure, not a skip (AGENTS.md, "Checks must be able to fail").\n' >&2
  exit 1
fi

# ── EMIT GITHUB ANNOTATIONS AS WELL AS CONSOLE LINES ────────────────────────
#
# The Actions log endpoint has been unreachable from this repository's
# development environment twice in one day: once for the Pages failure that took
# the website dark, and once for a fixture suite. Both times the only evidence
# obtainable was an annotation reading "Process completed with exit code 1."
# against a step number -- the log blob lives on a host this environment cannot
# reach, and the rendered job page requires sign-in even for a public repo.
#
# A gate whose failure reason is trapped in an unreadable log is a much weaker
# gate than it looks. Annotations are readable through the check-runs API
# without the log blob, so a failure that annotates is a failure that can still
# be diagnosed when the log cannot be fetched.
#
# Only active under GITHUB_ACTIONS=true, so local runs are unaffected.
annotate() { # $1 level, $2 message
  [ "${GITHUB_ACTIONS:-}" = "true" ] || return 0
  local m
  # Workflow commands are single-line and treat % as an escape introducer, so %
  # must be escaped FIRST, then the line terminators folded. Escaping in the
  # other order turns every folded newline into literal %250A text.
  m="$(printf '%s' "$2" | sed -e 's/%/%25/g' -e ':a' -e 'N' -e '$!ba' -e 's/\n/ | /g')"
  printf '::%s::%s\n' "$1" "$m"
}

note() { checks=$((checks + 1)); printf '  ok: %s\n' "$1"; }

fail() {
  printf '  FAIL: %s\n' "$1" >&2
  annotate error "pages-wellknown: $1"
  failures=$((failures + 1))
}

expect_pass() { # $1 description, rest = command. stderr is NOT swallowed: the
                # printed denominators are the evidence the check ran.
  local desc="$1"
  shift
  if "$@" >/dev/null; then
    note "$desc"
  else
    printf '  FAIL: expected success: %s\n' "$desc" >&2
    "$@" 2>&1 | sed 's/^/        /' >&2 || true
    failures=$((failures + 1))
  fi
}

expect_fail() { # $1 description, $2 expected output, rest = command
  local desc="$1" want="$2" out
  shift 2
  if out="$("$@" 2>&1)"; then
    fail "expected failure: $desc"
  elif ! printf '%s' "$out" | grep -qF -- "$want"; then
    printf '  FAIL: %s failed for the wrong reason (wanted %s):\n%s\n' "$desc" "$want" "$out" >&2
    failures=$((failures + 1))
  else
    note "$desc"
  fi
}

require_nonzero() { # $1 label, $2 count
  if [ -z "$2" ] || [ "$2" = 0 ]; then
    printf 'DENOMINATOR ZERO: extracted nothing for %s; refusing to run it\n' "$1" >&2
    return 1
  fi
}

# ── extract the step ────────────────────────────────────────────────────────
#
# Pulls the `run: |` block of the named step out of pages.yml and dedents it by
# the block's own indentation. Fails closed if the step is missing, if the block
# is empty, or if the dedent produces nothing runnable — a silently empty
# extraction would otherwise run an empty script, which exits 0, and report that
# the production step works.
extract_step() { # $1 workflow, $2 step-name substring, $3 destination
  awk -v want="$2" '
    index($0, "- name: ") && index($0, want) { instep = 1; next }
    instep && /^[[:space:]]*run:[[:space:]]*\|[[:space:]]*$/ { inblock = 1; next }
    instep && inblock {
      # The block ends at the first line dedented to or past the step marker.
      if ($0 ~ /^      - / || $0 ~ /^[a-z]/) exit
      line = $0
      sub(/^          /, "", line)
      print line
      n++
      next
    }
    instep && /^[[:space:]]*- / { exit }
    END {
      printf "extract: %d line(s) from the \"%s\" step\n", n, want > "/dev/stderr"
      if (n == 0) {
        printf "DENOMINATOR ZERO: no run block found for step \"%s\"\n", want > "/dev/stderr"
        exit 2
      }
    }
  ' "$1" > "$3"
}

STEP="$FIXTURE/publish-step.sh"

# ── build a minimal PATH ────────────────────────────────────────────────────
#
# Only the external commands the PRE-#147 version of this step already used, and
# which therefore demonstrably exist in the container: mkdir, cp, cat. Everything
# else the step needs must be a shell builtin or parameter expansion.
#
# This is the arm that actually catches the regression. Running under a normal
# PATH would pass whether or not the step used basename, because the development
# machine has basename; the container is the thing that may not.
MINBIN="$FIXTURE/minbin"
mkdir -p "$MINBIN"
for tool in mkdir cp cat; do
  src="$(command -v "$tool" || true)"
  if [ -z "$src" ]; then
    printf 'pages-wellknown: FAIL — %s is not available to build the minimal PATH\n' "$tool" >&2
    exit 1
  fi
  cp "$src" "$MINBIN/$tool"
done
# Assert the minimal PATH really is minimal: if basename were in it, the whole
# point of the exercise would be lost and the suite would pass vacuously.
for absent in basename head sed awk grep; do
  if [ -e "$MINBIN/$absent" ]; then
    printf 'pages-wellknown: FAIL — %s leaked into the minimal PATH\n' "$absent" >&2
    exit 1
  fi
done

make_tree() { # $1 name; creates a stub checkout, echoes its root
  local root="$FIXTURE/$1"
  mkdir -p "$root/.well-known" "$root/_site"
  printf 'Contact: mailto:role@example.org\nExpires: 2027-01-01T00:00:00.000Z\n' \
    > "$root/.well-known/security.txt"
  printf 'User-Agent: *\nDisallow-Training: yes\n' > "$root/.well-known/ai.txt"
  printf '/* TEAM */\nMaintainer: example\n' > "$root/.well-known/humans.txt"
  printf '%s\n' "$root"
}

run_step() { # $1 tree root, $2 = "full" | "minimal"
  local root="$1" mode="${2:-minimal}" path="$MINBIN"
  if [ "$mode" = full ]; then
    path="$MINBIN:$PATH"
  fi
  ( cd "$root" && env -i PATH="$path" HOME="$root" "$DASH" "$STEP" )
}

printf 'extracting the production step:\n'
expect_pass 'the publish step extracts from pages.yml and is non-empty' \
  extract_step "$WORKFLOW" 'Publish .well-known/' "$STEP"
lines="$(wc -l < "$STEP" | tr -d ' ')"
require_nonzero "lines extracted from the publish step" "$lines" || exit 1
printf 'extract: %d line(s) of production shell under test\n' "$lines" >&2

# The extracted text must be the production text and not a stale copy. Two
# markers that only exist in the real step.
expect_pass 'the extraction really is the current step, not a cached copy' \
  grep -qF 'RSR criterion 2.2.1 requires' "$STEP"
expect_pass 'the extracted step runs under dash, not bash' \
  grep -qF 'set -eu' "$STEP"

printf 'clean-tree fixtures (minimal PATH: mkdir, cp, cat only):\n'
clean="$(make_tree clean)"
expect_pass 'the production step publishes all three files with only mkdir/cp/cat available' \
  run_step "$clean" minimal
for f in security.txt ai.txt humans.txt; do
  expect_pass "published artifact contains $f" test -f "$clean/_site/.well-known/$f"
done
expect_pass 'published bytes equal the source bytes' \
  cmp -s "$clean/.well-known/security.txt" "$clean/_site/.well-known/security.txt"

printf 'firing fixtures:\n'
# A file added to .well-known/ must be published too — the step copies the whole
# directory, which is the property that stops the next addition 404-ing.
extra="$(make_tree extra-file)"
printf 'Contact: mailto:role@example.org\n' > "$extra/.well-known/security-acknowledgments.txt"
expect_pass 'a fourth file in .well-known/ is published without being named in the step' \
  run_step "$extra" minimal
expect_pass 'and it actually reached the artifact' \
  test -f "$extra/_site/.well-known/security-acknowledgments.txt"

# Missing one of the three RSR-named files must fail, naming the file.
for f in security.txt ai.txt humans.txt; do
  miss="$(make_tree "missing-$f")"
  rm -f "$miss/.well-known/$f"
  expect_fail "a checkout with no $f is rejected by name" \
    "requires .well-known/$f" run_step "$miss" minimal
done

# An empty directory must not read as "published successfully".
empty="$(make_tree empty-dir)"
rm -f "$empty"/.well-known/*
expect_fail 'an empty .well-known/ directory is rejected, not published as nothing' \
  'is empty or absent in the checkout' run_step "$empty" minimal

# THE REGRESSION ARM. A step that reaches for an external command outside the
# proven set must fail under the minimal PATH. This is the exact shape of the
# #147 breakage, reproduced as a fixture: take the production step, reintroduce
# basename(1), and confirm the minimal PATH rejects it. If this arm ever stops
# firing, the minimal PATH has leaked and every result above is worthless.
mutated="$FIXTURE/mutant-basename.sh"
sed 's|name=${f##\*/}|name=$(basename "$f")|' "$STEP" > "$mutated"
if cmp -s "$STEP" "$mutated"; then
  fail 'the basename mutant did not change the step text; the sed pattern no longer matches'
else
  mutant_tree="$(make_tree mutant-basename)"
  mutant_out="$( cd "$mutant_tree" && env -i PATH="$MINBIN" HOME="$mutant_tree" "$DASH" "$mutated" 2>&1 || true )"
  if [ -z "$mutant_out" ]; then
    fail 'a step using basename(1) SUCCEEDED under the minimal PATH — the minimal PATH is not minimal'
  elif ! printf '%s' "$mutant_out" | grep -q 'basename'; then
    # It must fail BECAUSE basename is absent. Any other failure — a dash that
    # could not be exec-ed, a stub tree that was never built — also exits
    # nonzero and would make this arm pass while proving nothing.
    printf '  FAIL: the basename mutant failed for an unrelated reason:\n%s\n' "$mutant_out" >&2
    failures=$((failures + 1))
  else
    note 'a step that reaches for basename(1) fails under the minimal PATH (the #147 regression)'
  fi
  # ...and the same mutant passes with a full PATH, which is what makes the
  # minimal PATH the load-bearing part of the arm rather than incidental.
  full_tree="$(make_tree mutant-basename-full)"
  if ( cd "$full_tree" && env -i PATH="$MINBIN:$PATH" HOME="$full_tree" "$DASH" "$mutated" ) >/dev/null 2>&1 \
     && [ -f "$full_tree/_site/.well-known/ai.txt" ]; then
    note 'the same mutant passes with a normal PATH, so the minimal PATH is what catches it'
  else
    fail 'the basename mutant failed even with a full PATH; the arm is not isolating the tool'
  fi
fi

# A step that stops copying altogether must fail rather than publish nothing.
nocopy="$FIXTURE/mutant-nocopy.sh"
sed '/^  cp "\$f" "_site\/.well-known\/\$name"$/d' "$STEP" > "$nocopy"
if cmp -s "$STEP" "$nocopy"; then
  fail 'the no-copy mutant did not change the step text; the sed pattern no longer matches'
else
  nocopy_tree="$(make_tree mutant-nocopy)"
  expect_fail 'a step that copies nothing is rejected rather than publishing an empty directory' \
    'did not reach _site/' \
    bash -c "cd '$nocopy_tree' && env -i PATH='$MINBIN' HOME='$nocopy_tree' '$DASH' '$nocopy'"
fi

if [ "$failures" -ne 0 ]; then
  annotate error "pages-wellknown: $failures assertion(s) FAILED, $checks passed -- see the per-assertion annotations above"
  printf 'pages .well-known publish fixtures: %d FAILURES, %d assertion(s) passed\n' \
    "$failures" "$checks" >&2
  exit 1
fi
printf 'pages .well-known publish fixtures: %d assertion(s) passed, 0 failed\n' "$checks"
