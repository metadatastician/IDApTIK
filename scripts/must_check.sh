#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Execute the physical-state probes declared in
# .machine_readable/contractiles/Mustfile.a2ml (the estate `must check` verb).
#
# Each `### probe` block's `- run:` command is executed with bash from the
# repository root. Probes with severity `critical` gate the exit status;
# `warning` probes are reported but do not fail the run. A probe whose tool is
# missing fails — an absent toolchain is a failure, never a skip.

set -uo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MUSTFILE="$REPO_DIR/.machine_readable/contractiles/Mustfile.a2ml"

if [ ! -r "$MUSTFILE" ]; then
  printf 'must: %s is missing\n' "$MUSTFILE" >&2
  exit 2
fi

# Emit one record per probe: name<TAB>severity<TAB>command
parse_probes() {
  awk '
    /^### / {
      if (name != "") emit()
      name = $2
      severity = "critical"
      run = ""
      next
    }
    /^- run:/     { sub(/^- run:[[:space:]]*/, ""); run = $0; next }
    /^- severity:/ { sub(/^- severity:[[:space:]]*/, ""); severity = $0; next }
    END { if (name != "") emit() }
    function emit() {
      if (run != "") printf "%s\t%s\t%s\n", name, severity, run
    }
  ' "$MUSTFILE"
}

critical_failures=0
warnings_failed=0
total=0

while IFS=$'\t' read -r name severity command; do
  [ -n "$name" ] || continue
  total=$((total + 1))
  if (cd "$REPO_DIR" && bash -c "$command") >/dev/null 2>&1; then
    printf '  ok    %s\n' "$name"
  else
    case "$severity" in
      warning)
        warnings_failed=$((warnings_failed + 1))
        printf '  WARN  %s\n' "$name" >&2
        ;;
      *)
        critical_failures=$((critical_failures + 1))
        detail="$(cd "$REPO_DIR" && bash -c "$command" 2>&1 | tail -3 || true)"
        printf '  FAIL  %s\n' "$name" >&2
        if [ -n "$detail" ]; then
          printf '%s\n' "$detail" | sed 's/^/        /' >&2
        fi
        ;;
    esac
  fi
done < <(parse_probes)

printf 'must: %d probes, %d critical failure(s), %d warning failure(s)\n' \
  "$total" "$critical_failures" "$warnings_failed"

[ "$critical_failures" -eq 0 ]
