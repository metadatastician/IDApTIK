# IDApTIK task runner. Run `just` (no args) to list recipes.
#
# Toolchains are pinned in mise.toml (+ rust-toolchain.toml for Rust).
# Recipes that build application code activate as the crates/server land; the
# environment recipes (setup/doctor/versions) work today.

set shell := ["bash", "-euo", "pipefail", "-c"]

# List available recipes.
default:
    @just --list

# --- Environment ---------------------------------------------------------------

# Install the pinned toolchains (mise) + Rust targets. Erlang/Elixir build from
# source on first run and take a while; subsequent runs are cached.
setup:
    mise trust
    mise install
    rustup target add wasm32-unknown-unknown
    @echo "Base toolchains ready. On Linux also run 'just bevy-linux-deps'; for Idris2 'just install-idris2'."

# Install Bevy's Linux system libraries (Debian/Ubuntu). Bevy needs these to
# build/run on Linux (audio, input, windowing); not needed on macOS/Windows.
bevy-linux-deps:
    sudo apt-get update
    sudo apt-get install -y pkg-config libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev

# Fast bootstrap for ephemeral/web sessions: mise + prebuilt tools only
# (skips the slow Erlang/Elixir source builds). Called by the SessionStart hook.
bootstrap:
    bash scripts/bootstrap.sh

# Provision Idris2 (+ Chez Scheme) via idris2-pack. Separate because it compiles
# a Scheme backend and is not needed for day-to-day gameplay work.
install-idris2:
    bash scripts/install-idris2.sh

# Print the version of every toolchain we depend on; flags anything missing.
doctor:
    @bash scripts/doctor.sh

# Show the pinned versions at a glance.
versions:
    @cat mise.toml rust-toolchain.toml

# --- Rust (gameplay truth) -----------------------------------------------------
# Activate once crates/ exists (see ADR-0003). Left as the intended commands.

build:
    cargo build --workspace

run-bevy:
    cargo run -p idaptik-bevy

run-fyrox:
    cargo run -p idaptik-fyrox

# Run the Ghost Lobby terminal frontend (ratatui/crossterm, ADR-0004).
run-tui:
    cargo run -p idaptik-tui

# Run a headless script and print the event-log + debrief + snapshot JSON.
headless SCRIPT:
    cargo run -q -p idaptik-tui -- --headless --script {{SCRIPT}}

# Verify a script replays deterministically (PASS/FAIL + exit code).
replay FILE:
    cargo run -q -p idaptik-tui -- --replay {{FILE}}

test:
    cargo test --workspace

# The Ghost Lobby scenario gate: core + tui + ffi (does NOT build bevy/fyrox).
test-ghost:
    cargo test -p idaptik-core -p idaptik-tui -p idaptik-ffi
    cargo clippy -p idaptik-core -p idaptik-tui --all-targets -- -D warnings
    cargo fmt --all -- --check

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --workspace --all-targets -- -D warnings

# --- Elixir (multiplayer / session) --------------------------------------------
# Bandit + Phoenix Channels, not LiveView (see ADR-0002). Activates with server/.

server-setup:
    cd server && mix deps.get

server:
    cd server && mix phx.server

server-test:
    cd server && mix test

# --- Config (Nickel) -----------------------------------------------------------

config-check:
    @if command -v nickel >/dev/null 2>&1; then nickel export config/default.ncl >/dev/null && nickel export config/grounded_slice.ncl >/dev/null && nickel export config/ghost_lobby_floor.ncl >/dev/null && just config-scenario-check && echo "config: ok (schema applied)"; else echo "nickel not installed: run 'just setup'"; fi

# Check the Nickel-authored scenario against scenario-schema.ncl: the good
# fixture must export and round-trip through the Rust validator unchanged, and
# every bad fixture must be rejected by the contract (mirroring the Rust
# ValidationError variants).
config-scenario-check:
    nickel export config/ghost_lobby_scenario.ncl --format json > /tmp/ghost_lobby_scenario_raw.json
    cargo test -q -p idaptik-core nickel_scenario_round_trips_and_validates -- --ignored
    @for f in config/scenario-fixtures/bad_*.ncl; do \
        if nickel export "$f" >/dev/null 2>&1; then \
            echo "FAIL: $f exported but the contract should reject it"; exit 1; \
        fi; \
        echo "rejected as expected: $f"; \
    done
    @echo "scenario config: ok"

# Re-export the Nickel graphs and rewrite the committed goldens they embed. The
# exports are read at compile time, so each regeneration is its own cargo run:
# the derived floor graph is rebuilt last, from the freshly embedded backbone.
config-regen:
    nickel export config/grounded_slice.ncl --format json > /tmp/grounded_slice_raw.json
    nickel export config/ghost_lobby_floor.ncl --format json > /tmp/ghost_lobby_floor_raw.json
    cargo test -p idaptik-core regenerate_slice_json -- --ignored
    cargo test -p idaptik-core regenerate_ghost_lobby_floor_json -- --ignored
    cargo test -p idaptik-core regenerate_floor_graph_json -- --ignored
    @echo "config: goldens regenerated"
