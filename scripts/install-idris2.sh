#!/usr/bin/env bash
# Provision Idris2 via idris2-pack (https://github.com/stefan-hoeck/idris2-pack).
# Idris2 is used for formally modelling the FFI/ABI contracts (see ADR-0001). It
# bootstraps through Chez Scheme + GMP, which is why it is a separate, opt-in step
# rather than a line in mise.toml.
set -euo pipefail

log() { printf '\033[1;34m[idris2]\033[0m %s\n' "$*"; }

if command -v idris2 >/dev/null 2>&1; then
  log "already installed: $(idris2 --version)"; exit 0
fi

# Prerequisites: a Scheme (Chez preferred) + GMP + build tooling.
missing=()
command -v scheme >/dev/null 2>&1 || command -v chezscheme >/dev/null 2>&1 || missing+=("chezscheme")
command -v gcc >/dev/null 2>&1 || command -v clang >/dev/null 2>&1 || missing+=("a C compiler")
if [ "${#missing[@]}" -ne 0 ]; then
  log "missing prerequisites: ${missing[*]}"
  log "on Debian/Ubuntu:  sudo apt-get install -y chezscheme libgmp3-dev build-essential"
  log "then re-run: just install-idris2"
  exit 1
fi

log "installing idris2-pack (this compiles the Idris2 compiler; ~a few minutes)…"
bash -c "$(curl -fsSL https://raw.githubusercontent.com/stefan-hoeck/idris2-pack/main/install.bash)"
log "done. Ensure ~/.pack/bin is on your PATH."
