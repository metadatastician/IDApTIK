#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

if [[ ! -x .githooks/pre-push ]]; then
  printf '%s\n' "error: .githooks/pre-push is missing or is not executable" >&2
  exit 1
fi

git config --local core.hooksPath .githooks
printf '%s\n' "Git hooks active: core.hooksPath=.githooks"
