#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source "$ROOT_DIR/scripts/lib/prd05a_e2e_common.sh"

timestamp="$(prd05a_now_utc)"
run_id="$(prd05a_run_id)"
mode="${1:-${PRD05A_SOAK_MODE:-pr}}"
report_md="local/reports/prd05a/C5-metamask-soak-report.md"
report_json="local/reports/prd05a/C5-metamask-soak-report.json"
runs_dir="local/reports/prd05a/soak/${run_id}"

runs=5
min_passes=5
if [[ "$mode" == "daily" ]]; then
  runs=20
  min_passes=19
elif [[ "$mode" == "custom" ]]; then
  runs="${PRD05A_SOAK_RUNS:-10}"
  min_passes="${PRD05A_SOAK_MIN_PASSES:-9}"
fi

mkdir -p "$runs_dir"

node_bin="$(prd05a_resolve_node || true)"
if [[ -z "$node_bin" ]]; then
  echo "Node v20 is required for soak report parsing." >&2
  exit 2
fi

pass_count=0
fail_count=0
blocked_count=0
taxonomy_summary="{}"

for ((i=1; i<=runs; i++)); do
  echo "[soak] run ${i}/${runs}"
  set +e
  scripts/run_prd05a_metamask_e2e.sh
  rc=$?
  set -e

  if [[ -f local/reports/prd05a/C5-metamask-e2e.json ]]; then
    cp local/reports/prd05a/C5-metamask-e2e.json "${runs_dir}/run-${i}.json"
  fi
  if [[ -f local/reports/prd05a/C5-metamask-e2e-report.md ]]; then
    cp local/reports/prd05a/C5-metamask-e2e-report.md "${runs_dir}/run-${i}.md"
  fi
  if [[ -f local/reports/prd05a/C5-metamask-e2e.log ]]; then
    cp local/reports/prd05a/C5-metamask-e2e.log "${runs_dir}/run-${i}.log"
  fi

  status="$("$node_bin" -p "require('./${runs_dir}/run-${i}.json').status" 2>/dev/null || echo "FAIL")"
  case "$status" in
    PASS) pass_count=$((pass_count + 1)) ;;
    BLOCKED) blocked_count=$((blocked_count + 1)) ;;
    *) fail_count=$((fail_count + 1)) ;;
  esac
  if [[ $rc -ne 0 ]]; then
    fail_count=$fail_count
  fi
done

status="FAIL"
if [[ $pass_count -ge $min_passes ]]; then
  status="PASS"
fi

cat >"$report_md" <<EOF
# C5 MetaMask Soak Report

Generated: ${timestamp}
Run ID: ${run_id}
Schema: ${PRD05A_SCHEMA_VERSION}

Mode: ${mode}
Runs: ${runs}
Minimum passes required: ${min_passes}

## Summary

- Status: ${status}
- Pass count: ${pass_count}
- Fail count: ${fail_count}
- Blocked count: ${blocked_count}

## Cadence Contract

- PR cadence: 5-run smoke soak
- Daily cadence: 20-run scheduled soak

## Artifacts

- Run directory: \`${runs_dir}\`
- JSON summary: \`${report_json}\`
EOF

cat >"$report_json" <<EOF
{
  "schema_version": "${PRD05A_SCHEMA_VERSION}",
  "generated": "${timestamp}",
  "run_id": "${run_id}",
  "mode": "${mode}",
  "runs": ${runs},
  "min_passes": ${min_passes},
  "status": "${status}",
  "pass_count": ${pass_count},
  "fail_count": ${fail_count},
  "blocked_count": ${blocked_count},
  "taxonomy_summary": ${taxonomy_summary},
  "artifacts": {
    "run_dir": "${runs_dir}",
    "markdown_report": "${report_md}",
    "json_report": "${report_json}"
  }
}
EOF

echo "wrote ${report_md}"
echo "wrote ${report_json}"

if [[ "$status" == "PASS" ]]; then
  exit 0
fi
exit 1
