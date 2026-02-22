#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source "$ROOT_DIR/scripts/lib/prd05a_e2e_common.sh"

timestamp="$(prd05a_now_utc)"
run_id="$(prd05a_run_id)"
report_md="local/reports/prd05a/C5-metamask-canary-report.md"
report_json="local/reports/prd05a/C5-metamask-canary-report.json"
gate_json="local/reports/prd05a/C5-metamask-e2e.json"
gate_log="local/reports/prd05a/C5-metamask-e2e.log"
gate_md="local/reports/prd05a/C5-metamask-e2e-report.md"

mkdir -p local/reports/prd05a

set +e
PRD05A_GATE_MODE=canary \
PRD05A_RELEASE_GATE_ENFORCE=0 \
scripts/run_prd05a_metamask_e2e.sh "$@"
gate_rc=$?
set -e

status="FAIL"
taxonomy="HARNESS_FAIL"
triage_label="triage/harness"
reason="MetaMask canary did not execute."
if [[ -f "$gate_json" ]]; then
  node_bin="$(prd05a_resolve_node || true)"
  if [[ -n "$node_bin" ]]; then
    status="$("$node_bin" -p "require('./${gate_json}').status" 2>/dev/null || echo "FAIL")"
    taxonomy="$("$node_bin" -p "require('./${gate_json}').taxonomy" 2>/dev/null || echo "HARNESS_FAIL")"
    triage_label="$("$node_bin" -p "require('./${gate_json}').triage_label" 2>/dev/null || echo "triage/harness")"
    reason="$("$node_bin" -p "require('./${gate_json}').reason" 2>/dev/null || echo "MetaMask canary failed")"
  fi
fi

cat >"$report_md" <<EOF
# C5 MetaMask Canary Report

Generated: ${timestamp}
Run ID: ${run_id}
Schema: ${PRD05A_SCHEMA_VERSION}

## Canary Result

- Underlying gate exit code: ${gate_rc}
- Status: ${status}
- Taxonomy: ${taxonomy}
- Triage label: ${triage_label}
- Reason: ${reason}

## Policy

- Canary lane is non-blocking for C5 release decisions.
- Any \`APP_FAIL\` or repeated \`WALLET_FAIL\` must open/refresh a tracking issue.
- If the same failure reproduces in blocking lane or manual sanity, escalate to release blocker.

## Artifacts

- Gate markdown: \`${gate_md}\`
- Gate JSON: \`${gate_json}\`
- Gate log: \`${gate_log}\`
EOF

cat >"$report_json" <<EOF
{
  "schema_version": "${PRD05A_SCHEMA_VERSION}",
  "generated": "${timestamp}",
  "run_id": "${run_id}",
  "gate_tier": "canary",
  "gate_effect": "CANARY",
  "status": "${status}",
  "taxonomy": "${taxonomy}",
  "triage_label": "${triage_label}",
  "reason": "${reason}",
  "underlying_exit_code": ${gate_rc},
  "artifacts": {
    "canary_markdown": "${report_md}",
    "canary_json": "${report_json}",
    "gate_markdown": "${gate_md}",
    "gate_json": "${gate_json}",
    "gate_log": "${gate_log}"
  }
}
EOF

echo "wrote ${report_md}"
echo "wrote ${report_json}"

if [[ "${PRD05A_CANARY_STRICT:-0}" == "1" && "$status" != "PASS" ]]; then
  exit 1
fi

exit 0
