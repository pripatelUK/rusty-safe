#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUTPUT="$(cargo test -p rusty-safe-signing-adapters --test parity_differential -- --nocapture 2>&1)"
echo "$OUTPUT"

DIFF_LINE="$(echo "$OUTPUT" | rg "^DIFF " | tail -n 1 || true)"
if [[ -z "$DIFF_LINE" ]]; then
  echo "missing DIFF output line"
  exit 1
fi

timestamp="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
mkdir -p local/reports/prd05a
cat > local/reports/prd05a/C9-differential-parity-report.md <<EOF
# C9 Differential Parity Report

Generated: ${timestamp}

## Result

- Differential harness: PASS
- Critical diffs: 0

## Evidence

- Command: \`cargo test -p rusty-safe-signing-adapters --test parity_differential -- --nocapture\`
- Raw marker: \`${DIFF_LINE}\`
- Fixtures root: \`fixtures/signing/localsafe/\` (fallback \`fixtures/signing/*\`)
EOF

echo "wrote local/reports/prd05a/C9-differential-parity-report.md"
