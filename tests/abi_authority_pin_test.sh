#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Offline fixture coverage for the ABI authority pin refreshed in PR #138.
#
# The production gate is embedded in a GitHub Actions workflow. This suite
# executes that exact run block with deterministic curl/sha256sum stubs, then
# independently checks the document-to-workflow revision/path/digest mapping.
# The independent comparison is intentionally stricter than comparing digest
# sets: swapping two valid digests between paths must fail.

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

trap 'rm -rf "$FIXTURE"' EXIT

note() { printf '  ok: %s\n' "$1"; }

fail() {
  printf '  FAIL: %s\n' "$1" >&2
  failures=$((failures + 1))
}

expect_pass() { # $1 description, rest = command
  local desc="$1"
  shift
  if "$@" >/dev/null 2>&1; then
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
    match($0, /^[[:space:]]*(rev|prev|old)=([0-9a-f]+)$/, value) {
      refs[value[1]] = value[2]
      next
    }
    match($0, /^[[:space:]]*check "\$(rev|prev|old)"[[:space:]]+([^[:space:]]+)[[:space:]]+([0-9a-f]{64})[[:space:]]*$/, row) {
      if (!(row[1] in refs)) {
        printf "undefined workflow revision variable: %s\n", row[1] > "/dev/stderr"
        exit 2
      }
      printf "%s\t%s\t%s\n", refs[row[1]], row[2], row[3]
    }
  ' "$1"
}

extract_document_tuples() { # $1 document
  awk -v current="$EXPECTED_CURRENT" -v previous="$EXPECTED_PREVIOUS" -v historical="$EXPECTED_HISTORICAL" '
    match($0, /^\| File @ `([^`]+)` \| sha256 \|$/, header) {
      ref = header[1]
      next
    }
    match($0, /^\| `([^`]+)`[^|]*\| `([0-9a-f]{64})` \|$/, row) {
      if (ref == "") {
        print "digest row before revision header" > "/dev/stderr"
        exit 2
      }
      resolved = ref
      if (ref == substr(current, 1, 8)) resolved = current
      if (ref == substr(previous, 1, 8)) resolved = previous
      if (ref == historical) resolved = historical
      printf "%s\t%s\t%s\n", resolved, row[1], row[2]
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
    /^### / { active = index($0, heading) > 0; next }
    active && match($0, /^\| `([^`]+)`[^|]*\| `[0-9a-f]{64}` \|$/, row) { print row[1] }
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
  diff -u "$expected" "$actual" >/dev/null || {
    printf 'ABI layer membership mismatch\n' >&2
    return 1
  }
}

extract_wire_entries() { # $1 document
  awk '
    match($0, /^\| ([0-9]+) \| ([a-z0-9-]+) \| ([0-9]+) \| ([a-z0-9-]+) \|$/, row) {
      printf "%s\t%s\n%s\t%s\n", row[1], row[2], row[3], row[4]
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

if [ "$failures" -ne 0 ]; then
  printf 'ABI authority pin fixtures: %d FAILURES\n' "$failures" >&2
  exit 1
fi
printf 'ABI authority pin fixtures: all clean and firing fixtures behaved\n'
