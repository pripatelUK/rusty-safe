#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source "$ROOT_DIR/scripts/lib/prd05a_e2e_common.sh"

timestamp="$(prd05a_now_utc)"
run_id="$(prd05a_run_id)"
report_path="local/reports/prd05a/C5-manual-metamask-sanity.md"
mode="${1:-template}"

mkdir -p local/reports/prd05a

if [[ "$mode" != "template" && "$mode" != "verify" ]]; then
  echo "usage: scripts/run_prd05a_manual_metamask_checklist.sh [template|verify]" >&2
  exit 2
fi

if [[ "$mode" == "template" ]]; then
  cat >"$report_path" <<EOF
# C5 Manual MetaMask Sanity Checklist

Generated: ${timestamp}
Run ID: ${run_id}
Schema: ${PRD05A_SCHEMA_VERSION}

Operator: ${PRD05A_MANUAL_OPERATOR:-PENDING}
Environment: ${PRD05A_MANUAL_ENVIRONMENT:-Chromium}
Wallet version: ${PRD05A_MANUAL_WALLET_VERSION:-PENDING}
Browser version: ${PRD05A_MANUAL_BROWSER_VERSION:-PENDING}
App commit: ${PRD05A_MANUAL_APP_COMMIT:-PENDING}

## Required Scenarios (MANUAL-MM-001..004)

- [ ] MANUAL-MM-001 connect (eth_requestAccounts)
- [ ] MANUAL-MM-002 message sign (personal_sign)
- [ ] MANUAL-MM-003 typed data sign (eth_signTypedData_v4)
- [ ] MANUAL-MM-004 transaction send (eth_sendTransaction)

## Notes

- Any failure must include reproduction details and screenshots.
- If a scenario fails, open an issue and link it below.

Issue links:
- PENDING
EOF
  echo "wrote ${report_path}"
  exit 0
fi

if [[ ! -f "$report_path" ]]; then
  echo "manual checklist missing: ${report_path}" >&2
  exit 1
fi

if rg -n "PENDING" "$report_path" >/dev/null 2>&1; then
  echo "manual checklist still contains PENDING placeholders: ${report_path}" >&2
  exit 1
fi

required_checks=(
  "MANUAL-MM-001"
  "MANUAL-MM-002"
  "MANUAL-MM-003"
  "MANUAL-MM-004"
)

for check in "${required_checks[@]}"; do
  if ! rg -n "\\- \\[x\\] ${check}" "$report_path" >/dev/null 2>&1; then
    echo "manual checklist missing completed check: ${check}" >&2
    exit 1
  fi
done

echo "manual checklist verification: PASS"
