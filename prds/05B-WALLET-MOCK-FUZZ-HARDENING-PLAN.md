# PRD 05B Wallet-Mock Fuzz Hardening Plan

Status: Planned (Deferred Until 05A E6/E7 Closure)  
Owner: Rusty Safe  
Depends on: `prds/05A-E2E-WALLET-RUNTIME-PLAN.md`  
Related: `prds/05A-RELEASE-GATE-CHECKLIST.md`

## 1. Executive Summary

Problem:
1. Deterministic scenario coverage in 05A is strong but still finite.
2. Sequence/state bugs in build/sign/share flows can appear outside hand-authored cases.

Solution:
1. Add a dedicated fuzz hardening track for wallet-mock, separate from 05A release gating.
2. Start as non-blocking nightly fuzzing with deterministic seeds and strict artifact capture.
3. Promote to blocking only after stability criteria are met.

## 2. Scope and Guardrails

In scope:
1. Wallet-mock-only state-machine fuzzing for Safe build/sign/share flow.
2. Deterministic counterexample minimization and reproducible replay metadata.
3. Promotion pipeline from accepted counterexample to deterministic regression scenario.

Out of scope:
1. MetaMask/Rabby/hardware wallet fuzzing.
2. Changes to 05A release criteria.
3. New product features outside localsafe parity.

Guardrails:
1. This plan must not block 05A release until explicit promotion criteria are met.
2. All additions map to parity reliability hardening, not feature expansion.

## 3. Decision Lock (Recommended Defaults)

1. Fuzz framework: `fast-check` state-machine model in `e2e/tests/wallet-mock/fuzz/`.
2. PR canary profile: `10 seeds x 75 transitions`.
3. Nightly profile: `50 seeds x 200 transitions`.
4. Transcript hash: canonical JSON (sorted keys, volatile fields removed) then SHA-256.
5. Promotion workflow: auto-generate regression manifest + human-reviewed commit.
6. Regression storage: `e2e/tests/wallet-mock/scenarios/regressions/`.
7. Artifact retention: all failing artifacts + 30 days of passing artifacts.

## 4. Core Design

Flow:
1. Fuzz runner executes deterministic seed matrix against wallet-mock state machine.
2. Invariant engine checks:
   - stable/canonical hash semantics,
   - signature validity and threshold monotonicity,
   - tamper rejection/quarantine behavior,
   - idempotent propose/confirm semantics.
3. On failure, minimizer emits smallest failing trace with replay metadata.
4. Accepted failures are promoted to deterministic regression scenarios.

## 5. Data and Artifact Contracts

Fuzz run metadata fields:
1. `seed`
2. `fuzz_profile`
3. `fuzz_transition_budget`
4. `model_version`
5. `invariant_set_version`
6. `transcript_sha256`
7. `counterexample_id` (nullable)
8. `counterexample_trace_path` (nullable)
9. `promotion_status` (`none|pending|promoted|rejected`)
10. `promotion_pr_reference` (nullable)

Artifacts:
1. `local/reports/prd05b/fuzz/nightly-report.md`
2. `local/reports/prd05b/fuzz/nightly-report.json`
3. `local/reports/prd05b/fuzz/counterexamples/*`
4. `local/reports/prd05b/fuzz/promotion-log.md`

## 6. Implementation Roadmap

### F0: Baseline Non-Blocking Fuzz Lane

Tasks:
1. Add `fast-check` state-machine skeleton and deterministic seed plumbing.
2. Add nightly command/script and artifact writer.
3. Add canonical transcript hash contract and validation tests.

Gate:
1. Nightly job runs and publishes complete artifacts.
2. 100% failed runs include reproducible counterexample payloads.

### F1: Invariant and Minimization Hardening

Tasks:
1. Implement invariant suite for `create -> sign -> share -> import -> confirm -> execute`.
2. Implement counterexample minimization and failure taxonomy.
3. Add replay command generation from artifacts.

Gate:
1. Minimization success >= 99% for fuzz failures.
2. Mean time to reproduce from artifact <= 10 minutes.

### F2: Promotion Pipeline

Tasks:
1. Auto-generate deterministic regression manifests from accepted counterexamples.
2. Add human-review workflow and rationale capture for rejected counterexamples.
3. Add promotion log and traceability checks.

Gate:
1. 100% accepted counterexamples promoted within 1 business day.
2. Regression manifests stored under `e2e/tests/wallet-mock/scenarios/regressions/`.

### F3: Optional Promotion to Blocking (Only After Stability)

Entry criteria:
1. 14 consecutive nightly runs with no untriaged counterexample backlog.
2. Harness failure rate <= 1% in nightly fuzz lane.
3. Runtime budget fit for CI.

Tasks:
1. Enable PR blocking fuzz profile (`10x75`) behind explicit release decision.
2. Update 05A/05A checklist links to include 05B gate.

Gate:
1. Blocking lane remains stable for 2 weeks after promotion.
2. No unresolved high-severity fuzz regressions.

## 7. Scripts and CI Surface

Planned scripts:
1. `scripts/run_prd05b_wallet_mock_fuzz_canary.sh`
2. `scripts/run_prd05b_wallet_mock_fuzz_nightly.sh`
3. `scripts/run_prd05b_wallet_mock_fuzz_promote.sh`

CI cadence:
1. PR: optional non-blocking canary (`10x75`) while in F0-F2.
2. Nightly: required non-blocking fuzz (`50x200`) during F0-F2.
3. Blocking PR mode allowed only in F3 after entry criteria.

## 8. Milestones

1. `N1` F0 baseline lane complete.
2. `N2` F1 invariants/minimization complete.
3. `N3` F2 promotion pipeline complete.
4. `N4` F3 blocking-promotion decision completed (go/no-go).

## 9. Branch, Commit, and Tag Discipline

Branch policy:
1. One branch per phase: `feat/prd05b-f<phase>-<slug>`.
2. Merge order: `F0 -> F1 -> F2 -> F3`.

Commit policy:
1. Commit at task boundaries with `F*-T*` IDs.
2. Add one `-gate-green` commit per completed phase.

Tag policy:
1. Tag each completed phase: `prd05b-f<phase>-gate`.
2. Tag blocking-promotion decision milestone: `prd05b-f3-decision`.

## 10. Handoff Rules with 05A

1. 05A remains release-blocking for signing parity through E7 only.
2. 05B runs in parallel/non-blocking mode until F3 decision criteria are met.
3. No 05B requirement can retroactively block 05A release without explicit product+engineering sign-off.
