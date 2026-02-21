#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source "$ROOT_DIR/scripts/lib/prd05a_e2e_common.sh"

timestamp="$(prd05a_now_utc)"
run_id="$(prd05a_run_id)"
report_path="local/reports/prd05a/C5-metamask-e2e-report.md"
json_path="local/reports/prd05a/C5-metamask-e2e.json"
log_path="local/reports/prd05a/C5-metamask-e2e.log"
driver_mode="${PRD05A_DRIVER_MODE:-synpress}"
release_gate_driver="synpress"
expected_locale_prefix="${PRD05A_EXPECTED_LOCALE_PREFIX:-en}"
profile_check_only="0"

if [[ "${1:-}" == "--profile-check" ]]; then
  profile_check_only="1"
fi

mkdir -p local/reports/prd05a

chromium_info="$(prd05a_chromium_version)"
chromium_bin="${chromium_info%%|*}"
chromium_version="${chromium_info#*|}"
taxonomy="ENV_BLOCKER"
status="BLOCKED"
reason=""
locale_value="en_US.UTF-8"

artifacts_json='{
    "log": "local/reports/prd05a/C5-metamask-e2e.log",
    "markdown_report": "local/reports/prd05a/C5-metamask-e2e-report.md",
    "json_report": "local/reports/prd05a/C5-metamask-e2e.json",
    "playwright_report_dir": "e2e/playwright-report-metamask"
  }'

write_reports() {
  cat >"$report_path" <<EOF
# C5 MetaMask E2E Report

Generated: ${timestamp}
Run ID: ${run_id}
Schema: ${PRD05A_SCHEMA_VERSION}

Status: ${status}
Taxonomy: ${taxonomy}

Reason:
- ${reason}

## Runtime Profile

- Driver mode: \`${driver_mode}\`
- Release-gate driver: \`${release_gate_driver}\`
- Node: \`${node_version}\`
- Chromium binary: \`${chromium_bin}\`
- Chromium version: \`${chromium_version}\`
- Locale: \`${locale_value}\` (expected prefix: \`${expected_locale_prefix}\`)
- Headed enforcement: \`enabled\`
- xvfb wrapper: \`forced on Linux by default (disable with PRD05A_FORCE_XVFB=0)\`

## Scope

- Chromium + MetaMask extension runtime via Synpress.
- Cache preflight that validates post-unlock state is not onboarding.
- EIP-1193 smoke coverage:
  - \`eth_requestAccounts\` (\`MM-PARITY-001\`)
  - \`personal_sign\` (\`MM-PARITY-002\`)
  - \`eth_signTypedData_v4\` (\`MM-PARITY-003\`)
  - \`eth_sendTransaction\` (\`MM-PARITY-004\`)

## Artifacts

- Log: \`${log_path}\`
- JSON: \`${json_path}\`
- Playwright report dir: \`e2e/playwright-report-metamask\`
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
    "$reason"
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

if [[ "${HEADLESS:-}" == "true" || "${HEADLESS:-}" == "1" ]]; then
  reason="HEADLESS mode is disallowed for extension E2E. Run headed with xvfb when DISPLAY is unavailable."
  write_reports
  exit 2
fi

if prd05a_should_use_xvfb && ! command -v xvfb-run >/dev/null 2>&1; then
  reason="xvfb-run is required for deterministic extension E2E on Linux (set PRD05A_FORCE_XVFB=0 to opt out)."
  write_reports
  exit 2
fi

if [[ "$profile_check_only" != "1" ]]; then
  if ! command -v anvil >/dev/null 2>&1; then
    reason="anvil is not available in PATH."
    write_reports
    exit 2
  fi

  if ! command -v trunk >/dev/null 2>&1; then
    reason="trunk is not available in PATH."
    write_reports
    exit 2
  fi
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
  echo "[header] node_version=${node_version}"
  echo "[header] chromium_bin=${chromium_bin}"
  echo "[header] chromium_version=${chromium_version}"
  echo "[header] lang=${LANG}"
  echo "[header] lc_all=${LC_ALL}"

  pushd e2e >/dev/null

  if [[ ! -d node_modules ]]; then
    if command -v npm >/dev/null 2>&1; then
      npm install --silent >/dev/null
    else
      echo "[setup] npm unavailable for dependency install"
      exit 20
    fi
  fi

  "$node_bin" ./node_modules/playwright/cli.js install chromium >/dev/null

  echo "[contract] running wallet driver contract checks"
  "$node_bin" --test ./tests/metamask/wallet-driver.contract.test.mjs || exit 22

  echo "[profile] running runtime profile check"
  PRD05A_EXPECTED_LOCALE_PREFIX="${expected_locale_prefix}" \
    prd05a_with_display "$node_bin" ./tests/metamask/runtime-profile-check.mjs || exit 21

  if [[ "$profile_check_only" == "1" ]]; then
    popd >/dev/null
    exit 0
  fi

  setup_force_flag="${PRD05A_METAMASK_FORCE_SETUP:-0}"
  setup_cmd=("$node_bin" ./node_modules/@synthetixio/synpress/dist/cli.js wallet-setup)
  if [[ "$setup_force_flag" == "1" ]]; then
    setup_cmd+=(--force)
  fi

  echo "[cache] building synpress metamask cache (headed)"
  prd05a_with_display "${setup_cmd[@]}" || exit 31

  echo "[preflight] validating cached metamask state"
  PRD05A_EXPECTED_LOCALE_PREFIX="${expected_locale_prefix}" \
    prd05a_with_display "$node_bin" ./tests/metamask/metamask-cache-preflight.mjs || exit 32

  echo "[test] running metamask playwright suite"
  prd05a_with_display "$node_bin" ./node_modules/playwright/cli.js test -c playwright.metamask.config.ts tests/metamask/metamask-eip1193.spec.mjs --project=chromium || exit 33

  popd >/dev/null
) >"$log_path" 2>&1
rc=$?
set -e

if [[ $rc -eq 0 ]]; then
  status="PASS"
  taxonomy="NONE"
  if [[ "$profile_check_only" == "1" ]]; then
    reason="Runtime profile check passed."
  else
    reason="C5 MetaMask runtime gate passed."
  fi
  write_reports
  exit 0
fi

status="FAIL"
taxonomy="APP_FAIL"
reason="MetaMask runtime gate failed. See ${log_path}."

case "$rc" in
  20|21)
    status="BLOCKED"
    taxonomy="ENV_BLOCKER"
    reason="Runtime profile prerequisites failed before wallet execution."
    ;;
  22)
    taxonomy="HARNESS_FAIL"
    reason="Wallet driver contract tests failed."
    ;;
  31|32)
    taxonomy="HARNESS_FAIL"
    reason="Wallet bootstrap or preflight convergence failed."
    ;;
  33)
    taxonomy="APP_FAIL"
    reason="Parity smoke assertions failed in runtime suite."
    ;;
  *)
    taxonomy="APP_FAIL"
    ;;
esac

if rg -qi "metamask had trouble starting|background connection unresponsive" "$log_path"; then
  taxonomy="WALLET_FAIL"
  reason="MetaMask extension runtime became unresponsive."
fi

write_reports
exit 1
