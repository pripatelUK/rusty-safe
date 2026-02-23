#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source "$ROOT_DIR/scripts/lib/prd05a_e2e_common.sh"

timestamp="$(prd05a_now_utc)"
run_id="$(prd05a_run_id)"
report_path="local/reports/prd05a/C5-wallet-mock-gate-report.md"
json_path="local/reports/prd05a/C5-wallet-mock-gate.json"
log_path="local/reports/prd05a/C5-wallet-mock-gate.log"
driver_mode="wallet-mock"
release_gate_driver="wallet-mock"
gate_tier="${PRD05A_GATE_MODE:-blocking}"
gate_effect="BLOCKING"
scenario_grep="${PRD05A_SCENARIO_GREP:-}"
expected_locale_prefix="${PRD05A_EXPECTED_LOCALE_PREFIX:-en}"
e2e_base_url="${PRD05A_E2E_BASE_URL:-http://localhost:7272}"
e2e_skip_webserver="${PRD05A_E2E_SKIP_WEBSERVER:-0}"
taxonomy="ENV_BLOCKER"
status="BLOCKED"
triage_label="triage/env"
reason=""
locale_value="en_US.UTF-8"

mkdir -p local/reports/prd05a

chromium_info="$(prd05a_chromium_version)"
chromium_bin="${chromium_info%%|*}"
chromium_version="${chromium_info#*|}"

write_reports() {
  local artifacts_json='{
    "log": "local/reports/prd05a/C5-wallet-mock-gate.log",
    "markdown_report": "local/reports/prd05a/C5-wallet-mock-gate-report.md",
    "json_report": "local/reports/prd05a/C5-wallet-mock-gate.json",
    "playwright_report_dir": "e2e/playwright-report-wallet-mock"
  }'

  cat >"$report_path" <<EOF
# C5 Wallet Mock Blocking Gate Report

Generated: ${timestamp}
Run ID: ${run_id}
Schema: ${PRD05A_SCHEMA_VERSION}

Status: ${status}
Taxonomy: ${taxonomy}
Triage Label: ${triage_label}

Reason:
- ${reason}

## Runtime Profile

- Driver mode: \`${driver_mode}\`
- Release-gate driver: \`${release_gate_driver}\`
- Gate tier: \`${gate_tier}\`
- Gate effect: \`${gate_effect}\`
- Node: \`${node_version}\`
- Chromium binary: \`${chromium_bin}\`
- Chromium version: \`${chromium_version}\`
- Locale: \`${locale_value}\` (expected prefix: \`${expected_locale_prefix}\`)
- Scenario filter: \`${scenario_grep:-all}\`
- Base URL: \`${e2e_base_url}\`
- Skip webserver: \`${e2e_skip_webserver}\`

## Scope

- Blocking deterministic lane using \`@synthetixio/ethereum-wallet-mock\`.
- Contract checks:
  - \`wallet-driver.contract\`
  - \`failure-taxonomy.contract\`
- Parity scenarios:
  - \`WM-PARITY-001\` \`eth_requestAccounts\`
  - \`WM-PARITY-002\` \`personal_sign\`
  - \`WM-PARITY-003\` \`eth_signTypedData_v4\`
  - \`WM-PARITY-004\` \`eth_sendTransaction\`
  - \`WM-PARITY-005\` \`accountsChanged\` recovery
  - \`WM-PARITY-006\` \`chainChanged\` recovery
- Build/sign/share scenarios:
  - \`WM-BSS-001\` tx lifecycle intent
  - \`WM-BSS-002\` ABI selector guard
  - \`WM-BSS-003\` manual signature idempotency
  - \`WM-BSS-004\` bundle roundtrip determinism
  - \`WM-BSS-005\` URL import key compatibility
  - \`WM-BSS-006\` tampered bundle rejection

## Artifacts

- Log: \`${log_path}\`
- JSON: \`${json_path}\`
- Playwright report dir: \`e2e/playwright-report-wallet-mock\`
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

if [[ ! "$driver_mode" =~ ^wallet-mock$ ]]; then
  reason="Unsupported PRD05A_DRIVER_MODE=${driver_mode}. Expected wallet-mock."
  write_reports
  exit 2
fi

export LANG="${PRD05A_LANG:-en_US.UTF-8}"
export LC_ALL="${PRD05A_LC_ALL:-en_US.UTF-8}"
locale_value="${LANG}"

set +e
(
  set -euo pipefail
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
  echo "[header] scenario_grep=${scenario_grep:-all}"
  echo "[header] e2e_base_url=${e2e_base_url}"
  echo "[header] e2e_skip_webserver=${e2e_skip_webserver}"

  pushd e2e >/dev/null

  if [[ ! -d node_modules ]]; then
    npm install --silent >/dev/null
  fi

  "$node_bin" ./node_modules/playwright/cli.js install chromium >/dev/null

  echo "[contract] running wallet driver contract checks"
  "$node_bin" --test ./tests/wallet-mock/wallet-driver.contract.test.mjs

  echo "[contract] running failure taxonomy checks"
  "$node_bin" --test ./tests/wallet-mock/failure-taxonomy.contract.test.mjs

  echo "[profile] running wallet-mock runtime profile check"
  PRD05A_EXPECTED_LOCALE_PREFIX="${expected_locale_prefix}" \
    "$node_bin" ./tests/wallet-mock/runtime-profile-check.mjs

  echo "[test] running wallet-mock playwright suite"
  playwright_cmd=(
    "$node_bin" ./node_modules/playwright/cli.js test
    -c playwright.wallet-mock.config.ts
    tests/wallet-mock/wallet-mock-eip1193.spec.mjs
    --project=chromium
  )
  if [[ -n "$scenario_grep" ]]; then
    playwright_cmd+=(--grep "$scenario_grep")
  fi
  "${playwright_cmd[@]}"

  popd >/dev/null
) >"$log_path" 2>&1
rc=$?
set -e

if [[ $rc -eq 0 ]]; then
  status="PASS"
  taxonomy="NONE"
  triage_label="triage/harness"
  reason="wallet-mock parity gate passed"
else
  status="FAIL"
  taxonomy="APP_FAIL"
  triage_label="triage/app"
  reason="wallet-mock parity gate failed (see log)"
  if rg -q "timeout" "$log_path"; then
    taxonomy="HARNESS_FAIL"
    triage_label="triage/harness"
  fi
fi

if rg -q "already used, make sure that nothing is running on the port/url" "$log_path"; then
  status="BLOCKED"
  taxonomy="ENV_BLOCKER"
  triage_label="triage/env"
  reason="playwright webServer startup blocked by existing process on base URL"
fi

write_reports

"$node_bin" "$ROOT_DIR/e2e/tests/wallet-mock/validate-evidence-schema.mjs" "$json_path" >>"$log_path" 2>&1 || {
  status="FAIL"
  taxonomy="HARNESS_FAIL"
  triage_label="triage/harness"
  reason="schema validation failed for wallet-mock gate report"
  write_reports
  exit 1
}

if [[ "$status" == "PASS" ]]; then
  exit 0
fi
exit 1
