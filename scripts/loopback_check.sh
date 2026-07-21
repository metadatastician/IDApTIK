#!/usr/bin/env bash
# The ADR-0006 §4 loopback gate, PlainWebSocketTransport configuration.
#
# Two seat processes on one host, a throwaway local relay, the shared wire
# fixture script split across the seats. PASS requires:
#
#   1. determinism  — both seats' artifacts are byte-identical, AND identical
#                     to `idaptik-tui --headless` on the same script (the
#                     network layer added and lost nothing);
#   2. loss handling — killing one seat mid-stream drives the other through
#                     PeerLost to a clean end (exit 0, status ended_peer_lost),
#                     not a crash or a hang.
#
# Requirements are hard: a missing toolchain FAILS the gate (estate doctrine —
# a gate that skips is a gate that lies).
set -euo pipefail
cd "$(dirname "$0")/.."

SCRIPT="${1:-fixtures/session_relay/capture_script.json}"
PORT="${IDAPTIK_LOOPBACK_PORT:-4013}"
URL="ws://127.0.0.1:${PORT}/socket/websocket"

command -v mix >/dev/null 2>&1 || { echo "FAIL: mix (Elixir) is required — run 'just setup'"; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "FAIL: cargo (Rust) is required"; exit 1; }
[ -f "$SCRIPT" ] || { echo "FAIL: script not found: $SCRIPT"; exit 1; }

echo "== build (seat binary + reference runner)"
cargo build -q -p idaptik-net -p idaptik-tui
SEAT=target/debug/idaptik-loopback-seat
TUI=target/debug/idaptik-tui

echo "== relay (throwaway, port ${PORT})"
(cd server && mix deps.get >/dev/null)
(cd server && IDAPTIK_PORT="$PORT" exec mix phx.server >/tmp/idaptik_loopback_relay.log 2>&1) &
RELAY_PID=$!
cleanup() {
    # mix execs the BEAM in the same process thanks to `exec`; kill the tree
    # in case the VM spawned helpers.
    kill "$RELAY_PID" 2>/dev/null || true
    pkill -P "$RELAY_PID" 2>/dev/null || true
    wait "$RELAY_PID" 2>/dev/null || true
}
trap cleanup EXIT

for _ in $(seq 1 60); do
    if curl -fsS "http://127.0.0.1:${PORT}/" >/dev/null 2>&1; then
        break
    fi
    if ! kill -0 "$RELAY_PID" 2>/dev/null; then
        echo "FAIL: relay died during startup — /tmp/idaptik_loopback_relay.log:"
        tail -20 /tmp/idaptik_loopback_relay.log
        exit 1
    fi
    sleep 1
done
curl -fsS "http://127.0.0.1:${PORT}/" >/dev/null || { echo "FAIL: relay never answered on :${PORT}"; exit 1; }

WORK="$(mktemp -d)"

echo "== run 1: determinism (two seats, one deterministic world)"
SID="loopback-$$-$RANDOM"
"$SEAT" --url "$URL" --session "$SID" --role infiltrator --script "$SCRIPT" \
    --out "$WORK/infiltrator.json" >"$WORK/infiltrator.meta" &
A=$!
"$SEAT" --url "$URL" --session "$SID" --role hacker --script "$SCRIPT" \
    --out "$WORK/hacker.json" >"$WORK/hacker.meta" &
B=$!
wait "$A" || { echo "FAIL: infiltrator seat exited non-zero"; cat "$WORK/infiltrator.meta" 2>/dev/null || true; exit 1; }
wait "$B" || { echo "FAIL: hacker seat exited non-zero"; cat "$WORK/hacker.meta" 2>/dev/null || true; exit 1; }
grep -q '"status":"completed"' "$WORK/infiltrator.meta" || { echo "FAIL: infiltrator did not complete:"; cat "$WORK/infiltrator.meta"; exit 1; }
grep -q '"status":"completed"' "$WORK/hacker.meta" || { echo "FAIL: hacker did not complete:"; cat "$WORK/hacker.meta"; exit 1; }
grep -q '"peer_digest_match":false' "$WORK/infiltrator.meta" "$WORK/hacker.meta" && { echo "FAIL: in-band digest mismatch"; exit 1; }

"$TUI" --headless --script "$SCRIPT" >"$WORK/reference.json"

cmp -s "$WORK/infiltrator.json" "$WORK/hacker.json" \
    || { echo "FAIL: the two seats observed different runs (infiltrator.json != hacker.json in $WORK)"; exit 1; }
cmp -s "$WORK/infiltrator.json" "$WORK/reference.json" \
    || { echo "FAIL: networked run differs from the headless reference (in $WORK)"; exit 1; }
echo "   both seats byte-identical, and identical to the headless reference"

echo "== run 2: connection loss (kill one seat mid-stream)"
SID="loopback-loss-$$-$RANDOM"
"$SEAT" --url "$URL" --session "$SID" --role infiltrator --script "$SCRIPT" \
    --out "$WORK/dying.json" --fail-after-seq 2 >"$WORK/dying.meta" &
A=$!
"$SEAT" --url "$URL" --session "$SID" --role hacker --script "$SCRIPT" \
    --out "$WORK/survivor.json" --grace-ms 2000 >"$WORK/survivor.meta" &
B=$!
set +e
wait "$A"; A_EXIT=$?
set -e
[ "$A_EXIT" -eq 3 ] || { echo "FAIL: dying seat should exit 3 (died on purpose), got $A_EXIT"; exit 1; }
wait "$B" || { echo "FAIL: surviving seat crashed instead of ending cleanly"; cat "$WORK/survivor.meta" 2>/dev/null || true; exit 1; }
grep -q '"status":"ended_peer_lost"' "$WORK/survivor.meta" \
    || { echo "FAIL: survivor did not take the PeerLost path:"; cat "$WORK/survivor.meta"; exit 1; }
echo "   survivor ended cleanly through PeerLost"

echo "PASS: loopback gate (determinism + loss handling) — artifacts in $WORK"
