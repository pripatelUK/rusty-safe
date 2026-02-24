# PRD 05A E2E Wallet Runtime Plan (Deterministic Wallet-Mock Gate Only)

Status: Active (Revised for Build/Sign/Share Priority Execution)
Owner: Rusty Safe
Parent PRD: `prds/05A-PRD-PARITY-WAVE.md`
Continuation Milestone: `C5` in `prds/05A-CONTINUATION-MILESTONES.md`

## 1. Executive Summary

Problem statement:
1. MetaMask extension automation is currently nondeterministic (startup/onboarding/popup lifecycle), causing false negatives in CI.
2. Release blocking on extension automation hides whether failures are app logic regressions or wallet runtime instability.
3. We need a production-grade path to validate Rusty Safe signing parity quickly without feature creep beyond localsafe parity scope.

Solution overview:
1. Make deterministic `ethereum-wallet-mock` parity tests the blocking CI/release gate for C5.
2. Keep 05A scoped to deterministic parity and release confidence for build/sign/share flows only.
3. Add explicit determinism contract gates (seed lock, scenario state isolation, and outbound-network guard).
4. Add reproducible replay gates so every failing run can be deterministically re-executed from evidence.
5. Defer wallet-mock counterexample fuzz hardening to dedicated follow-on plan `prds/05B-WALLET-MOCK-FUZZ-HARDENING-PLAN.md`.
6. Move MetaMask/Rabby/hardware validation tracks into dedicated follow-on plan `prds/05A-E2E-REAL-WALLET-HARDWARE-TRACK.md`.
7. Keep implementation staged with hard milestones, branch/commit discipline, and measurable SLOs.

Key innovations:

| Innovation | Why it matters |
|---|---|
| Single-lane deterministic acceptance model | Removes extension flake from release-critical CI decisions |
| Contract-tested `WalletDriver` + `gate_tier` manifest metadata | Prevents driver details from leaking into parity assertions |
| Reuse-first policy | Avoids reimplementing wallet harness internals without hard justification |
| Artifact schema with explicit gate effect metadata | Makes triage and release decisions explicit and auditable |
| Determinism + replay contract (seed + transcript hash) | Turns intermittent failures into reproducible engineering defects |

## 2. Scope and Guardrails

In scope:
1. Chromium E2E release gate for parity flows using `ethereum-wallet-mock`:
   - `eth_requestAccounts`
   - `personal_sign`
   - `eth_signTypedData_v4`
   - `eth_sendTransaction`
   - `accountsChanged` recovery
   - `chainChanged` recovery
2. CI hard-gate enforcement for deterministic lane only.
3. Priority closure of build/sign/share parity capabilities required for daily Rusty Safe usage:
   - `PARITY-TX-01` tx lifecycle (`create/hash/sign/propose/confirm/execute`)
   - `PARITY-TX-02` manual signature merge paths
   - `PARITY-ABI-01` ABI-assisted tx composition safety
   - `PARITY-COLLAB-01` import/export/share compatibility and integrity
4. Determinism hardening for wallet-mock lane:
   - deterministic seed injection and capture
   - scenario state-reset verification
   - outbound network policy enforcement for E2E runs

Out of scope:
1. MetaMask and Rabby automation requirements for 05A release.
2. Manual real-wallet RC sign-off requirements for 05A release.
3. Hardware passthrough acceptance (Ledger/Trezor) in 05A.
4. New connector ecosystem expansion or non-parity features.
5. Direct native HID browser integration in this wave.
6. Wallet-mock counterexample fuzz hardening and promotion gates (tracked in `prds/05B-WALLET-MOCK-FUZZ-HARDENING-PLAN.md`).

Anti-feature-creep policy:
1. Every task must map to `PARITY-*` and `C5`.
2. Non-parity additions require explicit PRD delta marked `parity-critical`.
3. Deferred real-wallet/hardware work goes to `prds/05A-E2E-REAL-WALLET-HARDWARE-TRACK.md`.
4. Test-harness improvements are allowed only if they reduce false negatives or improve reproducibility for existing `PARITY-*` flows.
5. Fuzz hardening additions go only in `prds/05B-WALLET-MOCK-FUZZ-HARDENING-PLAN.md` and are non-blocking for 05A release.

## 3. Target End State (Definition of Done for C5)

`C5` is complete only when all conditions are true:
1. `G-D1` deterministic release gate is green for `WM-PARITY-001..006` (blocking lane).
2. Deterministic reliability SLO is met:
   - Local: >= 95% pass over 20 consecutive runs.
   - CI: >= 99% pass over 50 scheduled runs.
3. Blocking scenario/gate runtime budgets are met with reproducible evidence.
4. Differential parity report is green for mandatory `PARITY-*` IDs.
5. All failures are classified (`ENV_BLOCKER|HARNESS_FAIL|APP_FAIL`) with reproducible artifacts.
6. Required checks in `prds/05A-RELEASE-GATE-CHECKLIST.md` are green.
7. Real-wallet/hardware tracks are explicitly marked out-of-scope for 05A and linked to `prds/05A-E2E-REAL-WALLET-HARDWARE-TRACK.md`.
8. Determinism contract is green:
   - same seed + same commit => stable transcript hash for deterministic scenarios,
   - scenario isolation checks prove no cross-test state leakage.
9. Replay contract is green:
   - every failed blocking run includes enough metadata to replay with one command.
10. Fuzz hardening is explicitly deferred and tracked in `prds/05B-WALLET-MOCK-FUZZ-HARDENING-PLAN.md`.

## 4. Core Architecture

System view:

```text
RustySafe Test Target (localhost)
          |
          v
Scenario Runner (Playwright)
          |
          +--> Determinism Guard
          |      - seed injection (`PRD05A_E2E_SEED`)
          |      - scenario reset/isolation checks
          |      - outbound network policy checks
          |
          +--> Control Plane (WalletDriver)
          |      - WalletMockDriver (blocking gate)
          |
          +--> Assertion Plane
          |      - EIP-1193 result assertions
          |      - UI state assertions
          |      - event recovery assertions
          |
          +--> Evidence Plane
                 - run JSON (`c5e2e-v1`)
                 - markdown summary
                 - trace/screenshot/log references
                 - gate effect marker (`BLOCKING`)
                 - transcript hash + replay command
```

Design principles:
1. Deterministic correctness is the release gate.
2. Keep driver-specific mechanics isolated from parity scenario definitions.
3. Fail closed for blocking lane.
4. Reuse existing libraries first; reimplement only for proven parity-critical gaps.
5. Keep scope strict to localsafe parity and avoid connector feature creep.
6. A failing run without replay metadata is an invalid test run.

Data flow:
1. Runner selects `gate_tier=blocking`.
2. Determinism Guard applies seed, resets scenario state, and validates network policy.
3. Control Plane boots selected driver and validates runtime profile.
4. Scenario executes app action + EIP-1193 request path.
5. Driver handles approval/rejection flow (or mock response in deterministic lane).
6. Assertion Plane validates parity outcomes and state transitions.
7. Evidence Plane writes artifacts with taxonomy, transcript hash, and replay command.

## 5. Reuse-First Policy (Do Not Reimplement Without Justification)

Rules:
1. Use `@synthetixio/ethereum-wallet-mock` directly for deterministic release-gate flows.
2. Wrapper code is allowed only for interface normalization (`WalletDriver`) and diagnostics.
3. Every workaround must include:
   - upstream/internal issue reference,
   - owner,
   - removal criteria,
   - review date.
4. Forking `ethereum-wallet-mock` is prohibited unless a blocker is proven and approved with a time-boxed rollback plan.

Reuse matrix:

| Concern | Preferred Source | Allowed Custom Layer |
|---|---|---|
| Deterministic wallet behavior | `@synthetixio/ethereum-wallet-mock` | thin adapter only |
| Scenario runner | Playwright | custom fixtures/manifests |
| Evidence formatting | local scripts | JSON schema writer |

## 6. Data Contracts

`WalletDriver` contract (required methods):
1. `bootstrapWallet()`
2. `connectToDapp()`
3. `approveSignature()`
4. `approveTransaction()`
5. `approveNetworkChange()`
6. `recoverFromFailure()`
7. `collectWalletDiagnostics()`

Scenario manifest contract:
1. `scenario_id` (e.g., `WM-PARITY-003`)
2. `parity_ids` (one or more `PARITY-*`)
3. `driver_mode` (`wallet-mock`)
4. `gate_tier` (`blocking`)
5. `steps`
6. `assertions`
7. `timeouts_ms`
8. `release_gate_driver` (`wallet-mock` for C5)
9. `seed` (deterministic replay seed)
10. `state_reset_mode` (`strict`)
11. `network_policy` (`local-only|allowlist`)

Evidence record contract:
1. `schema_version` (`c5e2e-v1`)
2. `status` (`PASS|FAIL|BLOCKED`)
3. `taxonomy` (`ENV_BLOCKER|HARNESS_FAIL|APP_FAIL`)
4. `gate_effect` (`BLOCKING`)
5. `driver`
6. `wallet_version`
7. `browser_version`
8. `run_id`
9. `artifacts` (`log`, `trace`, `screenshots`, `report`)
10. `reproducer_cmd`
11. `seed`
12. `transcript_sha256`
13. `wasm_artifact_sha256`
14. `state_reset_verified`
15. `network_policy_violations`

## 7. Test Inventory (Parity-Aligned)

Mandatory blocking parity scenarios (`wallet-mock`):
1. `WM-PARITY-001`: connect via `eth_requestAccounts`.
2. `WM-PARITY-002`: message signing via `personal_sign`.
3. `WM-PARITY-003`: typed data signing via `eth_signTypedData_v4`.
4. `WM-PARITY-004`: transaction send via `eth_sendTransaction`.
5. `WM-PARITY-005`: deterministic `accountsChanged` recovery.
6. `WM-PARITY-006`: deterministic `chainChanged` recovery.

Blocking harness determinism scenarios:
1. `WM-HARNESS-001`: profile/runtime preflight validation.
2. `WM-HARNESS-002`: schema+artifact completeness.
3. `WM-HARNESS-003`: taxonomy correctness on injected failures.
4. `WM-HARNESS-004`: strict scenario isolation (no cross-test state leakage).
5. `WM-HARNESS-005`: same seed replay produces stable transcript hash.

Blocking build/sign/share closure scenarios (priority add-on):
1. `WM-BSS-001`: tx flow `create -> sign -> propose -> confirm -> execute` is deterministic.
2. `WM-BSS-002`: ABI-assisted create path includes selector mismatch guard and explicit override handling.
3. `WM-BSS-003`: manual signature add/merge path is deterministic and idempotent.
4. `WM-BSS-004`: export/import bundle path validates digest/authenticity and deterministic merge semantics.
5. `WM-BSS-005`: localsafe URL keys (`importTx/importSig/importMsg/importMsgSig`) import successfully.
6. `WM-BSS-006`: tampered bundle (MAC/auth mismatch) is rejected and quarantined.
7. `WM-BSS-007`: interrupted flow resume (reload/reopen) preserves deterministic tx/message state.

Deferred scenarios:
1. Wallet-mock fuzz/promotion scenarios are tracked in `prds/05B-WALLET-MOCK-FUZZ-HARDENING-PLAN.md`.

## 8. Environment and Configuration Contract

Runtime profile requirements:
1. Deterministic gate (`wallet-mock`) may run headless or headed.
2. Node runtime is pinned to `v20.x` for all E2E commands.
3. Locale pinned to English for deterministic selector behavior.
4. Run headers must print app commit SHA, browser version, driver mode, and gate tier.
5. Every run must carry deterministic `seed` and expose it in artifacts.
6. Scenario reset is strict by default and must fail when residue is detected.

Required files:
1. `e2e/playwright.wallet-mock.config.ts` (blocking lane)
2. `e2e/tests/wallet-mock/*.mjs`
3. `scripts/run_prd05a_wallet_mock_gate.sh`
4. `scripts/run_prd05a_wallet_mock_soak.sh`
5. `scripts/run_prd05a_release_evidence.sh`
6. `scripts/run_prd05a_wallet_mock_runtime_slo.sh`
7. `scripts/run_prd05a_wallet_mock_determinism.sh`
8. `scripts/run_prd05a_wallet_mock_replay.sh`

Deferred reference:
1. `prds/05B-WALLET-MOCK-FUZZ-HARDENING-PLAN.md`

Required environment variables:
1. `PRD05A_NODE_BIN`
2. `PRD05A_E2E_BASE_URL`
3. `PRD05A_E2E_SKIP_WEBSERVER`
4. `PRD05A_GATE_MODE` (`blocking`)
5. `PRD05A_E2E_SEED`
6. `PRD05A_NETWORK_POLICY` (`local-only|allowlist`)

## 9. Storage, Artifacts, and Reporting

Artifact locations:
1. `local/reports/prd05a/C5-wallet-mock-gate-report.md`
2. `local/reports/prd05a/C5-wallet-mock-gate.json`
3. `local/reports/prd05a/C5-wallet-mock-soak-report.md`
4. `local/reports/prd05a/C5-wallet-mock-runtime-slo-report.md`
5. `local/reports/prd05a/C5-release-evidence-index.md`
6. `local/reports/prd05a/C5-wallet-mock-determinism-report.md`
7. `local/reports/prd05a/C5-wallet-mock-replay-report.md`

Reporting requirements:
1. Every run writes markdown summary and machine-readable JSON.
2. Failed runs include taxonomy, reproducible command, and trace path.
3. Release evidence index links all phase reports and lists open blockers.
4. Determinism report must include seed, transcript hash, and state-reset verification result.
5. Fuzz hardening reports are deferred to `prds/05B-WALLET-MOCK-FUZZ-HARDENING-PLAN.md`.

## 10. Implementation Roadmap (E0-E7)

| Phase | Objective | Complexity | Branch | Status |
|---|---|---|---|---|
| E0 | Deterministic runtime baseline and evidence schema | M | `feat/prd05a-e2e-e0-determinism` | Completed |
| E1 | Blocking wallet-mock parity suite | M | `feat/prd05a-e2e-e1-wallet-mock-gate` | Completed |
| E5 | CI hard gates, SLO policy, release readiness | M | `feat/prd05a-e2e-e5-ci-release-gate` | Completed |
| E6 | Determinism hardening (seed contract + state isolation + network policy) | M | `feat/prd05a-e2e-e6-determinism-hardening` | Planned |
| E7 | Replay automation + flake-budget enforcement | M | `feat/prd05a-e2e-e7-replay-flake-budget` | Planned |

Dependency order:
1. Completed path: `E0 -> E1 -> E5`.
2. Hardening path: `E6 -> E7`.
3. Fuzz hardening executes outside 05A in `prds/05B-WALLET-MOCK-FUZZ-HARDENING-PLAN.md`.
4. Real-wallet and hardware tracks execute outside 05A in `prds/05A-E2E-REAL-WALLET-HARDWARE-TRACK.md`.

## 11. Detailed Task List (Structured and Measurable)

### E0 Tasks
1. `E0-T1` enforce Node `v20` pin and runtime self-check.
2. `E0-T2` standardize run header metadata output (`gate_tier`, `driver_mode`, `wallet_version`).
3. `E0-T3` enforce artifact schema `c5e2e-v1` with JSON validation.
4. `E0-T4` add deterministic environment preflight tests.

E0 Gate:
1. 100% pass for preflight checks over 10 local runs.
2. No run starts without required metadata fields.

### E1 Tasks
1. `E1-T1` define `WalletDriver` contract with `wallet-mock` implementation.
2. `E1-T2` implement `WM-PARITY-001..006`.
3. `E1-T3` add deterministic negative-path assertions (rejects, chain mismatch, timeout handling).
4. `E1-T4` wire blocking gate script and CI check.

E1 Gate:
1. `WM-PARITY-001..006` all green in blocking lane.
2. Driver contract tests are green with no coverage regression.

### E5 Tasks
1. `E5-T1` enforce blocking gate in PR CI (`wallet-mock` only).
2. `E5-T2` add scheduled blocking soak (50-run window) and threshold checks.
3. `E5-T3` aggregate release evidence index and checklist links.
4. `E5-T4` enforce branch naming and `E*-T*` commit traceability in CI.

E5 Gate:
1. Blocking SLO thresholds are met.
2. Release checklist C5 section is fully green.

### E6 Tasks
1. `E6-T1` add deterministic seed contract to manifest + runner + evidence output.
2. `E6-T2` add strict scenario reset checks and hard-fail on state leakage.
3. `E6-T3` add outbound network policy check (`local-only|allowlist`) for blocking lane.
4. `E6-T4` add harness tests for `WM-HARNESS-004..005`.

E6 Gate:
1. Re-running the same deterministic suite with same seed across 20 runs yields stable transcript hash for deterministic scenarios.
2. Zero state-leak violations over 20-run deterministic loop.
3. Zero unapproved outbound network calls in blocking lane.

### E7 Tasks
1. `E7-T1` implement one-command replay from failed JSON artifact.
2. `E7-T2` wire replay verification script into CI artifacts on failure.
3. `E7-T3` enforce flake budget by failure class for blocking lane.
4. `E7-T4` publish failure trend report with reproducibility SLA.

E7 Gate:
1. 100% of blocking failures include executable replay command and required artifact set.
2. `HARNESS_FAIL` rate <= 1% over 100-run soak.
3. Mean time to reproduce failure <= 10 minutes using replay command.

## 12. Success Criteria and Gates

Functional criteria (blocking):
1. 100% pass for `WM-PARITY-001..006`.
2. 100% classification coverage for failed blocking runs.
3. 100% pass for `WM-BSS-001..007` in blocking lane.

Reliability criteria:
1. Blocking local: >= 95% pass over 20 consecutive runs.
2. Blocking CI: >= 99% pass over 50 scheduled runs.
3. `HARNESS_FAIL` <= 1% over 100-run soak.

Scope criteria:
1. C5 is deterministic wallet-mock parity focused with no connector ecosystem expansion.
2. Real-wallet and hardware acceptance are moved out of 05A scope to `prds/05A-E2E-REAL-WALLET-HARDWARE-TRACK.md`.

Performance criteria:
1. Blocking scenario p95 runtime <= 90s.
2. Blocking PR gate p95 runtime <= 15 minutes.

Determinism criteria:
1. Same commit + same seed => stable transcript hash for deterministic scenarios.
2. State-reset contract passes with zero leakage across 20 consecutive runs.
3. No network policy violations in blocking lane.

Operational criteria:
1. Mean time to classify failure <= 10 minutes.
2. Mean time to reproduce failure <= 10 minutes via recorded replay command/profile.

## 13. Failure Taxonomy and Recovery Strategy

Taxonomy:
1. `ENV_BLOCKER`: missing binaries, browser install, runner config.
2. `HARNESS_FAIL`: runner/fixture/driver orchestration failure.
3. `APP_FAIL`: Rusty Safe behavior diverges from parity expectation.

Recovery actions:
1. `ENV_BLOCKER`: fail fast; mark run `BLOCKED`; include remediation.
2. `HARNESS_FAIL` in blocking lane: one bounded retry with diagnostics, then hard fail.
3. `APP_FAIL` in blocking lane: no retry; hard fail and block merge.

## 14. CI/API Surface

Required commands:
1. `scripts/run_prd05a_wallet_mock_gate.sh`
2. `scripts/run_prd05a_wallet_mock_soak.sh`
3. `scripts/run_prd05a_wallet_mock_runtime_slo.sh`
4. `scripts/run_prd05a_release_evidence.sh`
5. `scripts/run_prd05a_wallet_mock_determinism.sh`
6. `scripts/run_prd05a_wallet_mock_replay.sh`

Command output contract:
1. Exit code `0` only on blocking-gate success.
2. Non-zero exits must still emit markdown and JSON artifacts.
3. Output includes:
   - `schema_version=c5e2e-v1`
   - `gate_tier`
   - `driver_mode`
   - `seed`
   - `transcript_sha256`
   - `taxonomy_summary`
   - `artifact_index`

CI cadence contract:
1. Pull request: blocking wallet-mock gate + 5-run mini soak.
2. Daily scheduled: blocking soak.
3. Weekly scheduled: 100-run soak with flake-budget evaluation.
4. Fuzz soak scheduling is tracked in `prds/05B-WALLET-MOCK-FUZZ-HARDENING-PLAN.md` and is non-blocking for 05A.

## 15. Risks, Trade-offs, and Mitigations

Risk: deterministic mock may miss wallet-extension-specific issues.
Mitigation: those risks are intentionally tracked in `prds/05A-E2E-REAL-WALLET-HARDWARE-TRACK.md`, not blocked in 05A.

Risk: maintaining multiple lanes increases complexity.
Mitigation: keep 05A to single blocking lane and move additional lanes to separate plan.

Risk: stricter determinism checks can block merges for harness defects unrelated to app logic.
Mitigation: enforce clear taxonomy + replay evidence so harness failures are quickly isolated and fixed.

Trade-off:
1. Blocking on deterministic lane improves reliability and velocity.
2. Real-wallet/hardware confidence shifts to follow-on plan to prevent 05A release coupling.
3. Strong determinism constraints increase short-term harness work but reduce long-term CI noise.
4. Fuzz hardening value is preserved but intentionally deferred to 05B to avoid over-coupling 05A release gates.

## 16. Branching, Commits, and Tags

Branch policy:
1. One branch per phase (`E0`, `E1`, `E5`, `E6`, `E7`) for active 05A scope.
2. Merge by dependency order for blocking path: `E0 -> E1 -> E5 -> E6 -> E7`.
3. Branch names must match `feat/prd05a-e2e-e<phase>-<slug>`.

Commit policy:
1. Commit at least once per completed task (`E*-T*`).
2. Add one explicit `-gate-green` commit per phase with linked evidence.
3. Every phase commit message must reference one or more `E*-T*` task IDs.

Tag policy:
1. Tag each green phase: `prd05a-e2e-e<phase>-gate`.
2. Final release candidate tag only after `E7` gate and checklist sign-off.

## 17. Immediate Next Actions

1. Keep `wallet-mock` as the only blocking lane.
2. Complete determinism hardening (`E6-T1..T4`) before any additional test expansion.
3. Complete replay/flake-budget automation (`E7-T1..T4`) so failures are reproducible by default.
4. Keep scope lock to localsafe parity IDs and reject any non-parity additions in C5.
5. Close remaining C5 release checklist items with explicit evidence updates.
6. Start 05B fuzz hardening only after 05A gates are green.

## 18. Priority Backlog (Build, Sign, Share)

Priority model:
1. `P0` means release-blocking for the core product goal (build/sign/share Safe transactions).
2. `P1` means reliability and confidence for release operation.
3. `P2` means deferred/non-blocking hardening.

### P0 (Release-Blocking)

1. `P0-1` Bundle authenticity parity:
   - Replace placeholder export signer/signature behavior with real signer-backed semantics.
   - Anchor: `crates/rusty-safe-signing-adapters/src/queue.rs`.
   - Gate: tamper/auth vectors pass; recovered exporter matches expected signer; invalid bundles are quarantined.
2. `P0-2` Tx lifecycle correctness:
   - Ensure deterministic `create/hash/sign/propose/confirm/execute` flow with idempotency and conflict safety.
   - Anchors: `crates/rusty-safe-signing-core/src/orchestrator.rs`, `crates/rusty-safe-signing-adapters/src/safe_service.rs`.
   - Gate: tx E2E pass with no duplicate side effects.
3. `P0-3` Manual signature parity:
   - Ensure manual signature add/merge is deterministic, idempotent, and signer-validated.
   - Anchor: `crates/rusty-safe-signing-adapters/tests/tx_manual_signature.rs`.
   - Gate: duplicate/invalid/recovery vectors all pass.
4. `P0-4` ABI-safe tx composition:
   - Enforce selector mismatch warning/ack behavior and deterministic encoding.
   - Anchor: `crates/rusty-safe-signing-adapters/tests/abi_builder.rs`.
   - Gate: ABI vectors green with explicit mismatch rejection path.
5. `P0-5` Import/export/share compatibility:
   - Keep deterministic merge and localsafe URL key compatibility.
   - Anchors: `crates/rusty-safe-signing-adapters/tests/import_export_merge.rs`, `crates/rusty-safe-signing-adapters/tests/url_import_compat.rs`.
   - Gate: `PARITY-COLLAB-01` vectors pass including `importTx/importSig/importMsg/importMsgSig`.
6. `P0-6` Build/sign/share blocking E2E lane:
   - Add `WM-BSS-001..007` to the wallet-mock blocking suite.
   - Anchors: `e2e/tests/wallet-mock/scenario-manifest.mjs`, `e2e/tests/wallet-mock/wallet-mock-eip1193.spec.mjs`.
   - Gate: 100% pass on `WM-PARITY-*` + `WM-BSS-*`.
7. `P0-7` Determinism contract hardening:
   - Enforce seed, state-reset isolation, and network policy checks for blocking lane.
   - Anchors: `e2e/tests/wallet-mock/runtime-profile-check.mjs`, `e2e/tests/wallet-mock/scenario-manifest.mjs`.
   - Gate: determinism report green with stable transcript hashes.

### P1 (Operational Confidence)

1. `P1-1` CI reliability closure:
   - Demonstrate `>=99%` pass over scheduled 50-run blocking soak.
2. `P1-2` Blocking performance closure:
   - Meet p95 scenario and gate runtime budgets.
3. `P1-3` Differential parity guardrail:
   - Keep localsafe fixture differential report green for mandatory `PARITY-*` flows.
4. `P1-4` Replayability closure:
   - Ensure all blocking failures are reproducible via a single replay command from artifacts.

### P2 (Deferred/Non-Blocking)

1. `P2-1` Real-wallet compatibility and canary matrix in `prds/05A-E2E-REAL-WALLET-HARDWARE-TRACK.md`.
2. `P2-2` Hardware passthrough acceptance (`H1`) in `prds/05A-E2E-REAL-WALLET-HARDWARE-TRACK.md`.
3. `P2-3` Wallet-mock counterexample fuzz hardening in `prds/05B-WALLET-MOCK-FUZZ-HARDENING-PLAN.md`.

## 19. Milestone Execution Order (Now)

1. `M1` Bundle + collaboration integrity closure (`P0-1`, `P0-5`) - `Completed 2026-02-23`:
   - Branch: `feat/prd05a-e2e-m1-bundle-collab`
   - Exit gate: auth + merge + URL compatibility vectors all green.
2. `M2` Tx/signing correctness closure (`P0-2`, `P0-3`, `P0-4`) - `Completed 2026-02-23`:
   - Branch: `feat/prd05a-e2e-m2-tx-sign-core`
   - Exit gate: tx/manual/ABI suites green with deterministic replay.
3. `M3` Blocking E2E build/sign/share closure (`P0-6`) - `Completed 2026-02-23`:
   - Branch: `feat/prd05a-e2e-m3-wallet-mock-bss`
   - Exit gate: `WM-PARITY-*` + baseline `WM-BSS-001..006` all green locally and in PR gate.
4. `M4` Release confidence closure (`P1-1`..`P1-3`) - `Completed 2026-02-23`:
   - Branch: `feat/prd05a-e2e-m4-release-confidence`
   - Exit gate: reliability/performance/differential/release-evidence items green for blocking wallet-mock lane.
   - Evidence:
     - `local/reports/prd05a/soak-wallet-mock/run-20260223T173417Z` (50/50 daily soak baseline).
     - `local/reports/prd05a/C5-wallet-mock-runtime-slo-report.md` (CI SLO + runtime p95 budgets).
     - `local/reports/prd05a/C6-performance-report.md`.
     - `local/reports/prd05a/C9-differential-parity-report.md`.
     - `local/reports/prd05a/C5-release-evidence-index.md`.
5. `M5` Determinism contract closure (`P0-7`, `E6-T1..T4`) - `Planned`:
   - Branch: `feat/prd05a-e2e-m5-determinism-contract`
   - Exit gate: seed/transcript/state-reset/network-policy evidence all green.
6. `M6` Replay and flake-budget closure (`P1-4`, `E7-T1..T4`) - `Planned`:
   - Branch: `feat/prd05a-e2e-m6-replay-flake-budget`
   - Exit gate: replay coverage 100% and harness-fail budget <= 1% over 100-run soak.

Commit and tag discipline:
1. Commit at task boundaries with task IDs in commit subject/body.
2. Add one explicit `-gate-green` commit at each milestone close.
3. Tag milestones as `prd05a-e2e-m<index>-gate`.

## 20. Post-M4 Execution Order

1. Complete `M5` determinism contract closure for wallet-mock.
2. Complete `M6` replay and flake-budget closure for wallet-mock.
3. Keep 05A scoped to deterministic wallet-mock release gating.
4. Execute real-wallet compatibility plan in `prds/05A-E2E-REAL-WALLET-HARDWARE-TRACK.md` as non-blocking follow-on work.
5. Execute deferred hardware passthrough track (`H1`) in `prds/05A-E2E-REAL-WALLET-HARDWARE-TRACK.md` as non-blocking follow-on work.
6. Execute deferred wallet-mock fuzz hardening in `prds/05B-WALLET-MOCK-FUZZ-HARDENING-PLAN.md` as follow-on work.
