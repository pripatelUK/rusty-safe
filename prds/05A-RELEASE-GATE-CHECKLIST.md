# PRD 05A Release Gate Checklist

Status: Active  
Owner: Rusty Safe

Authoritative C5 E2E execution plan:
1. `prds/05A-E2E-WALLET-RUNTIME-PLAN.md` (`E0`, `E1`, `E5`).

## Required Evidence

### 1. Security

- [ ] Security review completed for signing runtime integrations.
- [ ] No open critical/high findings.
- [ ] Signature-context and replay protections verified.

### 2. C5 Wallet Runtime Model

- [x] Deterministic blocking lane is `wallet-mock`.
- [x] 05A release criteria do not require MetaMask/Rabby automation.
- [x] 05A release criteria do not require hardware passthrough acceptance.
- [x] Real-wallet/hardware scope is moved to `prds/05A-E2E-REAL-WALLET-HARDWARE-TRACK.md`.

### 2.1 C5 E2E Phase Gates (`E0`, `E1`, `E5`)

- [x] `E0 Gate` green: deterministic preflight + Node `v20` pin + `c5e2e-v1` schema/artifact checks.
- [x] `E1 Gate` green: `WalletMockDriver` and `WM-PARITY-001..006` blocking lane scenarios.
- [x] `E5 Gate` implemented: CI hard-gate/SLO/release evidence index wiring is complete.
- [x] Real-wallet/hardware gates are tracked outside 05A in `prds/05A-E2E-REAL-WALLET-HARDWARE-TRACK.md`.

Required phase evidence:
1. `scripts/run_prd05a_wallet_mock_preflight.sh`
2. `scripts/run_prd05a_wallet_mock_gate.sh`
3. `scripts/run_prd05a_wallet_mock_soak.sh`
4. `scripts/run_prd05a_wallet_mock_runtime_slo.sh`
5. `scripts/run_prd05a_release_evidence.sh`
6. `scripts/check_prd05a_phase_discipline.sh`
7. `local/reports/prd05a/C5-wallet-mock-gate-report.md`
8. `local/reports/prd05a/C5-wallet-mock-soak-report.md`
9. `local/reports/prd05a/C5-wallet-mock-runtime-slo-report.md`
10. `local/reports/prd05a/C5-release-evidence-index.md`

### 3. Functional Parity

- [x] `PARITY-TX-01` complete.
- [x] `PARITY-TX-02` complete.
- [x] `PARITY-MSG-01` complete.
- [x] `PARITY-WC-01` complete.
- [x] `PARITY-ABI-01` complete.
- [x] `PARITY-COLLAB-01` complete.
- [x] Real-wallet/hardware parity acceptance is moved out of 05A scope.

### 4. Reliability and Performance

- [x] Blocking lane local SLO met (`>=95%` over 20 runs).
- [x] Blocking lane CI SLO met (`>=99%` over 50 runs).
- [x] Blocking scenario p95 runtime <= 90s.
- [x] Blocking PR gate p95 runtime <= 15 minutes.

### 5. CI Gates

- [x] `scripts/check_signing_boundaries.sh` passes.
- [x] `scripts/check_prd05a_traceability.sh` passes.
- [x] `cargo fmt --all -- --check` passes.
- [x] Strict clippy for signing crates passes.
- [x] `cargo test --workspace` passes.
- [x] PR blocking gate wired to `scripts/run_prd05a_wallet_mock_gate.sh` + 5-run soak.
- [x] Scheduled daily gate wired to `scripts/run_prd05a_wallet_mock_soak.sh daily` (50-run).

### 6. Milestone and Tag Discipline

- [x] `E0` committed with `E*-T*` and `-gate-green` marker.
- [x] `E1` committed with `E*-T*` and `-gate-green` marker.
- [x] `E5` committed/tagged.
- [x] Branch naming policy enforced (`feat/prd05a-e2e-(e<phase>|m<milestone>)-<slug>`).
- [x] Branch closure report completed (`prds/05A-M4-BRANCH-CLOSURE-REPORT.md`).

## Sign-off

- Engineering Lead: __________________ Date: __________
- Security Reviewer: _________________ Date: __________
- Product Owner: _____________________ Date: __________
