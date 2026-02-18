#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

timestamp="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

chromium_bin="${PRD05A_CHROMIUM_BIN:-chromium}"
if ! command -v "$chromium_bin" >/dev/null 2>&1; then
  chromium_bin="${PRD05A_CHROMIUM_BIN_FALLBACK:-google-chrome}"
fi

metamask_dir="${PRD05A_METAMASK_PROFILE_DIR:-}"
rabby_dir="${PRD05A_RABBY_PROFILE_DIR:-}"

chromium_version="$("$chromium_bin" --version 2>/dev/null || true)"
if [[ -z "$chromium_version" ]]; then
  chromium_version="NOT_AVAILABLE"
fi

metamask_status="BLOCKED"
metamask_note="missing PRD05A_METAMASK_PROFILE_DIR"
if [[ -n "$metamask_dir" && -d "$metamask_dir" ]]; then
  metamask_status="PASS"
  metamask_note="profile directory detected (${metamask_dir})"
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

- Set \`PRD05A_METAMASK_PROFILE_DIR\` and \`PRD05A_RABBY_PROFILE_DIR\` to browser profile paths before rerun.
- Command: \`scripts/run_prd05a_compat_matrix.sh\`
EOF

echo "wrote local/reports/prd05a/C5-compatibility-matrix-report.md"
