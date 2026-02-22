#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source "$ROOT_DIR/scripts/lib/prd05a_e2e_common.sh"

timestamp="$(prd05a_now_utc)"
run_id="$(prd05a_run_id)"
mode="${1:-${PRD05A_SOAK_MODE:-pr}}"
report_md="local/reports/prd05a/C5-wallet-mock-soak-report.md"
report_json="local/reports/prd05a/C5-wallet-mock-soak-report.json"
runs_dir="local/reports/prd05a/soak-wallet-mock/${run_id}"

runs=5
min_passes=5
if [[ "$mode" == "daily" ]]; then
  runs=50
  min_passes=49
elif [[ "$mode" == "custom" ]]; then
  runs="${PRD05A_SOAK_RUNS:-20}"
  min_passes="${PRD05A_SOAK_MIN_PASSES:-19}"
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
declare -A taxonomy_counts

for ((i=1; i<=runs; i++)); do
  echo "[soak-wallet-mock] run ${i}/${runs}"
  set +e
  scripts/run_prd05a_wallet_mock_gate.sh
  rc=$?
  set -e

  if [[ -f local/reports/prd05a/C5-wallet-mock-gate.json ]]; then
    cp local/reports/prd05a/C5-wallet-mock-gate.json "${runs_dir}/run-${i}.json"
  fi
  if [[ -f local/reports/prd05a/C5-wallet-mock-gate-report.md ]]; then
    cp local/reports/prd05a/C5-wallet-mock-gate-report.md "${runs_dir}/run-${i}.md"
  fi
  if [[ -f local/reports/prd05a/C5-wallet-mock-gate.log ]]; then
    cp local/reports/prd05a/C5-wallet-mock-gate.log "${runs_dir}/run-${i}.log"
  fi

  status="$("$node_bin" -p "require('./${runs_dir}/run-${i}.json').status" 2>/dev/null || echo "FAIL")"
  taxonomy="$("$node_bin" -p "require('./${runs_dir}/run-${i}.json').taxonomy" 2>/dev/null || echo "APP_FAIL")"
  taxonomy_counts["$taxonomy"]=$(( ${taxonomy_counts["$taxonomy"]:-0} + 1 ))

  case "$status" in
    PASS) pass_count=$((pass_count + 1)) ;;
    BLOCKED) blocked_count=$((blocked_count + 1)) ;;
    *) fail_count=$((fail_count + 1)) ;;
  esac

  if [[ $rc -ne 0 && "$status" == "PASS" ]]; then
    fail_count=$((fail_count + 1))
    pass_count=$((pass_count - 1))
  fi
done

status="FAIL"
if [[ $pass_count -ge $min_passes ]]; then
  status="PASS"
fi

taxonomy_summary="{"
for key in "${!taxonomy_counts[@]}"; do
  taxonomy_summary="${taxonomy_summary}\"$(prd05a_json_escape "$key")\": ${taxonomy_counts[$key]},"
done
taxonomy_summary="${taxonomy_summary%,}}"
if [[ "$taxonomy_summary" == "{" ]]; then
  taxonomy_summary="{}"
fi

pass_rate_pct="$(awk -v p="$pass_count" -v r="$runs" 'BEGIN { if (r == 0) { print "0.00" } else { printf "%.2f", (p*100)/r } }')"

cat >"$report_md" <<EOF
# C5 Wallet Mock Soak Report

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
- Pass rate: ${pass_rate_pct}%

## Cadence Contract

- PR cadence: 5-run smoke soak
- Daily cadence: 50-run scheduled soak
- SLO thresholds:
  - local target: >= 95% over 20 runs
  - PR target: 5/5 pass
  - daily target: 49/50 pass

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
  "pass_rate_pct": ${pass_rate_pct},
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
