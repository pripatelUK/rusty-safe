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

set +e
scripts/run_prd05a_metamask_e2e.sh
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

rabby_dir="${PRD05A_RABBY_PROFILE_DIR:-}"
rabby_status="BLOCKED"
rabby_note="missing PRD05A_RABBY_PROFILE_DIR"
if [[ -n "$rabby_dir" && -d "$rabby_dir" ]]; then
  rabby_status="PASS"
  rabby_note="profile directory detected (${rabby_dir})"
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
| Rabby | Chromium | ${rabby_status} | N/A | ${rabby_note} |

## Repro

- MetaMask gate command: \`scripts/run_prd05a_metamask_e2e.sh\`
- MetaMask reports:
  - \`local/reports/prd05a/C5-metamask-e2e-report.md\`
  - \`local/reports/prd05a/C5-metamask-e2e.json\`
- Rabby currently remains profile-based; set \`PRD05A_RABBY_PROFILE_DIR\` for matrix evidence.
- Command: \`scripts/run_prd05a_compat_matrix.sh\`
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
      "taxonomy": "N/A",
      "notes": "$(prd05a_json_escape "$rabby_note")"
    }
  ],
  "artifacts": {
    "markdown_report": "${report_path}",
    "json_report": "${json_path}",
    "metamask_json": "${metamask_json}"
  }
}
EOF

echo "wrote ${report_path}"
echo "wrote ${json_path}"
