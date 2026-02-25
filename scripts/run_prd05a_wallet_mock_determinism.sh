#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source "$ROOT_DIR/scripts/lib/prd05a_e2e_common.sh"

timestamp="$(prd05a_now_utc)"
run_id="$(prd05a_run_id)"
report_md="local/reports/prd05a/C5-wallet-mock-determinism-report.md"
report_json="local/reports/prd05a/C5-wallet-mock-determinism-report.json"
log_path="local/reports/prd05a/C5-wallet-mock-determinism.log"
driver_mode="wallet-mock"
release_gate_driver="wallet-mock"
gate_tier="${PRD05A_GATE_MODE:-blocking}"
gate_effect="BLOCKING"
sample_runs="${PRD05A_DETERMINISM_RUNS:-20}"
seed="${PRD05A_E2E_SEED:-test test test test test test test test test test test junk}"
network_policy="${PRD05A_NETWORK_POLICY:-local-only}"
network_allowlist="${PRD05A_NETWORK_ALLOWLIST:-}"
source_run_dir="${1:-}"

mkdir -p local/reports/prd05a

chromium_info="$(prd05a_chromium_version)"
chromium_bin="${chromium_info%%|*}"
chromium_version="${chromium_info#*|}"

node_bin="$(prd05a_resolve_node || true)"
if [[ -z "$node_bin" ]]; then
  echo "Node v20 is required for determinism report generation." >&2
  exit 2
fi
node_version="$(prd05a_node_version "$node_bin")"
node_major="$(prd05a_node_major "$node_bin")"
if [[ "$node_major" != "20" ]]; then
  echo "Node major version must be 20.x for C5 determinism gate (found ${node_version})." >&2
  exit 2
fi

if [[ -z "$source_run_dir" && -f local/reports/prd05a/C5-wallet-mock-soak-report.json ]]; then
  source_run_dir="$("$node_bin" -p "const fs=require('fs');const j=JSON.parse(fs.readFileSync('local/reports/prd05a/C5-wallet-mock-soak-report.json','utf8'));j?.artifacts?.run_dir ?? ''" 2>/dev/null || true)"
fi

if [[ -z "$source_run_dir" ]]; then
  source_run_dir="$(find local/reports/prd05a/soak-wallet-mock -maxdepth 1 -type d -name 'run-*' | sort | tail -n 1)"
fi

if [[ -z "$source_run_dir" || ! -d "$source_run_dir" ]]; then
  echo "No soak run directory found. Provide one explicitly: scripts/run_prd05a_wallet_mock_determinism.sh <run-dir>" >&2
  exit 2
fi

analysis_tmp="$(mktemp)"
trap 'rm -f "$analysis_tmp"' EXIT

"$node_bin" - \
  "$source_run_dir" \
  "$sample_runs" \
  "$network_policy" \
  "$network_allowlist" \
  "$log_path" \
  "$analysis_tmp" <<'NODE'
const crypto = require("crypto");
const fs = require("fs");
const path = require("path");

const [sourceRunDir, sampleRunsRaw, networkPolicy, networkAllowlistRaw, logPath, analysisPath] =
  process.argv.slice(2);
const sampleRuns = Number.parseInt(sampleRunsRaw, 10);
if (!Number.isFinite(sampleRuns) || sampleRuns <= 0) {
  throw new Error(`invalid sample runs: ${sampleRunsRaw}`);
}

function sortedRunJsonFiles(dir) {
  return fs
    .readdirSync(dir)
    .filter((name) => /^run-\d+\.json$/.test(name))
    .sort((a, b) => {
      const ai = Number(a.match(/^run-(\d+)\.json$/)[1]);
      const bi = Number(b.match(/^run-(\d+)\.json$/)[1]);
      return ai - bi;
    });
}

function stripAnsi(text) {
  return text.replace(/\u001b\[[0-9;]*m/g, "");
}

function sha256Hex(text) {
  return crypto.createHash("sha256").update(text, "utf8").digest("hex");
}

function parseUrls(text) {
  const urls = [];
  const rx = /https?:\/\/[^\s"'`)<]+/g;
  let m;
  while ((m = rx.exec(text)) !== null) {
    urls.push(m[0]);
  }
  return urls;
}

const runJsonFiles = sortedRunJsonFiles(sourceRunDir);
if (runJsonFiles.length < sampleRuns) {
  throw new Error(
    `source run dir has ${runJsonFiles.length} runs, requires at least ${sampleRuns}: ${sourceRunDir}`,
  );
}

const selectedRunJsonFiles = runJsonFiles.slice(0, sampleRuns);
const allowlistHosts = new Set(
  String(networkAllowlistRaw || "")
    .split(",")
    .map((s) => s.trim().toLowerCase())
    .filter(Boolean),
);
["localhost", "127.0.0.1", "::1", "[::1]", "localhost.", "ip6-localhost.", "ip6-loopback."].forEach(
  (h) => allowlistHosts.add(h),
);

const stateLeakRegex = /\b(state[-_ ]?leak|cross[-_ ]?test|residue|leakage)\b/i;
const statusCounts = { PASS: 0, FAIL: 0, BLOCKED: 0 };
const taxonomyCounts = {};
const transcriptHashes = [];
const stateLeakViolations = [];
const networkViolations = [];
const perRun = [];

for (const runJsonName of selectedRunJsonFiles) {
  const runJsonPath = path.join(sourceRunDir, runJsonName);
  const runBase = runJsonName.replace(/\.json$/, "");
  const runLogPath = path.join(sourceRunDir, `${runBase}.log`);
  const runDoc = JSON.parse(fs.readFileSync(runJsonPath, "utf8"));
  const status = String(runDoc.status || "FAIL");
  const taxonomy = String(runDoc.taxonomy || "APP_FAIL");
  statusCounts[status] = (statusCounts[status] ?? 0) + 1;
  taxonomyCounts[taxonomy] = (taxonomyCounts[taxonomy] ?? 0) + 1;

  const logText = fs.existsSync(runLogPath) ? fs.readFileSync(runLogPath, "utf8") : "";
  const cleanLog = stripAnsi(logText);
  const testSummaryMatch = cleanLog.match(/\n\s*(\d+)\s+passed(?:\s+\(\S+\))?/g) ?? [];
  const transcriptPayload = {
    status,
    taxonomy,
    reason: String(runDoc.reason || ""),
    driver_mode: String(runDoc.driver_mode || ""),
    release_gate_driver: String(runDoc.release_gate_driver || ""),
    gate_tier: String(runDoc.gate_tier || ""),
    gate_effect: String(runDoc.gate_effect || ""),
    test_summary_markers: testSummaryMatch.map((row) =>
      row
        .trim()
        .replace(/\(\d+(?:\.\d+)?(?:ms|s|m)\)/g, "(<DURATION>)"),
    ),
  };
  const canonicalTranscript = JSON.stringify(transcriptPayload);
  const transcriptSha256 = sha256Hex(canonicalTranscript);
  transcriptHashes.push(transcriptSha256);

  if (stateLeakRegex.test(logText)) {
    stateLeakViolations.push({
      run: runBase,
      reason: "state leakage marker found in log",
    });
  }

  const urls = parseUrls(logText);
  for (const url of urls) {
    try {
      const u = new URL(url);
      const host = String(u.hostname || "").toLowerCase();
      if (networkPolicy === "local-only" && !allowlistHosts.has(host)) {
        networkViolations.push({
          run: runBase,
          url,
          host,
          reason: "host not allowed under local-only policy",
        });
      } else if (networkPolicy === "allowlist" && !allowlistHosts.has(host)) {
        networkViolations.push({
          run: runBase,
          url,
          host,
          reason: "host not present in explicit allowlist",
        });
      }
    } catch (_error) {
      // Ignore unparsable URL fragments in logs.
    }
  }

  perRun.push({
    run: runBase,
    status,
    taxonomy,
    transcript_sha256: transcriptSha256,
    log_path: runLogPath,
  });
}

const uniqueTranscriptHashes = [...new Set(transcriptHashes)];
const passRuns = statusCounts.PASS ?? 0;
const failRuns = statusCounts.FAIL ?? 0;
const blockedRuns = statusCounts.BLOCKED ?? 0;
const stableTranscript = uniqueTranscriptHashes.length === 1;
const noStateLeak = stateLeakViolations.length === 0;
const noNetworkViolation = networkViolations.length === 0;
const allPass = passRuns === sampleRuns && failRuns === 0 && blockedRuns === 0;
const gatePass = stableTranscript && noStateLeak && noNetworkViolation && allPass;

const analysis = {
  source_run_dir: sourceRunDir,
  sample_runs: sampleRuns,
  selected_files: selectedRunJsonFiles,
  status_counts: statusCounts,
  taxonomy_counts: taxonomyCounts,
  stable_transcript: stableTranscript,
  unique_transcript_hash_count: uniqueTranscriptHashes.length,
  unique_transcript_hashes: uniqueTranscriptHashes,
  state_leak_violations: stateLeakViolations,
  network_policy: networkPolicy,
  network_allowlist_hosts: [...allowlistHosts].sort(),
  network_policy_violations: networkViolations,
  gate_pass: gatePass,
  per_run: perRun,
};

const logLines = [];
logLines.push(`# C5 Wallet Mock Determinism Analysis`);
logLines.push(`source_run_dir=${sourceRunDir}`);
logLines.push(`sample_runs=${sampleRuns}`);
logLines.push(`stable_transcript=${stableTranscript}`);
logLines.push(`unique_transcript_hash_count=${uniqueTranscriptHashes.length}`);
for (const h of uniqueTranscriptHashes) {
  logLines.push(`transcript_sha256=${h}`);
}
logLines.push(`state_leak_violations=${stateLeakViolations.length}`);
logLines.push(`network_policy_violations=${networkViolations.length}`);
logLines.push(`status_counts=${JSON.stringify(statusCounts)}`);
logLines.push(`taxonomy_counts=${JSON.stringify(taxonomyCounts)}`);
if (stateLeakViolations.length > 0) {
  logLines.push(`state_leak_details=${JSON.stringify(stateLeakViolations)}`);
}
if (networkViolations.length > 0) {
  logLines.push(`network_violation_details=${JSON.stringify(networkViolations)}`);
}
fs.writeFileSync(logPath, `${logLines.join("\n")}\n`);
fs.writeFileSync(analysisPath, JSON.stringify(analysis, null, 2));
NODE

analysis_status="$("$node_bin" -p "const fs=require('fs');const j=JSON.parse(fs.readFileSync('${analysis_tmp}','utf8'));j.gate_pass ? 'PASS' : 'FAIL'")"
stable_transcript="$("$node_bin" -p "const fs=require('fs');const j=JSON.parse(fs.readFileSync('${analysis_tmp}','utf8'));j.stable_transcript ? 'true' : 'false'")"
unique_hash_count="$("$node_bin" -p "const fs=require('fs');const j=JSON.parse(fs.readFileSync('${analysis_tmp}','utf8'));j.unique_transcript_hash_count")"
state_leak_violations="$("$node_bin" -p "const fs=require('fs');const j=JSON.parse(fs.readFileSync('${analysis_tmp}','utf8'));j.state_leak_violations.length")"
network_policy_violations="$("$node_bin" -p "const fs=require('fs');const j=JSON.parse(fs.readFileSync('${analysis_tmp}','utf8'));j.network_policy_violations.length")"
pass_runs="$("$node_bin" -p "const fs=require('fs');const j=JSON.parse(fs.readFileSync('${analysis_tmp}','utf8'));j.status_counts.PASS ?? 0")"
fail_runs="$("$node_bin" -p "const fs=require('fs');const j=JSON.parse(fs.readFileSync('${analysis_tmp}','utf8'));j.status_counts.FAIL ?? 0")"
blocked_runs="$("$node_bin" -p "const fs=require('fs');const j=JSON.parse(fs.readFileSync('${analysis_tmp}','utf8'));j.status_counts.BLOCKED ?? 0")"

status="FAIL"
taxonomy="HARNESS_FAIL"
triage_label="triage/harness"
reason="determinism gate failed"
if [[ "$analysis_status" == "PASS" ]]; then
  status="PASS"
  taxonomy="NONE"
  reason="determinism gate passed"
fi

artifacts_json="$(cat <<EOF
{
  "log": "${log_path}",
  "markdown_report": "${report_md}",
  "json_report": "${report_json}",
  "source_run_dir": "$(prd05a_json_escape "$source_run_dir")"
}
EOF
)"

cat >"$report_md" <<EOF
# C5 Wallet Mock Determinism Report

Generated: ${timestamp}
Run ID: ${run_id}
Schema: ${PRD05A_SCHEMA_VERSION}

## Gate Result

- Status: ${status}
- Taxonomy: ${taxonomy}
- Triage label: ${triage_label}
- Reason: ${reason}

## Inputs

- Source run dir: \`${source_run_dir}\`
- Sample runs: ${sample_runs}
- Seed: \`${seed}\`
- Network policy: \`${network_policy}\`
- Network allowlist: \`${network_allowlist:-<default-local-only>}\`

## Assertions

- Stable transcript hash across sample: ${stable_transcript}
- Unique transcript hash count: ${unique_hash_count}
- Pass runs: ${pass_runs}
- Fail runs: ${fail_runs}
- Blocked runs: ${blocked_runs}
- State-leak violations: ${state_leak_violations}
- Network-policy violations: ${network_policy_violations}

## Artifacts

- Analysis log: \`${log_path}\`
- JSON report: \`${report_json}\`
- Markdown report: \`${report_md}\`
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
  "$reason" \
  "$gate_tier" \
  "$gate_effect"

"$node_bin" - "$report_json" "$analysis_tmp" "$seed" <<'NODE'
const fs = require("fs");

const [reportJsonPath, analysisPath, seed] = process.argv.slice(2);
const report = JSON.parse(fs.readFileSync(reportJsonPath, "utf8"));
const analysis = JSON.parse(fs.readFileSync(analysisPath, "utf8"));

report.seed = seed;
report.source_run_dir = analysis.source_run_dir;
report.sample_runs = analysis.sample_runs;
report.status_counts = analysis.status_counts;
report.taxonomy_counts = analysis.taxonomy_counts;
report.stable_transcript = analysis.stable_transcript;
report.unique_transcript_hash_count = analysis.unique_transcript_hash_count;
report.unique_transcript_hashes = analysis.unique_transcript_hashes;
report.state_leak_violations = analysis.state_leak_violations;
report.network_policy = analysis.network_policy;
report.network_allowlist_hosts = analysis.network_allowlist_hosts;
report.network_policy_violations = analysis.network_policy_violations;

fs.writeFileSync(reportJsonPath, JSON.stringify(report, null, 2));
NODE

"$node_bin" "$ROOT_DIR/e2e/tests/wallet-mock/validate-evidence-schema.mjs" "$report_json" >/dev/null

echo "wrote ${report_md}"
echo "wrote ${report_json}"
echo "wrote ${log_path}"

if [[ "$status" == "PASS" ]]; then
  exit 0
fi
exit 1
