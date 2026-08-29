#!/usr/bin/env bash
# The ADR-0006 §4 loopback gate, PlainWebSocketTransport over burble game-session fabric.
#
# Seat processes on one host, a throwaway local burble server, shared fixture scripts.
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
# Requirements are hard: a missing toolchain or burble repo FAILS the gate (estate doctrine —
# a gate that skips is a gate that lies).
set -euo pipefail
cd "$(dirname "$0")/.."

SCRIPT="${1:-fixtures/session_relay/capture_script.json}"
LIVE_SCRIPT="${2:-fixtures/session_relay/live_script.json}"
SUPERVISED_SCRIPT="${3:-fixtures/session_relay/supervised_script.json}"
# Burble's test environment is deliberately offline-capable (no VeriSimDB)
# and binds its isolated endpoint on 4002. The development environment is not
# suitable for this gate: it requires a live VeriSimDB before the endpoint can
# start. Hosted runners are isolated, so the fixed test port cannot collide
# across jobs.
PORT=4002
URL="ws://127.0.0.1:${PORT}/voice/websocket?guest=true&display_name=loopback"
BURBLE_DIR="${IDAPTIK_BURBLE_DIR:-../burble}"

command -v mix >/dev/null 2>&1 || { echo "FAIL: mix (Elixir) is required — run 'just setup'"; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "FAIL: cargo (Rust) is required"; exit 1; }
[ -f "$SCRIPT" ] || { echo "FAIL: script not found: $SCRIPT"; exit 1; }
[ -f "$LIVE_SCRIPT" ] || { echo "FAIL: live script not found: $LIVE_SCRIPT"; exit 1; }
[ -f "$BURBLE_DIR/server/mix.exs" ] || { echo "FAIL: burble not found at $BURBLE_DIR — clone it first"; exit 1; }

echo "== build (seat binaries + reference runner)"
cargo build -q -p idaptik-net -p idaptik-tui
SEAT=target/debug/idaptik-loopback-seat
NETPLAY=target/debug/idaptik-netplay
TUI=target/debug/idaptik-tui

echo "== burble server (throwaway, port ${PORT})"
# Compile before starting the readiness clock. A cold hosted runner can spend
# more than a minute compiling Burble's dependencies; counting that as server
# startup previously killed a healthy first build before it could bind.
(cd "$BURBLE_DIR/server" && env MIX_ENV=test mix deps.get && env MIX_ENV=test mix compile)
(cd "$BURBLE_DIR/server" && MIX_ENV=test PHX_SERVER=true exec mix phx.server >/tmp/idaptik_loopback_relay.log 2>&1) &
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

echo "== run 1b: supervised determinism (two seats, in-sim supervision on)"
SID="loopback-supervised-$$-$RANDOM"
"$SEAT" --url "$URL" --session "$SID" --role infiltrator --script "$SUPERVISED_SCRIPT" \
    --out "$WORK/sup_infiltrator.json" >"$WORK/sup_infiltrator.meta" &
A=$!
"$SEAT" --url "$URL" --session "$SID" --role hacker --script "$SUPERVISED_SCRIPT" \
    --out "$WORK/sup_hacker.json" >"$WORK/sup_hacker.meta" &
B=$!
wait "$A" || { echo "FAIL: supervised infiltrator seat exited non-zero"; cat "$WORK/sup_infiltrator.meta" 2>/dev/null || true; exit 1; }
wait "$B" || { echo "FAIL: supervised hacker seat exited non-zero"; cat "$WORK/sup_hacker.meta" 2>/dev/null || true; exit 1; }
"$TUI" --headless --script "$SUPERVISED_SCRIPT" >"$WORK/sup_reference.json"
cmp -s "$WORK/sup_infiltrator.json" "$WORK/sup_hacker.json" \
    || { echo "FAIL: supervised seats observed different runs (in $WORK)"; exit 1; }
cmp -s "$WORK/sup_infiltrator.json" "$WORK/sup_reference.json" \
    || { echo "FAIL: supervised networked run differs from the headless reference (in $WORK)"; exit 1; }
grep -q '"TeamAttentionAllocated"' "$WORK/sup_reference.json" \
    || { echo "FAIL: supervised reference carries no supervision events — the leg is not exercising the slice"; exit 1; }
echo "   supervised seats byte-identical, identical to reference, supervision events present"

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
