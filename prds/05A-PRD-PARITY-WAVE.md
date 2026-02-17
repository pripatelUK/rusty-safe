# PRD 05A: WASM EIP-1193 Parity Wave

Status: Draft-Active  
Owner: Rusty Safe  
Target: Ship localsafe flow parity for browser-based signing in `rusty-safe`

Source of truth inputs:

- `prds/LOCALSAFE_SIGNING_FLOW_AND_CAPABILITIES.md` (baseline parity flows)
- `prds/04-SIGNING-INTEGRATION-VIABILITY-REPORT.md` (integration constraints)
- `prds/05B-PRD-HARDENING-WAVE.md` Appendix A (legacy combined PRD 05 full snapshot)

## 1. Executive Summary

### Problem Statement

`rusty-safe` has verification and hashing, but parity-signing workflows expected from localsafe are not yet end-to-end in-browser. Current user pain:

1. Cannot reliably complete tx/message threshold signing in one deterministic flow.
2. WalletConnect request handling does not yet provide complete tx + message signing parity.
3. Collaboration paths (import/export/share) require deterministic merge and recovery semantics.

### Solution Overview

Deliver a parity-first signing stack in WASM using injected wallet `EIP-1193` with deterministic FSM orchestration:

1. Implement tx pipeline: build/hash/sign/propose/confirm/execute.
2. Implement message pipeline: sign/collect/threshold output.
3. Implement WalletConnect tx + message request lifecycle including deferred tx response mode.
4. Implement collaboration primitives: import/export/share with deterministic signature merge.
5. Keep security baselines mandatory (signature recovery verification, chain/account binding, authenticated persisted state).

### Key Innovations (Parity Wave)

| Innovation | Why It Matters | Practical Benefit |
|---|---|---|
| Deterministic FSM transitions | Eliminates race-driven behavior drift | Replayable and testable outcomes |
| Single-tab writer authority (P0) | Removes early multi-tab complexity while preserving consistency | Faster delivery with safer mutations |
| Method-normalized signing surface | Unifies `personal_sign`, `eth_signTypedData_v4`, `eth_signTypedData` | Better dApp compatibility in parity wave |
| Signature context binding + recovery check | Blocks malformed/cross-flow signature injection | Security without native key custody |
| Compatibility matrix with hardware passthrough checks | Enforces realistic wallet behavior targets | Better real-world parity quality |

### Parity Delivery Contract (No Feature Creep)

The parity wave is constrained to localsafe-equivalent signing capabilities. Any feature outside this contract is deferred to `prds/05B-PRD-HARDENING-WAVE.md` or a later PRD.

| Capability Area | Must Ship In 05A | Explicitly Out Of Scope In 05A |
|---|---|---|
| Safe tx flow | Build/hash/sign/propose/confirm/execute | Batch UX redesign, account abstraction |
| Safe message flow | `personal_sign`, `eth_signTypedData`, `eth_signTypedData_v4`, guarded `eth_sign` | New proprietary signing methods |
| WalletConnect flow | tx request flow + message request flow + deferred tx response | Connector marketplace and connector plugins |
| Collaboration | Import/export/share parity with deterministic merge | Multi-tab lease arbitration and background reconcile |
| Wallet/browser surface | Chromium + MetaMask/Rabby + WalletConnect sessions (hardware passthrough) | Firefox/Safari/mobile/browser extension ecosystems |
| Hardware | Ledger/Trezor passthrough via wallet software | Direct HID or vendor SDK transport from WASM |

Change control rule:

1. New feature requests not mapped to localsafe capability parity are rejected in 05A by default.
2. Accepted exceptions require explicit owner sign-off and a PRD delta that states why parity is blocked without the exception.
3. If accepted, the exception must be tagged `parity-critical` in the roadmap and test gates.

## 2. Core Architecture

### System Diagram

```text
+-------------------------------+
| UI Shell Crate                |
| crates/rusty-safe             |
| (egui views + bridge only)    |
+---------------+---------------+
                |
                v
+-------------------------------+      +--------------------------------------+
| Signing Core Crate            |<---->| Signing Adapter Crate                |
| crates/rusty-safe-signing-core|      | crates/rusty-safe-signing-adapters   |
| domain.rs/state_machine.rs    |      | eip1193.rs/wc.rs/safe_service.rs     |
| ports.rs/orchestrator.rs      |      | queue.rs/preflight.rs/execute.rs     |
+-------------------------------+      +--------------------------------------+
             |
             v
+--------------------------------------+
| External Systems                     |
| EIP-1193 / WalletConnect / Safe API  |
+--------------------------------------+
```

### Design Principles

1. Parity first: ship baseline localsafe capability parity before P1/P2 optimizations.
2. Fail closed: signer mismatch, chain mismatch, or integrity failure blocks signing.
3. Deterministic boundaries: pure state machine logic, side effects via adapters.
4. Browser-native runtime: EIP-1193 path only, no native HID in parity wave.
5. Compatibility realism: Chromium + injected wallets + hardware passthrough through wallet software.
6. Non-creep discipline: no connector ecosystem expansion or policy engine work in this wave.
7. Hexagonal enforcement: domain/orchestration code depends only on ports traits, never concrete adapters.

### Data Flow Overview

1. User action or WalletConnect request enters orchestrator as typed command.
2. FSM applies transition and emits declarative side effects.
3. Adapter executes provider/service/storage side effect.
4. Adapter result re-enters FSM as deterministic event.
5. UI renders state snapshots and transition timeline.

### Architecture Pattern Decision

This plan follows a **modular hexagonal architecture** (ports-and-adapters) implemented as a Rust workspace, not a single-crate monolith.

Practical interpretation:

1. Hexagonal core:
   - `crates/rusty-safe-signing-core` contains domain models, FSM transitions, and orchestration logic.
   - Core depends only on trait-based ports (`ProviderPort`, `SafeServicePort`, `WalletConnectPort`, `QueuePort`).
2. Adapter ring:
   - `crates/rusty-safe-signing-adapters` implements the ports using EIP-1193, WalletConnect, storage, and Safe service adapters.
3. UI shell:
   - `crates/rusty-safe` renders egui and delegates signing operations through a narrow bridge API.
4. Existing verification workflows remain in `crates/rusty-safe` and are not migrated during parity wave.

Why this pattern for this codebase:

1. The current app already centralizes too much behavior in UI runtime (`crates/rusty-safe/src/app.rs`), so adding signing directly there would bloat the codebase.
2. Hexagonal workspace boundaries preserve determinism, enable unit testing without browser dependencies, and keep UI and adapter churn out of core signing logic.
3. It preserves a modular-monolith deployment while avoiding premature microservice complexity.

### Crate Skeleton Draft (A0 Baseline)

Skeleton crates are intentionally compile-only and non-invasive in A0. They establish boundaries first, then behavior is implemented in later phases.

Implemented baseline files:

1. `crates/rusty-safe-signing-core/Cargo.toml`
2. `crates/rusty-safe-signing-core/src/lib.rs`
3. `crates/rusty-safe-signing-core/src/domain.rs`
4. `crates/rusty-safe-signing-core/src/ports.rs`
5. `crates/rusty-safe-signing-core/src/state_machine.rs`
6. `crates/rusty-safe-signing-core/src/orchestrator.rs`
7. `crates/rusty-safe-signing-adapters/Cargo.toml`
8. `crates/rusty-safe-signing-adapters/src/lib.rs`
9. `crates/rusty-safe-signing-adapters/src/eip1193.rs`
10. `crates/rusty-safe-signing-adapters/src/safe_service.rs`
11. `crates/rusty-safe-signing-adapters/src/wc.rs`
12. `crates/rusty-safe-signing-adapters/src/queue.rs`
13. `crates/rusty-safe-signing-adapters/src/execute.rs`
14. `crates/rusty-safe-signing-adapters/src/preflight.rs`
15. `crates/rusty-safe-signing-adapters/src/config.rs`
16. `crates/rusty-safe/src/signing_bridge.rs`

Skeleton contract:

1. Core crate defines domain types, state machine transitions, and ports traits only.
2. Adapter crate implements ports with placeholder behavior (`PortError::NotImplemented`) until phase implementation.
3. `signing_bridge` is the only shell-facing entry point and must stay thin.
4. Existing verification tabs remain behavior-identical during A0 and A1.

### Egui Parity Implementation Plan

Parity requires UI workflow depth, but without importing business logic into `app.rs`.

Egui shell strategy:

1. Add one signing entry point in shell: `Tab::Signing`.
2. Keep existing tabs (`VerifySafeApi`, `Message`, `Eip712`, `Offline`) unchanged.
3. Move signing UI rendering into dedicated files; shell only forwards context and actions.

Planned shell modules:

1. `crates/rusty-safe/src/signing_ui/mod.rs`
2. `crates/rusty-safe/src/signing_ui/queue.rs`
3. `crates/rusty-safe/src/signing_ui/tx_details.rs`
4. `crates/rusty-safe/src/signing_ui/message_details.rs`
5. `crates/rusty-safe/src/signing_ui/wc_requests.rs`
6. `crates/rusty-safe/src/signing_ui/import_export.rs`
7. `crates/rusty-safe/src/signing_ui/state.rs`

Localsafe parity mapping (egui):

| Localsafe Capability | Egui Surface | Bridge Action |
|---|---|---|
| tx build/sign/propose/confirm/execute | `queue.rs` + `tx_details.rs` | `SigningBridge::dispatch_tx_*` |
| message sign + threshold progress | `message_details.rs` | `SigningBridge::dispatch_message_*` |
| WalletConnect tx/message handling | `wc_requests.rs` | `SigningBridge::dispatch_wc_*` |
| import/export/share merge | `import_export.rs` | `SigningBridge::dispatch_bundle_*` |

Egui rendering rules:

1. `signing_ui::*` contains view-state transformations and controls only.
2. Core/adapters crates contain all signing decisions and mutations.
3. Any new UI action must translate to a typed bridge command, not direct service calls.
4. Per-frame work is bounded: shell polls non-blocking result queues and renders snapshots.

## 3. Data Models

### Rust Type Definitions (Parity Scope)

```rust
pub struct TimestampMs(pub u64); // Unix epoch milliseconds only

pub enum MessageMethod {
    PersonalSign,
    EthSign,
    EthSignTypedData,
    EthSignTypedDataV4,
}

pub enum WcMethod {
    EthSendTransaction,
    PersonalSign,
    EthSign,
    EthSignTypedData,
    EthSignTypedDataV4,
}

pub enum TxStatus {
    Draft,
    Signing,
    Proposed,
    Confirming,
    ReadyToExecute,
    Executing,
    Executed,
    Failed,
    Cancelled,
}

pub enum MessageStatus {
    Draft,
    Signing,
    AwaitingThreshold,
    ThresholdMet,
    Responded,
    Failed,
    Cancelled,
}

pub enum WcStatus {
    Pending,
    Routed,
    AwaitingThreshold,
    RespondingImmediate,
    RespondingDeferred,
    Responded,
    Expired,
    Failed,
}

pub enum SignatureSource {
    InjectedProvider,
    WalletConnect,
    ImportedBundle,
    ManualEntry,
}

pub enum SignatureMethod {
    SafeTxHash,
    PersonalSign,
    EthSign,
    EthSignTypedData,
    EthSignTypedDataV4,
}

pub enum MacAlgorithm {
    HmacSha256V1,
}

pub struct PendingSafeTx {
    pub schema_version: u16,
    pub chain_id: u64,
    pub safe_address: Address,
    pub nonce: u64,
    pub payload: SafeTxData,
    pub safe_tx_hash: B256,
    pub signatures: Vec<CollectedSignature>,
    pub status: TxStatus,
    pub state_revision: u64,
    pub idempotency_key: String,
    pub created_at_ms: TimestampMs,
    pub updated_at_ms: TimestampMs,
    pub executed_tx_hash: Option<B256>,
    pub mac_algorithm: MacAlgorithm,
    pub mac_key_id: String,
    pub integrity_mac: B256,
}

pub struct PendingSafeMessage {
    pub schema_version: u16,
    pub chain_id: u64,
    pub safe_address: Address,
    pub method: MessageMethod,
    pub message_hash: B256,
    pub signatures: Vec<CollectedSignature>,
    pub status: MessageStatus,
    pub state_revision: u64,
    pub idempotency_key: String,
    pub created_at_ms: TimestampMs,
    pub updated_at_ms: TimestampMs,
    pub mac_algorithm: MacAlgorithm,
    pub mac_key_id: String,
    pub integrity_mac: B256,
}

pub struct PendingWalletConnectRequest {
    pub request_id: String,
    pub topic: String,
    pub chain_id: u64,
    pub method: WcMethod,
    pub status: WcStatus,
    pub linked_safe_tx_hash: Option<B256>,
    pub linked_message_hash: Option<B256>,
    pub created_at_ms: TimestampMs,
    pub updated_at_ms: TimestampMs,
    pub expires_at_ms: Option<TimestampMs>,
    pub state_revision: u64,
    pub correlation_id: String,
}

pub struct CollectedSignature {
    pub signer: Address,
    pub signature: Bytes,
    pub source: SignatureSource,
    pub method: SignatureMethod,
    pub chain_id: u64,
    pub safe_address: Address,
    pub payload_hash: B256,
    pub expected_signer: Address,
    pub recovered_signer: Option<Address>,
    pub added_at_ms: TimestampMs,
}

pub struct AppWriterLock {
    pub holder_tab_id: String,
    pub tab_nonce: B256,
    pub lock_epoch: u64,
    pub acquired_at_ms: TimestampMs,
    pub expires_at_ms: TimestampMs,
}
```

### Validation Rules

1. `PendingSafeTx.safe_tx_hash` must match recomputed payload hash.
2. `CollectedSignature` must match `(chain_id, safe_address, payload_hash)` of target flow.
3. `CollectedSignature.recovered_signer` must equal `expected_signer`.
4. `PendingWalletConnectRequest` must link to exactly one of `linked_safe_tx_hash` or `linked_message_hash`.
5. `state_revision` must update with CAS semantics on every mutation.
6. All timestamps are Unix epoch milliseconds (`TimestampMs`) and must be monotonic per object.
7. `integrity_mac` must verify before object is accepted for mutation.
8. `AppWriterLock` must enforce at-most-one active writer authority in P0 mode.

### Serialization And Integrity Contract

1. Canonical payload format is deterministic JSON with sorted keys and UTF-8 bytes.
2. `integrity_mac` is computed over the canonical payload with `integrity_mac` field omitted.
3. MAC algorithm is fixed to `HMAC-SHA256` (`MacAlgorithm::HmacSha256V1`) in parity wave.
4. Passphrase KDF:
   - primary: `Argon2id` (`m=65536`, `t=3`, `p=1`, output `32` bytes),
   - fallback: `PBKDF2-HMAC-SHA256` (`600000` iterations, output `32` bytes).
5. Key separation:
   - derive `root_key` from passphrase + random salt,
   - derive `enc_key_v1` and `mac_key_v1` via `HKDF-SHA256` with distinct `info` labels.
6. Export authenticity:
   - `bundle_digest = keccak256(canonical_export_without_signature_or_mac)`,
   - `bundle_signature = personal_sign("rusty-safe-export-v1:" || hex(bundle_digest))` by `exporter`,
   - importer must recover signer and match `exporter`.
7. Key rotation is out of scope for 05A and deferred to `05B`.

### Entity Relationships

1. `PendingSafeTx.signatures[*]` belongs to `PendingSafeTx.safe_tx_hash`.
2. `PendingSafeMessage.signatures[*]` belongs to `PendingSafeMessage.message_hash`.
3. `PendingWalletConnectRequest.linked_safe_tx_hash` references `PendingSafeTx.safe_tx_hash`.
4. `PendingWalletConnectRequest.linked_message_hash` references `PendingSafeMessage.message_hash`.
5. `AppWriterLock` governs all mutating flow commands in parity wave.

## 4. CLI/API Surface

There is no end-user CLI. Parity wave surfaces are internal commands + provider/service/WC APIs.

### Internal Commands

| Command | Purpose | Input Example | Output Example |
|---|---|---|---|
| `connect_provider` | Bind injected provider | `{ "command":"connect_provider", "provider_id":"io.metamask", "request_id":"req-1" }` | `{ "ok":true, "wallet":{"account":"0x...","chain_id":1} }` |
| `create_safe_tx` | Create tx draft | `{ "command":"create_safe_tx", "payload":{...}, "request_id":"req-2" }` | `{ "ok":true, "safe_tx_hash":"0xabc..." }` |
| `start_preflight` | Run decode/sim checks | `{ "command":"start_preflight", "safe_tx_hash":"0xabc...", "request_id":"req-3" }` | `{ "ok":true, "preflight":{"success":true} }` |
| `confirm_tx` | Confirm tx signature | `{ "command":"confirm_tx", "safe_tx_hash":"0xabc...", "signature":"0x...", "request_id":"req-4" }` | `{ "ok":true, "state":"Confirming" }` |
| `execute_tx` | Execute threshold tx | `{ "command":"execute_tx", "safe_tx_hash":"0xabc...", "request_id":"req-5" }` | `{ "ok":true, "executed_tx_hash":"0xdef..." }` |
| `sign_message` | Collect owner message signature | `{ "command":"sign_message", "message_hash":"0xaaa...", "method":"eth_signTypedData_v4", "request_id":"req-6" }` | `{ "ok":true, "status":"AwaitingThreshold" }` |
| `respond_wc` | Respond WalletConnect request | `{ "command":"respond_wc", "request_id":"wc-1", "mode":"deferred" }` | `{ "ok":true, "wc_status":"RespondingDeferred" }` |
| `import_bundle` | Import tx/message bundle | `{ "command":"import_bundle", "bundle":{...}, "request_id":"req-8" }` | `{ "ok":true, "merged":{"tx":2,"message":1} }` |
| `export_bundle` | Export signing bundle | `{ "command":"export_bundle", "flow_ids":["tx:0xabc..."], "request_id":"req-9" }` | `{ "ok":true, "bundle_digest":"0x..." }` |
| `acquire_writer_lock` | Acquire parity-wave writer lock | `{ "command":"acquire_writer_lock", "tab_id":"tab-9", "request_id":"req-10" }` | `{ "ok":true, "lock":{"lock_epoch":4} }` |

### EIP-1193 Methods (Parity Wave)

Required:

1. `eth_requestAccounts`
2. `eth_chainId`
3. `eth_signTypedData_v4`
4. `eth_signTypedData`
5. `personal_sign`
6. `eth_sendTransaction`

Guarded optional:

1. `eth_sign` (`allow_eth_sign=true` and explicit warning per request)

Example request:

```json
{
  "method": "eth_signTypedData",
  "params": [
    "0x1234...",
    "{\"types\":{...},\"domain\":{...},\"message\":{...}}"
  ]
}
```

### Error Response Format

```json
{
  "ok": false,
  "error": {
    "code": "SIGNER_MISMATCH",
    "message": "Recovered signer does not match expected owner",
    "retryable": false,
    "correlation_id": "corr-12ab"
  }
}
```

### Parity Error Code Registry

| Code | Retryable | Meaning |
|---|---|---|
| `CHAIN_MISMATCH` | false | Active provider chain differs from flow chain |
| `ACCOUNT_MISMATCH` | false | Active provider account differs from expected signer |
| `SIGNER_MISMATCH` | false | Recovered signer differs from expected signer |
| `UNSUPPORTED_METHOD` | false | Method unavailable for selected provider/session |
| `IDEMPOTENCY_CONFLICT` | true | Duplicate external action detected |
| `WRITER_LOCK_CONFLICT` | true | Lock epoch mismatch while mutating state |
| `WC_REQUEST_EXPIRED` | false | Request expired before threshold completion |
| `IMPORT_AUTH_FAILED` | false | Bundle signature or MAC validation failed |
| `INTEGRITY_MAC_INVALID` | false | Persisted object failed integrity validation |

## 5. Error Handling & Edge Cases

| Failure Mode | Detection | Recovery | Mitigation |
|---|---|---|---|
| Chain changed mid-flow | provider `chainChanged` / guard fail | Pause flow and require rebind | hard `CHAIN_MISMATCH` guard |
| Provider account switched | `accountsChanged` + signer mismatch | force re-auth and invalidate pending signature intent | account binding guard |
| Unsupported provider method | provider error | fallback map per capability profile | deterministic capability probe |
| Wrong signature returned | recovery mismatch | reject signature, keep flow active | signer recovery gate |
| Duplicate propose/confirm | idempotency conflict | collapse duplicate action | stable idempotency keys |
| Nonce conflict against remote state | safe service nonce mismatch | mark tx as conflicted and require explicit user fork | deterministic conflict state |
| WC request expired | `now >= expires_at_ms` | preserve local signatures, mark expired | resumable local artifacts |
| Deferred WC response after tx replaced | executed hash mismatch | require explicit user choice to send replacement hash | deferred-response verification gate |
| Writer lock lost | lock epoch mismatch | switch tab to read-only + reacquire | CAS writer-lock protocol |
| Import tampering | MAC or exporter signature fails | quarantine import | authenticated export/import format |

## 6. Integration Points

### External Dependencies

| Dependency | Parity-Wave Role |
|---|---|
| Injected wallet provider (`EIP-1193`) | signing, account identity, send tx |
| Safe Transaction Service | propose/confirm/query tx state |
| WalletConnect runtime | tx and message request lifecycle |
| `safe-hash-rs` | canonical hash/calldata compatibility |
| `alloy` | shared types + provider modeling |
| `safers-cli` (reference-only) | parity vectors and service payload reference (not runtime-linked) |

### Reuse Boundary Contract

1. `safe-hash-rs` is the canonical runtime hash/calldata implementation.
2. `alloy` is used for types, encoding, and provider abstractions only.
3. `safers-cli` is used for differential tests and payload reference only.
4. `localsafe.eth` is used as behavior oracle for parity acceptance tests.
5. No direct reuse of native HID/device code paths in parity wave.

### No-Reimplementation Policy (Mandatory)

| Capability | Source Of Truth | Rule |
|---|---|---|
| Safe tx hash / calldata hash / domain hash | `safe-hash-rs` (`safe-utils`) | Do not reimplement hashing logic in `rusty-safe` |
| Safe Transaction Service payload and confirmation models | `safe-hash-rs` (`safe-hash`) | Reuse upstream models; only add adapter conversion when unavoidable |
| Ethereum primitives, signatures, typed data encoding | `alloy` | Do not implement custom primitive/signature stacks |
| Safe behavior parity vectors | `localsafe.eth`, `safers-cli` fixtures | Use differential tests instead of forked logic |
| Hardware transport | wallet software passthrough | No native HID/vendor transport implementation in 05A |

Allowed custom implementation scope:

1. Deterministic FSM and orchestration policies.
2. Adapter composition, retries, idempotency, and error taxonomy normalization.
3. UI rendering and user interaction flows.

### Wallet + Hardware Passthrough Contract (P0)

1. Browser target: Chromium-based (`Chrome`, `Brave`).
2. Primary injected wallets: MetaMask and Rabby.
3. WalletConnect request handling is required for tx + message flows.
4. Ledger/Trezor passthrough through wallet software is required for:
   - MetaMask hardware-backed accounts,
   - Rabby hardware-backed accounts,
   - WalletConnect-connected wallet sessions that expose hardware-backed accounts.
5. No direct HID transport in parity wave.

### Secrets/Credentials

1. No private keys stored in `rusty-safe`.
2. Queue encryption and integrity keys are passphrase-derived with explicit key separation (`enc_key_v1`, `mac_key_v1`).
3. API keys (if any) are runtime-only and excluded from exports.

### Configuration Management

Required runtime config keys:

1. `allow_eth_sign` (default `false`)
2. `provider_capability_cache_ttl_ms`
3. `writer_lock_ttl_ms`
4. `safe_service_timeout_ms`
5. `wc_request_poll_interval_ms`
6. `import_max_bundle_bytes`
7. `import_max_object_count`

## 7. Storage & Persistence

### Directory Structure

```text
crates/rusty-safe/
  src/app.rs
  src/sidebar.rs
  src/ui.rs
  src/signing_bridge.rs
  src/signing_ui/mod.rs
  src/signing_ui/queue.rs
  src/signing_ui/tx_details.rs
  src/signing_ui/message_details.rs
  src/signing_ui/wc_requests.rs
  src/signing_ui/import_export.rs
  src/signing_ui/state.rs

crates/rusty-safe-signing-core/
  src/lib.rs
  src/domain.rs
  src/ports.rs
  src/state_machine.rs
  src/orchestrator.rs
  tests/domain_serialization.rs
  tests/state_machine_transitions.rs

crates/rusty-safe-signing-adapters/
  src/lib.rs
  src/eip1193.rs
  src/safe_service.rs
  src/wc.rs
  src/queue.rs
  src/execute.rs
  src/preflight.rs
  src/config.rs
  tests/tx_e2e.rs
  tests/message_e2e.rs
  tests/wc_deferred.rs
  tests/import_export_merge.rs
```

### Runtime Store Model

1. Primary: IndexedDB (`rusty_safe_signing_v1`) for tx/messages/WC/state log/writer lock.
2. Secondary: localStorage for non-sensitive preferences/config.
3. Sensitive records encrypted at rest (`AES-GCM-256`).
4. All persisted flow objects protected by `integrity_mac`.
5. Every object mutation requires `state_revision` CAS match.

### Export/Import Format

```json
{
  "schema_version": 1,
  "exported_at_ms": 1739750400000,
  "exporter": "0xOwner...",
  "bundle_digest": "0x...",
  "bundle_signature": "0x...",
  "txs": [],
  "messages": [],
  "wc_requests": [],
  "mac_algorithm": "hmac_sha256_v1",
  "mac_key_id": "key-v1",
  "integrity_mac": "0x..."
}
```

Import enforcement:

1. schema validation,
2. size/object-count/rate limits,
3. integrity MAC validation,
4. exporter authenticity validation,
5. deterministic merge with signature-context binding.

### Caching Strategy

1. Cache provider capabilities by provider fingerprint for `provider_capability_cache_ttl_ms`.
2. Cache Safe service status snapshots per `safe_tx_hash` with short-lived TTL.
3. Never cache signatures separately from flow-context fields.

## 8. Implementation Roadmap

### Complexity Legend

| Size | Definition |
|---|---|
| S | <= 2 engineer-days |
| M | 3-6 engineer-days |
| L | 7-12 engineer-days |

### Refactor Scope And LOC Control

This plan intentionally avoids a major upfront refactor. The rollout is staged so feature delivery starts after boundaries are in place, without rewriting existing verification workflows.

Required refactors in parity wave:

| Scope | Required Now | Reason | Expected Churn |
|---|---|---|---|
| Workspace crate split for signing | Yes (A0) | enforce architecture boundaries before behavior | Medium |
| Add `signing_bridge` in shell | Yes (A0) | single integration seam between egui and signing crates | Small |
| Add `Tab::Signing` and signing UI module dispatch | Yes (A1) | expose parity flows in egui without polluting `app.rs` | Small-Medium |
| Migrate existing verification tabs (`VerifySafeApi`, `Message`, `Eip712`, `Offline`) | No | avoid regressions and scope explosion | None |
| Rewrite existing decode/verify pipeline | No | out of parity signing scope | None |
| Reorganize non-signing state modules | No | defer to post-parity cleanup | None |

Churn guardrails:

1. A0: no behavioral changes in existing tabs.
2. A1-A2: `crates/rusty-safe/src/app.rs` net churn target <= 300 LOC per phase.
3. A1-A5: `>= 85%` of signing LOC lands outside `crates/rusty-safe/src/app.rs`.
4. Any change that requires moving existing verification logic must be explicitly tagged `post-parity` unless it blocks parity.

### Phase Plan (Parity Wave)

| Phase | Required Tasks | Deliverables | Depends On | Complexity | Parallelization Opportunities |
|---|---|---|---|---|---|
| A0 | Create signing crates and wire workspace dependencies; add `signing_bridge` in UI shell | `crates/rusty-safe-signing-core`, `crates/rusty-safe-signing-adapters`, `crates/rusty-safe/src/signing_bridge.rs` | none | M | Core/adapters crate scaffolding can run in parallel |
| A1 | Implement domain structs/enums; implement deterministic FSM skeleton; implement provider discovery + capability probe; add `Tab::Signing` and empty `signing_ui` surfaces | `crates/rusty-safe-signing-core/src/domain.rs`, `crates/rusty-safe-signing-core/src/state_machine.rs`, `crates/rusty-safe-signing-adapters/src/eip1193.rs`, `crates/rusty-safe/src/signing_ui/*` scaffold + passing unit tests | A0 | M | Provider adapter tests can run in parallel with FSM tests and shell scaffold |
| A2 | Implement tx lifecycle (`create -> sign -> propose -> confirm -> execute`); add idempotency keys and conflict handling; wire queue/tx-details egui flows | `crates/rusty-safe-signing-core/src/orchestrator.rs`, `crates/rusty-safe-signing-adapters/src/safe_service.rs`, `crates/rusty-safe-signing-adapters/src/execute.rs`, `crates/rusty-safe/src/signing_ui/queue.rs`, `crates/rusty-safe/src/signing_ui/tx_details.rs`; tx integration tests | A1 | L | Service adapter and execute-path tests can run in parallel with egui wiring |
| A3 | Implement message lifecycle and threshold progression; add method normalization; wire message egui flow | message transitions in `crates/rusty-safe-signing-core`, `crates/rusty-safe/src/signing_ui/message_details.rs` + message integration tests | A1 | M | Typed-data normalization and threshold tests in parallel with egui wiring |
| A4 | Implement WalletConnect request ingestion/routing/response; implement deferred tx response workflow; wire WC egui inbox | `crates/rusty-safe-signing-adapters/src/wc.rs`, `crates/rusty-safe/src/signing_ui/wc_requests.rs`, `crates/rusty-safe-signing-adapters/tests/wc_deferred.rs` | A2,A3 | L | tx WC and message WC tests in parallel |
| A5 | Implement import/export/share + deterministic merge + writer lock protocol; wire import/export egui flow; close capability matrix gaps | `crates/rusty-safe-signing-adapters/src/queue.rs`, `crates/rusty-safe/src/signing_ui/import_export.rs` + import/export tests + parity report | A2,A3,A4 | M | Import/export verification and lock contention tests in parallel |

### Phase Exit Gates

| Gate | Required Evidence | Threshold |
|---|---|---|
| A0 Gate | workspace builds with new crate boundaries and no behavior regression in existing verify tabs | `cargo check --workspace` green, existing verification smoke tests green, and `app.rs` churn <= 120 LOC |
| A1 Gate | FSM determinism tests; provider discovery tests; signing tab shell rendering | `>= 60` unit tests pass, `0` flaky failures over `3` repeated runs, and no signing business logic in shell |
| A2 Gate | tx end-to-end tests against Safe service mock + one live chain smoke | `100%` pass on mandatory tx cases; `0` duplicate propose/confirm side effects |
| A3 Gate | message method normalization and threshold tests | `100%` pass on `personal_sign`, `eth_signTypedData`, `eth_signTypedData_v4` vectors |
| A4 Gate | WalletConnect quick + deferred flow tests | `100%` pass on request lifecycle state transitions; deferred flow resumes after restart |
| A5 Gate | import/export authenticity + merge + lock conflict tests; parity matrix report | `100%` pass for mandatory localsafe parity capabilities listed in Section 1 |
| Release Gate | Full suite, security review, compatibility matrix run | No open critical findings; Chromium+MetaMask/Rabby matrix green; Ledger/Trezor passthrough smoke green |

### Branch + Commit Milestones

Phase branches:

1. `feat/prd05a-phase-a0-crate-boundaries`
2. `feat/prd05a-phase-a1-core`
3. `feat/prd05a-phase-a2-tx`
4. `feat/prd05a-phase-a3-message`
5. `feat/prd05a-phase-a4-walletconnect`
6. `feat/prd05a-phase-a5-collab-lock-matrix`

Per-phase commit contract:

1. Commit `phase/<id>-scaffold` after interfaces and test harness compile.
2. Commit `phase/<id>-feature-complete` after implementation is complete.
3. Commit `phase/<id>-gate-green` after all phase gates pass.
4. Tag `prd05a-<id>-gate` on merge-ready commit for rollback anchors.

Merge rules:

1. Merge gates: `cargo fmt --check`, `cargo clippy -- -D warnings`, touched-module tests.
2. Security-sensitive milestones (`A2`, `A4`, `A5`) require explicit security review sign-off.
3. No phase may start on a new branch until previous phase gate is green and tagged.
4. UI shell rule: no signing business logic in `crates/rusty-safe/src/app.rs`; only `signing_bridge` calls allowed.

### Success Criteria

| Metric | Target | Measurement Method |
|---|---|---|
| Localsafe capability parity coverage | `100%` of mandatory parity items | Capability checklist tied to Section 1 matrix |
| Deterministic replay consistency | `100%` deterministic outcomes | Replay transition logs in tests and compare final state hash |
| Idempotent side-effect safety | `0` duplicate external writes in retry tests | Adapter invocation counter assertions |
| Wallet compatibility | MetaMask + Rabby pass on Chromium | E2E matrix runs per release candidate |
| Hardware passthrough viability | Ledger/Trezor-backed account signing succeeds via wallet software | Manual smoke + scripted WC flow checks |
| UI shell bloat control | `>= 85%` of new signing LOC lands outside `crates/rusty-safe/src/app.rs` | Per-phase diffstat gate |
| Egui parity surface coverage | `100%` of mandatory parity surfaces mapped in this PRD render and dispatch bridge actions | UI checklist over `queue`, `tx_details`, `message_details`, `wc_requests`, `import_export` |

## 9. Testing Strategy

### Unit Test Approach

1. Hash and signature normalization vectors for tx/message methods.
2. Signature recovery and flow-context binding checks.
3. FSM legal transition and invariant tests.
4. Serialization/MAC determinism tests for persisted objects.

### Integration And E2E Approach

1. tx build -> sign -> propose -> confirm -> execute (`crates/rusty-safe-signing-adapters/tests/tx_e2e.rs`).
2. message sign -> threshold progression (`crates/rusty-safe-signing-adapters/tests/message_e2e.rs`).
3. WalletConnect quick and deferred response flows (`crates/rusty-safe-signing-adapters/tests/wc_deferred.rs`).
4. Import/export/share + merge determinism (`crates/rusty-safe-signing-adapters/tests/import_export_merge.rs`).
5. egui parity state/render tests for signing surfaces (`crates/rusty-safe/tests/signing_ui/*.rs`).
6. Chromium E2E runs with MetaMask and Rabby plus hardware-backed accounts.

### Negative/Fault Approach

1. malformed import bundle and invalid authenticity proof.
2. signer mismatch and unsupported method behavior.
3. writer lock conflict and recovery.
4. service timeout/retry budget and stale request expiration handling.

### Test Data Requirements

1. Transaction fixtures exported from localsafe-equivalent payloads (`fixtures/signing/tx/*.json`).
2. Message fixtures per method (`fixtures/signing/message/*.json`).
3. WalletConnect request fixtures for tx + message variants (`fixtures/signing/wc/*.json`).
4. Golden outputs for hash/signature normalization from `safe-hash-rs` and reference snapshots.

## 10. Comparison & Trade-offs

### Why This Wave Split

1. Delivers user-visible localsafe parity quickly.
2. Keeps critical security baselines intact.
3. Defers high-complexity operability subsystems to hardening wave.

### Alternatives Considered

| Approach | Why Not Chosen For 05A |
|---|---|
| Direct Ledger/Trezor SDK in WASM | Browser transport and UX complexity too high for parity timeline |
| Native desktop bridge for signing | Breaks browser-first parity objective |
| Full connector-ecosystem support in parity wave | Expands scope beyond localsafe parity requirements |

### Trade-offs

1. P0 uses single-tab writer lock (multi-tab collaboration deferred).
2. Advanced policy/reconcile workflows are not in this wave.
3. Hardware support is passthrough-only, not direct-device.

### Future Considerations

1. Move multi-tab lease protocol and reconcile engine into hardening wave.
2. Add richer policy guardrails and risk overrides in hardening wave.
3. Expand browser and connector ecosystem only after parity success criteria are met.

## Context Preservation Map

This split preserves all legacy context. Context relocation:

1. Parity flows, methods, and core acceptance targets are executed from this document.
2. Reliability/scale/policy/concurrency-hardening details are moved to `prds/05B-PRD-HARDENING-WAVE.md`.
3. Full combined historical PRD 05 snapshot is embedded in `prds/05B-PRD-HARDENING-WAVE.md` Appendix A.
