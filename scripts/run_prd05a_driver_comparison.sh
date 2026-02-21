#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source "$ROOT_DIR/scripts/lib/prd05a_e2e_common.sh"

timestamp="$(prd05a_now_utc)"
run_id="$(prd05a_run_id)"
report_md="local/reports/prd05a/C5-dappwright-investigation.md"
report_json="local/reports/prd05a/C5-dappwright-investigation.json"
runs_dir="local/reports/prd05a/driver-compare/${run_id}"
timeout_secs="${PRD05A_DRIVER_COMPARE_TIMEOUT_SECS:-180}"

mkdir -p "$runs_dir"

node_bin="$(prd05a_resolve_node || true)"
if [[ -z "$node_bin" ]]; then
  echo "Node v20 is required for driver comparison reporting." >&2
  exit 2
fi

modes=("synpress" "dappwright" "mixed")
probes=("bootstrap" "connect" "network")

declare -A status_map
declare -A taxonomy_map
declare -A reason_map

run_probe() {
  local mode="$1"
  local probe="$2"
  local scenario_grep=""
  local profile_flag=""
  local skip_preflight="0"
  local slug="${mode}-${probe}"
  local log_path="${runs_dir}/${slug}.log"
  local json_copy="${runs_dir}/${slug}.json"
  local report_copy="${runs_dir}/${slug}.md"

  rm -f local/reports/prd05a/C5-metamask-e2e.json
  rm -f local/reports/prd05a/C5-metamask-e2e-report.md
  rm -f local/reports/prd05a/C5-metamask-e2e.log

  if [[ "$probe" == "bootstrap" ]]; then
    profile_flag="--profile-check"
  elif [[ "$probe" == "connect" ]]; then
    scenario_grep="MM-PARITY-001"
    skip_preflight="1"
  elif [[ "$probe" == "network" ]]; then
    scenario_grep="MM-PARITY-004"
    skip_preflight="1"
  fi

  set +e
  env \
    PRD05A_DRIVER_MODE="$mode" \
    PRD05A_RELEASE_GATE_ENFORCE=0 \
    PRD05A_SCENARIO_GREP="$scenario_grep" \
    PRD05A_SKIP_PREFLIGHT="$skip_preflight" \
    timeout "$timeout_secs" ./scripts/run_prd05a_metamask_e2e.sh $profile_flag >"$log_path" 2>&1
  local rc=$?
  set -e

  if [[ -f local/reports/prd05a/C5-metamask-e2e.json ]]; then
    cp local/reports/prd05a/C5-metamask-e2e.json "$json_copy"
  fi
  if [[ -f local/reports/prd05a/C5-metamask-e2e-report.md ]]; then
    cp local/reports/prd05a/C5-metamask-e2e-report.md "$report_copy"
  fi

  local status="FAIL"
  local taxonomy="APP_FAIL"
  local reason="probe failed"
  if [[ -f "$json_copy" ]]; then
    status="$("$node_bin" -p "require('./${json_copy}').status" 2>/dev/null || echo "FAIL")"
    taxonomy="$("$node_bin" -p "require('./${json_copy}').taxonomy" 2>/dev/null || echo "APP_FAIL")"
    reason="$("$node_bin" -p "require('./${json_copy}').reason" 2>/dev/null || echo "probe failed")"
  elif [[ $rc -eq 124 ]]; then
    status="FAIL"
    taxonomy="HARNESS_FAIL"
    reason="probe timed out after ${timeout_secs}s"
  fi

  status_map["${mode}_${probe}"]="$status"
  taxonomy_map["${mode}_${probe}"]="$taxonomy"
  reason_map["${mode}_${probe}"]="$reason"
}

for mode in "${modes[@]}"; do
  for probe in "${probes[@]}"; do
    echo "[driver-compare] mode=${mode} probe=${probe}"
    run_probe "$mode" "$probe"
  done
done

promotion_criteria="dappwright promotion requires >=95% pass in 20-run CI soak and zero HARNESS_FAIL in 2 consecutive daily runs."
fallback_policy="if dappwright fails bootstrap/connect/network probes, release-gate driver remains synpress."

cat >"$report_md" <<EOF
# C5 dappwright Investigation (Driver Arbitration Report)

Generated: ${timestamp}
Run ID: ${run_id}
Schema: ${PRD05A_SCHEMA_VERSION}

## Driver Modes

- Supported modes: \`synpress\`, \`dappwright\`, \`mixed\`
- Release-gate driver policy (current): \`synpress\`
- Promotion criteria: ${promotion_criteria}
- Fallback policy: ${fallback_policy}

## Comparative Reliability (Bootstrap / Connect / Network)

| Driver | Bootstrap | Connect | Network |
|---|---|---|---|
| synpress | ${status_map[synpress_bootstrap]} (${taxonomy_map[synpress_bootstrap]}) | ${status_map[synpress_connect]} (${taxonomy_map[synpress_connect]}) | ${status_map[synpress_network]} (${taxonomy_map[synpress_network]}) |
| dappwright | ${status_map[dappwright_bootstrap]} (${taxonomy_map[dappwright_bootstrap]}) | ${status_map[dappwright_connect]} (${taxonomy_map[dappwright_connect]}) | ${status_map[dappwright_network]} (${taxonomy_map[dappwright_network]}) |
| mixed | ${status_map[mixed_bootstrap]} (${taxonomy_map[mixed_bootstrap]}) | ${status_map[mixed_connect]} (${taxonomy_map[mixed_connect]}) | ${status_map[mixed_network]} (${taxonomy_map[mixed_network]}) |

## Probe Reasons

- synpress/bootstrap: ${reason_map[synpress_bootstrap]}
- synpress/connect: ${reason_map[synpress_connect]}
- synpress/network: ${reason_map[synpress_network]}
- dappwright/bootstrap: ${reason_map[dappwright_bootstrap]}
- dappwright/connect: ${reason_map[dappwright_connect]}
- dappwright/network: ${reason_map[dappwright_network]}
- mixed/bootstrap: ${reason_map[mixed_bootstrap]}
- mixed/connect: ${reason_map[mixed_connect]}
- mixed/network: ${reason_map[mixed_network]}

## Artifacts

- Probe run directory: \`${runs_dir}\`
- JSON report: \`${report_json}\`
- Reproducer command: \`scripts/run_prd05a_driver_comparison.sh\`
EOF

cat >"$report_json" <<EOF
{
  "schema_version": "${PRD05A_SCHEMA_VERSION}",
  "generated": "${timestamp}",
  "run_id": "${run_id}",
  "release_gate_driver": "synpress",
  "promotion_criteria": "$(prd05a_json_escape "$promotion_criteria")",
  "fallback_policy": "$(prd05a_json_escape "$fallback_policy")",
  "matrix": [
    {
      "driver": "synpress",
      "bootstrap": {"status": "${status_map[synpress_bootstrap]}", "taxonomy": "${taxonomy_map[synpress_bootstrap]}", "reason": "$(prd05a_json_escape "${reason_map[synpress_bootstrap]}")"},
      "connect": {"status": "${status_map[synpress_connect]}", "taxonomy": "${taxonomy_map[synpress_connect]}", "reason": "$(prd05a_json_escape "${reason_map[synpress_connect]}")"},
      "network": {"status": "${status_map[synpress_network]}", "taxonomy": "${taxonomy_map[synpress_network]}", "reason": "$(prd05a_json_escape "${reason_map[synpress_network]}")"}
    },
    {
      "driver": "dappwright",
      "bootstrap": {"status": "${status_map[dappwright_bootstrap]}", "taxonomy": "${taxonomy_map[dappwright_bootstrap]}", "reason": "$(prd05a_json_escape "${reason_map[dappwright_bootstrap]}")"},
      "connect": {"status": "${status_map[dappwright_connect]}", "taxonomy": "${taxonomy_map[dappwright_connect]}", "reason": "$(prd05a_json_escape "${reason_map[dappwright_connect]}")"},
      "network": {"status": "${status_map[dappwright_network]}", "taxonomy": "${taxonomy_map[dappwright_network]}", "reason": "$(prd05a_json_escape "${reason_map[dappwright_network]}")"}
    },
    {
      "driver": "mixed",
      "bootstrap": {"status": "${status_map[mixed_bootstrap]}", "taxonomy": "${taxonomy_map[mixed_bootstrap]}", "reason": "$(prd05a_json_escape "${reason_map[mixed_bootstrap]}")"},
      "connect": {"status": "${status_map[mixed_connect]}", "taxonomy": "${taxonomy_map[mixed_connect]}", "reason": "$(prd05a_json_escape "${reason_map[mixed_connect]}")"},
      "network": {"status": "${status_map[mixed_network]}", "taxonomy": "${taxonomy_map[mixed_network]}", "reason": "$(prd05a_json_escape "${reason_map[mixed_network]}")"}
    }
  ],
  "artifacts": {
    "run_dir": "${runs_dir}",
    "markdown_report": "${report_md}",
    "json_report": "${report_json}"
  }
}
EOF

echo "wrote ${report_md}"
echo "wrote ${report_json}"
