#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Offline fixture coverage for the ABI authority pin refreshed in PR #138,
# and for the wire table that pin exists to protect (issue #139).
#
# The production gate is embedded in a GitHub Actions workflow. This suite
# executes that exact run block with deterministic curl/sha256sum stubs, then
# independently checks the document-to-workflow revision/path/digest mapping.
# The independent comparison is intentionally stricter than comparing digest
# sets: swapping two valid digests between paths must fail.
#
# ── WHY THIS FILE WAS REWRITTEN ──────────────────────────────────────────────
#
# Issue #141: PR #140 committed these 460 lines at mode 100755 and wired them
# into NOTHING. No justfile recipe, no workflow step. The ABI authority pin was
# therefore regression-guarded by a file that never ran, which is worse than no
# guard at all because the file reads as coverage to anyone auditing the tree.
# `just abi-authority-pin-test` and the `fixtures-and-must` job in
# .github/workflows/ci.yml now invoke it. tests/ci_security_config_test.sh's
# `check_no_dead_test_scripts` is the standing arm that stops the shape
# recurring for any future suite under tests/.
#
# The file also COULD NOT RUN. It exited 2 on the first awk expression, because
# five of them used gawk's three-argument match(target, regexp, array). mawk --
# /usr/bin/awk on a stock Debian/Ubuntu image, and the awk inside this
# repository's development containers -- does not implement it. The suite had
# never been executed once in the five days it sat there; had it been, that
# would have been obvious in seconds. Every awk program below is now POSIX:
# no three-argument match(), no `{n}` regex intervals, no gensub().
#
# Issue #139: the wire-table check here compared the document against a fixture
# file that was itself a transcription of the document, so it proved internal
# consistency only. A document that drifted from the generated `ffi/connectors.json`
# it claims to describe would have passed. `validate_wire_manifest` below now
# compares the document against the actual manifest bytes, and
# fixtures/abi/connectors-pinned.json is a verbatim copy of the file at the
# pinned revision -- self-verified by its own digest, so it cannot silently rot.

set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKFLOW="$REPO_DIR/.github/workflows/abi-conformance.yml"
DOCUMENT="$REPO_DIR/docs/abi/ABI-AUTHORITY.md"
FIXTURE="$(mktemp -d)"
STUBS="$FIXTURE/bin"
EXPECTED_CURRENT=f079bc06061951d8e4c12f86675f81e97ec6e3a7
EXPECTED_PREVIOUS=bcb8bb3730c2520117e565d80d5638ff384c1222
EXPECTED_HISTORICAL=2bbd3b2
failures=0
checks=0

trap 'rm -rf "$FIXTURE"' EXIT

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
  annotate error "abi-authority-pin: $1"
  failures=$((failures + 1))
}

# stdout is discarded but STDERR IS NOT. Every extractor below prints its own
# denominator to stderr, and a printed denominator that nobody can see is worth
# exactly as much as no denominator at all: the console transcript is the
# evidence that the check examined 19 tuples rather than zero and reported ok
# either way. This is why the passing runs below are noisy on purpose.
expect_pass() { # $1 description, rest = command
  local desc="$1"
  shift
  if "$@" >/dev/null; then
    note "$desc"
  else
    fail "expected success: $desc"
  fi
}

expect_fail() { # $1 description, $2 expected output, rest = command
  local desc="$1" want="$2" out
  shift 2
  if out="$("$@" 2>&1)"; then
    fail "expected failure: $desc"
  elif ! grep -qF "$want" <<<"$out"; then
    printf '  FAIL: %s failed for the wrong reason:\n%s\n' "$desc" "$out" >&2
    failures=$((failures + 1))
  else
    note "$desc"
  fi
}

expect_any_fail() { # $1 description, rest = command
  local desc="$1"
  shift
  if "$@" >/dev/null 2>&1; then
    fail "expected failure: $desc"
  else
    note "$desc"
  fi
}

extract_authority_script() { # $1 workflow, $2 destination
  awk '
    /^      - name: Re-derive pinned digests and diff against docs\/abi\/ABI-AUTHORITY.md$/ {
      found = 1
      next
    }
    found && /^        run: \|$/ {
      body = 1
      next
    }
    body && /^      - / { exit }
    body {
      sub(/^          /, "")
      print
    }
  ' "$1" > "$2"
  [ -s "$2" ] || {
    printf 'authority-pin run block not found in %s\n' "$1" >&2
    return 1
  }
}

extract_workflow_tuples() { # $1 workflow
  awk '
    # POSIX only. The gawk original used match($0, regexp, array), which mawk
    # does not implement -- the suite exited 2 here on any Debian/Ubuntu awk.
    /^[[:space:]]*(rev|prev|old)=[0-9a-f]+$/ {
      line = $0
      sub(/^[[:space:]]*/, "", line)
      eq = index(line, "=")
      refs[substr(line, 1, eq - 1)] = substr(line, eq + 1)
      next
    }
    /^[[:space:]]*check "\$(rev|prev|old)"/ {
      line = $0
      sub(/^[[:space:]]*check "\$/, "", line)
      q = index(line, "\"")
      if (q == 0) next
      name = substr(line, 1, q - 1)
      rest = substr(line, q + 1)
      sub(/^[[:space:]]+/, "", rest)
      nf = split(rest, f, /[[:space:]]+/)
      if (nf < 2) next
      digest = f[2]
      # No {64} interval -- mawk has no regex intervals. Length test instead.
      if (length(digest) != 64 || digest !~ /^[0-9a-f]+$/) next
      if (!(name in refs)) {
        printf "undefined workflow revision variable: %s\n", name > "/dev/stderr"
        exit 2
      }
      printf "%s\t%s\t%s\n", refs[name], f[1], digest
      tuples++
    }
    END {
      printf "workflow: %d revision/path/digest tuple(s) extracted\n", tuples > "/dev/stderr"
      if (tuples == 0) {
        print "DENOMINATOR ZERO: extracted no workflow tuples" > "/dev/stderr"
        exit 2
      }
    }
  ' "$1"
}

extract_document_tuples() { # $1 document
  awk -v current="$EXPECTED_CURRENT" -v previous="$EXPECTED_PREVIOUS" -v historical="$EXPECTED_HISTORICAL" '
    # POSIX only: field extraction by index()/substr(), not three-argument match().
    function cell(line, i,   n, c) { n = split(line, c, "|"); return (i <= n) ? c[i] : "" }
    function unback(text,   t) { t = text; sub(/^[^`]*`/, "", t); sub(/`.*$/, "", t); return t }

    /^\| File @ `[^`]+` \| sha256 \|$/ { ref = unback($0); next }
    /^\| `[^`]+`/ {
      if (ref == "") {
        print "digest row before revision header" > "/dev/stderr"
        exit 2
      }
      # The digest must be the LAST cell and must be a full 64-hex sha256, so a
      # table elsewhere in the document that happens to start with a backticked
      # token cannot be mistaken for a pin row.
      if (cell($0, 4) ~ /[^[:space:]]/) next
      dcell = cell($0, 3)
      if (dcell !~ /`[0-9a-f]+`/) next
      digest = unback(dcell)
      if (length(digest) != 64) next
      resolved = ref
      if (ref == substr(current, 1, 8)) resolved = current
      if (ref == substr(previous, 1, 8)) resolved = previous
      if (ref == historical) resolved = historical
      printf "%s\t%s\t%s\n", resolved, unback(cell($0, 2)), digest
      tuples++
    }
    END {
      printf "document: %d revision/path/digest tuple(s) extracted\n", tuples > "/dev/stderr"
      if (tuples == 0) {
        print "DENOMINATOR ZERO: extracted no document tuples" > "/dev/stderr"
        exit 2
      }
    }
  ' "$1"
}

reject_duplicate_keys() { # $1 tuple file, $2 source label
  local duplicates
  duplicates="$(cut -f1,2 "$1" | sort | uniq -d)"
  if [ -n "$duplicates" ]; then
    printf 'duplicate %s revision/path: %s\n' "$2" "$duplicates" >&2
    return 1
  fi
}

validate_tuple_mapping() { # $1 document, $2 workflow
  local doc_tuples="$FIXTURE/doc-tuples.$$" wf_tuples="$FIXTURE/wf-tuples.$$"
  extract_document_tuples "$1" | sort > "$doc_tuples"
  extract_workflow_tuples "$2" | sort > "$wf_tuples"

  if [ ! -s "$doc_tuples" ]; then
    printf 'no document tuples\n' >&2
    return 1
  fi
  if [ ! -s "$wf_tuples" ]; then
    printf 'no workflow tuples\n' >&2
    return 1
  fi
  reject_duplicate_keys "$doc_tuples" document || return 1
  reject_duplicate_keys "$wf_tuples" workflow || return 1
  if [ "$(wc -l < "$doc_tuples")" -ne 19 ] || [ "$(wc -l < "$wf_tuples")" -ne 19 ]; then
    printf 'expected 19 document and workflow tuples\n' >&2
    return 1
  fi
  diff -u "$doc_tuples" "$wf_tuples" >/dev/null || {
    printf 'revision/path/digest tuple mismatch\n' >&2
    return 1
  }
}

workflow_value() { # $1 variable, $2 workflow
  awk -v name="$1" '$0 ~ "^[[:space:]]*" name "=" { sub("^[[:space:]]*" name "=", ""); print; exit }' "$2"
}

validate_revisions() { # $1 document, $2 workflow
  local canonical current previous historical
  canonical="$(sed -n 's/^Canonical revision: `hyperpolymath\/hypatia@\([0-9a-f]*\)`.*/\1/p' "$1")"
  current="$(workflow_value rev "$2")"
  previous="$(workflow_value prev "$2")"
  historical="$(workflow_value old "$2")"
  [ "$canonical" = "$EXPECTED_CURRENT" ] || { printf 'wrong canonical revision\n' >&2; return 1; }
  [ "$current" = "$EXPECTED_CURRENT" ] || { printf 'wrong workflow current revision\n' >&2; return 1; }
  [ "$previous" = "$EXPECTED_PREVIOUS" ] || { printf 'wrong workflow previous revision\n' >&2; return 1; }
  [ "$historical" = "$EXPECTED_HISTORICAL" ] || { printf 'wrong workflow historical revision\n' >&2; return 1; }
}

extract_layer_paths() { # $1 heading pattern, $2 document
  awk -v heading="$1" '
    # POSIX only: no three-argument match(), no {64} regex interval.
    function cell(line, i,   n, c) { n = split(line, c, "|"); return (i <= n) ? c[i] : "" }
    function unback(text,   t) { t = text; sub(/^[^`]*`/, "", t); sub(/`.*$/, "", t); return t }
    /^### / { active = index($0, heading) > 0; next }
    active && /^\| `[^`]+`/ {
      if (cell($0, 4) ~ /[^[:space:]]/) next
      dcell = cell($0, 3)
      if (dcell !~ /`[0-9a-f]+`/) next
      if (length(unback(dcell)) != 64) next
      print unback(cell($0, 2))
      n++
    }
    END {
      printf "layer %s: %d pinned path(s)\n", heading, n > "/dev/stderr"
    }
  ' "$2"
}

validate_layers() { # $1 document
  local actual="$FIXTURE/layer-paths.$$" expected="$FIXTURE/expected-layer-paths"
  {
    printf '[normative]\n'
    extract_layer_paths 'Normative layer' "$1"
    printf '[generated]\n'
    extract_layer_paths 'Generated layer' "$1"
    printf '[implementation]\n'
    extract_layer_paths 'Implementation layer' "$1"
    printf '[superseded]\n'
    extract_layer_paths 'Superseded pin' "$1"
  } > "$actual"
  # Printed denominator, per the estate's vacuous-gate doctrine: 8 normative +
  # 3 generated + 1 implementation + 3 superseded = 15 pinned paths. The
  # historical pin table is deliberately excluded -- those digests are records
  # for audit, not pins the conformance job re-derives, and counting them here
  # would imply a gate that does not exist.
  local npaths
  npaths="$(grep -cv '^\[' "$actual")"
  if [ "$npaths" -ne 15 ]; then
    printf 'expected 15 pinned layer paths across 4 sections, extracted %d\n' "$npaths" >&2
    printf '(a renamed ### heading silently loses that whole layer)\n' >&2
  else
    printf 'layers: %d pinned path(s) across 4 sections\n' "$npaths" >&2
  fi
  diff -u "$expected" "$actual" >/dev/null || {
    printf 'ABI layer membership mismatch\n' >&2
    return 1
  }
}

extract_wire_entries() { # $1 document
  awk '
    # Two id/name pairs per markdown row, so eight rows carry sixteen entries.
    # POSIX only: no three-argument match(). The separator row |----|----| does
    # not match the digit pattern and the header row does not either, so neither
    # needs special-casing.
    /^\| [0-9]+ \| [a-z0-9-]+ \| [0-9]+ \| [a-z0-9-]+ \|$/ {
      n = split($0, c, "|")
      if (n != 6) next
      for (i = 2; i <= 5; i++) { sub(/^[[:space:]]+/, "", c[i]); sub(/[[:space:]]+$/, "", c[i]) }
      printf "%s\t%s\n%s\t%s\n", c[2], c[3], c[4], c[5]
      rows++
    }
    END {
      printf "wire table: %d markdown row(s), %d id/name entry(ies) extracted\n",
             rows, rows * 2 > "/dev/stderr"
      if (rows == 0) {
        print "DENOMINATOR ZERO: found no wire-table rows" > "/dev/stderr"
        exit 2
      }
    }
  ' "$1" | sort -n
}

validate_wire_table() { # $1 document
  local actual="$FIXTURE/wire-entries.$$"
  extract_wire_entries "$1" > "$actual"
  [ "$(wc -l < "$actual")" -eq 16 ] || { printf 'expected 16 wire entries\n' >&2; return 1; }
  [ "$(cut -f1 "$actual" | sort -n | uniq | wc -l)" -eq 16 ] || {
    printf 'wire ids are not unique\n' >&2
    return 1
  }
  diff -u "$FIXTURE/expected-wire-entries" "$actual" >/dev/null || {
    printf 'wire id/name mapping mismatch\n' >&2
    return 1
  }
}

# ── issue #139 ──────────────────────────────────────────────────────────────
#
# docs/abi/ABI-AUTHORITY.md transcribes the sixteen connector id/name pairs from
# `ffi/connectors.json`. Nothing ever compared the transcription against the
# file it transcribes, so a document that drifted from the generated manifest --
# the exact thing the whole pin apparatus exists to prevent -- would have passed
# every check in this suite. The prior wire-table fixture could not catch it
# because its expectation was itself a copy of the document.
#
# This compares ORDERED id/name pairs from both sides. Ordering is load-bearing
# upstream: connectors.json says so in its own `_note`, and `portFor = base + id
# + 1` means a renumbering silently moves every service onto another service's
# port.
extract_manifest_entries() { # $1 connectors.json
  awk '
    # POSIX only. The manifest is machine-generated and one pair per line, so a
    # flat scan is exact; no JSON parser is needed or wanted here.
    /"id":[[:space:]]*[0-9]+/ {
      line = $0
      if (!match(line, /"id":[[:space:]]*[0-9]+/)) next
      idpart = substr(line, RSTART, RLENGTH)
      sub(/^"id":[[:space:]]*/, "", idpart)
      if (!match(line, /"name":[[:space:]]*"[^"]*"/)) next
      namepart = substr(line, RSTART, RLENGTH)
      sub(/^"name":[[:space:]]*"/, "", namepart)
      sub(/"$/, "", namepart)
      printf "%s\t%s\n", idpart, namepart
      n++
    }
    END {
      printf "connectors.json: %d id/name pair(s) extracted\n", n > "/dev/stderr"
      if (n == 0) {
        print "DENOMINATOR ZERO: extracted no connector pairs from the manifest" > "/dev/stderr"
        exit 2
      }
    }
  ' "$1" | sort -n
}

validate_wire_manifest() { # $1 document, $2 connectors.json
  local from_doc="$FIXTURE/wire-doc.$$" from_json="$FIXTURE/wire-json.$$"
  extract_wire_entries "$1" > "$from_doc"
  extract_manifest_entries "$2" > "$from_json"

  local ndoc njson
  ndoc="$(wc -l < "$from_doc" | tr -d ' ')"
  njson="$(wc -l < "$from_json" | tr -d ' ')"
  printf 'wire table: %d of %d rows compared against %s\n' "$ndoc" "$njson" "$2" >&2

  [ "$ndoc" -ne 0 ] || { printf 'DENOMINATOR ZERO: no wire rows in the document\n' >&2; return 1; }
  [ "$njson" -ne 0 ] || { printf 'DENOMINATOR ZERO: no connector pairs in the manifest\n' >&2; return 1; }
  if [ "$ndoc" -ne "$njson" ]; then
    printf 'wire table row count %d != manifest connector count %d\n' "$ndoc" "$njson" >&2
    return 1
  fi
  if ! diff -u "$from_json" "$from_doc" > "$FIXTURE/wire-diff.$$" 2>&1; then
    printf 'wire table does not match the generated manifest (ordered id/name):\n' >&2
    sed 's/^/  /' "$FIXTURE/wire-diff.$$" >&2
    printf 'the document is a transcription of connectors.json; a disagreement here is a\n' >&2
    printf 'documentation defect, never a reason to renumber the manifest.\n' >&2
    return 1
  fi
}

# The committed fixture must be byte-identical to the file at the pinned
# revision, or the comparison above proves nothing about upstream. Its digest is
# pinned in docs/abi/ABI-AUTHORITY.md, so this is self-verifying: the fixture
# cannot silently rot, and moving the pin without moving the fixture fails here.
PINNED_CONNECTORS="$REPO_DIR/fixtures/abi/connectors-pinned.json"
PINNED_CONNECTORS_SHA256=a96fc2ef5cd83ea36be3953eadbf214eca012e7f6efebccaf4e48a8ae2adf330

validate_pinned_manifest_fixture() { # $1 connectors.json fixture
  local actual documented
  actual="$(sha256sum "$1" | cut -d' ' -f1)"
  documented="$(awk '
      /^\| File @ `[^`]+` \| sha256 \|$/ { ref = $0; sub(/^\| File @ `/, "", ref); sub(/`.*$/, "", ref); next }
      ref != "" && /^\| `ffi\/connectors.json` \| `[0-9a-f]+` \|$/ {
        d = $0; sub(/.*\| `/, "", d); sub(/` \|$/, "", d); print d; exit
      }
    ' "$DOCUMENT")"
  [ -n "$documented" ] || { printf 'DENOMINATOR ZERO: could not read the connectors.json pin from the document\n' >&2; return 1; }
  printf 'pinned manifest fixture: sha256 %s (document says %s for %s)\n' "$actual" "$documented" "$EXPECTED_CURRENT" >&2
  if [ "$actual" != "$documented" ]; then
    printf 'fixtures/abi/connectors-pinned.json is NOT the file the document pins\n' >&2
    printf '  fixture  %s\n' "$actual" >&2
    printf '  document %s\n' "$documented" >&2
    printf 're-copy it from hyperpolymath/hypatia at the canonical revision, or move the pin.\n' >&2
    return 1
  fi
  if [ "$actual" != "$PINNED_CONNECTORS_SHA256" ]; then
    printf 'fixture digest moved away from the value this suite was written against\n' >&2
    return 1
  fi
}

make_case() { # $1 case name
  local dir="$FIXTURE/$1"
  mkdir -p "$dir/docs/abi" "$dir/.github/workflows"
  cp "$DOCUMENT" "$dir/docs/abi/ABI-AUTHORITY.md"
  cp "$WORKFLOW" "$dir/.github/workflows/abi-conformance.yml"
  printf '%s\n' "$dir"
}

run_gate() { # $1 fixture root, remaining args are environment assignments
  local dir="$1" script="$1/authority-pin.sh"
  shift
  extract_authority_script "$dir/.github/workflows/abi-conformance.yml" "$script"
  (
    cd "$dir"
    env \
      PATH="$STUBS:$PATH" \
      ABI_TEST_UPSTREAM_MAP="$FIXTURE/upstream.tsv" \
      "$@" \
      bash "$script"
  )
}

mkdir -p "$STUBS"

# The curl stub maps each immutable upstream ref/path to the digest observed in
# the pristine workflow. The sha256sum stub returns that fixture digest. This
# keeps the suite offline while exercising the production run block itself.
cat > "$STUBS/curl" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
out=
url=
while [ "$#" -gt 0 ]; do
  case "$1" in
    -o) out="$2"; shift 2 ;;
    -*) shift ;;
    http://*|https://*) url="$1"; shift ;;
    *) shift ;;
  esac
done
[ -n "$url" ] || exit 2
relative="${url#https://raw.githubusercontent.com/hyperpolymath/hypatia/}"
ref="${relative%%/*}"
path="${relative#*/}"
if [ "$path" = "ffi/__no_such_file__.json" ]; then
  [ "${ABI_TEST_ALLOW_MISSING:-0}" = 1 ] || exit 22
  printf 'unexpected data\n'
  exit 0
fi
[ "$path" != "${ABI_TEST_FAIL_PATH:-}" ] || exit 22
digest="$(awk -F '\t' -v ref="$ref" -v path="$path" '$1 == ref && $2 == path { print $3; exit }' "$ABI_TEST_UPSTREAM_MAP")"
[ -n "$digest" ] || exit 22
if [ "$path" = "${ABI_TEST_MUTATE_PATH:-}" ]; then
  digest=0000000000000000000000000000000000000000000000000000000000000000
fi
if [ -n "$out" ]; then
  printf '%s\n' "$digest" > "$out"
else
  printf '%s\n' "$digest"
fi
STUB

cat > "$STUBS/sha256sum" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
IFS= read -r digest
case "$digest" in
  *[!0-9a-f]*|'') exit 2 ;;
esac
[ "${#digest}" -eq 64 ] || exit 2
printf '%s  -\n' "$digest"
STUB

chmod +x "$STUBS/curl" "$STUBS/sha256sum"
extract_workflow_tuples "$WORKFLOW" > "$FIXTURE/upstream.tsv"

cat > "$FIXTURE/expected-layer-paths" <<'EXPECTED'
[normative]
src/Hypatia/ABI/Types.idr
src/Hypatia/ABI/FFI.idr
src/Hypatia/ABI/REST.idr
src/Hypatia/ABI/GraphQL.idr
src/Hypatia/ABI/GRPC.idr
src/Hypatia/ABI/RuleEngine.idr
src/Hypatia/ABI/Gen.idr
src/abi/hypatia-abi-gen.ipkg
[generated]
ffi/connectors.json
ffi/zig/src/connector_generated.zig
clients/rust/hypatia-client/src/connector_generated.rs
[implementation]
ffi/zig/src/unified-api-adapter.zig
[superseded]
src/Hypatia/ABI/Types.idr
ffi/zig/src/unified-api-adapter.zig
ffi/connectors.json
EXPECTED

cat > "$FIXTURE/expected-wire-entries" <<'EXPECTED'
0	grpc
1	graphql
2	rest
3	flatbuffers
4	bebop
5	jsonrpc
6	websocket
7	mqtt
8	trpc
9	capnproto
10	soap
11	verisimdb-rest
12	bsp
13	scip
14	ipfs
15	arrow-flight
EXPECTED

printf 'production authority-pin fixtures:\n'
clean="$(make_case clean)"
expect_pass 'the real workflow run block accepts all 19 pinned fixtures' run_gate "$clean"
expect_fail 'a fetched-byte digest mismatch reports drift' 'DRIFT: src/Hypatia/ABI/Types.idr' \
  run_gate "$clean" ABI_TEST_MUTATE_PATH=src/Hypatia/ABI/Types.idr
expect_fail 'an unresolvable pinned file fails closed' 'FETCH FAILED: ffi/connectors.json' \
  run_gate "$clean" ABI_TEST_FAIL_PATH=ffi/connectors.json
expect_fail 'the nonexistent-path negative control proves it can fire' 'CONTROL FAILED: a nonexistent path returned data' \
  run_gate "$clean" ABI_TEST_ALLOW_MISSING=1

doc_drift="$(make_case doc-drift)"
sed -i '0,/5f6692718b262e9926179144fb527946cbe94503e37815473f18b9c37ed4f3f4/s//0000000000000000000000000000000000000000000000000000000000000000/' \
  "$doc_drift/docs/abi/ABI-AUTHORITY.md"
expect_fail 'a document-only digest change reports doc drift' 'DOC DRIFT:' run_gate "$doc_drift"

empty_doc="$(make_case empty-doc)"
: > "$empty_doc/docs/abi/ABI-AUTHORITY.md"
expect_any_fail 'an empty document cannot pass vacuously' run_gate "$empty_doc"

printf 'revision/path/digest mapping fixtures:\n'
expect_pass 'the document and workflow have the same 19 exact tuples' \
  validate_tuple_mapping "$DOCUMENT" "$WORKFLOW"

swapped="$(make_case swapped-digests)"
sed -i \
  -e 's/5f6692718b262e9926179144fb527946cbe94503e37815473f18b9c37ed4f3f4/SWAP_DIGEST/' \
  -e 's/e7fb117bfe137e76232ddb220db82a389808a63d3eda29e3d47057052366c669/5f6692718b262e9926179144fb527946cbe94503e37815473f18b9c37ed4f3f4/' \
  -e 's/SWAP_DIGEST/e7fb117bfe137e76232ddb220db82a389808a63d3eda29e3d47057052366c669/' \
  "$swapped/docs/abi/ABI-AUTHORITY.md"
expect_fail 'swapping two valid digests between paths is rejected' 'tuple mismatch' \
  validate_tuple_mapping "$swapped/docs/abi/ABI-AUTHORITY.md" "$swapped/.github/workflows/abi-conformance.yml"

duplicate="$(make_case duplicate-path)"
sed -i '/^| `src\/Hypatia\/ABI\/Types.idr` | `5f669/a | `src/Hypatia/ABI/Types.idr` | `e7fb117bfe137e76232ddb220db82a389808a63d3eda29e3d47057052366c669` |' \
  "$duplicate/docs/abi/ABI-AUTHORITY.md"
expect_fail 'duplicate revision/path rows are rejected' 'duplicate document revision/path' \
  validate_tuple_mapping "$duplicate/docs/abi/ABI-AUTHORITY.md" "$duplicate/.github/workflows/abi-conformance.yml"

no_checks="$(make_case no-workflow-checks)"
sed -i 's/^\([[:space:]]*\)check "/\1# removed check "/' "$no_checks/.github/workflows/abi-conformance.yml"
expect_fail 'a workflow with no pin checks cannot pass vacuously' 'no workflow tuples' \
  validate_tuple_mapping "$no_checks/docs/abi/ABI-AUTHORITY.md" "$no_checks/.github/workflows/abi-conformance.yml"

printf 'revision and documentation fixtures:\n'
expect_pass 'canonical, current, superseded, and historical revisions are exact' \
  validate_revisions "$DOCUMENT" "$WORKFLOW"
wrong_revision="$(make_case wrong-revision)"
sed -i "s/$EXPECTED_CURRENT/0000000000000000000000000000000000000000/" \
  "$wrong_revision/docs/abi/ABI-AUTHORITY.md"
expect_fail 'stale canonical revision metadata is rejected' 'wrong canonical revision' \
  validate_revisions "$wrong_revision/docs/abi/ABI-AUTHORITY.md" "$wrong_revision/.github/workflows/abi-conformance.yml"

expect_pass 'normative, generated, implementation, and superseded paths stay in their layers' \
  validate_layers "$DOCUMENT"
wrong_layer="$(make_case wrong-layer)"
sed -i 's/^### Generated layer/### Generated output/' "$wrong_layer/docs/abi/ABI-AUTHORITY.md"
expect_fail 'renaming or losing a load-bearing layer is rejected' 'layer membership mismatch' \
  validate_layers "$wrong_layer/docs/abi/ABI-AUTHORITY.md"

expect_pass 'all 16 connector ids retain their documented names' validate_wire_table "$DOCUMENT"
wrong_name="$(make_case wrong-name)"
sed -i 's/| 11 | verisimdb-rest |/| 11 | verismbd-rest |/' "$wrong_name/docs/abi/ABI-AUTHORITY.md"
expect_fail 'the former id-11 typo is a firing regression fixture' 'wire id/name mapping mismatch' \
  validate_wire_table "$wrong_name/docs/abi/ABI-AUTHORITY.md"
duplicate_id="$(make_case duplicate-id)"
sed -i 's/| 7 | mqtt | 15 | arrow-flight |/| 7 | mqtt | 7 | arrow-flight |/' \
  "$duplicate_id/docs/abi/ABI-AUTHORITY.md"
expect_fail 'duplicate wire ids are rejected' 'wire ids are not unique' \
  validate_wire_table "$duplicate_id/docs/abi/ABI-AUTHORITY.md"

expect_pass 'the independent 40-hex source-blob provenance pin is present' \
  grep -qF '`788493e6f54b6d436ea551dbb5d34c6bc0d819ab` for `src/Hypatia/ABI/Types.idr`' "$DOCUMENT"

printf 'wire table vs the generated manifest (issue #139):\n'
expect_pass 'the committed fixture is byte-identical to the pinned connectors.json' \
  validate_pinned_manifest_fixture "$PINNED_CONNECTORS"
expect_pass 'all 16 documented id/name pairs match the manifest, in order' \
  validate_wire_manifest "$DOCUMENT" "$PINNED_CONNECTORS"

# Firing fixtures. Each mutates ONE side and must be caught, because a check
# that agrees with itself is the failure mode issue #139 exists to close.
manifest_wrong_name="$FIXTURE/connectors-wrong-name.json"
sed 's/"name": "verisimdb-rest"/"name": "verismdb-rest"/' "$PINNED_CONNECTORS" > "$manifest_wrong_name"
expect_fail 'a manifest name the document misspells is rejected' \
  'does not match the generated manifest' \
  validate_wire_manifest "$DOCUMENT" "$manifest_wrong_name"

manifest_swapped="$FIXTURE/connectors-swapped-ids.json"
sed -e 's/{ "id": 3, "name": "flatbuffers" }/{ "id": 3, "name": "bebop" }/' \
    -e 's/{ "id": 4, "name": "bebop" }/{ "id": 4, "name": "flatbuffers" }/' \
    "$PINNED_CONNECTORS" > "$manifest_swapped"
expect_fail 'swapping two manifest ids against the document is rejected' \
  'does not match the generated manifest' \
  validate_wire_manifest "$DOCUMENT" "$manifest_swapped"

manifest_missing="$FIXTURE/connectors-missing-row.json"
sed '/{ "id": 9, "name": "capnproto" },/d' "$PINNED_CONNECTORS" > "$manifest_missing"
expect_fail 'a manifest missing one connector is rejected on the count' \
  'wire table row count 16 != manifest connector count 15' \
  validate_wire_manifest "$DOCUMENT" "$manifest_missing"

manifest_empty="$FIXTURE/connectors-empty.json"
printf '{ "connectors": [] }\n' > "$manifest_empty"
expect_fail 'an empty manifest cannot pass vacuously' \
  'DENOMINATOR ZERO' \
  validate_wire_manifest "$DOCUMENT" "$manifest_empty"

doc_wire_missing="$(make_case doc-wire-row-missing)"
sed -i '/^| 7 | mqtt | 15 | arrow-flight |$/d' "$doc_wire_missing/docs/abi/ABI-AUTHORITY.md"
expect_fail 'a document that dropped a wire row is rejected' \
  'wire table row count 14 != manifest connector count 16' \
  validate_wire_manifest "$doc_wire_missing/docs/abi/ABI-AUTHORITY.md" "$PINNED_CONNECTORS"

rotted_fixture="$FIXTURE/connectors-rotted.json"
printf '{ "connectors": [ { "id": 0, "name": "grpc" } ] }\n' > "$rotted_fixture"
expect_fail 'a fixture that is not the pinned file is rejected by its own digest' \
  'is NOT the file the document pins' \
  validate_pinned_manifest_fixture "$rotted_fixture"

if [ "$failures" -ne 0 ]; then
  annotate error "abi-authority-pin: $failures assertion(s) FAILED, $checks passed -- see the per-assertion annotations above"
  printf 'ABI authority pin fixtures: %d FAILURES, %d assertions passed\n' "$failures" "$checks" >&2
  exit 1
fi
printf 'ABI authority pin fixtures: %d assertion(s) passed, 0 failed\n' "$checks"
