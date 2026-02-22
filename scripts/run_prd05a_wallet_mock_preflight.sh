#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source "$ROOT_DIR/scripts/lib/prd05a_e2e_common.sh"

timestamp="$(prd05a_now_utc)"
run_id="$(prd05a_run_id)"
report_path="local/reports/prd05a/C5-e0-determinism-report.md"
json_path="local/reports/prd05a/C5-e0-determinism-report.json"
log_path="local/reports/prd05a/C5-wallet-mock-preflight.log"

driver_mode="wallet-mock"
release_gate_driver="wallet-mock"
gate_tier="${PRD05A_GATE_MODE:-blocking}"
gate_effect="BLOCKING"
expected_locale_prefix="${PRD05A_EXPECTED_LOCALE_PREFIX:-en}"
taxonomy="ENV_BLOCKER"
triage_label="triage/env"
status="BLOCKED"
reason=""
locale_value="en_US.UTF-8"

mkdir -p local/reports/prd05a

chromium_info="$(prd05a_chromium_version)"
chromium_bin="${chromium_info%%|*}"
chromium_version="${chromium_info#*|}"

write_reports() {
  local artifacts_json='{
    "log": "local/reports/prd05a/C5-wallet-mock-preflight.log",
    "markdown_report": "local/reports/prd05a/C5-e0-determinism-report.md",
    "json_report": "local/reports/prd05a/C5-e0-determinism-report.json"
  }'

  cat >"$report_path" <<EOF
# C5 E0 Deterministic Runtime Gate Report

Generated: ${timestamp}
Run ID: ${run_id}
Phase: E0
Scope: E0-T1, E0-T2, E0-T3, E0-T4

## Result

- Gate status: ${status}
- Taxonomy: ${taxonomy}
- Triage label: ${triage_label}
- Reason: ${reason}

## Determinism Assertions

- Node runtime pinned to v20.
- Locale pin + runtime profile check (expected prefix: \`${expected_locale_prefix}\`).
- Standardized run header metadata emitted in preflight log.
- Evidence JSON conforms to \`c5e2e-v1\` and references complete artifacts.

## Runtime Profile

- Driver mode: \`${driver_mode}\`
- Release-gate driver: \`${release_gate_driver}\`
- Gate tier: \`${gate_tier}\`
- Gate effect: \`${gate_effect}\`
- Node: \`${node_version}\`
- Chromium binary: \`${chromium_bin}\`
- Chromium version: \`${chromium_version}\`
- Locale: \`${locale_value}\`

## Artifacts

- Preflight log: \`${log_path}\`
- JSON report: \`${json_path}\`
- Markdown report: \`${report_path}\`
EOF

  prd05a_write_json \
    "$json_path" \
    "$PRD05A_SCHEMA_VERSION" \
    "$timestamp" \
    "$run_id" \
    "$status" \
    "$taxonomy" \
    "$driver_mode" \
    "$release_gate_driver" \
    "$node_version" \
    "$chromium_bin" \
    "$chromium_version" \
    "$locale_value" \
    "$artifacts_json" \
    "$triage_label" \
    "$reason" \
    "$gate_tier" \
    "$gate_effect"
}

node_bin="$(prd05a_resolve_node || true)"
if [[ -z "$node_bin" ]]; then
  node_version="NOT_AVAILABLE"
  reason="Node.js runtime is not available. Set PRD05A_NODE_BIN to a Node v20 binary."
  write_reports
  exit 2
fi

node_major="$(prd05a_node_major "$node_bin")"
node_version="$(prd05a_node_version "$node_bin")"
if [[ "$node_major" != "20" ]]; then
  reason="Node major version must be 20.x for deterministic C5 runs (found ${node_version})."
  write_reports
  exit 2
fi

export LANG="${PRD05A_LANG:-en_US.UTF-8}"
export LC_ALL="${PRD05A_LC_ALL:-en_US.UTF-8}"
locale_value="${LANG}"

set +e
(
  echo "[header] schema_version=${PRD05A_SCHEMA_VERSION}"
  echo "[header] run_id=${run_id}"
  echo "[header] driver_mode=${driver_mode}"
  echo "[header] release_gate_driver=${release_gate_driver}"
  echo "[header] gate_tier=${gate_tier}"
  echo "[header] gate_effect=${gate_effect}"
  echo "[header] node_version=${node_version}"
  echo "[header] chromium_bin=${chromium_bin}"
  echo "[header] chromium_version=${chromium_version}"
  echo "[header] lang=${LANG}"
  echo "[header] lc_all=${LC_ALL}"

  pushd e2e >/dev/null

  if [[ ! -d node_modules ]]; then
    npm install --silent >/dev/null
  fi

  "$node_bin" ./tests/wallet-mock/runtime-profile-check.mjs

  popd >/dev/null
) >"$log_path" 2>&1
rc=$?
set -e

if [[ $rc -ne 0 ]]; then
  status="FAIL"
  taxonomy="HARNESS_FAIL"
  triage_label="triage/harness"
  reason="wallet-mock runtime profile check failed (see preflight log)."
  write_reports
  "$node_bin" "$ROOT_DIR/e2e/tests/wallet-mock/validate-evidence-schema.mjs" "$json_path" >/dev/null
  exit 1
fi

PRD05A_PRECHECK_LOG_PATH="$log_path" \
  "$node_bin" --test "$ROOT_DIR/e2e/tests/wallet-mock/runtime-preflight.contract.test.mjs" >>"$log_path" 2>&1

status="PASS"
taxonomy="NONE"
triage_label="triage/harness"
reason="deterministic preflight checks passed"
write_reports

"$node_bin" "$ROOT_DIR/e2e/tests/wallet-mock/validate-evidence-schema.mjs" "$json_path" >>"$log_path" 2>&1

echo "wrote ${report_path}"
echo "wrote ${json_path}"
echo "wrote ${log_path}"
