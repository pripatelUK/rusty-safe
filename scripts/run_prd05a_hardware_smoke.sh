#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

timestamp="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

ledger_log="${PRD05A_LEDGER_SMOKE_LOG:-}"
trezor_log="${PRD05A_TREZOR_SMOKE_LOG:-}"

ledger_status="BLOCKED"
ledger_note="missing PRD05A_LEDGER_SMOKE_LOG"
if [[ -n "$ledger_log" && -f "$ledger_log" ]]; then
  if rg -n "PASS" "$ledger_log" >/dev/null 2>&1; then
    ledger_status="PASS"
    ledger_note="PASS marker found in ${ledger_log}"
  else
    ledger_status="FAIL"
    ledger_note="log present but PASS marker missing (${ledger_log})"
  fi
fi

trezor_status="BLOCKED"
trezor_note="missing PRD05A_TREZOR_SMOKE_LOG"
if [[ -n "$trezor_log" && -f "$trezor_log" ]]; then
  if rg -n "PASS" "$trezor_log" >/dev/null 2>&1; then
    trezor_status="PASS"
    trezor_note="PASS marker found in ${trezor_log}"
  else
    trezor_status="FAIL"
    trezor_note="log present but PASS marker missing (${trezor_log})"
  fi
fi

mkdir -p local/reports/prd05a
cat > local/reports/prd05a/C5-hardware-passthrough-smoke.md <<EOF
# C5 Hardware Passthrough Smoke

Generated: ${timestamp}

| Device | Status | Notes |
|---|---|---|
| Ledger (wallet passthrough) | ${ledger_status} | ${ledger_note} |
| Trezor (wallet passthrough) | ${trezor_status} | ${trezor_note} |

## Repro

- Provide smoke logs through \`PRD05A_LEDGER_SMOKE_LOG\` and \`PRD05A_TREZOR_SMOKE_LOG\`.
- Each log must include a literal \`PASS\` marker to satisfy release evidence gate.
EOF

echo "wrote local/reports/prd05a/C5-hardware-passthrough-smoke.md"
