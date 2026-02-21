#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source "$ROOT_DIR/scripts/lib/prd05a_e2e_common.sh"

timestamp="$(prd05a_now_utc)"
run_id="$(prd05a_run_id)"
report_md="local/reports/prd05a/C5-rabby-runtime-report.md"
report_json="local/reports/prd05a/C5-rabby-runtime-report.json"
profile_dir="${PRD05A_RABBY_PROFILE_DIR:-}"
probe_cmd="${PRD05A_RABBY_E2E_COMMAND:-}"

mkdir -p local/reports/prd05a

status="BLOCKED"
taxonomy="ENV_BLOCKER"
reason="missing PRD05A_RABBY_PROFILE_DIR"
probe_rc=0

if [[ -n "$profile_dir" && -d "$profile_dir" ]]; then
  status="PASS"
  taxonomy="NONE"
  reason="profile directory detected (${profile_dir})"
fi

if [[ -n "$probe_cmd" ]]; then
  set +e
  bash -lc "$probe_cmd" >/tmp/prd05a-rabby-probe.log 2>&1
  probe_rc=$?
  set -e
  if [[ $probe_rc -ne 0 ]]; then
    status="FAIL"
    taxonomy="HARNESS_FAIL"
    reason="rabby probe command failed (rc=${probe_rc})"
  fi
fi

cat >"$report_md" <<EOF
# C5 Rabby Runtime Report

Generated: ${timestamp}
Run ID: ${run_id}
Schema: ${PRD05A_SCHEMA_VERSION}

Status: ${status}
Taxonomy: ${taxonomy}

Reason:
- ${reason}

## Inputs

- Profile dir: \`${profile_dir:-unset}\`
- Optional probe command: \`${probe_cmd:-unset}\`
- Probe rc: \`${probe_rc}\`

## Notes

- Rabby runtime support in C5 is hot-wallet scope only.
- Hardware passthrough remains deferred to H1 (post E5 + 14 days).
EOF

cat >"$report_json" <<EOF
{
  "schema_version": "${PRD05A_SCHEMA_VERSION}",
  "generated": "${timestamp}",
  "run_id": "${run_id}",
  "status": "${status}",
  "taxonomy": "${taxonomy}",
  "reason": "$(prd05a_json_escape "$reason")",
  "profile_dir": "$(prd05a_json_escape "${profile_dir:-}")",
  "probe_command": "$(prd05a_json_escape "${probe_cmd:-}")",
  "probe_rc": ${probe_rc},
  "artifacts": {
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
if [[ "$status" == "BLOCKED" ]]; then
  exit 2
fi
exit 1

