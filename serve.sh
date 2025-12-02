#!/bin/bash
# Rusty-Safe - Serve script
# Builds and serves the WASM app using trunk

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$SCRIPT_DIR/crates/rusty-safe"

PORT="${PORT:-8080}"
HOST="${HOST:-0.0.0.0}"

cd "$APP_DIR"

echo "🔐 Starting Rusty-Safe..."
echo "   Address: http://$HOST:$PORT"
echo ""

exec trunk serve --address "$HOST" --port "$PORT" "$@"

