#!/bin/bash
# Capture screenshots of the current UI state

set -e

cd "$(dirname "$0")"

# Ensure screenshots directory exists
mkdir -p screenshots

# Run the capture script
bunx playwright test tests/capture.spec.ts --reporter=list 2>/dev/null || true

echo "Screenshots saved to e2e/screenshots/"
ls -la screenshots/*.png 2>/dev/null || echo "No screenshots found"
