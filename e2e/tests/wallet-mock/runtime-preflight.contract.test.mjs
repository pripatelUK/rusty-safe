import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

const REQUIRED_HEADERS = [
  "schema_version",
  "run_id",
  "driver_mode",
  "release_gate_driver",
  "gate_tier",
  "node_version",
  "chromium_bin",
  "chromium_version",
  "lang",
  "lc_all",
];

test("preflight runtime uses Node v20", () => {
  const major = process.versions.node.split(".")[0];
  assert.equal(major, "20");
});

test("preflight log includes required metadata headers", () => {
  const logPath = process.env.PRD05A_PRECHECK_LOG_PATH;
  assert.ok(logPath, "PRD05A_PRECHECK_LOG_PATH must be set");
  assert.ok(fs.existsSync(logPath), `preflight log missing: ${logPath}`);

  const logContents = fs.readFileSync(logPath, "utf8");
  for (const header of REQUIRED_HEADERS) {
    assert.match(
      logContents,
      new RegExp(`\\[header\\] ${header}=`),
      `missing header ${header} in preflight log`,
    );
  }
});
