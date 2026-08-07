#!/usr/bin/env bash
# a2ml-metadata-block
# id = "idaptik-game-launcher"
# type = "launcher"
# version = "0.2.0"
# app-name = "idaptik"
# app-display = "IDApTIK"
# app-url = "http://localhost:1984"
# standards-compliance = ["hyperpolymath-launcher-v1"]
# modes = ["runtime", "integration", "meta"]
# platforms = ["linux", "windows", "macos"]
# lifecycle-phases-covered = ["LM-LA-INSTALL", "LM-LA-RUN"]
# lifecycle-phases-deferred = []
# end-metadata-block

set -euo pipefail

APP_NAME="idaptik"
VERSION="0.2.0"
BUILD_SHA=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
PLATFORM=$(uname -s | tr '[:upper:]' '[:lower:]')

# Default port for multiplayer relay
PORT="${IDAPTIK_PORT:-1984}"

show_help() {
    echo "Usage: $0 [MODE] [OPTIONS]"
    echo ""
    echo "Modes:"
    echo "  --start     Run setup, doctor, and launch the game frontend (Bevy) in LOCAL mode."
    echo "  --host      Launch the game in MULTIPLAYER HOST mode."
    echo "              Relay must be running on port ${PORT} (or set IDAPTIK_PORT env var)."
    echo "  --join <host>  Launch in MULTIPLAYER JOIN mode, connecting to <host>."
    echo "              Port defaults to ${PORT} (or set IDAPTIK_PORT)."
    echo "  --stop [TYPE]  Stop the game and/or connection."
    echo "               Types: --game (game only), --connection (connection only),"
    echo "                      --all (both, default)"
    echo "  --status    Check if the game is running and connection status."
    echo "  --auto      Alias for --start (local mode)."
    echo "  --integ     (Stub) Integrate with desktop."
    echo "  --disinteg  (Stub) Remove desktop integration."
    echo "  --version   Print version info."
    echo "  --help      Show this help."
    echo ""
    echo "Multiplayer options (use with --host or --join):"
    echo "  --role infiltrator|hacker   Choose your role (default: infiltrator for host, hacker for join)"
    echo "  --session NAME              Session ID to join/host (default: ghost-lobby)"
    echo "  --url URL                  Relay URL (default: ws://127.0.0.1:${PORT}/socket/websocket)"
    echo "  --script PATH              Script file path (default: fixtures/session_relay/versus_script.json)"
    echo "  --input-delay N            Input delay in ticks for lockstep (default: 3)"
    echo ""
    echo "Examples:"
    echo "  $0 --start                # Local single-player"
    echo "  $0 --host                 # Host multiplayer (default role: infiltrator)"
    echo "  $0 --host --role hacker   # Host as hacker"
    echo "  $0 --join 192.168.1.100   # Join a host at 192.168.1.100"
    echo "  IDAPTIK_PORT=2000 $0 --join localhost  # Use custom port"
}

# Parse mode from first argument
MODE="${1:---auto}"
shift || true

# Parse multiplayer options (for --host and --join modes)
ROLE=""
SESSION=""
RELAY_URL=""
SCRIPT=""
INPUT_DELAY=""
HOST_ARG=""

while [ $# -gt 0 ]; do
    case "$1" in
        --role)
            ROLE="--role $2"
            shift 2
            ;;
        --session)
            SESSION="--session $2"
            shift 2
            ;;
        --url)
            RELAY_URL="--url $2"
            shift 2
            ;;
        --script)
            SCRIPT="--script $2"
            shift 2
            ;;
        --input-delay)
            INPUT_DELAY="--input-delay $2"
            shift 2
            ;;
        --host)
            MODE="--host"
            shift
            ;;
        --join)
            MODE="--join"
            HOST_ARG="$2"
            shift 2
            ;;
        *)
            # Unknown option, might be a host argument for --join
            if [ "$MODE" = "--join" ] && [ -z "$HOST_ARG" ]; then
                HOST_ARG="$1"
                shift
            else
                break
            fi
            ;;
    esac
done

case "$MODE" in
    --start|--auto)
        echo "[launcher] Preparing $APP_NAME for cleanest start..."
        ~/.local/bin/mise exec -- just setup || just setup
        ~/.local/bin/mise exec -- just doctor || just doctor
        echo "[launcher] Launching game (LOCAL mode)..."
        exec ~/.local/bin/mise exec -- just run-bevy
        ;;
    --host)
        echo "[launcher] Launching game (MULTIPLAYER HOST mode)..."
        echo "[launcher] NOTE: Relay must be running on port ${PORT} (or set IDAPTIK_PORT)"
        ~/.local/bin/mise exec -- just setup || just setup
        ~/.local/bin/mise exec -- just doctor || just doctor
        exec ~/.local/bin/mise exec -- cargo run -p idaptik-bevy -- \
            --host \
            ${ROLE:---role infiltrator} \
            ${SESSION:---session ghost-lobby} \
            ${RELAY_URL} \
            ${SCRIPT} \
            ${INPUT_DELAY:---input-delay 3}
        ;;
    --join)
        HOST="${HOST_ARG:-127.0.0.1}"
        echo "[launcher] Launching game (MULTIPLAYER JOIN mode) to ${HOST}..."
        ~/.local/bin/mise exec -- just setup || just setup
        ~/.local/bin/mise exec -- just doctor || just doctor
        exec ~/.local/bin/mise exec -- cargo run -p idaptik-bevy -- \
            --join "${HOST}" \
            ${ROLE:---role hacker} \
            ${SESSION:---session ghost-lobby} \
            ${RELAY_URL} \
            ${SCRIPT} \
            ${INPUT_DELAY:---input-delay 3}
        ;;
    --stop)
        STOP_TYPE="${1:---all}"
        case "$STOP_TYPE" in
            --game)
                echo "[launcher] Stopping game process..."
                pkill -f "idaptik-bevy" 2>/dev/null || echo "Game process not running."
                ;;
            --connection)
                echo "[launcher] Stopping network connection..."
                # In multiplayer mode, connection runs in a thread within the bevy process
                # Killing the bevy process will kill the connection
                pkill -f "idaptik-bevy" 2>/dev/null || echo "No game/connection process found."
                ;;
            --all)
                echo "[launcher] Stopping game and connection..."
                pkill -f "idaptik-bevy" 2>/dev/null || echo "No game/connection process found."
                ;;
            *)
                echo "Unknown stop type: $STOP_TYPE"
                echo "Usage: $0 --stop [--game|--connection|--all]"
                exit 1
                ;;
        esac
        ;;
    --status)
        if pgrep -f "idaptik-bevy" > /dev/null; then
            echo "Status: GAME RUNNING"
            # Check if this is a multiplayer session by looking at command line
            if pgrep -f "idaptik-bevy" | xargs -I{} ps -p {} -o args= 2>/dev/null | grep -qE "(--host|--join)"; then
                echo "Mode: MULTIPLAYER"
                # Try to determine if relay is reachable
                if command -v curl >/dev/null 2>&1; then
                    if curl -fsS "http://127.0.0.1:${PORT}/" >/dev/null 2>&1; then
                        echo "Relay: RUNNING on port ${PORT}"
                    else
                        echo "Relay: NOT RESPONDING on port ${PORT}"
                    fi
                else
                    echo "Relay: (curl not available, cannot check)"
                fi
            else
                echo "Mode: LOCAL"
            fi
            exit 0
        else
            echo "Status: STOPPED"
            exit 1
        fi
        ;;
    --integ|--disinteg)
        echo "[launcher] Mode $MODE is not fully implemented for this interactive application yet."
        ;;
    --version)
        echo "$APP_NAME $VERSION ($BUILD_SHA) [$PLATFORM]"
        ;;
    --help)
        show_help
        ;;
    *)
        echo "Unknown mode: $MODE"
        show_help
        exit 1
        ;;
esac
