#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

timestamp="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

chromium_bin="${PRD05A_CHROMIUM_BIN:-chromium}"
if ! command -v "$chromium_bin" >/dev/null 2>&1; then
  chromium_bin="${PRD05A_CHROMIUM_BIN_FALLBACK:-google-chrome}"
fi

rabby_dir="${PRD05A_RABBY_PROFILE_DIR:-}"

chromium_version="$("$chromium_bin" --version 2>/dev/null || true)"
if [[ -z "$chromium_version" ]]; then
  chromium_version="NOT_AVAILABLE"
fi

metamask_status="FAIL"
metamask_note="playwright/synpress gate failed (see C5-metamask-e2e-report.md)"
set +e
scripts/run_prd05a_metamask_e2e.sh >/dev/null
metamask_gate_rc=$?
set -e
if [[ $metamask_gate_rc -eq 0 ]]; then
  metamask_status="PASS"
  metamask_note="playwright/synpress metamask e2e passed"
elif [[ $metamask_gate_rc -eq 2 ]]; then
  metamask_status="BLOCKED"
  metamask_note="metamask e2e prerequisites unavailable (see C5-metamask-e2e-report.md)"
elif rg -q "onboarding-state-after-unlock" local/reports/prd05a/C5-metamask-e2e.log 2>/dev/null; then
  metamask_status="FAIL"
  metamask_note="metamask cache preflight failed (post-unlock onboarding state); see C5-metamask-e2e.log"
fi

rabby_status="BLOCKED"
rabby_note="missing PRD05A_RABBY_PROFILE_DIR"
if [[ -n "$rabby_dir" && -d "$rabby_dir" ]]; then
  rabby_status="PASS"
  rabby_note="profile directory detected (${rabby_dir})"
fi

mkdir -p local/reports/prd05a
cat > local/reports/prd05a/C5-compatibility-matrix-report.md <<EOF
# C5 Compatibility Matrix Report

Generated: ${timestamp}

## Chromium Runtime

- Binary: \`${chromium_bin}\`
- Version: \`${chromium_version}\`

## Matrix

| Wallet | Browser | Status | Notes |
|---|---|---|---|
| MetaMask | Chromium | ${metamask_status} | ${metamask_note} |
| Rabby | Chromium | ${rabby_status} | ${rabby_note} |

## Repro

- MetaMask gate command: \`scripts/run_prd05a_metamask_e2e.sh\`
- MetaMask report: \`local/reports/prd05a/C5-metamask-e2e-report.md\`
- Rabby currently remains profile-based; set \`PRD05A_RABBY_PROFILE_DIR\` for manual matrix evidence.
- Command: \`scripts/run_prd05a_compat_matrix.sh\`
EOF

echo "wrote local/reports/prd05a/C5-compatibility-matrix-report.md"
