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
release_gate_enforce="${PRD05A_RELEASE_GATE_ENFORCE:-1}"
allow_dappwright_release="${PRD05A_ALLOW_DAPPWRIGHT_RELEASE:-0}"
expected_locale_prefix="${PRD05A_EXPECTED_LOCALE_PREFIX:-en}"
scenario_grep="${PRD05A_SCENARIO_GREP:-}"
skip_preflight="${PRD05A_SKIP_PREFLIGHT:-0}"
e2e_base_url="${PRD05A_E2E_BASE_URL:-http://localhost:7272}"
e2e_skip_webserver="${PRD05A_E2E_SKIP_WEBSERVER:-0}"
e2e_port="$(printf '%s' "$e2e_base_url" | sed -E 's#^[a-zA-Z]+://[^:/]+:([0-9]+).*$#\1#')"
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
triage_label="triage/env"
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
Triage Label: ${triage_label}

Reason:
- ${reason}

## Runtime Profile

- Driver mode: \`${driver_mode}\`
- Release-gate driver: \`${release_gate_driver}\`
- Release-gate policy enforced: \`${release_gate_enforce}\`
- Release-gate override: \`${allow_dappwright_release}\`
- Node: \`${node_version}\`
- Chromium binary: \`${chromium_bin}\`
- Chromium version: \`${chromium_version}\`
- Locale: \`${locale_value}\` (expected prefix: \`${expected_locale_prefix}\`)
- Headed enforcement: \`enabled\`
- xvfb wrapper: \`forced on Linux by default (disable with PRD05A_FORCE_XVFB=0)\`
- Scenario filter: \`${scenario_grep:-all}\`
- Skip preflight: \`${skip_preflight}\`

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
    "$triage_label" \
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

if [[ ! "$driver_mode" =~ ^(synpress|dappwright|mixed)$ ]]; then
  reason="Unsupported PRD05A_DRIVER_MODE=${driver_mode}. Expected synpress|dappwright|mixed."
  write_reports
  exit 2
fi

if [[ "$release_gate_enforce" == "1" && "$allow_dappwright_release" != "1" && "$driver_mode" != "$release_gate_driver" ]]; then
  reason="Release-gate driver policy violation: driver_mode=${driver_mode}; required=${release_gate_driver} until dappwright promotion criteria is met."
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

requires_anvil="1"
if [[ -n "$scenario_grep" && "$scenario_grep" != *"MM-PARITY-004"* ]]; then
  requires_anvil="0"
fi

if [[ "$profile_check_only" != "1" && "$requires_anvil" == "1" ]]; then
  if ! command -v anvil >/dev/null 2>&1; then
    reason="anvil is not available in PATH."
    write_reports
    exit 2
  fi
fi

if [[ "$profile_check_only" != "1" ]]; then
  if ! command -v trunk >/dev/null 2>&1; then
    reason="trunk is not available in PATH."
    write_reports
    exit 2
  fi
fi

export LANG="${PRD05A_LANG:-en_US.UTF-8}"
export LC_ALL="${PRD05A_LC_ALL:-en_US.UTF-8}"
# trunk expects NO_COLOR to be a bool string; some environments set "1", which crashes startup.
if [[ "${NO_COLOR:-}" == "1" ]]; then
  export NO_COLOR="true"
fi
locale_value="${LANG}"

set +e
(
  echo "[header] schema_version=${PRD05A_SCHEMA_VERSION}"
  echo "[header] run_id=${run_id}"
  echo "[header] driver_mode=${driver_mode}"
  echo "[header] release_gate_driver=${release_gate_driver}"
  echo "[header] release_gate_policy_enforced=${release_gate_enforce}"
  echo "[header] release_gate_override=${allow_dappwright_release}"
  echo "[header] node_version=${node_version}"
  echo "[header] chromium_bin=${chromium_bin}"
  echo "[header] chromium_version=${chromium_version}"
  echo "[header] lang=${LANG}"
  echo "[header] lc_all=${LC_ALL}"
  echo "[header] scenario_grep=${scenario_grep:-all}"
  echo "[header] skip_preflight=${skip_preflight}"
  echo "[header] e2e_base_url=${e2e_base_url}"
  echo "[header] e2e_skip_webserver=${e2e_skip_webserver}"

  if [[ "$e2e_skip_webserver" != "1" && "$e2e_port" =~ ^[0-9]+$ ]]; then
    if command -v lsof >/dev/null 2>&1; then
      existing_web_pids="$(lsof -tiTCP:${e2e_port} -sTCP:LISTEN || true)"
      if [[ -n "$existing_web_pids" ]]; then
        echo "[webserver] terminating stale listeners on :${e2e_port}: ${existing_web_pids//$'\n'/,}"
        kill $existing_web_pids || true
        sleep 1
      fi
    fi
  fi

  pushd e2e >/dev/null

  if [[ ! -d node_modules ]]; then
    if command -v npm >/dev/null 2>&1; then
      npm install --silent >/dev/null
    else
      echo "[setup] npm unavailable for dependency install"
      exit 20
    fi
  fi
  if [[ ! -d node_modules/@tenkeylabs/dappwright ]]; then
    if command -v npm >/dev/null 2>&1; then
      npm install --silent >/dev/null
    else
      echo "[setup] npm unavailable for dappwright install"
      exit 20
    fi
  fi

  "$node_bin" ./node_modules/playwright/cli.js install chromium >/dev/null

  echo "[contract] running wallet driver contract checks"
  "$node_bin" --test ./tests/metamask/wallet-driver.contract.test.mjs || exit 22

  echo "[contract] running failure taxonomy checks"
  "$node_bin" --test ./tests/metamask/failure-taxonomy.contract.test.mjs || exit 23

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

  if [[ "$skip_preflight" == "1" ]]; then
    echo "[preflight] skipped by PRD05A_SKIP_PREFLIGHT=1"
  else
    echo "[preflight] validating cached metamask state"
    PRD05A_EXPECTED_LOCALE_PREFIX="${expected_locale_prefix}" \
      prd05a_with_display "$node_bin" ./tests/metamask/metamask-cache-preflight.mjs || exit 32
  fi

  echo "[test] running metamask playwright suite"
  playwright_cmd=(
    "$node_bin" ./node_modules/playwright/cli.js test
    -c playwright.metamask.config.ts
    tests/metamask/metamask-eip1193.spec.mjs
    --project=chromium
  )
  if [[ -n "$scenario_grep" ]]; then
    playwright_cmd+=(--grep "$scenario_grep")
  fi
  prd05a_with_display "${playwright_cmd[@]}" || exit 33

  popd >/dev/null
) >"$log_path" 2>&1
rc=$?
set -e

if [[ $rc -eq 0 ]]; then
  status="PASS"
  taxonomy="NONE"
  triage_label="triage/pass"
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
triage_label="triage/app"
reason="MetaMask runtime gate failed. See ${log_path}."

case "$rc" in
  20|21)
    status="BLOCKED"
    taxonomy="ENV_BLOCKER"
    triage_label="triage/env"
    reason="Runtime profile prerequisites failed before wallet execution."
    ;;
  22)
    taxonomy="HARNESS_FAIL"
    triage_label="triage/harness"
    reason="Wallet driver contract tests failed."
    ;;
  23)
    taxonomy="HARNESS_FAIL"
    triage_label="triage/harness"
    reason="Failure taxonomy contract tests failed."
    ;;
  31|32)
    taxonomy="HARNESS_FAIL"
    triage_label="triage/harness"
    reason="Wallet bootstrap or preflight convergence failed."
    ;;
  33)
    taxonomy="APP_FAIL"
    triage_label="triage/app"
    reason="Parity smoke assertions failed in runtime suite."
    ;;
  *)
    taxonomy="APP_FAIL"
    triage_label="triage/app"
    ;;
esac

if rg -qi "metamask had trouble starting|background connection unresponsive" "$log_path"; then
  taxonomy="WALLET_FAIL"
  triage_label="triage/wallet"
  reason="MetaMask extension runtime became unresponsive."
fi

write_reports
exit 1
