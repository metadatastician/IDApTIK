#!/usr/bin/env bash
# The ADR-0006 §4 loopback gate, PlainWebSocketTransport configuration.
#
# Seat processes on one host, a throwaway local relay, shared fixture scripts.
# PASS requires:
#
#   1. determinism  — both batch seats' artifacts are byte-identical, AND
#                     identical to `idaptik-tui --headless` on the same script
#                     (the network layer added and lost nothing);
#   2. loss handling — killing one batch seat mid-stream drives the other
#                     through PeerLost to a clean end (exit 0, status
#                     ended_peer_lost), not a crash or a hang;
#   3. live determinism — two *live* seats (delay-lockstep, real-time pacing,
#                     net:commit watermarks, a mid-run pause window) produce
#                     artifacts byte-identical to the headless reference;
#   4. resync       — killing a live seat mid-run (inside the pause window),
#                     then rejoining it, hands over the survivor's
#                     RuntimeSnapshot; BOTH seats still end byte-identical to
#                     the reference — the rejoined process reconstructs the
#                     whole run it half-missed.
#
# Requirements are hard: a missing toolchain FAILS the gate (estate doctrine —
# a gate that skips is a gate that lies).
set -euo pipefail
cd "$(dirname "$0")/.."

SCRIPT="${1:-fixtures/session_relay/capture_script.json}"
LIVE_SCRIPT="${2:-fixtures/session_relay/live_script.json}"
PORT="${IDAPTIK_LOOPBACK_PORT:-4013}"
URL="ws://127.0.0.1:${PORT}/socket/websocket"

command -v mix >/dev/null 2>&1 || { echo "FAIL: mix (Elixir) is required — run 'just setup'"; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "FAIL: cargo (Rust) is required"; exit 1; }
[ -f "$SCRIPT" ] || { echo "FAIL: script not found: $SCRIPT"; exit 1; }
[ -f "$LIVE_SCRIPT" ] || { echo "FAIL: live script not found: $LIVE_SCRIPT"; exit 1; }

echo "== build (seat binaries + reference runner)"
cargo build -q -p idaptik-net -p idaptik-tui
SEAT=target/debug/idaptik-loopback-seat
NETPLAY=target/debug/idaptik-netplay
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

"$TUI" --headless --script "$LIVE_SCRIPT" >"$WORK/live_reference.json"

echo "== run 3: live determinism (delay-lockstep seats, pause window, 2 ms pacing)"
SID="live-$$-$RANDOM"
"$NETPLAY" --url "$URL" --session "$SID" --role infiltrator --script "$LIVE_SCRIPT" \
    --tick-ms 2 --input-delay 2 --out "$WORK/live_infiltrator.json" >"$WORK/live_infiltrator.meta" 2>/dev/null &
A=$!
"$NETPLAY" --url "$URL" --session "$SID" --role hacker --script "$LIVE_SCRIPT" \
    --tick-ms 2 --input-delay 5 --out "$WORK/live_hacker.json" >"$WORK/live_hacker.meta" 2>/dev/null &
B=$!
wait "$A" || { echo "FAIL: live infiltrator exited non-zero"; cat "$WORK/live_infiltrator.meta" 2>/dev/null || true; exit 1; }
wait "$B" || { echo "FAIL: live hacker exited non-zero"; cat "$WORK/live_hacker.meta" 2>/dev/null || true; exit 1; }
grep -q '"status":"completed"' "$WORK/live_infiltrator.meta" || { echo "FAIL: live infiltrator did not complete:"; cat "$WORK/live_infiltrator.meta"; exit 1; }
grep -q '"status":"completed"' "$WORK/live_hacker.meta" || { echo "FAIL: live hacker did not complete:"; cat "$WORK/live_hacker.meta"; exit 1; }
grep -q '"peer_digest_match":false' "$WORK/live_infiltrator.meta" "$WORK/live_hacker.meta" && { echo "FAIL: live in-band digest mismatch"; exit 1; }
cmp -s "$WORK/live_infiltrator.json" "$WORK/live_reference.json" \
    || { echo "FAIL: live infiltrator differs from the headless reference (in $WORK)"; exit 1; }
cmp -s "$WORK/live_hacker.json" "$WORK/live_reference.json" \
    || { echo "FAIL: live hacker differs from the headless reference (in $WORK)"; exit 1; }
echo "   both live seats byte-identical to the headless reference (mismatched input delays included)"

echo "== run 4: live resync (die inside the pause window, rejoin, snapshot hand-off)"
SID="live-resync-$$-$RANDOM"
"$NETPLAY" --url "$URL" --session "$SID" --role infiltrator --script "$LIVE_SCRIPT" \
    --tick-ms 2 --input-delay 3 --grace-ms 3000 --rejoin-window-ms 20000 \
    --out "$WORK/live_survivor.json" >"$WORK/live_survivor.meta" 2>/dev/null &
A=$!
"$NETPLAY" --url "$URL" --session "$SID" --role hacker --script "$LIVE_SCRIPT" \
    --tick-ms 2 --input-delay 3 --die-at-step 8 >"$WORK/live_dying.meta" 2>/dev/null &
B=$!
set +e
wait "$B"; B_EXIT=$?
set -e
[ "$B_EXIT" -eq 3 ] || { echo "FAIL: dying live seat should exit 3 (died on purpose), got $B_EXIT"; exit 1; }
"$NETPLAY" --url "$URL" --session "$SID" --role hacker --script "$LIVE_SCRIPT" \
    --tick-ms 2 --input-delay 3 --rejoin --join-timeout-ms 20000 \
    --out "$WORK/live_rejoined.json" >"$WORK/live_rejoined.meta" 2>/dev/null &
B=$!
wait "$A" || { echo "FAIL: live survivor exited non-zero"; cat "$WORK/live_survivor.meta" 2>/dev/null || true; exit 1; }
wait "$B" || { echo "FAIL: rejoined seat exited non-zero"; cat "$WORK/live_rejoined.meta" 2>/dev/null || true; exit 1; }
grep -q '"status":"completed"' "$WORK/live_survivor.meta" || { echo "FAIL: live survivor did not complete:"; cat "$WORK/live_survivor.meta"; exit 1; }
grep -q '"status":"completed"' "$WORK/live_rejoined.meta" || { echo "FAIL: rejoined seat did not complete:"; cat "$WORK/live_rejoined.meta"; exit 1; }
grep -q '"peer_digest_match":false' "$WORK/live_survivor.meta" "$WORK/live_rejoined.meta" && { echo "FAIL: resync in-band digest mismatch"; exit 1; }
cmp -s "$WORK/live_survivor.json" "$WORK/live_reference.json" \
    || { echo "FAIL: survivor differs from the headless reference after resync (in $WORK)"; exit 1; }
cmp -s "$WORK/live_rejoined.json" "$WORK/live_reference.json" \
    || { echo "FAIL: rejoined seat did not reconstruct the reference run (in $WORK)"; exit 1; }
echo "   death + rejoin + snapshot resync: both seats byte-identical to the headless reference"

echo "PASS: loopback gate (determinism + loss + live lockstep + resync) — artifacts in $WORK"
