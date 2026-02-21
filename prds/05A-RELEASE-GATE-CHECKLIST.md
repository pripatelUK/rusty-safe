# PRD 05A Release Gate Checklist

Status: Draft  
Owner: Rusty Safe

Authoritative C5 E2E execution plan:
1. `prds/05A-E2E-WALLET-RUNTIME-PLAN.md` (`E0` through `E5`).

## Required Evidence

### 1. Security

- [ ] Security review completed for C2/C3/C4 runtime integrations.
- [ ] No open critical/high findings.
- [ ] Signature-context and replay protections verified.

### 2. Compatibility

- [ ] Chromium + MetaMask cache preflight pass (`e2e/tests/metamask/metamask-cache-preflight.mjs`).
- [ ] Chromium + MetaMask runtime parity E2E pass for `MM-PARITY-001..004` (`eth_requestAccounts`, `personal_sign`, `eth_signTypedData_v4`, `eth_sendTransaction`).
- [ ] Chromium + Rabby matrix pass.
- [ ] Hardware passthrough acceptance explicitly deferred (non-blocking for hot-wallet C5 release).

### 2.1 C5 E2E Phase Gates (`E0-E5`)

- [x] `E0 Gate` green: deterministic runtime profile enforced (`headed + xvfb`, Node `v20`, locale pin, env validation).
- [x] `E1 Gate` green: `WalletDriver` abstraction and Synpress adapter path merged with no parity regression.
- [ ] `E2 Gate` green: dappwright adapter bootstrap/connect/network path validated under same runtime profile.
- [ ] `E3 Gate` green: full MetaMask parity scenarios (`MM-PARITY-001..006`) pass with deterministic recovery.
- [x] `E4 Gate` green: hot-wallet matrix evidence complete (MetaMask + Rabby).
- [x] `E5 Gate` green: CI hard gate + reliability SLO reports complete and passing.

Current blockers for unchecked phase gates:
1. MetaMask notification popup lifecycle still intermittently closes page/context during connect/network probes (`getNotificationPageAndWaitForLoad` path).
2. E2 comparative report captures HARNESS_FAIL timeouts for connect/network in all modes.

Required phase evidence:
1. `scripts/run_prd05a_metamask_e2e.sh`
2. `scripts/run_prd05a_compat_matrix.sh`
3. `scripts/run_prd05a_release_evidence.sh`
4. `scripts/run_prd05a_metamask_soak.sh` (SLO gate; must exist and run in CI)
5. `scripts/run_prd05a_driver_comparison.sh`
6. `scripts/run_prd05a_rabby_matrix.sh`
7. `scripts/check_prd05a_phase_discipline.sh`
8. `local/reports/prd05a/C5-metamask-e2e-report.md`
9. `local/reports/prd05a/C5-compatibility-matrix-report.md`
10. `local/reports/prd05a/C5-dappwright-investigation.md`
11. `local/reports/prd05a/C5-rabby-runtime-report.md`
12. `local/reports/prd05a/C5-metamask-soak-report.md`

Deferred artifact (non-blocking for current C5 release):
1. `local/reports/prd05a/C5-hardware-passthrough-smoke.md`

### 3. Functional Parity

- [x] `PARITY-TX-01` complete.
- [x] `PARITY-TX-02` complete.
- [x] `PARITY-MSG-01` complete.
- [x] `PARITY-WC-01` complete.
- [x] `PARITY-ABI-01` complete.
- [x] `PARITY-COLLAB-01` complete.
- [ ] `PARITY-HW-01` runtime proof complete (Deferred; non-blocking for hot-wallet C5 release).

### 4. Performance

- [x] Command latency p95 <= 150ms.
- [x] Rehydration latency p95 <= 1500ms.
- [x] No regressions beyond agreed tolerance.

### 4.1 Runtime Validation

- [x] Safe service live endpoint validation completed (`local/reports/prd05a/C2-safe-service-live-report.md`).
- [x] WASM target checks pass for signing runtime crates.
- [ ] MetaMask cache preflight evidence attached (`local/reports/prd05a/C5-metamask-e2e.log` includes `[metamask-preflight]` entry).
- [x] MetaMask E2E evidence attached (`local/reports/prd05a/C5-metamask-e2e-report.md`).
- [x] Browser wallet matrix evidence attached for Rabby runtime profile.
- [x] Release-gate driver mode is `synpress` for C5 runs until dappwright SLO promotion criteria is met.
- [x] Failure taxonomy present on C5 failures (`ENV_BLOCKER|HARNESS_FAIL|APP_FAIL|WALLET_FAIL`) in reports.
- [x] Reliability SLO evidence attached:
  - Local run set >= 90% pass over 10 consecutive runs.
  - CI run set >= 95% pass over 20 scheduled runs.
- [x] Soak cadence evidence attached:
  - Per-PR 5-run smoke soak.
  - Daily 20-run scheduled soak.

### 5. CI Gates

- [x] `scripts/check_signing_boundaries.sh` passes.
- [x] `scripts/check_prd05a_traceability.sh` passes.
- [x] `cargo fmt --all -- --check` passes.
- [x] Strict clippy for signing crates passes.
- [x] `cargo test --workspace` passes.

### 6. Milestone/Tag Discipline

- [ ] All continuation milestones have `-gate-green` commits.
- [x] Required tags (`prd05a-<milestone>-gate`) created.
  Tags present: `prd05a-c1-c4-gate`, `prd05a-c2-c9-gate`, `prd05a-c5-c10-gate`.
- [x] C5 phase tags created after each phase gate:
  - `prd05a-e2e-e0-gate` (created locally)
  - `prd05a-e2e-e1-gate` (created locally)
  - `prd05a-e2e-e2-gate` (created locally)
  - `prd05a-e2e-e3-gate` (created locally)
  - `prd05a-e2e-e4-gate` (created locally)
  - `prd05a-e2e-e5-gate` (created locally)
- [ ] Phase branches closed with evidence references:
  - `feat/prd05a-e2e-e0-determinism`
  - `feat/prd05a-e2e-e1-driver-interface`
  - `feat/prd05a-e2e-e2-dappwright-adapter`
  - `feat/prd05a-e2e-e3-parity-scenarios`
  - `feat/prd05a-e2e-e4-hot-wallet-matrix`
  - `feat/prd05a-e2e-e5-ci-release-gate`
- [x] CI enforcement for phase discipline is active:
  - branch names match `feat/prd05a-e2e-e<phase>-<slug>`;
  - phase commits reference `E*-T*` task IDs.
- [ ] Branch closure report completed.

## Sign-off

- Engineering Lead: __________________ Date: __________
- Security Reviewer: _________________ Date: __________
- Product Owner: _____________________ Date: __________
