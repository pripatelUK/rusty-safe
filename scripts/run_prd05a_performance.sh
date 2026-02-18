#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUTPUT="$(cargo test -p rusty-safe-signing-adapters --test performance_budget -- --nocapture 2>&1)"
echo "$OUTPUT"

PERF_LINE="$(echo "$OUTPUT" | rg "^PERF " | tail -n 1 || true)"
if [[ -z "$PERF_LINE" ]]; then
  echo "missing PERF output line"
  exit 1
fi

command_p95="$(echo "$PERF_LINE" | sed -n 's/.*command_p95_ms=\([0-9]\+\).*/\1/p')"
rehydration_p95="$(echo "$PERF_LINE" | sed -n 's/.*rehydration_p95_ms=\([0-9]\+\).*/\1/p')"
command_budget="$(echo "$PERF_LINE" | sed -n 's/.*budget_command_ms=\([0-9]\+\).*/\1/p')"
rehydration_budget="$(echo "$PERF_LINE" | sed -n 's/.*budget_rehydration_ms=\([0-9]\+\).*/\1/p')"
timestamp="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

mkdir -p local/reports/prd05a
cat > local/reports/prd05a/C6-performance-report.md <<EOF
# C6 Performance Report

Generated: ${timestamp}

## Result

- Command p95: ${command_p95}ms (budget ${command_budget}ms)
- Rehydration p95: ${rehydration_p95}ms (budget ${rehydration_budget}ms)

## Evidence

- Command: \`cargo test -p rusty-safe-signing-adapters --test performance_budget -- --nocapture\`
- Raw marker: \`${PERF_LINE}\`
EOF

echo "wrote local/reports/prd05a/C6-performance-report.md"
