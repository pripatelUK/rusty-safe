# PRD 05A E2E Wallet Runtime Plan (MetaMask-First, Complete Gate)

Status: Draft  
Owner: Rusty Safe  
Parent PRD: `prds/05A-PRD-PARITY-WAVE.md`  
Continuation Milestone: `C5` in `prds/05A-CONTINUATION-MILESTONES.md`

## 1. Executive Summary

Problem statement:
1. Current MetaMask E2E is release-blocking due to nondeterministic onboarding state, popup lifecycle races, and extension startup failures.
2. Existing pass/fail checks do not give enough structure to identify whether failures are harness, wallet runtime, app logic, or environment.
3. A parity claim is only credible if mandatory EIP-1193 flows are both correct and repeatably green.

Solution overview:
1. Implement a complete E2E architecture with clear boundaries between wallet control, scenario assertions, and evidence reporting.
2. Keep scope strictly tied to 05A localsafe parity (`PARITY-*`) and explicitly reject connector-expansion work in this plan.
3. Run staged `E0-E5` milestones with quantitative gates, reliability SLOs, and milestone tag discipline.

Key innovations:

| Innovation | Why it matters |
|---|---|
| Control Plane / Assertion Plane / Evidence Plane split | Reduces coupling and prevents harness logic from hiding app regressions |
| Reuse-first policy for wallet tooling | Avoids reimplementing Synpress/dappwright internals unless required |
| Contract-tested `WalletDriver` interface | Lets us swap drivers without rewriting parity scenarios |
| Reliability SLO gate + soak runs | Turns flaky one-off passes into measurable release confidence |
| Failure taxonomy with mandatory artifact schema | Speeds triage and improves defect attribution quality |

## 2. Scope and Guardrails

In scope:
1. Chromium E2E for MetaMask parity flows (`eth_requestAccounts`, `personal_sign`, `eth_signTypedData_v4`, `eth_sendTransaction`).
2. Event recovery coverage for `accountsChanged` and `chainChanged`.
3. Compatibility evidence for Rabby and hardware passthrough (Ledger/Trezor via wallet software path).
4. CI-hard gating for `C5` release-readiness.

Out of scope:
1. New wallet connector ecosystem expansion.
2. Direct native HID integration in browser runtime.
3. Non-parity product features not mapped to `PARITY-*`.

Anti-feature-creep policy:
1. Every task in this plan must map to `PARITY-*` and `C5`.
2. Any non-parity addition requires an explicit PRD delta marked `parity-critical`.
3. Non-parity work is deferred to `prds/05B-PRD-HARDENING-WAVE.md` or later PRDs.

## 3. Target End State (Definition of Done for C5)

`C5` is complete only when all conditions are true:
1. `G-M1` preflight gate is green with deterministic non-onboarding, unlocked wallet state.
2. `G-M2` runtime parity gate is green for `MM-PARITY-001..004`.
3. `MM-PARITY-005..006` event recovery tests are green.
4. Reliability SLO is met:
   - Local: >= 90% pass over 10 consecutive runs.
   - CI: >= 95% pass over 20 scheduled runs.
5. Compatibility evidence exists for Rabby, Ledger passthrough, and Trezor passthrough.
6. All failures are classified (`ENV_BLOCKER|HARNESS_FAIL|APP_FAIL|WALLET_FAIL`) with reproducible artifacts.
7. `prds/05A-RELEASE-GATE-CHECKLIST.md` C5-related checks are fully green.

## 4. Core Architecture

System view:

```text
RustySafe Test Target (localhost)
          |
          v
Scenario Runner (Playwright)
          |
          +--> Control Plane (WalletDriver)
          |      - SynpressDriver
          |      - DappwrightDriver
          |      - DirectPopupDriver (strict fallback)
          |
          +--> Assertion Plane
          |      - EIP-1193 result assertions
          |      - UI state assertions
          |      - Event recovery assertions
          |
          +--> Evidence Plane
                 - structured run JSON
                 - markdown summary
                 - trace/screenshot/log references
```

Design principles:
1. Isolate wallet-control mechanics from parity assertions.
2. Prefer deterministic startup configuration over retry-heavy recovery.
3. Fail closed in release gates; do not allow silent fallback modes.
4. Keep driver-specific selectors isolated from parity scenario definitions.
5. Enforce reproducibility through pinned runtime profiles and explicit artifacts.
6. Reuse existing libraries first; reimplement only when parity-critical gaps are proven.

Data flow:
1. Runner starts deterministic browser runtime profile.
2. Control Plane bootstraps wallet to ready state.
3. Scenario executes app action + direct EIP-1193 request path.
4. Control Plane approves/rejects wallet prompts.
5. Assertion Plane validates outputs and state transitions.
6. Evidence Plane persists structured outputs for gate decisions.

## 5. Reuse-First Policy (Do Not Reimplement Without Justification)

Rules:
1. Use Synpress/dappwright capabilities directly when they satisfy requirements.
2. Wrapper code is allowed only to normalize interface contracts (`WalletDriver`) and collect diagnostics.
3. Custom popup automation is allowed only for parity-critical gaps that cannot be solved upstream in time.
4. Every custom workaround must include:
   - issue reference (upstream or internal),
   - removal criterion,
   - owner and review date.

Reuse matrix:

| Concern | Preferred Source | Allowed Custom Layer |
|---|---|---|
| Wallet bootstrap | Synpress/dappwright | lightweight adapter only |
| Wallet approvals | Synpress/dappwright action APIs | fallback popup broker with diagnostics |
| Scenario runner | Playwright | custom fixtures only |
| Evidence formatting | local scripts | additional JSON schema writer |

## 6. Data Contracts

`WalletDriver` contract (required methods):
1. `bootstrapWallet()`
2. `connectToDapp()`
3. `approveSignature()`
4. `approveTransaction()`
5. `approveNetworkChange()`
6. `recoverFromCrashOrOnboarding()`
7. `collectWalletDiagnostics()`

Scenario manifest contract:
1. `scenario_id` (e.g. `MM-PARITY-003`)
2. `parity_ids` (one or more `PARITY-*`)
3. `driver_mode` (`synpress|dappwright|mixed`)
4. `steps`
5. `assertions`
6. `timeouts_ms`

Evidence record contract:
1. `status` (`PASS|FAIL|BLOCKED`)
2. `taxonomy` (`ENV_BLOCKER|HARNESS_FAIL|APP_FAIL|WALLET_FAIL`)
3. `driver`
4. `wallet_version`
5. `browser_version`
6. `run_id`
7. `artifacts` (`log`, `trace`, `screenshots`, `report`)
8. `reproducer_cmd`

## 7. Test Inventory (Parity-Aligned)

Mandatory parity scenarios:
1. `MM-PARITY-001`: connect via `eth_requestAccounts`.
2. `MM-PARITY-002`: message signing via `personal_sign`.
3. `MM-PARITY-003`: typed data signing via `eth_signTypedData_v4`.
4. `MM-PARITY-004`: transaction send via `eth_sendTransaction`.
5. `MM-PARITY-005`: `accountsChanged` deterministic recovery.
6. `MM-PARITY-006`: `chainChanged` deterministic recovery.

Harness determinism scenarios:
1. `MM-HARNESS-001`: onboarding preflight convergence in bounded attempts.
2. `MM-HARNESS-002`: crash-screen restart recovery.
3. `MM-HARNESS-003`: popup detection and approval routing stability.
4. `MM-HARNESS-004`: runtime profile validation (headed, locale, Node pin).

Compatibility evidence scenarios:
1. `MATRIX-001`: Chromium + MetaMask parity smoke.
2. `MATRIX-002`: Chromium + Rabby parity smoke.
3. `MATRIX-003`: Ledger passthrough smoke via wallet software route.
4. `MATRIX-004`: Trezor passthrough smoke via wallet software route.

## 8. Environment and Configuration Contract

Runtime profile requirements:
1. Chromium extension runs in headed mode only.
2. CI wraps runs with `xvfb-run --auto-servernum`.
3. Node runtime is pinned to `v20.x` for E2E commands.
4. Locale is pinned to English for extension selectors.
5. Extension and browser versions are printed in every run header.

Required files:
1. `e2e/playwright.metamask.config.ts`
2. `e2e/tests/metamask/*.mjs`
3. `scripts/run_prd05a_metamask_e2e.sh`
4. `scripts/run_prd05a_compat_matrix.sh`
5. `scripts/run_prd05a_metamask_soak.sh` (new)

Required environment variables:
1. `PRD05A_NODE_BIN`
2. `PRD05A_METAMASK_PASSWORD`
3. `PRD05A_METAMASK_SEED`
4. `PRD05A_E2E_BASE_URL`
5. `PRD05A_E2E_SKIP_WEBSERVER`

## 9. Storage, Artifacts, and Reporting

Artifact locations:
1. `local/reports/prd05a/C5-metamask-e2e-report.md`
2. `local/reports/prd05a/C5-metamask-e2e.log`
3. `local/reports/prd05a/C5-compatibility-matrix-report.md`
4. `local/reports/prd05a/C5-hardware-passthrough-smoke.md`
5. `local/reports/prd05a/C5-dappwright-investigation.md`
6. `local/reports/prd05a/C5-metamask-soak-report.md` (new)

Reporting requirements:
1. Every gate run writes both markdown summary and machine-readable JSON.
2. Failed runs must include taxonomy code, reproducible command, and trace path.
3. Release gate summary links all phase reports and indicates open blockers.

## 10. Implementation Roadmap (E0-E5)

| Phase | Objective | Complexity | Branch |
|---|---|---|---|
| E0 | Deterministic runtime baseline | M | `feat/prd05a-e2e-e0-determinism` |
| E1 | Driver contract and adapter boundary | M | `feat/prd05a-e2e-e1-driver-interface` |
| E2 | dappwright integration and driver arbitration | M | `feat/prd05a-e2e-e2-dappwright-adapter` |
| E3 | Full parity scenario hardening | L | `feat/prd05a-e2e-e3-parity-scenarios` |
| E4 | Compatibility matrix and hardware evidence | M | `feat/prd05a-e2e-e4-matrix-hardware` |
| E5 | CI hard gates and release readiness | M | `feat/prd05a-e2e-e5-ci-release-gate` |

Dependency order:
1. `E0 -> E1 -> E2 -> E3 -> E4 -> E5`
2. `E4` can start in parallel with late `E3` only after `E0` is green.

## 11. Detailed Task List (Structured and Measurable)

### E0 Tasks
1. `E0-T1` enforce headed + `xvfb` profile in scripts and CI.
2. `E0-T2` enforce Node `v20` pin and runtime self-check.
3. `E0-T3` enforce locale self-check at wallet startup.
4. `E0-T4` standardize run header metadata output.

E0 Gate:
1. 100% pass for profile self-check tests over 10 local runs.
2. No run starts without required metadata fields.

### E1 Tasks
1. `E1-T1` define `WalletDriver` interface and fixture wiring.
2. `E1-T2` implement `SynpressDriver`.
3. `E1-T3` add driver contract tests for bootstrap/connect/approve primitives.
4. `E1-T4` migrate existing MetaMask tests to scenario-manifest form.

E1 Gate:
1. Driver contract tests are green.
2. Existing parity scenarios run without reduced coverage.

### E2 Tasks
1. `E2-T1` implement `DappwrightDriver`.
2. `E2-T2` add arbitration mode (`synpress|dappwright|mixed`) in runner.
3. `E2-T3` add comparative reliability report for bootstrap/connect/network.
4. `E2-T4` define release-driver promotion criteria and fallback policy.

E2 Gate:
1. dappwright path is green for bootstrap/connect/network.
2. Comparative report exists and is linked in C5 evidence.

### E3 Tasks
1. `E3-T1` implement stable parity scenarios `MM-PARITY-001..004`.
2. `E3-T2` implement event-recovery scenarios `MM-PARITY-005..006`.
3. `E3-T3` add negative-path assertions (user reject, popup timeout, chain mismatch).
4. `E3-T4` add flake triage labels from failure taxonomy.

E3 Gate:
1. `MM-PARITY-001..006` all green in candidate driver mode.
2. Negative-path tests produce expected taxonomy labels.

### E4 Tasks
1. `E4-T1` run Rabby parity matrix and capture evidence.
2. `E4-T2` run Ledger passthrough smoke and capture reproducible logs.
3. `E4-T3` run Trezor passthrough smoke and capture reproducible logs.
4. `E4-T4` publish matrix summary with known limitations.

E4 Gate:
1. Matrix report includes PASS/FAIL with reproduction details for each row.
2. Hardware evidence files exist and are reviewable.

### E5 Tasks
1. `E5-T1` add soak script and scheduled CI runs for SLO measurement.
2. `E5-T2` enforce hard gate on reliability SLO thresholds.
3. `E5-T3` aggregate release evidence and checklist links.
4. `E5-T4` close phase with branch/tag discipline artifacts.

E5 Gate:
1. Local and CI SLO thresholds are met.
2. Release checklist C5 section is fully green.

## 12. Success Criteria and Gates

Functional criteria:
1. 100% pass for `MM-PARITY-001..006` in gate runs.
2. 100% classification coverage for failed runs.

Reliability criteria:
1. Local >= 90% pass over 10 consecutive runs.
2. CI >= 95% pass over 20 scheduled runs.
3. Zero unclassified failures in release candidate window.

Performance criteria:
1. Per-scenario p95 runtime <= 120s for MetaMask parity scenarios.
2. End-to-end gate job p95 runtime <= 25 minutes.

Operational criteria:
1. Mean time to classify failure <= 10 minutes from artifacts.
2. Mean time to reproduce a failure <= 20 minutes using recorded command and profile.

## 13. Failure Taxonomy and Recovery Strategy

Taxonomy:
1. `ENV_BLOCKER`: missing binaries, browser install, misconfigured CI runner.
2. `HARNESS_FAIL`: preflight convergence, popup routing, driver startup failures.
3. `APP_FAIL`: Rusty Safe flow, state, or assertion mismatch.
4. `WALLET_FAIL`: extension crash/unresponsive behavior independent of app logic.

Recovery actions:
1. `ENV_BLOCKER`: fail fast, mark run blocked, include remediation hint.
2. `HARNESS_FAIL`: retry once with diagnostics mode, then hard fail.
3. `APP_FAIL`: no automatic retry in release gate path.
4. `WALLET_FAIL`: capture traces, screenshots, wallet diagnostics; open upstream/internal issue link.

## 14. CI/API Surface

Required commands:
1. `scripts/run_prd05a_metamask_e2e.sh`
2. `scripts/run_prd05a_compat_matrix.sh`
3. `scripts/run_prd05a_release_evidence.sh`
4. `scripts/run_prd05a_metamask_soak.sh`

Command output contract:
1. Exit code `0` only on gate success.
2. Non-zero exits must still emit markdown and JSON artifacts.
3. Output includes `driver_mode`, `taxonomy_summary`, and artifact index.

## 15. Risks, Trade-offs, and Mitigations

Risk: dual-driver architecture adds maintenance overhead.  
Mitigation: sunset criteria for secondary driver after promotion gate.

Risk: extension popup races remain even after driver upgrades.  
Mitigation: popup diagnostics, bounded retries, and strict taxonomy.

Risk: locale/OS differences break selectors.  
Mitigation: runtime locale check and profile pinning in CI.

Trade-off:
1. Headed + `xvfb` runs are heavier than headless runs, but provide materially better extension stability.
2. Additional soak runs increase CI cost, but are required for reliable release-quality confidence.

## 16. Branching, Commits, and Tags

Branch policy:
1. One branch per phase (`E0-E5`).
2. Merge strictly by dependency order.

Commit policy:
1. Commit at least once per completed task (`E*-T*`).
2. Add one explicit `-gate-green` commit per phase with linked evidence.

Tag policy:
1. Tag each green phase: `prd05a-e2e-e<phase>-gate`.
2. Final release candidate tag only after `E5` gate + checklist sign-off.

## 17. Immediate Next Actions

1. Approve this revised plan as authoritative for C5 execution.
2. Start `E0` and produce first deterministic-profile evidence set.
3. Add `scripts/run_prd05a_metamask_soak.sh` and wire scheduled CI.
4. Update `prds/05A-RELEASE-GATE-CHECKLIST.md` when each phase gate turns green.
