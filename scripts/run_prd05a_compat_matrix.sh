#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source "$ROOT_DIR/scripts/lib/prd05a_e2e_common.sh"

timestamp="$(prd05a_now_utc)"
run_id="$(prd05a_run_id)"
report_path="local/reports/prd05a/C5-compatibility-matrix-report.md"
json_path="local/reports/prd05a/C5-compatibility-matrix-report.json"

mkdir -p local/reports/prd05a

node_bin="$(prd05a_resolve_node || true)"
if [[ -z "$node_bin" ]]; then
  echo "Node v20 is required to parse matrix JSON outputs" >&2
  exit 2
fi

chromium_info="$(prd05a_chromium_version)"
chromium_bin="${chromium_info%%|*}"
chromium_version="${chromium_info#*|}"
metamask_timeout_secs="${PRD05A_MATRIX_METAMASK_TIMEOUT_SECS:-240}"
metamask_profile_only="${PRD05A_MATRIX_METAMASK_PROFILE_ONLY:-1}"

set +e
if [[ "$metamask_profile_only" == "1" ]]; then
  timeout "$metamask_timeout_secs" scripts/run_prd05a_metamask_e2e.sh --profile-check
else
  timeout "$metamask_timeout_secs" scripts/run_prd05a_metamask_e2e.sh
fi
metamask_rc=$?
set -e

metamask_json="local/reports/prd05a/C5-metamask-e2e.json"
metamask_status="BLOCKED"
metamask_taxonomy="ENV_BLOCKER"
metamask_note="missing metamask json report"
if [[ -f "$metamask_json" ]]; then
  metamask_status="$("$node_bin" -p "require('./${metamask_json}').status" 2>/dev/null || echo "FAIL")"
  metamask_taxonomy="$("$node_bin" -p "require('./${metamask_json}').taxonomy" 2>/dev/null || echo "APP_FAIL")"
  metamask_note="$("$node_bin" -p "require('./${metamask_json}').reason" 2>/dev/null || echo "metamask gate failed")"
fi
if [[ $metamask_rc -eq 0 ]]; then
  metamask_status="PASS"
fi

set +e
scripts/run_prd05a_rabby_matrix.sh
rabby_rc=$?
set -e

rabby_json="local/reports/prd05a/C5-rabby-runtime-report.json"
rabby_status="BLOCKED"
rabby_taxonomy="ENV_BLOCKER"
rabby_note="missing rabby json report"
if [[ -f "$rabby_json" ]]; then
  rabby_status="$("$node_bin" -p "require('./${rabby_json}').status" 2>/dev/null || echo "BLOCKED")"
  rabby_taxonomy="$("$node_bin" -p "require('./${rabby_json}').taxonomy" 2>/dev/null || echo "ENV_BLOCKER")"
  rabby_note="$("$node_bin" -p "require('./${rabby_json}').reason" 2>/dev/null || echo "rabby probe failed")"
fi
if [[ $rabby_rc -eq 0 ]]; then
  rabby_status="PASS"
fi

cat >"$report_path" <<EOF
# C5 Compatibility Matrix Report

Generated: ${timestamp}
Run ID: ${run_id}
Schema: ${PRD05A_SCHEMA_VERSION}

## Chromium Runtime

- Binary: \`${chromium_bin}\`
- Version: \`${chromium_version}\`

## Matrix

| Wallet | Browser | Status | Taxonomy | Notes |
|---|---|---|---|---|
| MetaMask | Chromium | ${metamask_status} | ${metamask_taxonomy} | ${metamask_note} |
| Rabby | Chromium | ${rabby_status} | ${rabby_taxonomy} | ${rabby_note} |

## Repro

- MetaMask gate command: \`scripts/run_prd05a_metamask_e2e.sh\`
- MetaMask reports:
  - \`local/reports/prd05a/C5-metamask-e2e-report.md\`
  - \`local/reports/prd05a/C5-metamask-e2e.json\`
- MetaMask runtime mode in matrix: \`profile-only=${metamask_profile_only}\`, timeout=\`${metamask_timeout_secs}s\`
- Rabby gate command: \`scripts/run_prd05a_rabby_matrix.sh\`
- Rabby reports:
  - \`local/reports/prd05a/C5-rabby-runtime-report.md\`
  - \`local/reports/prd05a/C5-rabby-runtime-report.json\`
- Command: \`scripts/run_prd05a_compat_matrix.sh\`

## Deferred Hardware Track (H1, Non-blocking for C5 Hot-wallet Release)

- Owner: Security lead
- Target: E5 gate date + 14 calendar days
- Status: deferred, non-blocking
EOF

cat >"$json_path" <<EOF
{
  "schema_version": "${PRD05A_SCHEMA_VERSION}",
  "generated": "${timestamp}",
  "run_id": "${run_id}",
  "chromium_bin": "$(prd05a_json_escape "$chromium_bin")",
  "chromium_version": "$(prd05a_json_escape "$chromium_version")",
  "matrix": [
    {
      "wallet": "MetaMask",
      "browser": "Chromium",
      "status": "${metamask_status}",
      "taxonomy": "${metamask_taxonomy}",
      "notes": "$(prd05a_json_escape "$metamask_note")"
    },
    {
      "wallet": "Rabby",
      "browser": "Chromium",
      "status": "${rabby_status}",
      "taxonomy": "${rabby_taxonomy}",
      "notes": "$(prd05a_json_escape "$rabby_note")"
    }
  ],
  "artifacts": {
    "markdown_report": "${report_path}",
    "json_report": "${json_path}",
    "metamask_json": "${metamask_json}",
    "rabby_json": "${rabby_json}"
  },
  "deferred_hardware_track": {
    "id": "H1",
    "owner": "Security lead",
    "target": "E5 gate date + 14 calendar days",
    "blocking": false
  }
}
EOF

echo "wrote ${report_path}"
echo "wrote ${json_path}"
