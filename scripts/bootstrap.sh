#!/usr/bin/env bash
# Fast, idempotent toolchain bootstrap for ephemeral environments (Claude Code
# web sessions, CI). Ensures mise is present and installs the *prebuilt* tools
# (Zig, Just, Nickel) so they resolve immediately. It deliberately does NOT
# build Erlang/Elixir from source here — that is slow; run `just setup` for the
# full set. See docs/adr/0001-toolchain-and-runtime-management.md.
set -euo pipefail
cd "$(dirname "$0")/.."

log() { printf '\033[1;34m[bootstrap]\033[0m %s\n' "$*"; }

# 1. Ensure mise is available.
if ! command -v mise >/dev/null 2>&1; then
  log "installing mise…"
  curl -fsSL https://mise.run | sh
  export PATH="$HOME/.local/bin:$PATH"
fi

# 2. Trust this repo's config so mise will use it non-interactively.
mise trust --quiet 2>/dev/null || mise trust 2>/dev/null || true

# 3. Install the fast, prebuilt tools now. Erlang/Elixir are left for `just setup`.
for tool in zig just nickel; do
  log "ensuring ${tool}…"
  mise install "${tool}" 2>&1 | sed 's/^/  /' || log "WARN: could not install ${tool} (continuing)"
done

# 4. Make sure the pinned Rust toolchain is available (rustup honours rust-toolchain.toml).
if command -v rustup >/dev/null 2>&1; then
  rustup show active-toolchain >/dev/null 2>&1 || rustup toolchain install "$(grep -oP 'channel = "\K[^"]+' rust-toolchain.toml || echo stable)"
  rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
fi

log "done. Run 'just doctor' to see what is present, or 'just setup' for the full toolchain."
