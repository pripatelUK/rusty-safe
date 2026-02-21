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
4. Contract-call authoring lacks a parity-safe ABI-assisted builder path, forcing error-prone raw calldata entry.

### Solution Overview

Deliver a parity-first signing stack in WASM using injected wallet `EIP-1193` with deterministic FSM orchestration:

1. Implement tx pipeline: build/hash/sign/propose/confirm/execute.
2. Implement message pipeline: sign/collect/threshold output.
3. Implement WalletConnect tx + message request lifecycle including deferred tx response mode.
4. Implement ABI-assisted tx builder (ABI paste/import + method form + calldata preview + manual override warning).
5. Implement collaboration primitives: import/export/share with deterministic signature merge and localsafe URL key compatibility.
6. Keep security baselines mandatory (signature recovery verification, chain/account binding, authenticated persisted state).

### Key Innovations (Parity Wave)

| Innovation | Why It Matters | Practical Benefit |
|---|---|---|
| Deterministic FSM transitions | Eliminates race-driven behavior drift | Replayable and testable outcomes |
| Single-tab writer authority (P0) | Removes early multi-tab complexity while preserving consistency | Faster delivery with safer mutations |
| Method-normalized signing surface | Unifies `personal_sign`, `eth_signTypedData_v4`, `eth_signTypedData` | Better dApp compatibility in parity wave |
| Signature context binding + recovery check | Blocks malformed/cross-flow signature injection | Security without native key custody |
| ABI-assisted tx composition with selector verification | Lowers tx construction errors while retaining expert raw override path | Safer contract-call signing parity |
| URL share schema compatibility layer | Preserves localsafe collaboration links (`importTx/importSig/importMsg/importMsgSig`) | Easier migration and cross-tool workflow |
| Compatibility matrix with hardware passthrough checks | Enforces realistic wallet behavior targets | Better real-world parity quality |

### Parity Delivery Contract (No Feature Creep)

The parity wave is constrained to localsafe-equivalent signing capabilities. Any feature outside this contract is deferred to `prds/05B-PRD-HARDENING-WAVE.md` or a later PRD.

| Capability Area | Must Ship In 05A | Explicitly Out Of Scope In 05A |
|---|---|---|
| Safe tx flow | Build/hash/sign/propose/confirm/execute with raw + ABI-assisted authoring + manual signature entry | Batch UX redesign, account abstraction |
| Safe message flow | `personal_sign`, `eth_signTypedData`, `eth_signTypedData_v4`, guarded `eth_sign` | New proprietary signing methods |
| WalletConnect flow | Session lifecycle (`pair/approve/reject/disconnect`) + tx request flow + message request flow + deferred tx response | Connector marketplace and connector plugins |
| Collaboration | Import/export/share parity with deterministic merge + URL import compatibility keys | Multi-tab lease arbitration and background reconcile |
| Wallet/browser surface | Chromium + MetaMask/Rabby + WalletConnect sessions (hardware passthrough) | Firefox/Safari/mobile/browser extension ecosystems |
| Hardware | Ledger/Trezor passthrough via wallet software | Direct HID or vendor SDK transport from WASM |

Connector ecosystem definition in this PRD:

1. Additional wallet connectors beyond MetaMask/Rabby/injected EIP-1193 and WalletConnect-managed sessions.
2. Provider plugin marketplace abstractions and custom connector SDK integrations.

Change control rule:

1. New feature requests not mapped to localsafe capability parity are rejected in 05A by default.
2. Accepted exceptions require explicit owner sign-off and a PRD delta that states why parity is blocked without the exception.
3. If accepted, the exception must be tagged `parity-critical` in the roadmap and test gates.

### Parity Traceability Matrix (Scope Lock)

All implementation work in 05A must map to at least one parity capability ID below. This is the primary anti-feature-creep control.

| Parity ID | Capability | Localsafe Behavior Anchor | 05A Target Sections |
|---|---|---|---|
| `PARITY-TX-01` | Safe tx lifecycle (build/hash/sign/propose/confirm/execute) | `app/hooks/useSafe.ts`, `app/safe/[address]/tx/[txHash]/TxDetailsClient.tsx` | Sections 1, 2, 4, 8 |
| `PARITY-TX-02` | Manual tx signature entry and merge | `app/safe/[address]/tx/[txHash]/TxDetailsClient.tsx` | Sections 1, 2, 4, 9 |
| `PARITY-MSG-01` | Safe message signing + threshold behavior | `app/safe/[address]/message/[messageHash]/MessageDetailsClient.tsx` | Sections 1, 2, 4, 9 |
| `PARITY-WC-01` | WalletConnect session lifecycle + tx/sign request routing + deferred tx response | `app/provider/WalletConnectProvider.tsx`, `app/components/WalletConnectRequestHandler.tsx`, `app/safe/[address]/wc-*` | Sections 1, 2, 4, 8 |
| `PARITY-ABI-01` | ABI-assisted tx composition with selector-safe behavior | `app/safe/[address]/new-tx/NewSafeTxClient.tsx`, `app/utils/abiEncoder.ts` | Sections 1, 2, 4, 9 |
| `PARITY-COLLAB-01` | Import/export/share merge + URL key compatibility (`importTx/importSig/importMsg/importMsgSig`) | `app/safe/[address]/SafeDashboardClient.tsx` | Sections 1, 4, 7, 8, 9 |
| `PARITY-HW-01` | Ledger/Trezor passthrough via wallet software (no direct HID) | Wallet connector behavior via injected provider + WC | Sections 1, 6, 8, 9 |

Scope-lock execution rules:

1. Every roadmap task, PR description, and milestone artifact must reference one or more `PARITY-*` IDs.
2. Any work item without a `PARITY-*` mapping is out of scope for 05A and must be moved to `05B` or a new PRD.
3. The release gate requires a signed parity traceability report at `local/reports/prd05a/parity-traceability.md`.

Feature parking lot (explicitly deferred from 05A):

1. Connector ecosystem expansion beyond MetaMask/Rabby + WalletConnect-managed sessions.
2. Native HID/vendor SDK transport for Ledger/Trezor.
3. Multi-tab lease arbitration/reconcile engine and background sync.
4. Non-signing dashboard modules (token portfolio, owner-management UX redesign, Safe creation UX changes).

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

### Boundary Enforcement (Architecture Fitness)

Hexagonal architecture is not considered satisfied by crate layout alone; boundaries must be continuously enforced.

Boundary rules:

1. `rusty-safe-signing-core` must not depend on UI/runtime/transport crates (`egui`, `eframe`, `reqwest`, `web-sys`, `tokio`).
2. `rusty-safe-signing-adapters` may depend on transport/runtime crates but must not import shell UI modules.
3. `crates/rusty-safe` shell may call signing only via `signing_bridge`; no direct adapter calls.
4. Side effects must originate in adapters; state transition legality must live in core.

Boundary checks required in CI for A1+:

1. dependency checks via `cargo tree` assertions for forbidden deps in core.
2. import-boundary checks via `rg` assertions preventing cross-layer direct imports.
3. review check: any boundary exception requires architecture-owner approval and a PRD delta.

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

### UI Flow Contract (Parity-Critical)

UI parity is mandatory for 05A delivery. Behavior parity is not considered complete unless the egui shell can drive all required flows end-to-end with deterministic state transitions and recovery UX.

#### Navigation And Surface Contract

`Tab::Signing` contains five deterministic surfaces:

1. `SigningSurface::Queue` (`signing_ui/queue.rs`)
2. `SigningSurface::TxDetails` (`signing_ui/tx_details.rs`)
3. `SigningSurface::MessageDetails` (`signing_ui/message_details.rs`)
4. `SigningSurface::WalletConnect` (`signing_ui/wc_requests.rs`)
5. `SigningSurface::ImportExport` (`signing_ui/import_export.rs`)

Navigation rules:

1. Queue is the default entry surface.
2. All deep surfaces (`TxDetails`, `MessageDetails`, `WalletConnect`) must preserve selected flow context after refresh/rehydration.
3. Every surface action dispatches a typed bridge command and receives structured result events.
4. Switching surfaces must not mutate signing state by itself.

#### Shared Shell Elements

All signing surfaces render the following shared elements:

1. Provider status badge: disconnected/connected, active account, active chain.
2. Safe context card: safe address, threshold, owner count, nonce summary.
3. Flow health bar: state, last update time, correlation id, retry state.
4. Alert banner stack: highest-severity first (`error`, `warning`, `info`).

#### Surface Specifications

##### Queue Surface

Purpose:

1. Show pending tx/message/WalletConnect flows and their state at a glance.
2. Start new flows and resume existing flows.

Required data columns:

1. Flow type (`tx`, `message`, `wc-tx`, `wc-sign`)
2. Target hash (`safe_tx_hash` or `message_hash`)
3. State
4. Signature progress (`m / threshold`)
5. Updated timestamp
6. Origin (`local`, `imported`, `walletconnect`)
7. Build source (`raw`, `abi`, `url-import`) for tx flows

Action gating:

| Action | Enabled When | Disabled Reason |
|---|---|---|
| Open details | flow exists | flow missing or deleted |
| Create tx (raw) | provider connected + writer lock held | provider disconnected or lock guard failed |
| Create tx (ABI) | ABI parsed + method args valid + writer lock held | ABI parse/arg validation/lock guard failed |
| Sign | provider connected + chain/account match + writer lock held | provider/chain/account/lock guard failed |
| Propose | tx has at least one valid signature | missing valid signatures |
| Execute | threshold reached and preflight succeeded | threshold/preflight guard failed |
| Respond WC | linked WC request active | request expired or not linked |

##### Tx Details Surface

Purpose:

1. Drive tx flow: sign -> propose -> confirm -> execute.
2. Expose all deterministic flow state and side-effect outcomes.

Required sections:

1. Tx summary (to, value, operation, nonce, safe tx hash).
2. Tx composition panel:
   - raw calldata editor,
   - ABI method form (ABI input + method selector + typed args),
   - decoded calldata preview and mismatch/override warning.
3. Signature table (signer, source, method, recovered signer, added timestamp).
4. Manual signature panel (signer address + signature bytes input + validation result).
5. Preflight panel (simulation result, warnings, blocking issues).
6. Side-effect timeline (propose/confirm/execute attempt history).

Action gating:

| Action | Enabled When | Disabled Reason |
|---|---|---|
| Sign tx | signer guard passes | signer/account/chain mismatch |
| Apply ABI composition | ABI parses and args encode deterministically | ABI parse failure or selector mismatch |
| Override with raw calldata | user confirms warning and payload validates | invalid calldata or warning not acknowledged |
| Add signature (manual/import) | signature context validates and signer recovers to Safe owner | invalid signature format or signer mismatch |
| Propose tx | tx status allows propose and idempotency key free | already proposed or in-flight |
| Confirm tx | tx exists remotely and signature is valid | remote state mismatch or bad signature |
| Execute tx | threshold reached + preflight pass + chain match | threshold/preflight/chain guard failed |

##### Message Details Surface

Purpose:

1. Drive message signing and threshold aggregation.
2. Show method-normalized message payload and signature provenance.

Required sections:

1. Method display (`personal_sign`, `eth_signTypedData`, `eth_signTypedData_v4`, guarded `eth_sign`).
2. Normalized payload/hash panel.
3. Signature progress and threshold indicator.
4. Response eligibility panel (for linked WC requests).

Action gating:

| Action | Enabled When | Disabled Reason |
|---|---|---|
| Sign message | provider connected + method supported + signer guard passes | method unsupported or signer guard failed |
| Add signature (manual/import) | signature context validates | chain/safe/hash context mismatch |
| Respond threshold result | threshold reached and request active | threshold not met or request expired |

##### WalletConnect Surface

Purpose:

1. Present request inbox and deterministic request lifecycle.
2. Support quick and deferred tx response modes and sign-method responses.

Required request metadata:

1. Request id
2. Topic
3. Method
4. Origin dApp metadata (if provided)
5. Expiry timestamp
6. Linked flow reference
7. Session status (`proposed`, `approved`, `rejected`, `disconnected`)
8. Provider capability snapshot (including `wallet_getCapabilities` availability)

Action gating:

| Action | Enabled When | Disabled Reason |
|---|---|---|
| Accept and route | request active + method supported | expired or unsupported method |
| Quick response | flow can produce immediate result | flow not ready |
| Deferred response | request active and linked tx flow exists | missing linked tx |
| Reject | request active | already responded/expired |
| Approve session | session proposed and origin verified | session already finalized or origin check failed |
| Disconnect session | session approved and topic active | session inactive/disconnected |

##### Import/Export Surface

Purpose:

1. Import/export/share tx/message/WC bundles.
2. Explain authenticity and merge outcomes clearly.

Required sections:

1. Import drop zone/text area.
2. URL payload paste field for localsafe-compatible keys (`importTx`, `importSig`, `importMsg`, `importMsgSig`).
3. Validation output panel (schema/mac/signature/context checks).
4. Merge summary panel (added/updated/skipped/conflicted counts).
5. Export options panel (selected flows, bundle digest/signature preview + optional URL payload output).

Action gating:

| Action | Enabled When | Disabled Reason |
|---|---|---|
| Import bundle | input present and parseable | empty or malformed input |
| Import URL payload | key present and payload parseable | missing key, malformed payload, or unsupported schema version |
| Apply merge | validation succeeded | validation/auth/context failed |
| Export bundle | at least one flow selected | no selected flows |
| Copy/share payload | export artifact generated | no export artifact |

#### Critical User Journeys (Parity Paths)

##### Journey 1: Native Tx Signing

1. User enters `Queue` and opens tx.
2. `TxDetails` shows tx summary + composition mode (`raw` or `abi`).
3. User can compose calldata from ABI method form or raw input.
4. User signs directly or adds a validated external signature; signature table updates.
5. User proposes/confirms; timeline logs side effects.
6. When threshold met, execute action is enabled and dispatches `execute_tx`.
7. Executed hash is persisted and shown in timeline.

##### Journey 2: Transaction Collaboration

1. User opens `ImportExport`.
2. User imports bundle or URL payload; validation/match checks run.
3. Merge result is shown with deterministic counters.
4. User opens merged flow in `TxDetails` and continues signing/execution.

##### Journey 3: WalletConnect Tx Deferred Response

1. Request appears in `WalletConnect`.
2. User selects deferred mode and links/creates tx flow.
3. User completes threshold and execute in `TxDetails`.
4. `WalletConnect` request becomes response-eligible and sends executed tx hash.

##### Journey 4: WalletConnect Message Signing

1. Sign request appears in `WalletConnect`.
2. User routes to `MessageDetails`.
3. Message signatures are collected until threshold.
4. Encoded signatures are sent as request response.

##### Journey 5: WalletConnect Session Lifecycle

1. Session proposal appears in `WalletConnect` with dApp metadata.
2. User approves or rejects session.
3. Approved session can receive tx/sign requests and expose capability snapshot.
4. User can disconnect session and requests are archived deterministically.

#### Error And Recovery UX Contract

UI must provide deterministic recovery actions for each blocking class:

| Error Code | UI Behavior | Recovery Action |
|---|---|---|
| `CHAIN_MISMATCH` | blocking banner + highlight chain field | `Connect correct chain` CTA |
| `ACCOUNT_MISMATCH` | blocking banner + signer mismatch detail | `Switch account` CTA |
| `SIGNER_MISMATCH` | inline signature row error + toast | discard invalid signature, keep flow active |
| `INVALID_SIGNATURE_FORMAT` | inline form error in signature panel | correct signer/signature format and retry |
| `UNSUPPORTED_METHOD` | request card warning + disabled action | choose supported method path or reject request |
| `ABI_PARSE_FAILED` | inline form field errors in tx composition panel | correct ABI JSON and retry encoding |
| `ABI_SELECTOR_MISMATCH` | blocking warning on tx composition panel | confirm override or return to ABI-driven encoding |
| `WC_REQUEST_EXPIRED` | request card locked as expired | keep local artifacts; allow export/share |
| `WC_SESSION_NOT_APPROVED` | blocking banner in WalletConnect surface | approve/reject session before request action |
| `WRITER_LOCK_CONFLICT` | read-only mode banner | `Reacquire lock` CTA |
| `URL_IMPORT_SCHEMA_INVALID` | import compatibility panel warning | use accepted key/version or import bundle path |
| `IMPORT_AUTH_FAILED` | import result marked quarantined | inspect details; allow copy of rejection report |
| `INTEGRITY_MAC_INVALID` | object quarantine alert | prevent mutation and offer export of intact flows only |

Recovery UX requirements:

1. Every blocking error displays one primary CTA and one secondary dismiss/details action.
2. Error banners must include correlation id.
3. Recovery actions must be idempotent and safe to retry.
4. Flow state remains visible even when actions are blocked.

#### UI Acceptance Checklist (Parity Gate Input)

A5 parity gate requires all checks to pass:

1. Each required surface renders with no missing mandatory sections.
2. All action buttons follow documented gating rules.
3. All five critical user journeys execute successfully.
4. Blocking errors render deterministic recovery UX.
5. Refresh/rehydration preserves selected flow context and status.
6. No signing business logic leaks into egui shell files.

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

pub enum TxBuildSource {
    RawCalldata,
    AbiMethodForm,
    UrlImport,
}

pub struct AbiMethodContext {
    pub abi_digest: B256,
    pub method_signature: String, // e.g. transfer(address,uint256)
    pub method_selector: [u8; 4],
    pub encoded_args: Bytes,
    pub raw_calldata_override: bool,
}

pub enum WcSessionStatus {
    Proposed,
    Approved,
    Rejected,
    Disconnected,
}

pub struct ProviderCapabilitySnapshot {
    pub wallet_get_capabilities_supported: bool,
    pub capabilities_json: Option<String>,
    pub collected_at_ms: TimestampMs,
}

pub struct WcSessionContext {
    pub topic: String,
    pub status: WcSessionStatus,
    pub dapp_name: Option<String>,
    pub dapp_url: Option<String>,
    pub dapp_icons: Vec<String>,
    pub capability_snapshot: Option<ProviderCapabilitySnapshot>,
    pub updated_at_ms: TimestampMs,
}

pub enum UrlImportKey {
    ImportTx,
    ImportSig,
    ImportMsg,
    ImportMsgSig,
}

pub struct UrlImportEnvelope {
    pub key: UrlImportKey,
    pub schema_version: u16,
    pub payload_base64url: String,
}

pub struct PendingSafeTx {
    pub schema_version: u16,
    pub chain_id: u64,
    pub safe_address: Address,
    pub nonce: u64,
    pub payload: SafeTxData,
    pub build_source: TxBuildSource,
    pub abi_context: Option<AbiMethodContext>,
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
    pub session_status: WcSessionStatus,
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
2. `PendingSafeTx.build_source=AbiMethodForm` requires non-empty `abi_context`.
3. `AbiMethodContext.method_selector` must match the first four bytes of encoded calldata when `raw_calldata_override=false`.
4. `CollectedSignature` must match `(chain_id, safe_address, payload_hash)` of target flow.
5. `CollectedSignature.recovered_signer` must equal `expected_signer`.
6. `PendingWalletConnectRequest` must link to exactly one of `linked_safe_tx_hash` or `linked_message_hash`.
7. `PendingWalletConnectRequest.session_status` must be `Approved` before request response side effects are dispatched.
8. `state_revision` must update with CAS semantics on every mutation.
9. All timestamps are Unix epoch milliseconds (`TimestampMs`) and must be monotonic per object.
10. `integrity_mac` must verify before object is accepted for mutation.
11. `UrlImportEnvelope.key` must be one of `importTx`, `importSig`, `importMsg`, `importMsgSig`.
12. `AppWriterLock` must enforce at-most-one active writer authority in P0 mode.

### Deterministic Transition Log Contract

Each mutating command must emit a transition log record to guarantee replay and recovery determinism.

```rust
pub struct CommandEnvelope {
    pub command_id: String,        // stable UUIDv7
    pub correlation_id: String,    // cross-surface tracing key
    pub parity_capability_id: String, // PARITY-* mapping
    pub idempotency_key: String,
    pub issued_at_ms: TimestampMs,
    pub command_kind: String,
}

pub struct TransitionLogRecord {
    pub event_seq: u64,            // monotonic per-flow
    pub command_id: String,
    pub flow_id: String,
    pub state_before: String,
    pub state_after: String,
    pub side_effect_key: Option<String>,
    pub side_effect_dispatched: bool,
    pub side_effect_outcome: Option<String>,
    pub recorded_at_ms: TimestampMs,
}
```

Log invariants:

1. `event_seq` must increment by exactly `1` within a flow.
2. Side effects must be idempotent by `(flow_id, side_effect_key)`.
3. Replay from persisted log must produce an identical final state hash.
4. Recovered state after refresh/restart must preserve flow visibility and actionable status.

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
8. `UrlImportKey` serializes to exact localsafe-compatible query keys (`importTx`, `importSig`, `importMsg`, `importMsgSig`).

### Entity Relationships

1. `PendingSafeTx.signatures[*]` belongs to `PendingSafeTx.safe_tx_hash`.
2. `PendingSafeMessage.signatures[*]` belongs to `PendingSafeMessage.message_hash`.
3. `PendingWalletConnectRequest.linked_safe_tx_hash` references `PendingSafeTx.safe_tx_hash`.
4. `PendingWalletConnectRequest.linked_message_hash` references `PendingSafeMessage.message_hash`.
5. `PendingWalletConnectRequest.topic` references `WcSessionContext.topic`.
6. `AppWriterLock` governs all mutating flow commands in parity wave.

## 4. CLI/API Surface

There is no end-user CLI. Parity wave surfaces are internal commands + provider/service/WC APIs.

### Internal Commands

| Command | Purpose | Input Example | Output Example |
|---|---|---|---|
| `connect_provider` | Bind injected provider | `{ "command":"connect_provider", "provider_id":"io.metamask", "request_id":"req-1" }` | `{ "ok":true, "wallet":{"account":"0x...","chain_id":1} }` |
| `create_safe_tx` | Create tx draft | `{ "command":"create_safe_tx", "payload":{...}, "request_id":"req-2" }` | `{ "ok":true, "safe_tx_hash":"0xabc..." }` |
| `create_safe_tx_from_abi` | Create tx draft from ABI + method args | `{ "command":"create_safe_tx_from_abi", "to":"0xContract...", "abi_json":"[{...}]", "method":"transfer(address,uint256)", "args":["0xabc...", "100"], "request_id":"req-2b" }` | `{ "ok":true, "safe_tx_hash":"0xabc...", "build_source":"abi" }` |
| `add_tx_signature` | Add external/manual signature to tx | `{ "command":"add_tx_signature", "safe_tx_hash":"0xabc...", "signer":"0xOwner...", "signature":"0x...", "request_id":"req-2c" }` | `{ "ok":true, "signature_count":2 }` |
| `start_preflight` | Run decode/sim checks | `{ "command":"start_preflight", "safe_tx_hash":"0xabc...", "request_id":"req-3" }` | `{ "ok":true, "preflight":{"success":true} }` |
| `confirm_tx` | Confirm tx signature | `{ "command":"confirm_tx", "safe_tx_hash":"0xabc...", "signature":"0x...", "request_id":"req-4" }` | `{ "ok":true, "state":"Confirming" }` |
| `execute_tx` | Execute threshold tx | `{ "command":"execute_tx", "safe_tx_hash":"0xabc...", "request_id":"req-5" }` | `{ "ok":true, "executed_tx_hash":"0xdef..." }` |
| `sign_message` | Collect owner message signature | `{ "command":"sign_message", "message_hash":"0xaaa...", "method":"eth_signTypedData_v4", "request_id":"req-6" }` | `{ "ok":true, "status":"AwaitingThreshold" }` |
| `wc_session_action` | Approve/reject/disconnect WalletConnect session | `{ "command":"wc_session_action", "topic":"wc-topic-1", "action":"approve", "request_id":"req-6b" }` | `{ "ok":true, "session_status":"Approved" }` |
| `respond_wc` | Respond WalletConnect request | `{ "command":"respond_wc", "request_id":"wc-1", "mode":"deferred" }` | `{ "ok":true, "wc_status":"RespondingDeferred" }` |
| `import_bundle` | Import tx/message bundle | `{ "command":"import_bundle", "bundle":{...}, "request_id":"req-8" }` | `{ "ok":true, "merged":{"tx":2,"message":1} }` |
| `import_url_payload` | Import localsafe-compatible URL payload | `{ "command":"import_url_payload", "key":"importSig", "payload":"eyJzY2hlbWEiOi4uLn0", "request_id":"req-8b" }` | `{ "ok":true, "merged":{"tx":1,"message":0} }` |
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
2. `wallet_getCapabilities` (if supported by wallet; absence must not block parity flow)

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

Capabilities probe example:

```json
{
  "method": "wallet_getCapabilities",
  "params": []
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
| `WC_SESSION_NOT_APPROVED` | false | WalletConnect request attempted without approved session |
| `ABI_PARSE_FAILED` | false | ABI JSON could not be parsed or validated |
| `ABI_SELECTOR_MISMATCH` | false | Encoded selector does not match chosen method |
| `INVALID_SIGNATURE_FORMAT` | false | Signature bytes or signer address format is invalid |
| `URL_IMPORT_SCHEMA_INVALID` | false | URL share payload key or schema is invalid |
| `IMPORT_AUTH_FAILED` | false | Bundle signature or MAC validation failed |
| `INTEGRITY_MAC_INVALID` | false | Persisted object failed integrity validation |

## 5. Error Handling & Edge Cases

| Failure Mode | Detection | Recovery | Mitigation |
|---|---|---|---|
| Chain changed mid-flow | provider `chainChanged` / guard fail | Pause flow and require rebind | hard `CHAIN_MISMATCH` guard |
| Provider account switched | `accountsChanged` + signer mismatch | force re-auth and invalidate pending signature intent | account binding guard |
| Unsupported provider method | provider error | fallback map per capability profile | deterministic capability probe |
| ABI parse or encoding failure | ABI parser/selector validation fails | keep draft in editable mode with field-level errors | typed ABI schema + selector consistency check |
| Manual calldata override mismatch | method selector differs from ABI method | require explicit override acknowledgement | mismatch warning gate before sign/propose |
| Manual signature format or signer mismatch | signature parser/recovery guard fails | keep tx active and mark signature row rejected | signature format validation + signer recovery gate |
| Wrong signature returned | recovery mismatch | reject signature, keep flow active | signer recovery gate |
| Duplicate propose/confirm | idempotency conflict | collapse duplicate action | stable idempotency keys |
| Nonce conflict against remote state | safe service nonce mismatch | mark tx as conflicted and require explicit user fork | deterministic conflict state |
| WC request expired | `now >= expires_at_ms` | preserve local signatures, mark expired | resumable local artifacts |
| WC request on unapproved session | `session_status != Approved` | block response and offer session approval/reject | explicit WC session lifecycle contract |
| Deferred WC response after tx replaced | executed hash mismatch | require explicit user choice to send replacement hash | deferred-response verification gate |
| Writer lock lost | lock epoch mismatch | switch tab to read-only + reacquire | CAS writer-lock protocol |
| URL share payload incompatibility | unknown key/version or decode error | quarantine payload and show compatibility diagnostics | strict URL schema parsing with accepted-key allowlist |
| Import tampering | MAC or exporter signature fails | quarantine import | authenticated export/import format |

## 6. Integration Points

### External Dependencies

| Dependency | Parity-Wave Role |
|---|---|
| Injected wallet provider (`EIP-1193`) | signing, account identity, send tx |
| Safe Transaction Service | propose/confirm/query tx state |
| WalletConnect runtime | tx and message request lifecycle |
| `safe-hash-rs` | canonical hash/calldata compatibility |
| `alloy` | shared types + provider modeling + ABI encoding/selector utilities |
| `safers-cli` (reference-only) | parity vectors and service payload reference (not runtime-linked) |
| `localsafe.eth` reference app | URL share key semantics + WC behavior oracle |

### Reuse Boundary Contract

1. `safe-hash-rs` is the canonical runtime hash/calldata implementation.
2. `alloy` is used for types, encoding, provider abstractions, and ABI encode/decode only.
3. `safers-cli` is used for differential tests and payload reference only.
4. `localsafe.eth` is used as behavior oracle for parity acceptance tests and URL share key compatibility.
5. No direct reuse of native HID/device code paths in parity wave.

### No-Reimplementation Policy (Mandatory)

| Capability | Source Of Truth | Rule |
|---|---|---|
| Safe tx hash / calldata hash / domain hash | `safe-hash-rs` (`safe-utils`) | Do not reimplement hashing logic in `rusty-safe` |
| Safe Transaction Service payload and confirmation models | `safe-hash-rs` (`safe-hash`) | Reuse upstream models; only add adapter conversion when unavoidable |
| Ethereum primitives, signatures, typed data encoding | `alloy` | Do not implement custom primitive/signature stacks |
| ABI parsing, selector derivation, calldata encoding | `alloy` ABI modules | Do not create custom ABI parser/encoder stack |
| URL share payload key semantics (`importTx/importSig/importMsg/importMsgSig`) | `localsafe.eth` behavior contract | Preserve key compatibility and parser behavior |
| Safe behavior parity vectors | `localsafe.eth`, `safers-cli` fixtures | Use differential tests instead of forked logic |
| Hardware transport | wallet software passthrough | No native HID/vendor transport implementation in 05A |

Allowed custom implementation scope:

1. Deterministic FSM and orchestration policies.
2. Adapter composition, retries, idempotency, and error taxonomy normalization.
3. UI rendering and user interaction flows.

No-reimplementation enforcement workflow:

1. Any PR touching guarded capabilities (hashing, ABI encoding, typed-data signing, URL key semantics) must cite the source-of-truth package/file used.
2. If upstream cannot be reused, PR must include a short exception note explaining why adapter conversion is insufficient.
3. Differential tests against upstream/reference outputs are mandatory for guarded-capability changes.
4. Gate reviewers must reject PRs that introduce duplicate implementations of guarded capabilities without approved exception notes.

### Wallet + Hardware Passthrough Contract (P0)

1. Browser target: Chromium-based (`Chrome`, `Brave`).
2. Primary injected wallets: MetaMask and Rabby.
3. WalletConnect session lifecycle handling (`pair`, `approve`, `reject`, `disconnect`) is required.
4. WalletConnect request handling is required for tx + message flows.
5. Provider capability snapshot should attempt `wallet_getCapabilities`; unsupported providers must degrade gracefully.
6. Ledger/Trezor passthrough through wallet software is required for:
   - MetaMask hardware-backed accounts,
   - Rabby hardware-backed accounts,
   - WalletConnect-connected wallet sessions that expose hardware-backed accounts.
7. No direct HID transport in parity wave.

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
8. `abi_max_bytes`
9. `url_import_max_payload_bytes`
10. `wc_session_idle_timeout_ms`
11. `command_latency_budget_ms`
12. `rehydration_budget_ms`

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
  tests/abi_builder.rs
  tests/tx_e2e.rs
  tests/tx_manual_signature.rs
  tests/message_e2e.rs
  tests/wc_deferred.rs
  tests/wc_session_lifecycle.rs
  tests/import_export_merge.rs
  tests/url_import_compat.rs
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

### URL Share Compatibility Contract

localsafe-compatible query keys must be supported for parity migration:

1. `importTx`
2. `importSig`
3. `importMsg`
4. `importMsgSig`

Decoder rules:

1. Accept base64url payloads and percent-encoded payloads.
2. Reject unknown keys with `URL_IMPORT_SCHEMA_INVALID`.
3. Enforce payload byte limits via `url_import_max_payload_bytes`.
4. Route decoded payload through the same validation/merge pipeline as bundle import.

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
| A0 | Create signing crates and wire workspace dependencies; add `signing_bridge`; initialize parity traceability file with `PARITY-*` IDs (`PARITY-TX-01`, `PARITY-MSG-01`) | `crates/rusty-safe-signing-core`, `crates/rusty-safe-signing-adapters`, `crates/rusty-safe/src/signing_bridge.rs`, `local/reports/prd05a/parity-traceability.md` (seed) | none | M | Core/adapters crate scaffolding can run in parallel |
| A1 | Implement domain structs/enums; deterministic FSM skeleton; provider discovery + capability probe; add `Tab::Signing` scaffolds; enable architecture boundary checks in CI (`PARITY-WC-01`) | `crates/rusty-safe-signing-core/src/domain.rs`, `crates/rusty-safe-signing-core/src/state_machine.rs`, `crates/rusty-safe-signing-adapters/src/eip1193.rs`, `crates/rusty-safe/src/signing_ui/*`, boundary-check scripts/config + passing tests | A0 | M | Provider adapter tests can run in parallel with FSM tests and shell scaffold |
| A2 | Implement tx lifecycle (`create -> sign -> propose -> confirm -> execute`); idempotency/conflict handling; ABI-assisted tx composition + selector checks; manual tx signature ingestion + validation; transition log + deterministic replay for tx flows (`PARITY-TX-01`, `PARITY-TX-02`, `PARITY-ABI-01`) | `crates/rusty-safe-signing-core/src/orchestrator.rs`, `crates/rusty-safe-signing-core/src/state_machine.rs`, `crates/rusty-safe-signing-adapters/src/safe_service.rs`, `crates/rusty-safe-signing-adapters/src/execute.rs`, `crates/rusty-safe/src/signing_ui/queue.rs`, `crates/rusty-safe/src/signing_ui/tx_details.rs`; tx + ABI + replay tests | A1 | L | Service adapter and execute-path tests can run in parallel with egui wiring |
| A3 | Implement message lifecycle and threshold progression; method normalization; message replay log coverage; wire message egui flow (`PARITY-MSG-01`) | message transitions in `crates/rusty-safe-signing-core`, `crates/rusty-safe/src/signing_ui/message_details.rs` + message integration and replay tests | A1 | M | Typed-data normalization and threshold tests in parallel with egui wiring |
| A4 | Implement WalletConnect session lifecycle (`pair/approve/reject/disconnect`) and request ingestion/routing/response; deferred tx response workflow; `wallet_getCapabilities` graceful probe; define and test navigation-away request policy (`PARITY-WC-01`, `PARITY-HW-01`) | `crates/rusty-safe-signing-adapters/src/wc.rs`, `crates/rusty-safe-signing-adapters/src/eip1193.rs`, `crates/rusty-safe/src/signing_ui/wc_requests.rs`, `crates/rusty-safe-signing-adapters/tests/wc_deferred.rs` + WC policy tests | A2,A3 | L | tx WC and message WC tests in parallel |
| A5 | Implement import/export/share + deterministic merge + writer lock protocol; localsafe URL key import compatibility; finalize parity traceability report and close matrix gaps (`PARITY-COLLAB-01`) | `crates/rusty-safe-signing-adapters/src/queue.rs`, `crates/rusty-safe/src/signing_ui/import_export.rs`, `crates/rusty-safe-signing-adapters/tests/url_import_compat.rs`, `local/reports/prd05a/parity-traceability.md` (final) + parity report | A2,A3,A4 | M | Import/export verification and lock contention tests in parallel |

### Phase Exit Gates

| Gate | Required Evidence | Threshold |
|---|---|---|
| A0 Gate | workspace builds with new crate boundaries, seeded parity traceability report, and no behavior regression in existing verify tabs | `cargo check --workspace` green, existing verification smoke tests green, `app.rs` churn <= 120 LOC, and initial `PARITY-*` mapping file exists |
| A1 Gate | FSM determinism tests; provider discovery tests; signing tab shell rendering; architecture boundary checks | `>= 60` unit tests pass, `0` flaky failures over `3` repeated runs, boundary checks green, and no signing business logic in shell |
| A2 Gate | tx end-to-end tests + ABI/manual signature vectors + tx replay determinism | `100%` pass on mandatory tx cases, ABI selector checks, manual signature validation/recovery checks, `0` duplicate propose/confirm side effects, replay hash stable across `3` replays |
| A3 Gate | message method normalization, threshold tests, and message replay determinism | `100%` pass on `personal_sign`, `eth_signTypedData`, `eth_signTypedData_v4` vectors and replay hash stable across `3` replays |
| A4 Gate | WalletConnect session lifecycle + quick/deferred flow tests + request policy tests | `100%` pass on session/request lifecycle transitions; deferred flow resumes after restart; navigation-away policy behavior tested and deterministic |
| A5 Gate | import/export authenticity + merge + lock conflict tests + URL compatibility tests + final traceability report | `100%` pass for mandatory localsafe parity capabilities listed in Section 1 including URL keys, with no unmapped implementation tasks |
| Release Gate | Full suite, security review, compatibility matrix run, performance budget run, and signed traceability report | No open critical findings; Chromium+MetaMask/Rabby matrix green; Ledger/Trezor passthrough smoke green; performance budgets meet Section 8/9 thresholds; `PARITY-*` coverage is complete |

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
5. Every phase commit message must include covered `PARITY-*` IDs.
6. `phase/<id>-gate-green` commit must include/update the milestone artifact for that phase.

Merge rules:

1. Merge gates: `cargo fmt --check`, `cargo clippy -- -D warnings`, touched-module tests.
2. Security-sensitive milestones (`A2`, `A4`, `A5`) require explicit security review sign-off.
3. No phase may start on a new branch until previous phase gate is green and tagged.
4. UI shell rule: no signing business logic in `crates/rusty-safe/src/app.rs`; only `signing_bridge` calls allowed.

### Milestone Artifacts (Required For Measurability)

| Phase | Artifact Path | Required Contents |
|---|---|---|
| A0 | `local/reports/prd05a/A0-boundary-bootstrap.md` | crate boundary summary, initial `PARITY-*` ID list, known risks |
| A1 | `local/reports/prd05a/A1-boundary-checks.md` | CI boundary check outputs, dependency guard results, violations (if any) |
| A2 | `local/reports/prd05a/A2-tx-parity-report.md` | tx + ABI + manual signature parity checklist, replay determinism evidence |
| A3 | `local/reports/prd05a/A3-message-parity-report.md` | message-method parity checklist and threshold/replay evidence |
| A4 | `local/reports/prd05a/A4-wc-parity-report.md` | WC lifecycle matrix, deferred-response tests, request policy outcomes |
| A5 | `local/reports/prd05a/parity-traceability.md` | final parity matrix with `PARITY-*` coverage, deferred-item list, sign-off |

Milestone artifact rules:

1. Artifacts are mandatory gate evidence, not optional notes.
2. Each artifact must include test run references and commit hash anchors.
3. A phase cannot be tagged `gate-green` without its artifact committed.

### Success Criteria

| Metric | Target | Measurement Method |
|---|---|---|
| Localsafe capability parity coverage | `100%` of mandatory parity items | Capability checklist tied to Section 1 matrix |
| Parity scope-lock compliance | `100%` of implemented tasks mapped to `PARITY-*` IDs | Traceability report audit with zero unmapped tasks |
| Hexagonal boundary integrity | `0` boundary violations | CI boundary-check outputs and import/dependency guard reports |
| Deterministic replay consistency | `100%` deterministic outcomes | Replay transition logs in tests and compare final state hash |
| Idempotent side-effect safety | `0` duplicate external writes in retry tests | Adapter invocation counter assertions |
| ABI composition correctness | `100%` of parity ABI vectors encode expected calldata | Differential fixtures + selector assertions |
| Manual signature parity | `100%` pass for tx/message manual signature add + signer recovery vectors | Integration tests for `add_tx_signature` and message signature ingestion |
| Wallet compatibility | MetaMask automated pass on Chromium + Rabby evidence tracked | Driver-agnostic E2E gate (`MM-PARITY-001..004`) + Rabby matrix report |
| MetaMask gate determinism | `100%` pass on cache bootstrap preflight before runtime smoke | `e2e/tests/metamask/metamask-cache-preflight.mjs` (bootstrap must end in non-onboarding state) |
| Hardware passthrough viability | Ledger/Trezor-backed account signing succeeds via wallet software | Manual smoke + scripted WC flow checks |
| WalletConnect lifecycle robustness | `100%` pass for pair/approve/reject/disconnect + request routing | WC lifecycle integration test suite |
| URL share compatibility | `100%` pass on `importTx/importSig/importMsg/importMsgSig` fixtures | URL import compatibility suite |
| Local command latency budget | `p95 <= 150ms` for non-network command handling | Bench/integration timing on command-dispatch + reducer path |
| Rehydration budget | `p95 <= 1500ms` for restoring 100 mixed flows | Browser E2E startup/rehydration timing harness |
| UI shell bloat control | `>= 85%` of new signing LOC lands outside `crates/rusty-safe/src/app.rs` | Per-phase diffstat gate |
| Egui parity surface coverage | `100%` of mandatory parity surfaces mapped in this PRD render and dispatch bridge actions | UI checklist over `queue`, `tx_details`, `message_details`, `wc_requests`, `import_export` |

### MetaMask-First Phase Gate (C5)

The C5 gate is MetaMask-first by design. Rabby and hardware passthrough remain required artifacts, but they do not replace the MetaMask runtime gate.
Execution follows `prds/05A-E2E-WALLET-RUNTIME-PLAN.md` (`E0-E5`) and is parity-scoped only.

Gate sequence:

1. `G-M1` cache bootstrap preflight: `e2e/tests/metamask/metamask-cache-preflight.mjs` must pass (including deterministic bootstrap recovery when cache opens on onboarding route).
2. `G-M2` runtime parity: `MM-PARITY-001` pass (`eth_requestAccounts`).
3. `G-M2` runtime parity: `MM-PARITY-002` pass (`personal_sign`).
4. `G-M2` runtime parity: `MM-PARITY-003` pass (`eth_signTypedData_v4`).
5. `G-M2` runtime parity: `MM-PARITY-004` pass (`eth_sendTransaction`).
6. `G-M2` runtime recovery: `MM-PARITY-005..006` pass (`accountsChanged` / `chainChanged` deterministic recovery).
7. Reliability SLO pass:
   - local >= 90% pass over 10 consecutive runs;
   - CI >= 95% pass over 20 scheduled runs.

Failure taxonomy for C5 reporting:

1. `ENV_BLOCKER`: Node/Chromium/anvil/trunk/runtime profile prerequisites missing.
2. `HARNESS_FAIL`: preflight convergence, popup routing, or driver startup cannot establish deterministic wallet context.
3. `APP_FAIL`: runtime reaches dapp flow and fails on Rusty Safe/provider semantics.
4. `WALLET_FAIL`: extension crash/unresponsive behavior independent of app logic.

Execution policy:

1. C5 cannot be tagged `gate-green` without a passing `G-M1` and `G-M2`.
2. `HARNESS_FAIL` is release-blocking for MetaMask parity, not a soft warning.
3. Rabby/hardware evidence stays in C5 but is tracked after MetaMask gate is green.
4. C5 is not release-ready until all `E0-E5` phase gates are green.

## 9. Testing Strategy

### Unit Test Approach

1. Hash and signature normalization vectors for tx/message methods.
2. Signature recovery and flow-context binding checks.
3. FSM legal transition and invariant tests.
4. Serialization/MAC determinism tests for persisted objects.
5. ABI parse/encode/selector consistency vectors.
6. URL key parser and decode validation tests.
7. Architecture boundary tests (forbidden imports/dependencies across core, adapters, shell).
8. Transition log invariant tests (`event_seq`, idempotency key behavior, replay hash stability).

### Integration And E2E Approach

1. tx build -> sign -> propose -> confirm -> execute (`crates/rusty-safe-signing-adapters/tests/tx_e2e.rs`).
2. message sign -> threshold progression (`crates/rusty-safe-signing-adapters/tests/message_e2e.rs`).
3. manual tx signature ingestion and signer recovery (`crates/rusty-safe-signing-adapters/tests/tx_manual_signature.rs`).
4. ABI-assisted tx composition and raw-override warnings (`crates/rusty-safe-signing-adapters/tests/abi_builder.rs`).
5. WalletConnect session lifecycle + quick/deferred response flows (`crates/rusty-safe-signing-adapters/tests/wc_deferred.rs`, `crates/rusty-safe-signing-adapters/tests/wc_session_lifecycle.rs`).
6. Import/export/share + merge determinism (`crates/rusty-safe-signing-adapters/tests/import_export_merge.rs`).
7. MetaMask cache preflight (`e2e/tests/metamask/metamask-cache-preflight.mjs`) validating post-unlock non-onboarding state.
8. Chromium + MetaMask extension E2E parity smoke (`e2e/tests/metamask/metamask-eip1193.spec.mjs`).
9. Localsafe URL key compatibility (`crates/rusty-safe-signing-adapters/tests/url_import_compat.rs`).
10. egui parity state/render tests for signing surfaces (`crates/rusty-safe/tests/signing_ui/*.rs`).
11. Chromium E2E runs with MetaMask and Rabby plus hardware-backed accounts.
12. Differential parity harness: compare 05A outputs against localsafe fixture snapshots for `PARITY-*` capabilities.
13. Performance budget suite for command latency and rehydration thresholds.

### Negative/Fault Approach

1. malformed import bundle and invalid authenticity proof.
2. signer mismatch and unsupported method behavior.
3. ABI parse failure, selector mismatch, and unsafe raw override attempts.
4. malformed manual signature bytes/address and non-owner recovered signer.
5. writer lock conflict and recovery.
6. WalletConnect unapproved session or expired session handling.
7. service timeout/retry budget and stale request expiration handling.
8. restart/crash recovery during deferred response and in-flight signature collection.

### Test Data Requirements

1. Transaction fixtures exported from localsafe-equivalent payloads (`fixtures/signing/tx/*.json`).
2. Message fixtures per method (`fixtures/signing/message/*.json`).
3. WalletConnect request fixtures for tx + message variants (`fixtures/signing/wc/*.json`).
4. ABI fixtures (`fixtures/signing/abi/*.json`) including selector mismatch cases.
5. Manual signature fixtures (`fixtures/signing/signature/*.json`) including invalid-format and wrong-signer cases.
6. URL payload fixtures for `importTx`, `importSig`, `importMsg`, `importMsgSig` (`fixtures/signing/url/*.txt`).
7. Golden outputs for hash/signature normalization from `safe-hash-rs` and reference snapshots.
8. Replay-log fixtures (`fixtures/signing/replay/*.json`) for deterministic recovery checks.

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
4. Evaluate optional parity enhancements not required for 05A (advanced connectors, richer session analytics) in later PRDs.

## 11. Continuation Milestones (Post-A5)

This section tracks the remaining productionization milestones requested after the A0-A5 parity implementation landed.

### C-Milestone Matrix

| Milestone | Scope | PARITY Link | Branch | Gate | Status |
|---|---|---|---|---|---|
| C1 | Real WASM/browser EIP-1193 transport + `accountsChanged` / `chainChanged` handling | `PARITY-WC-01`, `PARITY-HW-01` | `feat/prd05a-c1-eip1193-runtime` | Adapter integration tests + chromium smoke | Completed |
| C2 | Real Safe Transaction Service integration (timeouts/retries/idempotency) | `PARITY-TX-01` | `feat/prd05a-c2-safe-service-runtime` | propose/confirm/execute E2E against service sandbox | Completed |
| C3 | Real WalletConnect runtime integration (`pair/approve/reject/disconnect` + request routing) | `PARITY-WC-01` | `feat/prd05a-c3-walletconnect-runtime` | WC lifecycle + deferred response browser E2E | Completed |
| C4 | Full storage/export crypto spec (Argon2id/PBKDF2, HKDF, HMAC-SHA256, AES-GCM) | `PARITY-COLLAB-01` | `feat/prd05a-c4-crypto-storage` | deterministic import/export auth vectors + tamper tests | Completed |
| C5 | Chromium compatibility matrix with MetaMask/Rabby + Ledger/Trezor passthrough smoke | `PARITY-HW-01` | `feat/prd05a-c5-compat-matrix` | `E0-E5` gate sequence defined in `prds/05A-E2E-WALLET-RUNTIME-PLAN.md` | In Progress (MetaMask runtime + SLO + Rabby/hardware evidence pending) |
| C6 | Performance harness (`p95` command latency and rehydration) | `PARITY-TX-01`, `PARITY-MSG-01`, `PARITY-COLLAB-01` | `feat/prd05a-c6-performance-harness` | budgets in Section 8/9 met in CI artifacts | Completed |
| C7 | CI pipeline enforcement for boundary/traceability/signing clippy/tests | All mandatory `PARITY-*` | `feat/prd05a-c7-ci-gates` | CI workflow green on PR + push | Completed |
| C8 | Repo formatting debt cleanup enabling `cargo fmt --all --check` gate | N/A (repo hygiene gate) | `feat/prd05a-c8-formatting-gate` | fmt check green in CI | Completed |
| C9 | Differential parity validation against localsafe fixture snapshots | All mandatory `PARITY-*` | `feat/prd05a-c9-differential-harness` | differential report with zero critical diffs | Completed |
| C10 | Final release gate evidence package (security sign-off + matrix + milestone/tag discipline) | All mandatory `PARITY-*` | `feat/prd05a-c10-release-evidence` | signed release checklist artifact | Completed (pending manual sign-off signatures) |

### C5 E2E Execution Phases (E0-E5)

C5 is executed through the dedicated E2E phase model in `prds/05A-E2E-WALLET-RUNTIME-PLAN.md`.

| Phase | Objective | Branch | Gate | Status |
|---|---|---|---|---|
| E0 | Deterministic runtime baseline (`headed+xvfb`, Node pin, locale/profile checks) | `feat/prd05a-e2e-e0-determinism` | runtime-profile self-checks green + metadata contract present | Planned |
| E1 | `WalletDriver` contract + Synpress adapter boundary | `feat/prd05a-e2e-e1-driver-interface` | driver contract tests green + no parity coverage regression | Planned |
| E2 | dappwright adapter + driver arbitration (`synpress|dappwright|mixed`) | `feat/prd05a-e2e-e2-dappwright-adapter` | bootstrap/connect/network reliability report attached | Planned |
| E3 | full MetaMask parity scenarios (`MM-PARITY-001..006`) + negative-path taxonomy | `feat/prd05a-e2e-e3-parity-scenarios` | mandatory parity scenarios green + taxonomy labels validated | Planned |
| E4 | Rabby matrix + Ledger/Trezor passthrough evidence | `feat/prd05a-e2e-e4-matrix-hardware` | matrix evidence published with reproducible artifacts | Planned |
| E5 | reliability SLO enforcement + release checklist closure | `feat/prd05a-e2e-e5-ci-release-gate` | local/CI SLO thresholds met + C5 checklist fully green | Planned |

### C5 Task List (Structured)

1. `E0-T1` enforce headed + `xvfb` runtime profile in scripts/CI.
2. `E0-T2` enforce Node `v20` pin and startup self-check.
3. `E0-T3` enforce locale self-check for extension selector stability.
4. `E1-T1` define `WalletDriver` interface and fixture wiring.
5. `E1-T2` implement `SynpressDriver` and adapter contract tests.
6. `E2-T1` implement `DappwrightDriver` and arbitration mode.
7. `E2-T2` publish comparative reliability report (bootstrap/connect/network).
8. `E3-T1` implement `MM-PARITY-001..004` runtime scenarios.
9. `E3-T2` implement `MM-PARITY-005..006` event-recovery scenarios.
10. `E3-T3` implement negative-path assertions with taxonomy validation.
11. `E4-T1` execute Rabby matrix and publish evidence.
12. `E4-T2` execute Ledger/Trezor passthrough smokes and publish evidence.
13. `E5-T1` add soak harness (`scripts/run_prd05a_metamask_soak.sh`) and schedule CI runs.
14. `E5-T2` enforce SLO gates and close release checklist C5 section.

### C5 Success Criteria (Quantitative)

1. Functional: `MM-PARITY-001..006` are 100% green in gate runs.
2. Reliability:
   - local >= 90% pass over 10 consecutive runs;
   - CI >= 95% pass over 20 scheduled runs.
3. Classification: 100% failed runs carry taxonomy labels (`ENV_BLOCKER|HARNESS_FAIL|APP_FAIL|WALLET_FAIL`).
4. Performance:
   - per-scenario p95 <= 120s;
   - C5 gate job p95 <= 25 minutes.
5. Release-readiness: C5-related checks in `prds/05A-RELEASE-GATE-CHECKLIST.md` are fully green.

### C-Milestone Commit Contract

Each continuation milestone follows the same contract:

1. `milestone/<c-id>-scaffold` commit: interfaces/harness compile.
2. `milestone/<c-id>-feature-complete` commit: implementation complete.
3. `milestone/<c-id>-gate-green` commit: gate evidence committed.
4. Tag format: `prd05a-<c-id>-gate`.
5. Commit messages must include relevant `PARITY-*` IDs.

For C5 E2E execution phases, enforce phase-level discipline:

1. One branch per `E0-E5` phase as listed above.
2. Commit at least once per completed `E*-T*` task.
3. Add one `-gate-green` commit per phase with evidence links.
4. Tag each green phase: `prd05a-e2e-e<phase>-gate`.

## Context Preservation Map

This split preserves all legacy context. Context relocation:

1. Parity flows, methods, and core acceptance targets are executed from this document.
2. Reliability/scale/policy/concurrency-hardening details are moved to `prds/05B-PRD-HARDENING-WAVE.md`.
3. Full combined historical PRD 05 snapshot is embedded in `prds/05B-PRD-HARDENING-WAVE.md` Appendix A.
