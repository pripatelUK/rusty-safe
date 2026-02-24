# PRD 05A Companion: Real-Wallet And Hardware Validation Track

Status: Draft-Active (Non-Blocking)  
Owner: Rusty Safe  
Companion to: `prds/05A-E2E-WALLET-RUNTIME-PLAN.md`

## 1. Purpose

This plan isolates real-wallet and hardware validation from 05A deterministic release gating.

Goals:
1. Validate Rusty Safe signing flows against real browser wallets (MetaMask, Rabby).
2. Validate Ledger/Trezor passthrough behavior through supported browser wallets.
3. Collect reproducible confidence evidence without coupling release decisions to extension/runtime flake.

Non-goals:
1. No changes to 05A release blocking criteria.
2. No connector feature expansion beyond localsafe parity methods.
3. No direct native HID/WebUSB signing implementation in this track.

## 2. Scope

In scope:
1. Chromium-based real-wallet automation and canary lanes:
   - MetaMask
   - Rabby
2. Real-wallet parity scenarios for:
   - `eth_requestAccounts`
   - `personal_sign`
   - `eth_signTypedData_v4`
   - `eth_sendTransaction`
   - `accountsChanged`
   - `chainChanged`
3. Hardware passthrough smoke checks (Ledger/Trezor) routed through wallet software.

Out of scope:
1. Blocking PR/merge gates for 05A.
2. New signing methods outside 05A parity contract.
3. Hardware-native connector implementation.

## 3. Architecture

```text
Playwright Scenario Runner
      |
      +--> Wallet Driver Contract
      |      - MetaMaskDriver (primary canary lane)
      |      - RabbyDriver (secondary canary lane)
      |
      +--> Optional Automation Backend
      |      - Dappwright path (preferred for MetaMask stability)
      |      - Synpress path (fallback/benchmark, non-authoritative)
      |
      +--> Evidence Plane
             - canary JSON + markdown
             - trace/video/screenshots
             - failure taxonomy
```

Rules:
1. Driver API stays aligned with `WalletDriver` contract from 05A deterministic plan.
2. Real-wallet failures never block deterministic 05A release gates.
3. Every workaround must include owner, removal condition, and review date.

## 4. Phases And Milestones

### R0: Runtime Matrix Baseline

Objective:
1. Pin wallet/browser/runtime versions and establish reproducible startup contract.

Deliverables:
1. Version matrix file under `local/reports/prd05a/real-wallet/`.
2. Preflight script for extension load, onboarding state, and unlock state.
3. Failure taxonomy mapping (`ENV_BLOCKER|HARNESS_FAIL|APP_FAIL|WALLET_RUNTIME_FAIL`).

Exit gate:
1. 10 consecutive preflight runs complete with full artifact coverage.

### R1: MetaMask Canary Lane

Objective:
1. Run non-blocking nightly MetaMask parity checks with reproducible diagnostics.

Deliverables:
1. `scripts/run_prd05a_metamask_canary.sh`.
2. MetaMask driver implementation under `e2e/tests/real-wallet/drivers/`.
3. Canary report and failure trend summary.

Exit gate:
1. 14-day canary pass rate >= 90%.
2. No unresolved `APP_FAIL` for mandatory parity IDs.

### R2: Rabby Canary Lane

Objective:
1. Add Rabby parity confidence lane with same taxonomy/evidence contract.

Deliverables:
1. `scripts/run_prd05a_rabby_canary.sh`.
2. Rabby driver implementation under `e2e/tests/real-wallet/drivers/`.
3. Weekly comparative reliability report (MetaMask vs Rabby).

Exit gate:
1. 14-day canary pass rate >= 90%.
2. Taxonomy classification coverage = 100% for failures.

### R3: Hardware Passthrough Smoke

Objective:
1. Validate ledger/trezor passthrough through wallet software for signing-critical paths.

Deliverables:
1. `scripts/run_prd05a_hardware_passthrough_smoke.sh`.
2. Manual/assisted checklist for Ledger and Trezor:
   - connect
   - account exposure
   - typed-data signing
   - transaction approval
3. Reproducible logs with wallet/runtime versions and device firmware versions.

Exit gate:
1. One green smoke run per wallet/device matrix row.

### R4: Promotion Decision

Objective:
1. Decide whether any real-wallet lane can be promoted from non-blocking to advisory gate.

Deliverables:
1. Promotion decision memo with reliability and triage metrics.
2. Explicit recommendation:
   - keep non-blocking; or
   - promote one lane to advisory hard-check.

Exit gate:
1. CTO/engineering sign-off on recommendation.

## 5. Test Contract

Mandatory real-wallet scenario IDs:
1. `RW-PARITY-001` connect (`eth_requestAccounts`)
2. `RW-PARITY-002` message sign (`personal_sign`)
3. `RW-PARITY-003` typed data sign (`eth_signTypedData_v4`)
4. `RW-PARITY-004` transaction send (`eth_sendTransaction`)
5. `RW-PARITY-005` `accountsChanged` recovery
6. `RW-PARITY-006` `chainChanged` recovery

Hardware smoke IDs:
1. `RW-HW-001` Ledger passthrough typed-data sign
2. `RW-HW-002` Ledger passthrough transaction send
3. `RW-HW-003` Trezor passthrough typed-data sign
4. `RW-HW-004` Trezor passthrough transaction send

## 6. Success Criteria

1. Real-wallet canary reports are generated nightly with full evidence artifacts.
2. Failure taxonomy coverage is 100% across all canary runs.
3. `APP_FAIL` items produce linked bug/issues with repro commands.
4. Hardware passthrough matrix has explicit pass/fail evidence for all planned rows.
5. No real-wallet/hardware requirement is listed as 05A release blocker.

## 7. CI And Operations

Required commands:
1. `scripts/run_prd05a_metamask_canary.sh`
2. `scripts/run_prd05a_rabby_canary.sh`
3. `scripts/run_prd05a_hardware_passthrough_smoke.sh`
4. `scripts/run_prd05a_real_wallet_evidence.sh`

Cadence:
1. Nightly: MetaMask and Rabby canaries (non-blocking).
2. Weekly: comparative reliability report.
3. On-demand: hardware passthrough smoke.

## 8. Branch And Tag Discipline

Branch policy:
1. `feat/prd05a-rw-r0-baseline`
2. `feat/prd05a-rw-r1-metamask-canary`
3. `feat/prd05a-rw-r2-rabby-canary`
4. `feat/prd05a-rw-r3-hardware-smoke`
5. `feat/prd05a-rw-r4-promotion-decision`

Commit policy:
1. Include `R*-T*` task IDs in commit messages.
2. Add one `-gate-green` commit at each phase close.

Tag policy:
1. `prd05a-rw-r0-gate` through `prd05a-rw-r4-gate`.

## 9. Dependencies

1. `prds/05A-E2E-WALLET-RUNTIME-PLAN.md` stays source of truth for blocking deterministic release.
2. This companion plan cannot change 05A blocking criteria without explicit PRD delta and sign-off.
