import fs from "node:fs";
import path from "node:path";

const rootDir = path.resolve(new URL("../../..", import.meta.url).pathname);

const evidencePath = process.argv[2];
if (!evidencePath) {
  console.error("usage: node validate-evidence-schema.mjs <evidence-json-path>");
  process.exit(2);
}

const resolvedEvidencePath = path.resolve(rootDir, evidencePath);
if (!fs.existsSync(resolvedEvidencePath)) {
  console.error(`[schema] missing evidence file: ${resolvedEvidencePath}`);
  process.exit(2);
}

const raw = fs.readFileSync(resolvedEvidencePath, "utf8");
let data;
try {
  data = JSON.parse(raw);
} catch (error) {
  console.error(`[schema] invalid json: ${resolvedEvidencePath}`);
  console.error(String(error?.message ?? error));
  process.exit(2);
}

const requiredEnvelopeFields = [
  "schema_version",
  "generated",
  "run_id",
  "status",
  "taxonomy",
  "driver_mode",
  "release_gate_driver",
  "triage_label",
  "node_version",
  "chromium_bin",
  "chromium_version",
  "locale",
  "reason",
  "artifacts",
];

for (const field of requiredEnvelopeFields) {
  if (!(field in data)) {
    console.error(`[schema] missing field: ${field}`);
    process.exit(2);
  }
}

if (data.schema_version !== "c5e2e-v1") {
  console.error(`[schema] schema_version mismatch: ${data.schema_version}`);
  process.exit(2);
}

if (!["PASS", "FAIL", "BLOCKED"].includes(data.status)) {
  console.error(`[schema] invalid status: ${data.status}`);
  process.exit(2);
}

if (!["NONE", "ENV_BLOCKER", "HARNESS_FAIL", "APP_FAIL", "WALLET_FAIL"].includes(data.taxonomy)) {
  console.error(`[schema] invalid taxonomy: ${data.taxonomy}`);
  process.exit(2);
}

if (data.gate_tier && !["blocking", "canary", "manual"].includes(data.gate_tier)) {
  console.error(`[schema] invalid gate_tier: ${data.gate_tier}`);
  process.exit(2);
}

if (data.gate_effect && !["BLOCKING", "CANARY", "MANUAL"].includes(data.gate_effect)) {
  console.error(`[schema] invalid gate_effect: ${data.gate_effect}`);
  process.exit(2);
}

if (typeof data.artifacts !== "object" || data.artifacts === null) {
  console.error("[schema] artifacts must be an object");
  process.exit(2);
}

const requiredArtifacts = ["log", "markdown_report", "json_report"];
for (const key of requiredArtifacts) {
  const value = data.artifacts[key];
  if (typeof value !== "string" || value.length === 0) {
    console.error(`[schema] missing artifacts.${key}`);
    process.exit(2);
  }
  const artifactPath = path.resolve(rootDir, value);
  if (!fs.existsSync(artifactPath)) {
    console.error(`[schema] artifact missing on disk: ${value}`);
    process.exit(2);
  }
}

console.log(`[schema] PASS ${evidencePath}`);
