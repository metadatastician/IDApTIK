#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Offline regression coverage for the CI hardening in PR #143.
#
# The production inputs are declarative YAML, so this suite checks their
# security-relevant structure directly and then mutates temporary copies to
# prove each assertion can fail. It intentionally needs no network, action
# runner, or project toolchain.

set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEPENDABOT="$REPO_DIR/.github/dependabot.yml"
CODEQL="$REPO_DIR/.github/workflows/codeql.yml"
FIXTURE="$(mktemp -d)"
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

check_dependabot_limit() { # $1 configuration
  awk '
    function raw_value_after_colon(line, value) {
      value = line
      sub(/^[^:]+:[[:space:]]*/, "", value)
      sub(/[[:space:]]+#.*$/, "", value)
      return value
    }

    function value_after_colon(line, value) {
      value = raw_value_after_colon(line)
      if (value ~ /^".*"$/) {
        sub(/^"/, "", value)
        sub(/"$/, "", value)
      }
      return value
    }

    /^  - package-ecosystem:[[:space:]]*/ {
      ecosystem = value_after_colon($0)
      in_github_actions = ecosystem == "github-actions"
      if (in_github_actions) {
        github_actions_blocks++
      }
      next
    }

    in_github_actions && /^    open-pull-requests-limit:[[:space:]]*/ {
      limits++
      limit = raw_value_after_colon($0)
    }

    END {
      if (github_actions_blocks != 1) {
        printf "expected exactly one github-actions update block, found %d\n", github_actions_blocks > "/dev/stderr"
        exit 1
      }
      if (limits != 1) {
        printf "expected exactly one github-actions open-pull-requests-limit, found %d\n", limits > "/dev/stderr"
        exit 1
      }
      if (limit != "2") {
        printf "github-actions open-pull-requests-limit must be integer 2, found %s\n", limit > "/dev/stderr"
        exit 1
      }
    }
  ' "$1"
}

check_codeql_pins() { # $1 workflow
  awk '
    /^[[:space:]]*- name:[[:space:]]*/ {
      step = $0
      sub(/^[[:space:]]*- name:[[:space:]]*/, "", step)
      sub(/[[:space:]]+$/, "", step)
      next
    }

    /^[[:space:]]*uses:[[:space:]]*github\/codeql-action\// {
      ref = $0
      sub(/^[[:space:]]*uses:[[:space:]]*/, "", ref)
      sub(/[[:space:]]+#.*$/, "", ref)

      if (split(ref, parts, "@") != 2) {
        printf "malformed CodeQL action reference: %s\n", ref > "/dev/stderr"
        failed = 1
        next
      }

      action = parts[1]
      pin = parts[2]
      if (length(pin) != 40 || pin !~ /^[0-9a-f]+$/) {
        printf "CodeQL action must use a full lowercase commit SHA: %s\n", ref > "/dev/stderr"
        failed = 1
      }

      if (action == "github/codeql-action/init") {
        init_count++
        init_pin = pin
        if (step != "Initialize CodeQL") {
          printf "CodeQL init action is attached to unexpected step: %s\n", step > "/dev/stderr"
          failed = 1
        }
      } else if (action == "github/codeql-action/analyze") {
        analyze_count++
        analyze_pin = pin
        if (step != "Perform CodeQL Analysis") {
          printf "CodeQL analyze action is attached to unexpected step: %s\n", step > "/dev/stderr"
          failed = 1
        }
      } else {
        printf "unexpected CodeQL action entry point: %s\n", action > "/dev/stderr"
        failed = 1
      }
    }

    END {
      if (init_count != 1) {
        printf "expected exactly one CodeQL init action, found %d\n", init_count > "/dev/stderr"
        failed = 1
      }
      if (analyze_count != 1) {
        printf "expected exactly one CodeQL analyze action, found %d\n", analyze_count > "/dev/stderr"
        failed = 1
      }
      if (init_count == 1 && analyze_count == 1 && init_pin != analyze_pin) {
        printf "CodeQL init and analyze actions must use the same commit SHA\n" > "/dev/stderr"
        failed = 1
      }
      exit failed
    }
  ' "$1"
}

printf 'production configuration:\n'
expect_pass "GitHub Actions updates are capped at two open pull requests" \
  check_dependabot_limit "$DEPENDABOT"
expect_pass "CodeQL init and analyze use one immutable commit pin" \
  check_codeql_pins "$CODEQL"

printf 'Dependabot firing fixtures:\n'
awk '
  /^  - package-ecosystem:[[:space:]]*"github-actions"/ {
    in_actions = 1
    print
    next
  }
  in_actions && /^    open-pull-requests-limit:/ { next }
  in_actions && /^  - package-ecosystem:/ { in_actions = 0 }
  { print }
' "$DEPENDABOT" > "$FIXTURE/dependabot-missing.yml"
expect_fail "a missing GitHub Actions limit is rejected" \
  "expected exactly one github-actions open-pull-requests-limit" \
  check_dependabot_limit "$FIXTURE/dependabot-missing.yml"

awk '
  /^  - package-ecosystem:[[:space:]]*"github-actions"/ { in_actions = 1 }
  in_actions && /^    open-pull-requests-limit:/ {
    print "    open-pull-requests-limit: 3"
    in_actions = 0
    next
  }
  { print }
' "$DEPENDABOT" > "$FIXTURE/dependabot-too-high.yml"
expect_fail "a limit above the requested boundary is rejected" \
  "github-actions open-pull-requests-limit must be integer 2" \
  check_dependabot_limit "$FIXTURE/dependabot-too-high.yml"

awk '
  /^  - package-ecosystem:[[:space:]]*"github-actions"/ { in_actions = 1 }
  in_actions && /^    open-pull-requests-limit:/ {
    print "    open-pull-requests-limit: \"2\""
    in_actions = 0
    next
  }
  { print }
' "$DEPENDABOT" > "$FIXTURE/dependabot-string-limit.yml"
expect_fail "a string-valued limit is rejected" \
  "github-actions open-pull-requests-limit must be integer 2" \
  check_dependabot_limit "$FIXTURE/dependabot-string-limit.yml"

awk '
  /^  - package-ecosystem:[[:space:]]*"github-actions"/ { in_actions = 1 }
  in_actions && /^    open-pull-requests-limit:/ { held = $0; next }
  in_actions && /^  - package-ecosystem:[[:space:]]*"cargo"/ {
    in_actions = 0
    print
    print held
    next
  }
  { print }
' "$DEPENDABOT" > "$FIXTURE/dependabot-wrong-block.yml"
expect_fail "a limit placed on another ecosystem does not satisfy the policy" \
  "expected exactly one github-actions open-pull-requests-limit" \
  check_dependabot_limit "$FIXTURE/dependabot-wrong-block.yml"

awk '
  { print }
  /^  - package-ecosystem:[[:space:]]*"github-actions"/ { print }
' "$DEPENDABOT" > "$FIXTURE/dependabot-duplicate-block.yml"
expect_fail "duplicate GitHub Actions update blocks are rejected" \
  "expected exactly one github-actions update block" \
  check_dependabot_limit "$FIXTURE/dependabot-duplicate-block.yml"

printf 'CodeQL firing fixtures:\n'
sed '0,/@[0-9a-f]\{40\}/s//@v4.38.0/' "$CODEQL" > "$FIXTURE/codeql-tag.yml"
expect_fail "a movable version tag is rejected" \
  "CodeQL action must use a full lowercase commit SHA" \
  check_codeql_pins "$FIXTURE/codeql-tag.yml"

sed '0,/@[0-9a-f]\{40\}/s//@0123456789abcdef/' "$CODEQL" > "$FIXTURE/codeql-short-sha.yml"
expect_fail "an abbreviated commit SHA is rejected" \
  "CodeQL action must use a full lowercase commit SHA" \
  check_codeql_pins "$FIXTURE/codeql-short-sha.yml"

awk '
  /uses:[[:space:]]*github\/codeql-action\/analyze@/ {
    sub(/@[0-9a-f]{40}/, "@0000000000000000000000000000000000000000")
  }
  { print }
' "$CODEQL" > "$FIXTURE/codeql-divergent.yml"
expect_fail "init and analyze cannot drift to different revisions" \
  "CodeQL init and analyze actions must use the same commit SHA" \
  check_codeql_pins "$FIXTURE/codeql-divergent.yml"

sed '0,/github\/codeql-action\/init/s//github\/codeql-action\/upload-sarif/' \
  "$CODEQL" > "$FIXTURE/codeql-wrong-entry-point.yml"
expect_fail "an unexpected CodeQL action entry point is rejected" \
  "unexpected CodeQL action entry point" \
  check_codeql_pins "$FIXTURE/codeql-wrong-entry-point.yml"

if (( failures > 0 )); then
  printf '%d CI security configuration test(s) failed\n' "$failures" >&2
  exit 1
fi

printf 'CI security configuration fixtures: all passed\n'
