# PRD 05A Continuation Milestones

Status: Active  
Owner: Rusty Safe

This document operationalizes the post-A5 continuation work into milestone execution units.

## Milestone Plan

### C1: EIP-1193 Runtime Adapter

Objective:
1. Replace deterministic/mock provider transport with real browser runtime integration.

Deliverables:
1. WASM EIP-1193 request transport (`eth_requestAccounts`, `eth_chainId`, sign methods, `eth_sendTransaction`).
2. Event handling for `accountsChanged` and `chainChanged`.
3. Error normalization into PRD error registry.

Gate:
1. Browser integration tests on Chromium pass.
2. Manual account/chain switch flow proves deterministic lock behavior.

### C2: Safe Service Runtime Adapter

Objective:
1. Replace in-memory service stubs with real Safe Transaction Service integration.

Deliverables:
1. Real propose/confirm/status/execute adapters with timeout and retry policy.
2. Idempotency key propagation and duplicate suppression.
3. Service payload compatibility tests.

Gate:
1. Tx E2E path against service sandbox passes.

### C3: WalletConnect Runtime Integration

Objective:
1. Replace in-memory WalletConnect state with live runtime sessions and requests.

Deliverables:
1. `pair/approve/reject/disconnect` live session lifecycle.
2. Live tx/message request routing to tx/message flows.
3. Deferred-response behavior over real WC transport.

Gate:
1. WC lifecycle and deferred response browser E2E pass.

### C4: Crypto Storage/Export Spec

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

### C5: Compatibility Matrix

Objective:
1. Prove runtime wallet/hardware passthrough viability for target browsers/wallets.

Deliverables:
1. Chromium + MetaMask matrix run.
2. Chromium + Rabby matrix run.
3. Ledger/Trezor passthrough smoke logs for wallet-backed accounts.

Gate:
1. Compatibility report committed with pass/fail and known limitations.

### C6: Performance Harness

Objective:
1. Enforce PRD command/rehydration performance budgets.

Deliverables:
1. Command latency capture harness.
2. Rehydration timing harness for mixed flows.
3. CI thresholds and regression alerts.

Gate:
1. `p95 <= 150ms` command path and `p95 <= 1500ms` rehydration path in evidence runs.

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

### C9: Differential Parity Harness

Objective:
1. Compare parity-flow outputs against localsafe fixture snapshots.

Deliverables:
1. Fixture import pipeline for tx/message/WC parity cases.
2. Diff reporter with severity categories.
3. CI gate to block critical diffs.

Gate:
1. Zero critical diffs across mandatory `PARITY-*` flows.

### C10: Release Evidence and Discipline

Objective:
1. Complete release-gate evidence package and milestone/tag discipline.

Deliverables:
1. Security review sign-off record.
2. Compatibility matrix sign-off.
3. Performance and parity traceability sign-off.
4. Phase tags and branch closure report.

Gate:
1. `prds/05A-RELEASE-GATE-CHECKLIST.md` is fully signed.
