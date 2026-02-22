# PRD 05A E2E Wallet Runtime Plan (Deterministic Gate + Real-Wallet Sanity + Canary)

Status: Draft (Revised)  
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
2. Add manual real-wallet (MetaMask hot-wallet) sanity validation as required release evidence.
3. Keep automated MetaMask/Rabby runs as non-blocking nightly canary lanes for drift detection and triage.
4. Keep implementation staged (`E0-E5`) with hard milestones, branch/commit discipline, and measurable SLOs.

Key innovations:

| Innovation | Why it matters |
|---|---|
| Two-lane acceptance model (blocking deterministic + non-blocking real-wallet canary) | Unblocks delivery while still tracking real-wallet risk |
| Contract-tested `WalletDriver` + `gate_tier` manifest metadata | Prevents driver details from leaking into parity assertions |
| Reuse-first policy | Avoids reimplementing Synpress/dappwright internals without hard justification |
| Mandatory manual sanity evidence | Guarantees at least one real-wallet validation before release |
| Artifact schema with gate effect (`BLOCKING` vs `CANARY`) | Makes triage and release decisions explicit and auditable |

## 2. Scope and Guardrails

In scope:
1. Chromium E2E release gate for parity flows using `ethereum-wallet-mock`:
   - `eth_requestAccounts`
   - `personal_sign`
   - `eth_signTypedData_v4`
   - `eth_sendTransaction`
   - `accountsChanged` recovery
   - `chainChanged` recovery
2. Manual MetaMask hot-wallet sanity flow evidence before release candidate sign-off.
3. Non-blocking nightly canary automation for MetaMask (required) and Rabby (targeted).
4. CI hard-gate enforcement for deterministic lane only.

Out of scope:
1. Making MetaMask automation a blocking CI gate in C5.
2. Hardware passthrough acceptance (Ledger/Trezor) for C5.
3. New connector ecosystem expansion or non-parity features.
4. Direct native HID browser integration in this wave.

Anti-feature-creep policy:
1. Every task must map to `PARITY-*` and `C5`.
2. Non-parity additions require explicit PRD delta marked `parity-critical`.
3. Deferred non-parity/hardening work goes to `prds/05B-PRD-HARDENING-WAVE.md`.

Deferred track policy:
1. Hardware passthrough acceptance (`H1`) is deferred until C5 hot-wallet objectives are complete.
2. Hardware evidence is non-blocking for C5 release.
3. `H1` owner: Security lead.
4. `H1` target: `E5` gate date + 14 calendar days.

## 3. Target End State (Definition of Done for C5)

`C5` is complete only when all conditions are true:
1. `G-D1` deterministic release gate is green for `WM-PARITY-001..006` (blocking lane).
2. Deterministic reliability SLO is met:
   - Local: >= 95% pass over 20 consecutive runs.
   - CI: >= 99% pass over 50 scheduled runs.
3. Manual MetaMask sanity checklist is green for release candidate:
   - connect
   - `personal_sign`
   - `eth_signTypedData_v4`
   - `eth_sendTransaction`
4. MetaMask canary report exists for last 5 nightly runs with taxonomy and repro commands.
5. If Rabby canary is enabled, report exists with same artifact standard.
6. All failures are classified (`ENV_BLOCKER|HARNESS_FAIL|APP_FAIL|WALLET_FAIL`) with reproducible artifacts.
7. `prds/05A-RELEASE-GATE-CHECKLIST.md` C5 checks are fully green.
8. Deferred hardware track (`H1`) is documented and marked non-blocking.

## 4. Core Architecture

System view:

```text
RustySafe Test Target (localhost)
          |
          v
Scenario Runner (Playwright)
          |
          +--> Control Plane (WalletDriver)
          |      - WalletMockDriver (blocking gate)
          |      - SynpressMetaMaskDriver (canary lane)
          |      - DappwrightDriver (optional canary comparator)
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
                 - gate effect marker
```

Design principles:
1. Deterministic correctness is the release gate.
2. Real-wallet automation is observability/canary, not release-critical for C5.
3. Manual real-wallet sanity is mandatory release evidence.
4. Keep driver-specific mechanics isolated from parity scenario definitions.
5. Fail closed for blocking lane; fail open with escalation for canary lane.
6. Reuse existing libraries first; reimplement only for proven parity-critical gaps.
7. Keep scope strict to localsafe parity and avoid connector feature creep.

Data flow:
1. Runner selects `gate_tier` (`blocking|canary|manual`).
2. Control Plane boots selected driver and validates runtime profile.
3. Scenario executes app action + EIP-1193 request path.
4. Driver handles approval/rejection flow (or mock response in deterministic lane).
5. Assertion Plane validates parity outcomes and state transitions.
6. Evidence Plane writes artifacts with taxonomy and gate effect.

## 5. Reuse-First Policy (Do Not Reimplement Without Justification)

Rules:
1. Use `@synthetixio/ethereum-wallet-mock` directly for deterministic release-gate flows.
2. Use Synpress/dappwright APIs directly for MetaMask/Rabby canary flows where possible.
3. Wrapper code is allowed only for interface normalization (`WalletDriver`) and diagnostics.
4. Custom popup automation is allowed only when parity-critical gap is proven and cannot be solved upstream in time.
5. Every workaround must include:
   - upstream/internal issue reference,
   - owner,
   - removal criteria,
   - review date.

Reuse matrix:

| Concern | Preferred Source | Allowed Custom Layer |
|---|---|---|
| Deterministic wallet behavior | `@synthetixio/ethereum-wallet-mock` | thin adapter only |
| MetaMask bootstrap/approve | Synpress/dappwright | diagnostics wrapper only |
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
3. `driver_mode` (`wallet-mock|synpress|dappwright|manual`)
4. `gate_tier` (`blocking|canary|manual`)
5. `steps`
6. `assertions`
7. `timeouts_ms`
8. `release_gate_driver` (`wallet-mock` for C5)

Evidence record contract:
1. `schema_version` (`c5e2e-v1`)
2. `status` (`PASS|FAIL|BLOCKED`)
3. `taxonomy` (`ENV_BLOCKER|HARNESS_FAIL|APP_FAIL|WALLET_FAIL`)
4. `gate_effect` (`BLOCKING|CANARY|MANUAL`)
5. `driver`
6. `wallet_version`
7. `browser_version`
8. `run_id`
9. `artifacts` (`log`, `trace`, `screenshots`, `report`)
10. `reproducer_cmd`

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

Non-blocking MetaMask canary scenarios:
1. `MM-CANARY-001`: connect + `personal_sign` happy path.
2. `MM-CANARY-002`: `eth_signTypedData_v4` happy path.
3. `MM-CANARY-003`: `eth_sendTransaction` happy path.

Non-blocking Rabby canary scenarios (if enabled):
1. `RB-CANARY-001`: connect + `personal_sign`.
2. `RB-CANARY-002`: typed data signing.

Manual release sanity scenarios:
1. `MANUAL-MM-001`: connect.
2. `MANUAL-MM-002`: `personal_sign`.
3. `MANUAL-MM-003`: `eth_signTypedData_v4`.
4. `MANUAL-MM-004`: `eth_sendTransaction`.

Deferred hardware scenarios (post `E5`):
1. `MATRIX-HW-001`: Ledger passthrough smoke via wallet software.
2. `MATRIX-HW-002`: Trezor passthrough smoke via wallet software.

## 8. Environment and Configuration Contract

Runtime profile requirements:
1. Deterministic gate (`wallet-mock`) may run headless or headed.
2. Extension canary runs must run headed Chromium; CI wraps with `xvfb-run --auto-servernum`.
3. Node runtime is pinned to `v20.x` for all E2E commands.
4. Locale pinned to English for extension-based selectors.
5. Run headers must print app commit SHA, browser version, driver mode, gate tier, and wallet version.

Required files:
1. `e2e/playwright.config.ts` (blocking lane)
2. `e2e/playwright.metamask.config.ts` (canary lane)
3. `e2e/tests/wallet-mock/*.mjs`
4. `e2e/tests/metamask-canary/*.mjs`
5. `scripts/run_prd05a_wallet_mock_gate.sh`
6. `scripts/run_prd05a_wallet_mock_soak.sh`
7. `scripts/run_prd05a_metamask_canary.sh`
8. `scripts/run_prd05a_release_evidence.sh`
9. `scripts/run_prd05a_manual_metamask_checklist.sh`

Required environment variables:
1. `PRD05A_NODE_BIN`
2. `PRD05A_E2E_BASE_URL`
3. `PRD05A_E2E_SKIP_WEBSERVER`
4. `PRD05A_GATE_MODE` (`blocking|canary|manual`)
5. `PRD05A_METAMASK_PASSWORD` (manual/canary only)
6. `PRD05A_METAMASK_SEED` (manual/canary only where policy allows)

## 9. Storage, Artifacts, and Reporting

Artifact locations:
1. `local/reports/prd05a/C5-wallet-mock-gate-report.md`
2. `local/reports/prd05a/C5-wallet-mock-gate.json`
3. `local/reports/prd05a/C5-wallet-mock-soak-report.md`
4. `local/reports/prd05a/C5-metamask-canary-report.md`
5. `local/reports/prd05a/C5-rabby-canary-report.md` (if enabled)
6. `local/reports/prd05a/C5-manual-metamask-sanity.md`
7. `local/reports/prd05a/C5-release-evidence-index.md`

Deferred hardware artifact:
1. `local/reports/prd05a/C5-hardware-passthrough-smoke.md` (post `E5`, non-blocking for C5)

Reporting requirements:
1. Every run writes markdown summary and machine-readable JSON.
2. Failed runs include taxonomy, reproducible command, and trace path.
3. Release evidence index links all phase reports and lists open blockers.

## 10. Implementation Roadmap (E0-E5)

| Phase | Objective | Complexity | Branch |
|---|---|---|---|
| E0 | Deterministic runtime baseline and evidence schema | M | `feat/prd05a-e2e-e0-determinism` |
| E1 | Blocking wallet-mock parity suite | M | `feat/prd05a-e2e-e1-wallet-mock-gate` |
| E2 | Manual MetaMask release sanity workflow | S | `feat/prd05a-e2e-e2-manual-sanity` |
| E3 | MetaMask non-blocking canary lane | M | `feat/prd05a-e2e-e3-metamask-canary` |
| E4 | Rabby canary + optional dappwright comparison | M | `feat/prd05a-e2e-e4-rabby-canary` |
| E5 | CI hard gates, SLO policy, release readiness | M | `feat/prd05a-e2e-e5-ci-release-gate` |

Dependency order:
1. `E0 -> E1 -> E2 -> E5` is the minimum path to C5 release readiness.
2. `E3` runs in parallel after `E0` and feeds canary evidence.
3. `E4` is parallel/optional for C5 release but required for full hot-wallet matrix completeness target.

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

### E2 Tasks
1. `E2-T1` define manual MetaMask release checklist with exact steps and evidence capture format.
2. `E2-T2` implement helper script for checklist scaffolding and artifact templates.
3. `E2-T3` wire checklist requirement into `prds/05A-RELEASE-GATE-CHECKLIST.md`.

E2 Gate:
1. Manual checklist template is executable and repeatable by another engineer.
2. Release checklist includes hard requirement for `MANUAL-MM-001..004`.

### E3 Tasks
1. `E3-T1` implement `MM-CANARY-001..003` with Synpress as primary canary driver.
2. `E3-T2` add canary failure diagnostics and taxonomy mapping.
3. `E3-T3` publish nightly canary summary report and trend view.

E3 Gate:
1. Nightly MetaMask canary job produces artifacts for 5 consecutive days.
2. Canary failures are triaged with issue links within one business day.

### E4 Tasks
1. `E4-T1` implement Rabby canary scenarios (`RB-CANARY-001..002`) if connector path exists.
2. `E4-T2` optionally run dappwright side-by-side for MetaMask canary reliability comparison.
3. `E4-T3` publish hot-wallet canary matrix summary (MetaMask + Rabby).
4. `E4-T4` document deferred hardware track `H1` owner and target date.

E4 Gate:
1. Matrix report exists with PASS/FAIL and repro command per row.
2. Deferred hardware track is documented and marked non-blocking for C5.

### E5 Tasks
1. `E5-T1` enforce blocking gate in PR CI (`wallet-mock` only).
2. `E5-T2` add scheduled blocking soak (50-run window) and threshold checks.
3. `E5-T3` enforce canary jobs as non-blocking but mandatory-reporting.
4. `E5-T4` aggregate release evidence index and checklist links.
5. `E5-T5` enforce branch naming and `E*-T*` commit traceability in CI.

E5 Gate:
1. Blocking SLO thresholds are met.
2. Manual sanity evidence is present for release candidate.
3. Release checklist C5 section is fully green.

## 12. Success Criteria and Gates

Functional criteria (blocking):
1. 100% pass for `WM-PARITY-001..006`.
2. 100% classification coverage for failed blocking runs.

Reliability criteria:
1. Blocking local: >= 95% pass over 20 consecutive runs.
2. Blocking CI: >= 99% pass over 50 scheduled runs.
3. Canary: trend reported; no hard pass threshold for C5 release blocking.

Scope criteria:
1. C5 is hot-wallet parity focused with no connector ecosystem expansion.
2. Hardware passthrough acceptance is explicitly deferred and non-blocking.

Performance criteria:
1. Blocking scenario p95 runtime <= 90s.
2. Blocking PR gate p95 runtime <= 15 minutes.
3. Nightly canary p95 runtime <= 30 minutes.

Operational criteria:
1. Mean time to classify failure <= 10 minutes.
2. Mean time to reproduce failure <= 20 minutes via recorded command/profile.
3. Canary-to-issue triage SLA <= 1 business day.

## 13. Failure Taxonomy and Recovery Strategy

Taxonomy:
1. `ENV_BLOCKER`: missing binaries, browser install, runner config.
2. `HARNESS_FAIL`: runner/fixture/driver orchestration failure.
3. `APP_FAIL`: Rusty Safe behavior diverges from parity expectation.
4. `WALLET_FAIL`: wallet extension/runtime failure independent of app behavior.

Recovery actions:
1. `ENV_BLOCKER`: fail fast; mark run `BLOCKED`; include remediation.
2. `HARNESS_FAIL` in blocking lane: one bounded retry with diagnostics, then hard fail.
3. `APP_FAIL` in blocking lane: no retry; hard fail and block merge.
4. `WALLET_FAIL` in canary lane: do not block merge; emit incident/report and trend.
5. `APP_FAIL` in canary lane: open high-priority issue; if reproducible in blocking/manual lane, immediately promote to blocker.

## 14. CI/API Surface

Required commands:
1. `scripts/run_prd05a_wallet_mock_gate.sh`
2. `scripts/run_prd05a_wallet_mock_soak.sh`
3. `scripts/run_prd05a_metamask_canary.sh`
4. `scripts/run_prd05a_compat_matrix.sh`
5. `scripts/run_prd05a_manual_metamask_checklist.sh`
6. `scripts/run_prd05a_release_evidence.sh`

Command output contract:
1. Exit code `0` only on blocking-gate success.
2. Non-zero exits must still emit markdown and JSON artifacts.
3. Output includes:
   - `schema_version=c5e2e-v1`
   - `gate_tier`
   - `driver_mode`
   - `taxonomy_summary`
   - `artifact_index`

CI cadence contract:
1. Pull request: blocking wallet-mock gate + 5-run mini soak.
2. Daily scheduled: blocking soak + MetaMask canary (and Rabby if enabled).
3. Release candidate: mandatory manual MetaMask sanity checklist evidence.

## 15. Risks, Trade-offs, and Mitigations

Risk: deterministic mock may miss wallet-extension-specific issues.  
Mitigation: mandatory manual sanity + nightly canary lane with escalation.

Risk: canary non-blocking policy could hide accumulating wallet drift.  
Mitigation: trend-based alerting and mandatory triage SLA.

Risk: maintaining multiple lanes increases complexity.  
Mitigation: strict `gate_tier` contracts and minimized custom driver code.

Trade-off:
1. Blocking on deterministic lane improves reliability and velocity, but shifts some real-wallet risk to canary/manual evidence.
2. Manual release sanity adds a small operational cost, but gives high-confidence real-wallet validation without CI flake.

## 16. Branching, Commits, and Tags

Branch policy:
1. One branch per phase (`E0-E5`).
2. Merge by dependency order for blocking path: `E0 -> E1 -> E2 -> E5`.
3. Branch names must match `feat/prd05a-e2e-e<phase>-<slug>`.

Commit policy:
1. Commit at least once per completed task (`E*-T*`).
2. Add one explicit `-gate-green` commit per phase with linked evidence.
3. Every phase commit message must reference one or more `E*-T*` task IDs.

Tag policy:
1. Tag each green phase: `prd05a-e2e-e<phase>-gate`.
2. Final release candidate tag only after `E5` gate and checklist sign-off.

## 17. Immediate Next Actions

1. Approve this revised two-lane plan as authoritative for C5.
2. Start `E0` and produce deterministic schema/profile evidence.
3. Implement `E1` blocking `wallet-mock` parity suite and gate scripts.
4. Implement `E2` manual MetaMask release sanity checklist artifacts.
5. Wire `E3` nightly MetaMask canary as non-blocking with taxonomy reporting.
6. Update `prds/05A-RELEASE-GATE-CHECKLIST.md` when each phase gate turns green.
