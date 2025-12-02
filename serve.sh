#!/bin/bash
# Rusty-Safe - Serve script
# Builds and serves the WASM app using trunk

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$SCRIPT_DIR/crates/rusty-safe"

PORT="${PORT:-8080}"
HOST="${HOST:-0.0.0.0}"
DEBUG="${DEBUG:-1}"

# trunk doesn't like NO_COLOR
unset NO_COLOR

cd "$APP_DIR"

echo "🔐 Starting Rusty-Safe..."
echo "   Address: http://$HOST:$PORT"
if [ "$DEBUG" = "1" ]; then
    echo "   Debug: enabled (check browser console for logs)"
fi
echo ""

# Kill any existing trunk serve
pkill -f "trunk serve" 2>/dev/null || true
sleep 1

exec trunk serve --address "$HOST" --port "$PORT" "$@"
