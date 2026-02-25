# PRD 05A Continuation Milestones

Status: Active
Owner: Rusty Safe

This document operationalizes the post-A5 continuation work into milestone execution units.

## Milestone Plan

### C1: EIP-1193 Runtime Adapter (Completed)

Objective:
1. Replace deterministic/mock provider transport with real browser runtime integration.

Deliverables:
1. WASM EIP-1193 request transport (`eth_requestAccounts`, `eth_chainId`, sign methods, `eth_sendTransaction`).
2. Event handling for `accountsChanged` and `chainChanged`.
3. Error normalization into PRD error registry.

Gate:
1. Browser integration tests on Chromium pass.
2. Manual account/chain switch flow proves deterministic lock behavior.

Delivered:
1. Runtime-capable adapter modes in `crates/rusty-safe-signing-adapters/src/eip1193.rs`.
2. Deterministic `accountsChanged` / `chainChanged` event drain contract.
3. Runtime adapter tests in `crates/rusty-safe-signing-adapters/tests/runtime_adapters.rs`.
4. True async WASM provider path for `eth_sign*` + `eth_sendTransaction` via `window.ethereum.request(...)`.
5. App-level async action hooks for selected tx/message flows in `crates/rusty-safe/src/app.rs` and `crates/rusty-safe/src/signing_bridge.rs`.
6. WASM target compile proof:
   - `cargo check -p rusty-safe-signing-adapters --target wasm32-unknown-unknown`
   - `cargo check -p rusty-safe --target wasm32-unknown-unknown`

### C2: Safe Service Runtime Adapter (Completed)

Objective:
1. Replace in-memory service stubs with real Safe Transaction Service integration.

Deliverables:
1. Real propose/confirm/status/execute adapters with timeout and retry policy.
2. Idempotency key propagation and duplicate suppression.
3. Service payload compatibility tests.

Gate:
1. Tx E2E path against service sandbox passes.

Delivered:
1. HTTP runtime mode with retries/idempotency in `crates/rusty-safe-signing-adapters/src/safe_service.rs`.
2. Runtime adapter integration test via mock service in `crates/rusty-safe-signing-adapters/tests/runtime_adapters.rs`.
3. Live endpoint validation script/report (non-destructive probes):
   - `scripts/run_prd05a_safe_service_live.sh`
   - `local/reports/prd05a/C2-safe-service-live-report.md`

### C3: WalletConnect Runtime Integration (Completed)

Objective:
1. Replace in-memory WalletConnect state with live runtime sessions and requests.

Deliverables:
1. `pair/approve/reject/disconnect` live session lifecycle.
2. Live tx/message request routing to tx/message flows.
3. Deferred-response behavior over real WC transport.

Gate:
1. WC lifecycle and deferred response browser E2E pass.

Delivered:
1. Runtime bridge mode in `crates/rusty-safe-signing-adapters/src/wc.rs`.
2. `wc_pair` command path wired through core/shell (`orchestrator.rs`, `signing_bridge.rs`, `signing_ui/wc_requests.rs`).
3. Runtime adapter integration test in `crates/rusty-safe-signing-adapters/tests/runtime_adapters.rs`.
4. Browser runtime contract + async bridge methods (`window.__rustySafeWalletConnect.request(...)`) for pair/session/sync paths.
5. Runtime sync hook to hydrate queue requests from live walletconnect runtime in `crates/rusty-safe/src/signing_bridge.rs`.

### C4: Crypto Storage/Export Spec (Completed)

Objective:
1. Implement authenticated encrypted persistence/export contract from PRD.

Deliverables:
1. Argon2id + PBKDF2 fallback key derivation.
2. HKDF key separation (`enc_key_v1`, `mac_key_v1`).
3. AES-GCM encrypted records + HMAC-SHA256 integrity checks.
4. Import quarantine on auth/MAC failure.

Gate:
1. Auth/tamper negative vectors pass.
2. Backward-compatible import path validated.

Delivered:
1. Crypto primitives in `crates/rusty-safe-signing-adapters/src/crypto.rs`.
2. Encrypted export and authenticated import in `crates/rusty-safe-signing-adapters/src/queue.rs`.
3. Bundle schema extension in `crates/rusty-safe-signing-core/src/domain.rs`.

### C5: Wallet Runtime Compatibility and Gating (Baseline Complete, Hardening Active)

Objective:
1. Establish deterministic blocking release gates for signing parity using `wallet-mock`.
2. Maintain strict localsafe parity scope with deterministic CI/release evidence.
3. Keep real-wallet and hardware validation out of 05A scope.
4. Keep wallet-mock fuzz hardening out of 05A release gates and defer it to 05B.

Deliverables:
1. E0 deterministic preflight/runtime evidence:
   - `scripts/run_prd05a_wallet_mock_preflight.sh`
   - `e2e/tests/schemas/c5e2e-v1.schema.json`
   - `e2e/tests/wallet-mock/runtime-profile-check.mjs`
   - `e2e/tests/wallet-mock/runtime-preflight.contract.test.mjs`
2. E1 blocking parity lane:
   - `scripts/run_prd05a_wallet_mock_gate.sh`
   - `scripts/run_prd05a_wallet_mock_soak.sh`
   - `scripts/run_prd05a_wallet_mock_runtime_slo.sh`
   - `e2e/playwright.wallet-mock.config.ts`
   - `e2e/tests/wallet-mock/wallet-mock-eip1193.spec.mjs`
   - `e2e/tests/wallet-mock/drivers/*`
3. E5 release evidence aggregation:
   - `scripts/run_prd05a_release_evidence.sh`
   - `local/reports/prd05a/C5-release-evidence-index.md` and JSON counterpart.
4. Follow-on real-wallet/hardware scope:
   - `prds/05A-E2E-REAL-WALLET-HARDWARE-TRACK.md`
5. Follow-on wallet-mock fuzz hardening scope:
   - `prds/05B-WALLET-MOCK-FUZZ-HARDENING-PLAN.md`

Gate:
1. Blocking lane: `WM-PARITY-001..006` must pass.
2. Blocking SLOs:
   - local >= 95% over 20 runs;
   - CI >= 99% over 50 runs.
3. Performance and differential evidence remain green for required `PARITY-*` IDs.
4. Wallet-mock fuzz hardening remains explicitly non-blocking for 05A release.

Delivered:
1. `E0` complete and tagged (`prd05a-e2e-e0-gate`).
2. `E1` complete and tagged (`prd05a-e2e-e1-gate`).
3. `E5` release hard-gate script updated for blocking lane evidence index.
4. `M4` reliability closure complete with 50-run baseline SLO evidence (`local/reports/prd05a/C5-wallet-mock-runtime-slo-report.md`).
5. `E6` determinism contract closure completed:
   - `scripts/run_prd05a_wallet_mock_determinism.sh`
   - `local/reports/prd05a/C5-wallet-mock-determinism-report.md`
   - `local/reports/prd05a/C5-wallet-mock-determinism-report.json`
6. `E7` replay and flake-budget enforcement remains active.
7. Wallet-mock fuzz hardening moved to follow-on plan `prds/05B-WALLET-MOCK-FUZZ-HARDENING-PLAN.md`.

Open items:
1. Complete `E7` replay coverage + flake-budget enforcement.
2. Real-wallet compatibility, canary, and hardware passthrough acceptance are moved to `prds/05A-E2E-REAL-WALLET-HARDWARE-TRACK.md`.
3. Wallet-mock fuzz hardening is moved to `prds/05B-WALLET-MOCK-FUZZ-HARDENING-PLAN.md`.

### C6: Performance Harness (Completed)

Objective:
1. Enforce PRD command/rehydration performance budgets.

Deliverables:
1. Command latency capture harness.
2. Rehydration timing harness for mixed flows.
3. CI thresholds and regression alerts.

Gate:
1. `p95 <= 150ms` command path and `p95 <= 1500ms` rehydration path in evidence runs.

Delivered:
1. Performance budget test `crates/rusty-safe-signing-adapters/tests/performance_budget.rs`.
2. Harness/report script `scripts/run_prd05a_performance.sh`.
3. Evidence artifact `local/reports/prd05a/C6-performance-report.md`.

### C7: CI Gate Enforcement (Completed)

Objective:
1. Enforce signing architecture and parity gate checks in CI.

Delivered:
1. `.github/workflows/prd05a-signing-gates.yml`
2. `scripts/check_signing_boundaries.sh`
3. `scripts/check_prd05a_traceability.sh`

### C8: Formatting Gate Cleanup (Completed)

Objective:
1. Remove pre-existing formatting blockers so `cargo fmt --all -- --check` can gate CI.

Delivered:
1. Cleaned trailing whitespace debt in `crates/rusty-safe/src/sidebar.rs`.
2. `cargo fmt --all` now succeeds.

### C9: Differential Parity Harness (Completed)

Objective:
1. Compare parity-flow outputs against localsafe fixture snapshots.

Deliverables:
1. Fixture import pipeline for tx/message/WC parity cases.
2. Diff reporter with severity categories.
3. CI gate to block critical diffs.

Gate:
1. Zero critical diffs across mandatory `PARITY-*` flows.

Delivered:
1. Fixture set under `fixtures/signing/*`.
2. Differential harness test `crates/rusty-safe-signing-adapters/tests/parity_differential.rs`.
3. Harness/report script `scripts/run_prd05a_differential.sh`.
4. Evidence artifact `local/reports/prd05a/C9-differential-parity-report.md`.

### C10: Release Evidence and Discipline (Completed, Sign-off Pending)

Objective:
1. Complete release-gate evidence package and milestone/tag discipline.

Deliverables:
1. Security review sign-off record.
2. Compatibility matrix sign-off.
3. Performance and parity traceability sign-off.
4. Phase tags and branch closure report.

Gate:
1. `prds/05A-RELEASE-GATE-CHECKLIST.md` is fully signed.

Delivered:
1. End-to-end evidence runner `scripts/run_prd05a_release_evidence.sh`.
2. Release evidence summary `local/reports/prd05a/C10-release-evidence-summary.md`.
3. Milestone branch closure report `prds/05A-M4-BRANCH-CLOSURE-REPORT.md`.
