#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

timestamp="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
report_path="local/reports/prd05a/C5-metamask-e2e-report.md"
log_path="local/reports/prd05a/C5-metamask-e2e.log"

mkdir -p local/reports/prd05a

node_bin="${PRD05A_NODE_BIN:-}"
if [[ -z "$node_bin" ]]; then
  if [[ -x "$HOME/.nvm/versions/node/v20.19.6/bin/node" ]]; then
    node_bin="$HOME/.nvm/versions/node/v20.19.6/bin/node"
  elif command -v node >/dev/null 2>&1; then
    node_bin="$(command -v node)"
  fi
fi

if [[ -z "$node_bin" ]] || [[ ! -x "$node_bin" ]]; then
  cat >"$report_path" <<EOF
# C5 MetaMask E2E Report

Generated: ${timestamp}

Status: BLOCKED

Reason:
- Node.js runtime is not available. Set \`PRD05A_NODE_BIN\` to a valid Node binary.

EOF
  echo "wrote $report_path"
  exit 2
fi

if ! command -v anvil >/dev/null 2>&1; then
  cat >"$report_path" <<EOF
# C5 MetaMask E2E Report

Generated: ${timestamp}

Status: BLOCKED

Reason:
- \`anvil\` is not available in PATH.

EOF
  echo "wrote $report_path"
  exit 2
fi

if ! command -v trunk >/dev/null 2>&1; then
  cat >"$report_path" <<EOF
# C5 MetaMask E2E Report

Generated: ${timestamp}

Status: BLOCKED

Reason:
- \`trunk\` is not available in PATH.

EOF
  echo "wrote $report_path"
  exit 2
fi

pushd e2e >/dev/null

# Install deps when missing.
if [[ ! -d node_modules ]]; then
  if command -v bun >/dev/null 2>&1; then
    bun install >/dev/null
  elif command -v npm >/dev/null 2>&1; then
    npm install >/dev/null
  else
    cat >"../${report_path}" <<EOF
# C5 MetaMask E2E Report

Generated: ${timestamp}

Status: BLOCKED

Reason:
- Neither \`bun\` nor \`npm\` is available to install e2e dependencies.

EOF
    echo "wrote ../${report_path}"
    popd >/dev/null
    exit 2
  fi
fi

# Ensure Chromium runtime exists for Playwright.
"$node_bin" ./node_modules/playwright/cli.js install chromium >/dev/null

setup_force_flag="${PRD05A_METAMASK_FORCE_SETUP:-0}"
setup_cmd=("$node_bin" ./node_modules/@synthetixio/synpress/dist/cli.js wallet-setup --headless)
if [[ "$setup_force_flag" == "1" ]]; then
  setup_cmd+=(--force)
fi

set +e
(
  echo "[cache] building synpress metamask cache"
  HEADLESS=true "${setup_cmd[@]}"
  setup_rc=$?
  if [[ $setup_rc -ne 0 ]]; then
    echo "[cache] failed rc=${setup_rc}"
    exit $setup_rc
  fi

  echo "[preflight] validating cached metamask state after unlock"
  HEADLESS=true "$node_bin" ./tests/metamask/metamask-cache-preflight.mjs
  preflight_rc=$?
  if [[ $preflight_rc -ne 0 ]]; then
    echo "[preflight] failed rc=${preflight_rc}"
    exit $preflight_rc
  fi

  echo "[test] running metamask playwright suite"
  HEADLESS=true "$node_bin" ./node_modules/playwright/cli.js test -c playwright.metamask.config.ts tests/metamask/metamask-eip1193.spec.mjs --project=chromium
) >"../${log_path}" 2>&1
rc=$?
set -e

popd >/dev/null

status="FAIL"
if [[ $rc -eq 0 ]]; then
  status="PASS"
fi

cat >"$report_path" <<EOF
# C5 MetaMask E2E Report

Generated: ${timestamp}

Status: ${status}

## Scope

- Chromium + MetaMask extension runtime via Synpress.
- Cache preflight that validates post-unlock state is not onboarding.
- EIP-1193 smoke coverage:
  - \`eth_requestAccounts\`
  - \`personal_sign\`
  - \`eth_signTypedData_v4\`
  - \`eth_sendTransaction\`

## Artifacts

- Log: \`${log_path}\`
- Playwright report dir: \`e2e/playwright-report-metamask\`

## Configuration

- Wallet password env: \`PRD05A_METAMASK_PASSWORD\` (optional, default deterministic test value)
- Wallet seed env: \`PRD05A_METAMASK_SEED\` (optional, defaults to Foundry test mnemonic)
- Setup force env: \`PRD05A_METAMASK_FORCE_SETUP=1\` (optional; rebuilds Synpress cache)
- Recipient env: \`PRD05A_METAMASK_RECIPIENT\` (optional)
- Base URL env: \`PRD05A_E2E_BASE_URL\` (optional, default \`http://localhost:7272\`)
- Skip webserver env: \`PRD05A_E2E_SKIP_WEBSERVER=1\` (optional)
- Node binary env: \`PRD05A_NODE_BIN\` (optional override)

EOF

echo "wrote $report_path"

if [[ "$status" == "PASS" ]]; then
  exit 0
fi

exit 1
