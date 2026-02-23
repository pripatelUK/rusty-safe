#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source "$ROOT_DIR/scripts/lib/prd05a_e2e_common.sh"

run_dir="${1:-}"
if [[ -z "$run_dir" ]]; then
  run_dir="$(find local/reports/prd05a/soak-wallet-mock -maxdepth 1 -type d -name 'run-*' | sort | tail -n 1)"
fi

if [[ -z "$run_dir" || ! -d "$run_dir" ]]; then
  echo "wallet-mock soak run directory not found" >&2
  exit 2
fi

timestamp="$(prd05a_now_utc)"
run_id="$(prd05a_run_id)"
report_md="local/reports/prd05a/C5-wallet-mock-runtime-slo-report.md"
report_json="local/reports/prd05a/C5-wallet-mock-runtime-slo-report.json"

mkdir -p local/reports/prd05a

node_bin="$(prd05a_resolve_node || true)"
if [[ -z "$node_bin" ]]; then
  echo "Node v20 is required for runtime SLO report generation." >&2
  exit 2
fi

"$node_bin" - "$run_dir" "$report_json" "$report_md" "$timestamp" "$run_id" "$PRD05A_SCHEMA_VERSION" <<'NODE'
const fs = require("fs");
const path = require("path");

const [runDir, reportJson, reportMd, timestamp, runId, schemaVersion] = process.argv.slice(2);

const statusFiles = fs
  .readdirSync(runDir)
  .filter((name) => /^run-\d+\.json$/.test(name))
  .sort((a, b) => {
    const ai = Number(a.match(/^run-(\d+)\.json$/)[1]);
    const bi = Number(b.match(/^run-(\d+)\.json$/)[1]);
    return ai - bi;
  });

if (statusFiles.length === 0) {
  throw new Error(`no run-*.json files found in ${runDir}`);
}

const scenarioDurationsMs = [];
const gateDurationsMs = [];
let passCount = 0;
let failCount = 0;
let blockedCount = 0;

function parseDurationMs(raw, unit) {
  const value = Number(raw);
  if (!Number.isFinite(value)) {
    return null;
  }
  if (unit === "ms") {
    return value;
  }
  if (unit === "s") {
    return value * 1000;
  }
  if (unit === "m") {
    return value * 60 * 1000;
  }
  return null;
}

for (const statusFile of statusFiles) {
  const base = statusFile.replace(/\.json$/, "");
  const jsonPath = path.join(runDir, statusFile);
  const logPath = path.join(runDir, `${base}.log`);

  const statusDoc = JSON.parse(fs.readFileSync(jsonPath, "utf8"));
  switch (statusDoc.status) {
    case "PASS":
      passCount += 1;
      break;
    case "BLOCKED":
      blockedCount += 1;
      break;
    default:
      failCount += 1;
      break;
  }

  if (!fs.existsSync(logPath)) {
    continue;
  }
  const logText = fs.readFileSync(logPath, "utf8");

  const scenarioRegex = /WM-[A-Z0-9-]+[^\n]*\((\d+(?:\.\d+)?)(ms|s)\)/g;
  let scenarioMatch;
  while ((scenarioMatch = scenarioRegex.exec(logText)) !== null) {
    const ms = parseDurationMs(scenarioMatch[1], scenarioMatch[2]);
    if (ms !== null) {
      scenarioDurationsMs.push(ms);
    }
  }

  const gateRegex = /\n\s*\d+\s+passed\s+\((\d+(?:\.\d+)?)(ms|s|m)\)/g;
  let gateMatch;
  while ((gateMatch = gateRegex.exec(logText)) !== null) {
    const ms = parseDurationMs(gateMatch[1], gateMatch[2]);
    if (ms !== null) {
      gateDurationsMs.push(ms);
    }
  }
}

function percentile95(values) {
  if (!values.length) {
    return null;
  }
  const sorted = [...values].sort((a, b) => a - b);
  const idx = Math.max(0, Math.min(sorted.length - 1, Math.ceil(sorted.length * 0.95) - 1));
  return sorted[idx];
}

function roundMs(value) {
  if (value === null) {
    return null;
  }
  return Math.round(value);
}

const runs = statusFiles.length;
const passRatePct = runs === 0 ? 0 : (passCount * 100) / runs;
const scenarioP95Ms = roundMs(percentile95(scenarioDurationsMs));
const gateP95Ms = roundMs(percentile95(gateDurationsMs));
const scenarioMaxMs = scenarioDurationsMs.length ? Math.round(Math.max(...scenarioDurationsMs)) : null;
const gateMaxMs = gateDurationsMs.length ? Math.round(Math.max(...gateDurationsMs)) : null;

const ciSloTargetRuns = 50;
const ciSloTargetPassRate = 99;
const scenarioBudgetMs = 90_000;
const gateBudgetMs = 15 * 60_000;

const ciSloMet = runs >= ciSloTargetRuns && passRatePct >= ciSloTargetPassRate;
const scenarioBudgetMet = scenarioP95Ms !== null && scenarioP95Ms <= scenarioBudgetMs;
const gateBudgetMet = gateP95Ms !== null && gateP95Ms <= gateBudgetMs;

const jsonDoc = {
  schema_version: schemaVersion,
  generated: timestamp,
  run_id: runId,
  source_run_dir: runDir,
  runs,
  pass_count: passCount,
  fail_count: failCount,
  blocked_count: blockedCount,
  pass_rate_pct: Number(passRatePct.toFixed(2)),
  metrics: {
    scenario_samples: scenarioDurationsMs.length,
    gate_samples: gateDurationsMs.length,
    scenario_p95_ms: scenarioP95Ms,
    scenario_max_ms: scenarioMaxMs,
    gate_p95_ms: gateP95Ms,
    gate_max_ms: gateMaxMs
  },
  thresholds: {
    ci_slo: {
      min_runs: ciSloTargetRuns,
      min_pass_rate_pct: ciSloTargetPassRate,
      met: ciSloMet
    },
    scenario_p95_budget_ms: {
      budget_ms: scenarioBudgetMs,
      met: scenarioBudgetMet
    },
    gate_p95_budget_ms: {
      budget_ms: gateBudgetMs,
      met: gateBudgetMet
    }
  },
  status: ciSloMet && scenarioBudgetMet && gateBudgetMet ? "PASS" : "FAIL",
  artifacts: {
    markdown_report: reportMd,
    json_report: reportJson
  }
};

fs.writeFileSync(reportJson, JSON.stringify(jsonDoc, null, 2));

const md = `# C5 Wallet Mock Runtime SLO Report

Generated: ${timestamp}
Run ID: ${runId}
Schema: ${schemaVersion}

Source run directory: \`${runDir}\`

## Reliability

- Runs: ${runs}
- Pass count: ${passCount}
- Fail count: ${failCount}
- Blocked count: ${blockedCount}
- Pass rate: ${passRatePct.toFixed(2)}%
- CI SLO (\`>=99%\` over \`50\` runs): ${ciSloMet ? "PASS" : "FAIL"}

## Runtime Budgets

- Scenario samples: ${scenarioDurationsMs.length}
- Scenario p95: ${scenarioP95Ms ?? "n/a"} ms (budget: ${scenarioBudgetMs} ms) => ${scenarioBudgetMet ? "PASS" : "FAIL"}
- Scenario max: ${scenarioMaxMs ?? "n/a"} ms
- Gate samples: ${gateDurationsMs.length}
- Gate p95: ${gateP95Ms ?? "n/a"} ms (budget: ${gateBudgetMs} ms) => ${gateBudgetMet ? "PASS" : "FAIL"}
- Gate max: ${gateMaxMs ?? "n/a"} ms

## Overall

- Status: ${jsonDoc.status}
`;

fs.writeFileSync(reportMd, md);
console.log(`wrote ${reportMd}`);
console.log(`wrote ${reportJson}`);
NODE
