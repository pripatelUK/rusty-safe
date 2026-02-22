# PRD 05A Release Gate Checklist

Status: Active  
Owner: Rusty Safe

Authoritative C5 E2E execution plan:
1. `prds/05A-E2E-WALLET-RUNTIME-PLAN.md` (`E0` through `E5`).

## Required Evidence

### 1. Security

- [ ] Security review completed for signing runtime integrations.
- [ ] No open critical/high findings.
- [ ] Signature-context and replay protections verified.

### 2. C5 Wallet Runtime Model

- [x] Deterministic blocking lane is `wallet-mock`.
- [x] MetaMask automation is non-blocking canary.
- [x] Manual MetaMask sanity (`MANUAL-MM-001..004`) is required before RC sign-off.
- [x] Hardware passthrough acceptance is deferred and non-blocking for C5.

### 2.1 C5 E2E Phase Gates (`E0-E5`)

- [x] `E0 Gate` green: deterministic preflight + Node `v20` pin + `c5e2e-v1` schema/artifact checks.
- [x] `E1 Gate` green: `WalletMockDriver` and `WM-PARITY-001..006` blocking lane scenarios.
- [x] `E2 Gate` green: manual MetaMask release checklist workflow (`scripts/run_prd05a_manual_metamask_checklist.sh`) is implemented.
- [ ] `E3 Gate` green: MetaMask nightly canary (`MM-CANARY-001..003`) artifacts for 5 consecutive days.
- [ ] `E4 Gate` green: Rabby canary matrix evidence completed (if enabled).
- [ ] `E5 Gate` green: CI hard-gate/SLO/release evidence index complete and passing.

Required phase evidence:
1. `scripts/run_prd05a_wallet_mock_preflight.sh`
2. `scripts/run_prd05a_wallet_mock_gate.sh`
3. `scripts/run_prd05a_wallet_mock_soak.sh`
4. `scripts/run_prd05a_manual_metamask_checklist.sh`
5. `scripts/run_prd05a_release_evidence.sh`
6. `scripts/check_prd05a_phase_discipline.sh`
7. `local/reports/prd05a/C5-wallet-mock-gate-report.md`
8. `local/reports/prd05a/C5-wallet-mock-soak-report.md`
9. `local/reports/prd05a/C5-manual-metamask-sanity.md`
10. `local/reports/prd05a/C5-release-evidence-index.md`

Deferred artifact (non-blocking for C5):
1. `local/reports/prd05a/C5-hardware-passthrough-smoke.md`

### 3. Functional Parity

- [x] `PARITY-TX-01` complete.
- [x] `PARITY-TX-02` complete.
- [x] `PARITY-MSG-01` complete.
- [x] `PARITY-WC-01` complete.
- [x] `PARITY-ABI-01` complete.
- [x] `PARITY-COLLAB-01` complete.
- [ ] `PARITY-HW-01` runtime proof complete (Deferred; non-blocking for C5 hot-wallet release).

### 4. Reliability and Performance

- [ ] Blocking lane local SLO met (`>=95%` over 20 runs).
- [ ] Blocking lane CI SLO met (`>=99%` over 50 runs).
- [ ] Blocking scenario p95 runtime <= 90s.
- [ ] Blocking PR gate p95 runtime <= 15 minutes.

### 5. CI Gates

- [ ] `scripts/check_signing_boundaries.sh` passes.
- [ ] `scripts/check_prd05a_traceability.sh` passes.
- [ ] `cargo fmt --all -- --check` passes.
- [ ] Strict clippy for signing crates passes.
- [ ] `cargo test --workspace` passes.
- [ ] PR blocking gate wired to `scripts/run_prd05a_wallet_mock_gate.sh` + 5-run soak.
- [ ] Scheduled daily gate wired to `scripts/run_prd05a_wallet_mock_soak.sh daily` (50-run).

### 6. Milestone and Tag Discipline

- [x] `E0` committed with `E*-T*` and `-gate-green` marker.
- [x] `E1` committed with `E*-T*` and `-gate-green` marker.
- [x] `E2` committed with `E*-T*` and `-gate-green` marker.
- [ ] `E3` committed/tagged.
- [ ] `E4` committed/tagged.
- [ ] `E5` committed/tagged.
- [x] Phase branch naming policy enforced (`feat/prd05a-e2e-e<phase>-<slug>`).
- [ ] Branch closure report completed.

## Sign-off

- Engineering Lead: __________________ Date: __________
- Security Reviewer: _________________ Date: __________
- Product Owner: _____________________ Date: __________
