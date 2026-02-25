#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source "$ROOT_DIR/scripts/lib/prd05a_e2e_common.sh"

timestamp="$(prd05a_now_utc)"
run_id="$(prd05a_run_id)"
report_md="local/reports/prd05a/C5-wallet-mock-replay-report.md"
report_json="local/reports/prd05a/C5-wallet-mock-replay-report.json"
log_path="local/reports/prd05a/C5-wallet-mock-replay.log"
replay_dir="local/reports/prd05a/replay-wallet-mock/${run_id}"
driver_mode="wallet-mock"
release_gate_driver="wallet-mock"
gate_tier="${PRD05A_GATE_MODE:-blocking}"
gate_effect="BLOCKING"
window_runs="${PRD05A_REPLAY_WINDOW_RUNS:-100}"
max_harness_fail_pct="${PRD05A_REPLAY_MAX_HARNESS_FAIL_PCT:-1}"
inject_locale_prefix="${PRD05A_REPLAY_INJECT_LOCALE_PREFIX:-zz}"
failure_json_path="${1:-}"

mkdir -p local/reports/prd05a "$replay_dir"

chromium_info="$(prd05a_chromium_version)"
chromium_bin="${chromium_info%%|*}"
chromium_version="${chromium_info#*|}"

node_bin="$(prd05a_resolve_node || true)"
if [[ -z "$node_bin" ]]; then
  echo "Node v20 is required for replay report generation." >&2
  exit 2
fi
node_version="$(prd05a_node_version "$node_bin")"
node_major="$(prd05a_node_major "$node_bin")"
if [[ "$node_major" != "20" ]]; then
  echo "Node major version must be 20.x for C5 replay gate (found ${node_version})." >&2
  exit 2
fi

injection_cmd=""
injection_rc=0
injection_elapsed_seconds=0
replay_cmd=""
replay_rc=0
replay_elapsed_seconds=0
replay_reproduced="false"
artifacts_complete="false"

{
  echo "[header] schema_version=${PRD05A_SCHEMA_VERSION}"
  echo "[header] run_id=${run_id}"
  echo "[header] gate_tier=${gate_tier}"
  echo "[header] gate_effect=${gate_effect}"
  echo "[header] node_version=${node_version}"
  echo "[header] chromium_bin=${chromium_bin}"
  echo "[header] chromium_version=${chromium_version}"
  echo "[header] replay_window_runs=${window_runs}"
  echo "[header] max_harness_fail_pct=${max_harness_fail_pct}"
  echo "[header] inject_locale_prefix=${inject_locale_prefix}"
} >"$log_path"

if [[ -z "$failure_json_path" ]]; then
  injection_cmd="PRD05A_E2E_SKIP_WEBSERVER=1 PRD05A_EXPECTED_LOCALE_PREFIX=${inject_locale_prefix} PRD05A_GATE_ATTEMPTS=1 PRD05A_SKIP_RUNTIME_PROFILE=0 scripts/run_prd05a_wallet_mock_gate.sh"
  echo "[inject] command=${injection_cmd}" >>"$log_path"
  injection_started="$(date +%s)"
  set +e
  bash -lc "$injection_cmd" >>"$log_path" 2>&1
  injection_rc=$?
  set -e
  injection_elapsed_seconds=$(( $(date +%s) - injection_started ))

  if [[ $injection_rc -eq 0 ]]; then
    echo "[inject] expected failure but command succeeded" >>"$log_path"
  fi

  if [[ -f local/reports/prd05a/C5-wallet-mock-gate.json ]]; then
    cp local/reports/prd05a/C5-wallet-mock-gate.json "$replay_dir/injected-failure.json"
  fi
  if [[ -f local/reports/prd05a/C5-wallet-mock-gate.log ]]; then
    cp local/reports/prd05a/C5-wallet-mock-gate.log "$replay_dir/injected-failure.log"
  fi
  if [[ -f local/reports/prd05a/C5-wallet-mock-gate-report.md ]]; then
    cp local/reports/prd05a/C5-wallet-mock-gate-report.md "$replay_dir/injected-failure.md"
  fi

  failure_json_path="$replay_dir/injected-failure.json"
else
  echo "[inject] using provided failure json: ${failure_json_path}" >>"$log_path"
fi

if [[ ! -f "$failure_json_path" ]]; then
  echo "[replay] missing failure json: ${failure_json_path}" >>"$log_path"
  failure_json_path=""
fi

if [[ -n "$failure_json_path" ]]; then
  replay_cmd="PRD05A_E2E_SKIP_WEBSERVER=1 PRD05A_EXPECTED_LOCALE_PREFIX=${inject_locale_prefix} PRD05A_GATE_ATTEMPTS=1 PRD05A_SKIP_RUNTIME_PROFILE=0 scripts/run_prd05a_wallet_mock_gate.sh"
  echo "[replay] command=${replay_cmd}" >>"$log_path"
  replay_started="$(date +%s)"
  set +e
  bash -lc "$replay_cmd" >>"$log_path" 2>&1
  replay_rc=$?
  set -e
  replay_elapsed_seconds=$(( $(date +%s) - replay_started ))

  if [[ $replay_rc -ne 0 ]]; then
    replay_reproduced="true"
  fi

  if [[ -f local/reports/prd05a/C5-wallet-mock-gate.json ]]; then
    cp local/reports/prd05a/C5-wallet-mock-gate.json "$replay_dir/replay-result.json"
  fi
  if [[ -f local/reports/prd05a/C5-wallet-mock-gate.log ]]; then
    cp local/reports/prd05a/C5-wallet-mock-gate.log "$replay_dir/replay-result.log"
  fi
  if [[ -f local/reports/prd05a/C5-wallet-mock-gate-report.md ]]; then
    cp local/reports/prd05a/C5-wallet-mock-gate-report.md "$replay_dir/replay-result.md"
  fi

  artifacts_complete="$("$node_bin" - "$failure_json_path" <<'NODE'
const fs = require("fs");
const failurePath = process.argv[2];
try {
  const doc = JSON.parse(fs.readFileSync(failurePath, "utf8"));
  const artifacts = doc?.artifacts ?? {};
  const required = ["log", "markdown_report", "json_report"];
  for (const key of required) {
    const p = artifacts[key];
    if (typeof p !== "string" || p.length === 0 || !fs.existsSync(p)) {
      console.log("false");
      process.exit(0);
    }
  }
  console.log("true");
} catch (_error) {
  console.log("false");
}
NODE
)"
fi

analysis_tmp="$(mktemp)"
trap 'rm -f "$analysis_tmp"' EXIT

"$node_bin" - \
  "$window_runs" \
  "$max_harness_fail_pct" \
  "$analysis_tmp" <<'NODE'
const fs = require("fs");
const path = require("path");

const [windowRunsRaw, maxHarnessFailPctRaw, outPath] = process.argv.slice(2);
const windowRuns = Number.parseInt(windowRunsRaw, 10);
const maxHarnessFailPct = Number.parseFloat(maxHarnessFailPctRaw);

const soakRoot = "local/reports/prd05a/soak-wallet-mock";
const analysis = {
  window_runs_required: windowRuns,
  max_harness_fail_pct: maxHarnessFailPct,
  window_available: false,
  selected_window_runs: 0,
  selected_window_start: null,
  selected_window_end: null,
  harness_fail_count: 0,
  harness_fail_rate_pct: null,
  status_counts: { PASS: 0, FAIL: 0, BLOCKED: 0 },
  window_met: false,
};

if (!fs.existsSync(soakRoot)) {
  fs.writeFileSync(outPath, JSON.stringify(analysis, null, 2));
  process.exit(0);
}

const dirs = fs
  .readdirSync(soakRoot)
  .filter((name) => name.startsWith("run-"))
  .sort();

const entries = [];
for (const d of dirs) {
  const dirPath = path.join(soakRoot, d);
  if (!fs.statSync(dirPath).isDirectory()) {
    continue;
  }
  const files = fs
    .readdirSync(dirPath)
    .filter((name) => /^run-\d+\.json$/.test(name))
    .sort((a, b) => {
      const ai = Number(a.match(/^run-(\d+)\.json$/)[1]);
      const bi = Number(b.match(/^run-(\d+)\.json$/)[1]);
      return ai - bi;
    });
  for (const fileName of files) {
    const doc = JSON.parse(fs.readFileSync(path.join(dirPath, fileName), "utf8"));
    entries.push({
      run_dir: d,
      file: fileName,
      status: String(doc.status || "FAIL"),
      taxonomy: String(doc.taxonomy || "APP_FAIL"),
    });
  }
}

if (entries.length < windowRuns || windowRuns <= 0) {
  analysis.selected_window_runs = entries.length;
  fs.writeFileSync(outPath, JSON.stringify(analysis, null, 2));
  process.exit(0);
}

analysis.window_available = true;
const window = entries.slice(-windowRuns);
analysis.selected_window_runs = window.length;
analysis.selected_window_start = `${window[0].run_dir}/${window[0].file}`;
analysis.selected_window_end = `${window[window.length - 1].run_dir}/${window[window.length - 1].file}`;

for (const entry of window) {
  analysis.status_counts[entry.status] = (analysis.status_counts[entry.status] ?? 0) + 1;
  if (entry.taxonomy === "HARNESS_FAIL") {
    analysis.harness_fail_count += 1;
  }
}
analysis.harness_fail_rate_pct = Number(
  ((analysis.harness_fail_count * 100) / window.length).toFixed(2),
);
analysis.window_met = analysis.harness_fail_rate_pct <= maxHarnessFailPct;

fs.writeFileSync(outPath, JSON.stringify(analysis, null, 2));
NODE

window_available="$("$node_bin" -p "const fs=require('fs');const j=JSON.parse(fs.readFileSync('${analysis_tmp}','utf8'));j.window_available ? 'true' : 'false'")"
selected_window_runs="$("$node_bin" -p "const fs=require('fs');const j=JSON.parse(fs.readFileSync('${analysis_tmp}','utf8'));j.selected_window_runs")"
harness_fail_count="$("$node_bin" -p "const fs=require('fs');const j=JSON.parse(fs.readFileSync('${analysis_tmp}','utf8'));j.harness_fail_count")"
harness_fail_rate_pct="$("$node_bin" -p "const fs=require('fs');const j=JSON.parse(fs.readFileSync('${analysis_tmp}','utf8'));j.harness_fail_rate_pct ?? 'n/a'")"
window_met="$("$node_bin" -p "const fs=require('fs');const j=JSON.parse(fs.readFileSync('${analysis_tmp}','utf8'));j.window_met ? 'true' : 'false'")"
window_start="$("$node_bin" -p "const fs=require('fs');const j=JSON.parse(fs.readFileSync('${analysis_tmp}','utf8'));j.selected_window_start ?? ''")"
window_end="$("$node_bin" -p "const fs=require('fs');const j=JSON.parse(fs.readFileSync('${analysis_tmp}','utf8'));j.selected_window_end ?? ''")"

status="FAIL"
taxonomy="HARNESS_FAIL"
triage_label="triage/harness"
reason_parts=()

if [[ "$failure_json_path" == "" ]]; then
  reason_parts+=("missing failure artifact")
fi
if [[ "$replay_reproduced" != "true" ]]; then
  reason_parts+=("replay command did not reproduce failure")
fi
if [[ "$artifacts_complete" != "true" ]]; then
  reason_parts+=("required artifact set incomplete for failure")
fi
if [[ "$window_available" != "true" ]]; then
  reason_parts+=("insufficient soak runs for ${window_runs}-run window")
fi
if [[ "$window_met" != "true" ]]; then
  reason_parts+=("HARNESS_FAIL rate ${harness_fail_rate_pct}% exceeds ${max_harness_fail_pct}%")
fi

if [[ ${#reason_parts[@]} -eq 0 ]]; then
  status="PASS"
  taxonomy="NONE"
  reason="replay gate passed"
else
  reason="$(IFS='; '; echo "${reason_parts[*]}")"
fi

artifacts_json="$(cat <<EOF
{
  "log": "${log_path}",
  "markdown_report": "${report_md}",
  "json_report": "${report_json}",
  "replay_run_dir": "$(prd05a_json_escape "$replay_dir")"
}
EOF
)"

cat >"$report_md" <<EOF
# C5 Wallet Mock Replay Report

Generated: ${timestamp}
Run ID: ${run_id}
Schema: ${PRD05A_SCHEMA_VERSION}

## Gate Result

- Status: ${status}
- Taxonomy: ${taxonomy}
- Triage label: ${triage_label}
- Reason: ${reason:-replay gate failed}

## Replay Drill

- Failure artifact: \`${failure_json_path:-<missing>}\`
- Injection command: \`${injection_cmd:-<none>}\`
- Injection exit code: ${injection_rc}
- Injection duration: ${injection_elapsed_seconds}s
- Replay command: \`${replay_cmd:-<none>}\`
- Replay exit code: ${replay_rc}
- Replay duration: ${replay_elapsed_seconds}s
- Replay reproduced failure: ${replay_reproduced}
- Failure artifact set complete: ${artifacts_complete}

## E7 Budget Window

- Required runs: ${window_runs}
- Selected runs: ${selected_window_runs}
- Window start: \`${window_start:-n/a}\`
- Window end: \`${window_end:-n/a}\`
- HARNESS_FAIL count: ${harness_fail_count}
- HARNESS_FAIL rate: ${harness_fail_rate_pct}%
- Max allowed HARNESS_FAIL rate: ${max_harness_fail_pct}%
- Window policy met: ${window_met}

## Artifacts

- Replay log: \`${log_path}\`
- JSON report: \`${report_json}\`
- Markdown report: \`${report_md}\`
- Replay run dir: \`${replay_dir}\`
EOF

prd05a_write_json \
  "$report_json" \
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
  "${LANG:-en_US.UTF-8}" \
  "$artifacts_json" \
  "$triage_label" \
  "${reason:-replay gate failed}" \
  "$gate_tier" \
  "$gate_effect"

"$node_bin" - "$report_json" "$analysis_tmp" "$failure_json_path" "$replay_cmd" "$replay_reproduced" "$artifacts_complete" "$replay_dir" "$window_runs" "$max_harness_fail_pct" <<'NODE'
const fs = require("fs");

const [
  reportPath,
  analysisPath,
  failureJsonPath,
  replayCmd,
  replayReproduced,
  artifactsComplete,
  replayDir,
  windowRuns,
  maxHarnessFailPct,
] = process.argv.slice(2);

const report = JSON.parse(fs.readFileSync(reportPath, "utf8"));
const analysis = JSON.parse(fs.readFileSync(analysisPath, "utf8"));

report.failure_artifact_path = failureJsonPath || null;
report.replay_command = replayCmd || null;
report.replay_reproduced = replayReproduced === "true";
report.failure_artifacts_complete = artifactsComplete === "true";
report.replay_run_dir = replayDir;
report.harness_fail_window = {
  required_runs: Number(windowRuns),
  max_harness_fail_pct: Number(maxHarnessFailPct),
  window_available: Boolean(analysis.window_available),
  selected_window_runs: Number(analysis.selected_window_runs),
  selected_window_start: analysis.selected_window_start ?? null,
  selected_window_end: analysis.selected_window_end ?? null,
  harness_fail_count: Number(analysis.harness_fail_count),
  harness_fail_rate_pct: analysis.harness_fail_rate_pct,
  status_counts: analysis.status_counts,
  window_met: Boolean(analysis.window_met),
};

fs.writeFileSync(reportPath, JSON.stringify(report, null, 2));
NODE

"$node_bin" "$ROOT_DIR/e2e/tests/wallet-mock/validate-evidence-schema.mjs" "$report_json" >/dev/null

echo "wrote ${report_md}"
echo "wrote ${report_json}"
echo "wrote ${log_path}"

if [[ "$status" == "PASS" ]]; then
  exit 0
fi
exit 1
