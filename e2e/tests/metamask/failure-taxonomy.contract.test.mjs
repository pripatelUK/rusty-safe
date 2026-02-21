import assert from "node:assert/strict";
import test from "node:test";

import { classifyFailureTaxonomy, taxonomyTriageLabel } from "./failure-taxonomy.mjs";

test("negative user rejection maps to APP_FAIL", () => {
  const taxonomy = classifyFailureTaxonomy({ code: 4001, message: "User rejected the request." });
  assert.equal(taxonomy, "APP_FAIL");
  assert.equal(taxonomyTriageLabel(taxonomy), "triage/app");
});

test("negative popup timeout maps to HARNESS_FAIL", () => {
  const taxonomy = classifyFailureTaxonomy({
    message: "[getNotificationPageAndWaitForLoad] Failed to get notification page",
  });
  assert.equal(taxonomy, "HARNESS_FAIL");
  assert.equal(taxonomyTriageLabel(taxonomy), "triage/harness");
});

test("negative chain mismatch maps to APP_FAIL", () => {
  const taxonomy = classifyFailureTaxonomy(new Error("chain mismatch expected 0x1 got 0x539"));
  assert.equal(taxonomy, "APP_FAIL");
  assert.equal(taxonomyTriageLabel(taxonomy), "triage/app");
});

