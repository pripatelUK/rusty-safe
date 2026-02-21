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
profile_only="${PRD05A_SOAK_PROFILE_ONLY:-0}"

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
declare -A taxonomy_counts

for ((i=1; i<=runs; i++)); do
  echo "[soak] run ${i}/${runs}"
  set +e
  if [[ "$profile_only" == "1" ]]; then
    scripts/run_prd05a_metamask_e2e.sh --profile-check
  else
    scripts/run_prd05a_metamask_e2e.sh
  fi
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
# C5 MetaMask Soak Report

Generated: ${timestamp}
Run ID: ${run_id}
Schema: ${PRD05A_SCHEMA_VERSION}

Mode: ${mode}
Runs: ${runs}
Minimum passes required: ${min_passes}
Profile-only: ${profile_only}

## Summary

- Status: ${status}
- Pass count: ${pass_count}
- Fail count: ${fail_count}
- Blocked count: ${blocked_count}
- Pass rate: ${pass_rate_pct}%

## Cadence Contract

- PR cadence: 5-run smoke soak
- Daily cadence: 20-run scheduled soak
- SLO thresholds:
  - local target: >= 90% over 10 runs (reference)
  - PR target: 5/5 pass
  - daily target: 19/20 pass

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
  "profile_only": ${profile_only},
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
