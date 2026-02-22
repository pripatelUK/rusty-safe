import assert from "node:assert/strict";
import test from "node:test";

import { classifyFailureTaxonomy, taxonomyTriageLabel } from "./failure-taxonomy.mjs";

test("user rejection maps to APP_FAIL", () => {
  const taxonomy = classifyFailureTaxonomy({ code: 4001, message: "User rejected the request." });
  assert.equal(taxonomy, "APP_FAIL");
  assert.equal(taxonomyTriageLabel(taxonomy), "triage/app");
});

test("timeout maps to HARNESS_FAIL", () => {
  const taxonomy = classifyFailureTaxonomy(new Error("wallet-mock-request-timeout-30000ms"));
  assert.equal(taxonomy, "HARNESS_FAIL");
  assert.equal(taxonomyTriageLabel(taxonomy), "triage/harness");
});

test("node mismatch maps to ENV_BLOCKER", () => {
  const taxonomy = classifyFailureTaxonomy(new Error("Node major version must be 20.x"));
  assert.equal(taxonomy, "ENV_BLOCKER");
  assert.equal(taxonomyTriageLabel(taxonomy), "triage/env");
});

test("chain mismatch maps to APP_FAIL", () => {
  const taxonomy = classifyFailureTaxonomy(new Error("chain mismatch expected 0x1 got 0xa"));
  assert.equal(taxonomy, "APP_FAIL");
  assert.equal(taxonomyTriageLabel(taxonomy), "triage/app");
});
