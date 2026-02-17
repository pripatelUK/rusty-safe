# PRD 05: WASM EIP-1193 Signing Architecture For Localsafe Parity

Status: Draft  
Owner: Rusty Safe  
Target: Ship a production signing architecture in `rusty-safe` with feature parity to core `localsafe.eth` flows

## 1. Executive Summary

### Problem Statement

`rusty-safe` can currently verify and hash Safe payloads, but it cannot complete end-to-end signing operations in-browser. Users cannot reliably:

1. Collect threshold signatures across tx/message flows.
2. Drive execution after threshold is met.
3. Complete WalletConnect request lifecycles with deterministic recovery after refresh/crash.

### Solution Overview

Implement a deterministic signing subsystem in `crates/rusty-safe/src/signing/` that uses:

1. WASM + injected wallet (`EIP-1193`) as the production signer path.
2. Pure-Rust finite state machines for tx/message/WalletConnect orchestration.
3. `safe-hash-rs` as canonical hash/service model foundation.
4. Alloy types and transport abstractions for strongly typed provider interactions.
5. Idempotent external side effects and durable local persistence for reliability.

### Key Innovations

| Innovation | Why It Matters | User/Operator Benefit |
|---|---|---|
| Deterministic transition engine | Removes race-driven ambiguity in signing flows | Reproducible outcomes and easier debugging |
| Idempotent side-effect model | Prevents duplicate propose/confirm/respond under retries | Safer recovery and fewer duplicate service artifacts |
| `EIP-6963` + legacy fallback provider discovery | Handles multi-wallet environments predictably | Better wallet compatibility and explicit wallet identity |
| Preflight-first execution policy | Catches risky/invalid actions before sending | Fewer failed/unsafe executes |
| Differential parity testing against `safe-hash-rs`/`safers-cli` | Prevents subtle hash/signature drift | Higher correctness confidence before release |

## Problem (Detailed Context)

`rusty-safe` currently provides verification and hash computation, but no integrated transaction/message signing, signature collection, or WalletConnect request handling. The product needs a single runtime architecture for browser WASM that:

1. Uses direct injected wallet communication (`EIP-1193`) as the default signer path.
2. Reuses robust Rust components from Alloy where they fit.
3. Reuses Safe-specific logic patterns from `safers-cli` without importing its native-only hardware runtime.
4. Reuses `safe-hash-rs` runtime-safe primitives already used in verification paths.
5. Reaches practical parity with `localsafe.eth` signing workflows.

Current verification-first posture is visible in:

- `crates/rusty-safe/src/app.rs:277`
- `crates/rusty-safe/src/app.rs:861`
- `crates/rusty-safe/src/state.rs:323`

## Goals

1. Add full Safe tx signing and execution workflows in-browser (WASM).
2. Add full Safe message signing workflows (including threshold collection).
3. Add WalletConnect request handling parity for tx and sign methods.
4. Keep verification-first gating and hardening requirements intact.
5. Define explicit wiring boundaries for Alloy, `safers-cli`, and `safe-hash-rs`.

## Non-Goals (This PRD)

1. Building custom Ledger/Trezor firmware or native HID flows in WASM.
2. Replacing existing verification tabs and hash behavior.
3. Full backend coordination service (state remains local-first in scope of parity).

## 2. Core Architecture

### System Diagram

```text
+----------------------------+       +----------------------+
| UI Layer (egui screens)    |       | WalletConnect Client |
| app.rs / signing panels    |       | (session + requests) |
+-------------+--------------+       +----------+-----------+
              |                                 |
              v                                 v
+-----------------------------------------------------------+
| Signing Orchestrator (single-writer event loop)          |
| crates/rusty-safe/src/signing/orchestrator.rs            |
+------------------+----------------------+-----------------+
                   |                      |
                   v                      v
+------------------------+      +---------------------------+
| State Machine Core     |      | Adapters                  |
| signing/state_machine  |      | eip1193 / safe_service    |
| signing/domain         |      | queue / telemetry         |
+-----------+------------+      +------------+--------------+
            |                                |
            v                                v
+-------------------------+      +--------------------------+
| Hash/Calldata Engine    |      | External Systems         |
| safe-utils / hash.rs    |      | Injected wallet provider |
+-------------------------+      | Safe Tx Service          |
                                 +--------------------------+
```

### Data Flow Overview

1. User action or WalletConnect request enters orchestrator as a typed event.
2. State machine validates guards and emits next state plus declarative side effects.
3. Adapter executes side effect (`EIP-1193`, Safe Service, persistence) with idempotency context.
4. Adapter result is fed back as a new event and applied deterministically.
5. UI renders from persisted state snapshots and transition logs only.

### Design Principles

1. Deterministic first: all signing-critical logic must live in pure Rust state machines with replayable transitions.
2. Ports and adapters: provider, WalletConnect, Safe Service, storage, and telemetry are adapters around a shared core.
3. Fail closed: uncertainty in chain/account/domain/signature ownership blocks signing or execution by default.
4. Idempotent by construction: every external side effect (propose/confirm/respond/execute) requires stable idempotency keys.
5. Compatibility before optimization: support multiple injected provider patterns (`EIP-6963` + legacy `window.ethereum`) from day one.
6. Progressive enhancement: parity with `localsafe.eth` is P0; additional risk controls and operability are P1+.

### Architecture Decision

### Primary Runtime Path

Use **WASM + direct EIP-1193** as the production signing path.

Rationale:

1. Browser wallets already solve account management and hardware passthrough (Ledger/Trezor through wallet).
2. Avoids native HID dependencies that are incompatible with current pure WASM deployment.
3. Minimizes platform-specific transport complexity.

### Runtime Architecture Pattern

Split the signing subsystem into four layers:

1. Core domain layer (pure Rust, deterministic):
   - tx/message state machines
   - signature normalization + merge
   - threshold evaluation
   - idempotency key generation
2. Adapter layer (impure/runtime specific):
   - EIP-1193 provider adapter
   - WalletConnect adapter
   - Safe Transaction Service adapter
   - persistence adapter
3. Orchestration layer:
   - drives transitions based on user and network events
   - enforces guards before mutating state
4. UI layer:
   - declarative rendering from state
   - no direct provider/service calls outside orchestrators

This architecture enables deterministic unit/property tests for critical behavior and isolates browser/runtime instability to adapter boundaries.

### State Machine Model

Define explicit finite state machines for `PendingSafeTx`, `PendingSafeMessage`, and `WalletConnectRequest`.

1. Every transition must declare:
   - allowed current states
   - required guards
   - side effects
   - compensating behavior on failure
2. Illegal transitions are rejected and recorded as structured errors.
3. Transition log entries are persisted to support crash recovery and debugging.

## Alloy Wiring

Use Alloy as the **core Rust abstraction layer** for primitives, signing types, and provider architecture.

1. Use Alloy types for canonical domain models (`Address`, `B256`, `U256`, tx structs).
2. Implement an EIP-1193 transport adapter that can be connected through Alloy provider extension points:
   - `deps/alloy/crates/transport/src/trait.rs:5`
   - `deps/alloy/crates/transport/src/connect.rs:17`
   - `deps/alloy/crates/provider/src/builder.rs:466`
3. Treat direct Alloy hardware signers as future experimental modules, not P0 shipping path:
   - `deps/alloy/crates/signer-ledger/src/signer.rs:91`
   - `deps/alloy/crates/signer-trezor/src/signer.rs:42`

## `safers-cli` Wiring

Use `safers-cli` as a **logic/reference donor**, not as a runtime dependency for signing transport.

Reusable patterns to port or mirror:

1. Safe tx hash generation invariants:
   - `deps/safers-cli/src/utils.rs:160`
2. Safe Transaction Service payload model:
   - `deps/safers-cli/src/types.rs:15`
3. Proposal/confirmation endpoint usage:
   - `deps/safers-cli/src/commands.rs:1072`
   - `deps/safers-cli/src/commands.rs:1272`

Do not embed as-is in WASM:

1. Native HID hardware transport:
   - `deps/safers-cli/src/hardware_wallet.rs:517`
2. CLI stdin/passphrase runtime assumptions:
   - `deps/safers-cli/src/hardware_wallet.rs:130`

## `safe-hash-rs` Wiring

Use `safe-hash-rs` as the **primary shared verification/signing primitive layer** where it already fits runtime constraints.

Runtime-safe reuse targets:

1. Keep `safe-utils` hashers as canonical hashing implementation for tx/message/domain:
   - `deps/safe-hash-rs/crates/safe-utils/src/hasher.rs:127`
2. Keep typed-data hash logic through `Eip712Hasher`:
   - `deps/safe-hash-rs/crates/safe-utils/src/eip712.rs:20`
3. Reuse Safe Transaction Service models/fetch/validation from `safe-hash`:
   - `deps/safe-hash-rs/crates/safe-hash/src/api.rs:24`
   - `deps/safe-hash-rs/crates/safe-hash/src/api.rs:123`
4. Reuse `FullTx::calldata()` / `calldata_hash()` to generate deterministic `execTransaction` call payload at execute time:
   - `deps/safe-hash-rs/crates/safe-utils/src/hasher.rs:89`

Do not pull into app runtime:

1. CLI-only flows and blocking output paths:
   - `deps/safe-hash-rs/crates/safe-hash/src/main.rs:25`
   - `deps/safe-hash-rs/crates/safe-hash/src/lib.rs:17`
2. Any `cli`-feature-only dependency behavior:
   - `deps/safe-hash-rs/crates/safe-hash/Cargo.toml:18`

## Target Module Topology

Add a signing subsystem under `crates/rusty-safe/src/signing/`:

1. `signing/domain.rs`
   - Canonical tx/message/signature structs and state enums.
2. `signing/eip1193.rs`
   - WASM bindings for provider request dispatch and normalized events.
3. `signing/signer.rs`
   - `SafeSigner` trait + injected-wallet signer implementation.
4. `signing/providers.rs`
   - Provider discovery/selection registry (`EIP-6963` + legacy `window.ethereum`) and capability metadata.
5. `signing/state_machine.rs`
   - Deterministic transition logic for tx/message/WalletConnect lifecycle.
6. `signing/preflight.rs`
   - Simulation, policy checks, and pre-sign/pre-execute risk evaluation.
7. `signing/safe_service.rs`
   - Safe Transaction Service client: propose, list, confirm, and status polling, reusing `safe-hash` API models where possible.
8. `signing/hash.rs`
   - Safe tx/message hash pipeline; parity tests against existing `safe-hash-rs` and `safers-cli` vectors.
9. `signing/queue.rs`
   - Local tx/message/signature queue persistence, merge logic, provenance metadata, integrity fields.
10. `signing/wc.rs`
   - WalletConnect request normalization, lifecycle, and response modes.
11. `signing/execute.rs`
   - Final execution path once threshold is met, including deferred WC response resolution and deterministic `execTransaction` calldata generation via `safe-utils::FullTx`.
12. `signing/telemetry.rs`
   - Structured event/error emission and redaction-safe diagnostics.

## Product Requirements

## Functional Requirements

FR-1 Provider Discovery And Connect

1. WASM app must discover injected providers through `EIP-6963` when available and fallback to legacy `window.ethereum`.
2. App must support deterministic provider selection with explicit user choice when multiple providers exist.
3. App must detect chain/account/provider changes and revalidate Safe context ownership.
4. App must show provider identity (`name`, `rdns`, account, chain) in signing flows.

FR-2 Signer Abstraction

1. Introduce a `SafeSigner` trait with methods for tx typed-data signing, personal message signing, and account identity.
2. Injected wallet signer must implement method mapping for:
   - `eth_signTypedData_v4`
   - `personal_sign`
   - `eth_sign` (only when explicitly needed and supported)
3. Signing method used per flow must be stored in signature provenance.
4. Signature normalization rules (including `eth_sign` `v` normalization) must be centralized and deterministic.

FR-3 Safe Transaction Pipeline

1. User can create/import tx payloads, compute hashes, and sign.
2. App can propose tx to Safe Transaction Service and collect confirmations.
3. App can execute once threshold signatures are available, using deterministic `execTransaction` calldata construction.
4. Manual signature import/export and merge behavior must remain supported.

FR-4 Safe Message Pipeline

1. User can sign Safe messages with method-specific normalization.
2. App collects threshold signatures and outputs deterministic combined signature bytes.
3. Manual import/export/merge of message signatures must be supported.

FR-5 WalletConnect Pipeline

1. Support request lifecycle for:
   - `eth_sendTransaction`
   - `personal_sign`
   - `eth_sign`
   - `eth_signTypedData`
   - `eth_signTypedData_v4`
2. Support quick response (Safe tx hash) and deferred response (on-chain tx hash after execution).
3. On request expiry/cancel, preserve local signing progress and expose recovery UX.

FR-6 Local Collaboration

1. Queue state must support export/import/share-link flows for tx and message signatures.
2. Signature merge must be deterministic by signer address and signature bytes.
3. Nonce/hash collisions must surface explicit conflict UI and resolution path.

FR-7 Nonce And Idempotency Controls

1. Proposal and confirmation requests must carry deterministic idempotency keys derived from `(chain_id, safe, safe_tx_hash, signer, action)`.
2. Queue must detect duplicate propose/confirm attempts and collapse retries safely.
3. Nonce acquisition must support optimistic flow with service reconciliation and explicit stale-nonce remediation UX.

FR-8 Preflight Risk Evaluation

1. Before sign and before execute, app must run preflight checks:
   - calldata decode
   - target/method risk hints
   - simulation (`eth_call`) and gas estimation
2. Preflight failures hard-block execution and can soft-block signing based on policy configuration.
3. Preflight report must be persisted with tx state for reproducibility.

FR-9 Contract Owner Compatibility

1. Signature collection and threshold checks must support owner types beyond EOAs where Safe semantics require it (for example `EIP-1271` contract owners).
2. Owner validation path must distinguish `EOA`, `ContractOwner`, and `Unknown` with deterministic handling rules.

FR-10 Recovery And Resume

1. On app refresh/crash, in-flight tx/message/WalletConnect flows must resume from persisted state without loss of signed artifacts.
2. Deferred WalletConnect responses must survive app restarts until explicit completion or expiration.

## Security Requirements

SR-1 Verify-Before-Sign Gate

1. Sign action is blocked until current payload hash and decode are computed.
2. Any decode/hash parse failure blocks signing until resolved.

SR-2 Chain And Account Binding

1. Signer account must be confirmed as Safe owner before signature acceptance.
2. Chain mismatch between UI, Safe context, and wallet provider must hard-block signing/execution.

SR-3 Signature Provenance And Integrity

1. Every signature stores source, method, signer, and timestamp metadata.
2. Persisted queue objects include integrity metadata; failed integrity checks fail closed.

SR-4 WalletConnect Origin Safety

1. Request pages show dApp origin metadata before user action.
2. Unsupported/expired requests are rejected deterministically with explicit reason.

SR-5 Transport Hardening

1. EIP-1193 adapter must sanitize method/param encoding and decode provider errors into stable app errors.
2. No native HID transport paths are enabled in WASM build.

SR-6 Safe Context Attestation

1. App must verify Safe version/domain inputs against on-chain context before signature acceptance.
2. Any mismatch between expected domain separator inputs and resolved Safe context must hard-block signing.

SR-7 Local Data Hardening

1. Persisted signing queue must be versioned with schema migration guards and integrity verification.
2. Corrupt or untrusted imported state must be quarantined, never auto-merged silently.

## Non-Functional Requirements

NFR-1 Reliability

1. All networked side effects must use bounded retry with jittered exponential backoff and explicit retry budgets.
2. Side effects must be safely replayable using idempotency keys.
3. Crash recovery must preserve all signed payloads and signatures.

NFR-2 Performance

1. Hashing/merge operations should complete in under 50 ms for typical single-tx flows on commodity laptops.
2. Queue load/rehydration target: under 200 ms for 500 pending objects.
3. UI should remain responsive during service polling and provider event bursts.

NFR-3 Operability

1. Emit structured telemetry for lifecycle events, errors, and retry behavior with redaction-safe payloads.
2. Expose per-flow diagnostics (current state, last transition, last external error) for support and debugging.

NFR-4 Compatibility

1. P0 wallet matrix must include at least MetaMask and one alternate injected provider.
2. Provider capability probing must gracefully degrade when method support differs by wallet.

## 3. Data Models

### Equivalent Rust Type Definitions (Schema Source of Truth)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingSafeTx {
    pub schema_version: u16,
    pub chain_id: u64,
    pub safe_address: Address,
    pub safe_version: String,
    pub nonce: u64,
    pub payload: SafeTxData,
    pub safe_tx_hash: B256,
    pub signatures: Vec<CollectedSignature>,
    pub status: TxStatus,
    pub state_revision: u64,
    pub origin: FlowOrigin,
    pub idempotency_key: String,
    pub service_tx_id: Option<String>,
    pub preflight: Option<PreflightReport>,
    pub integrity_hash: B256,
    pub last_error: Option<FlowError>,
    pub retry_count: u32,
    pub created_at: u64,
    pub updated_at: u64,
    pub executed_tx_hash: Option<B256>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TxStatus {
    Draft,
    Preflighted,
    Proposed,
    Confirming,
    ReadyToExecute,
    Executed,
    Failed,
}
```

### Cross-Flow Model Inventory

```text
ConnectedWallet {
  provider_id: String
  provider_rdns: Option<String>
  wallet_label: String
  chain_id: u64
  account: Address
  capabilities: Vec<SignerCapability>
  connected_at: u64
}

PendingSafeTx {
  schema_version: u16
  chain_id: u64
  safe_address: Address
  safe_version: String
  nonce: u64
  payload: SafeTxData
  safe_tx_hash: B256
  signatures: Vec<CollectedSignature>
  status: Draft | Preflighted | Proposed | Confirming | ReadyToExecute | Executed | Failed
  state_revision: u64
  origin: Manual | WalletConnect { request_id: String } | Import
  idempotency_key: String
  service_tx_id: Option<String>
  preflight: Option<PreflightReport>
  integrity_hash: B256
  last_error: Option<FlowError>
  retry_count: u32
  created_at: u64
  updated_at: u64
  executed_tx_hash: Option<B256>
}

PendingSafeMessage {
  schema_version: u16
  chain_id: u64
  safe_address: Address
  method: PersonalSign | EthSign | TypedDataV4 | TypedData
  message_hash: B256
  signatures: Vec<CollectedSignature>
  status: Draft | Collecting | ThresholdMet | Responding | Responded | Failed
  state_revision: u64
  origin: Manual | WalletConnect { request_id: String } | Import
  idempotency_key: String
  integrity_hash: B256
  last_error: Option<FlowError>
  retry_count: u32
  created_at: u64
  updated_at: u64
}

PendingWalletConnectRequest {
  schema_version: u16
  request_id: String
  topic: String
  origin: String
  chain_id: u64
  method: EthSendTransaction | PersonalSign | EthSign | EthSignTypedData | EthSignTypedDataV4
  status: Received | Validated | Queued | RespondingQuick | RespondingDeferred | Responded | Rejected | Expired | Failed
  linked_safe_tx_hash: Option<B256>
  idempotency_key: String
  state_revision: u64
  last_error: Option<FlowError>
  retry_count: u32
  expires_at: Option<u64>
  created_at: u64
  updated_at: u64
}

CollectedSignature {
  signer: Address
  signature: Bytes
  source: Wallet | Manual | Import
  method: SafeTypedData | PersonalSign | EthSign
  added_at: u64
}

PreflightReport {
  simulated: bool
  success: bool
  gas_estimate: Option<U256>
  warnings: Vec<String>
  generated_at: u64
}

TransitionLogEntry {
  flow_id: String
  state_revision: u64
  event: String
  from_state: String
  to_state: String
  guard_result: Passed | Failed
  side_effect: None | Requested | Succeeded | Failed
  timestamp: u64
}
```

### Validation Rules

| Entity | Validation Rules |
|---|---|
| `ConnectedWallet` | `chain_id > 0`; `account` must be checksummed/display-normalized; `provider_id` non-empty |
| `PendingSafeTx` | `safe_tx_hash` must match recomputed hash for `payload`; `state_revision` monotonic; `idempotency_key` stable per action; `status=Executed` requires `executed_tx_hash` |
| `PendingSafeMessage` | `message_hash` must match normalized message content; `status=Responded` requires linked completion event |
| `PendingWalletConnectRequest` | `request_id` unique per topic; expiration must be checked before response; unsupported method transitions only to `Rejected` |
| `CollectedSignature` | `signer` must be Safe owner or allowed owner type; signature bytes must parse and normalize before merge |
| `PreflightReport` | `generated_at` must be <= execution attempt time; stale preflight must be invalidated by payload/chain/account changes |
| `TransitionLogEntry` | `(flow_id, state_revision)` unique; `from_state -> to_state` must be legal per FSM table |

### Entity Relationships

1. `PendingSafeTx.signatures[*].signer` references Safe owners resolved for `(chain_id, safe_address)`.
2. `PendingWalletConnectRequest.linked_safe_tx_hash` references `PendingSafeTx.safe_tx_hash`.
3. `PendingSafeMessage.origin.WalletConnect.request_id` references `PendingWalletConnectRequest.request_id`.
4. `TransitionLogEntry.flow_id` references one of `PendingSafeTx`/`PendingSafeMessage`/`PendingWalletConnectRequest`.
5. `PreflightReport` belongs to exactly one `PendingSafeTx` revision.

## 4. CLI/API Surface

There is no end-user CLI in P0 scope. The execution surface consists of:

1. Internal orchestrator commands (typed events).
2. Wallet/provider JSON-RPC calls (`EIP-1193`).
3. Safe Transaction Service HTTP endpoints.
4. WalletConnect response API.

### 4.1 Internal Command Surface (Orchestrator)

| Command | Purpose | Input | Output |
|---|---|---|---|
| `connect_provider` | Bind active wallet provider/account | `{ "command":"connect_provider", "provider_id":"io.metamask", "request_id":"req-1" }` | `{ "ok":true, "wallet":{"provider_id":"io.metamask","account":"0x...","chain_id":1} }` |
| `start_preflight` | Run simulation/risk checks | `{ "command":"start_preflight", "safe_tx_hash":"0xabc...", "request_id":"req-2" }` | `{ "ok":true, "preflight":{"simulated":true,"success":true,"gas_estimate":"210000"} }` |
| `propose_tx` | Submit tx to Safe service | `{ "command":"propose_tx", "safe_tx_hash":"0xabc...", "request_id":"req-3" }` | `{ "ok":true, "service_tx_id":"svc-123", "state":"Proposed" }` |
| `confirm_tx` | Submit signer confirmation | `{ "command":"confirm_tx", "safe_tx_hash":"0xabc...", "signature":"0x...", "request_id":"req-4" }` | `{ "ok":true, "state":"Confirming" }` |
| `execute_tx` | Execute threshold-met tx | `{ "command":"execute_tx", "safe_tx_hash":"0xabc...", "request_id":"req-5" }` | `{ "ok":true, "executed_tx_hash":"0xdef...", "state":"Executed" }` |
| `respond_wc` | Complete WalletConnect request | `{ "command":"respond_wc", "request_id":"wc-7", "mode":"quick" }` | `{ "ok":true, "wc_status":"Responded" }` |

Command error envelope:

```json
{
  "ok": false,
  "error": {
    "code": "CHAIN_MISMATCH",
    "message": "Wallet chain does not match Safe chain",
    "retryable": false,
    "correlation_id": "corr-93c2"
  }
}
```

### 4.2 `EIP-1193` Request Surface

Required methods:

1. `eth_requestAccounts`
2. `eth_chainId`
3. `eth_signTypedData_v4`
4. `personal_sign`
5. `eth_sendTransaction`
6. Optional fallback: `eth_sign`

Example (`eth_signTypedData_v4`):

```json
{
  "method": "eth_signTypedData_v4",
  "params": [
    "0x1234...abcd",
    "{\"domain\":{\"chainId\":1,\"verifyingContract\":\"0xSafe...\"},\"types\":{\"SafeTx\":[...]},\"message\":{...}}"
  ]
}
```

Example success response:

```json
{
  "result": "0x5d0f...1c"
}
```

Example provider error:

```json
{
  "error": {
    "code": 4001,
    "message": "User rejected the request."
  }
}
```

Mapped internal error:

```json
{
  "code": "PROVIDER_REJECTED",
  "retryable": false
}
```

### 4.3 Safe Transaction Service Endpoint Surface

Base URL: `https://safe-transaction-<chain>.safe.global/api/v1`

Endpoints in scope:

1. `GET /safes/{safe_address}/`
2. `GET /multisig-transactions/{safe_tx_hash}/`
3. `POST /safes/{safe_address}/multisig-transactions/`
4. `POST /multisig-transactions/{safe_tx_hash}/confirmations/`

Propose example:

```bash
curl -X POST "$SAFE_TX_SERVICE/api/v1/safes/0xSafe.../multisig-transactions/" \
  -H "Content-Type: application/json" \
  -d '{
    "safe": "0xSafe...",
    "to": "0xTarget...",
    "value": "0",
    "data": "0x...",
    "operation": 0,
    "safeTxGas": "0",
    "baseGas": "0",
    "gasPrice": "0",
    "gasToken": "0x0000000000000000000000000000000000000000",
    "refundReceiver": "0x0000000000000000000000000000000000000000",
    "nonce": 42,
    "contractTransactionHash": "0xabc...",
    "sender": "0xOwner...",
    "signature": "0x..."
  }'
```

Propose success (example):

```json
{
  "safeTxHash": "0xabc...",
  "nonce": 42
}
```

Confirm example:

```bash
curl -X POST "$SAFE_TX_SERVICE/api/v1/multisig-transactions/0xabc.../confirmations/" \
  -H "Content-Type: application/json" \
  -d '{ "signature": "0x..." }'
```

Service error (example):

```json
{
  "code": 409,
  "message": "Transaction already exists"
}
```

Mapped internal error:

```json
{
  "code": "DUPLICATE_SIDE_EFFECT",
  "retryable": false
}
```

### 4.4 WalletConnect Response Surface

Supported request methods:

1. `eth_sendTransaction`
2. `personal_sign`
3. `eth_sign`
4. `eth_signTypedData`
5. `eth_signTypedData_v4`

Quick response payload example:

```json
{
  "id": 717,
  "jsonrpc": "2.0",
  "result": "0xSAFE_TX_HASH"
}
```

Deferred response payload example:

```json
{
  "id": 717,
  "jsonrpc": "2.0",
  "result": "0xONCHAIN_EXEC_TX_HASH"
}
```

Reject response payload example:

```json
{
  "id": 717,
  "jsonrpc": "2.0",
  "error": {
    "code": 4001,
    "message": "User rejected request"
  }
}
```

## 5. Error Handling & Edge Cases

| Failure Mode | Detection | Recovery Strategy | Mitigation |
|---|---|---|---|
| Wallet chain changed mid-flow | `chainChanged` event or pre-sign guard failure | Pause flow, require explicit rebind/retry | Hard guard in FSM (`CHAIN_MISMATCH`) |
| Provider does not support method | `Unsupported method` provider error | Try configured fallback method if policy allows | Capability probing + deterministic fallback map |
| User rejects signature | Provider error `4001` | Keep flow state; allow retry without state loss | Preserve payload hash and pending action |
| Duplicate propose/confirm attempt | 409/conflict or matching idempotency replay | Collapse duplicates to single logical action | Stable idempotency keys + dedupe |
| Safe service timeout/rate limit | HTTP timeout/429/5xx | Retry with bounded exponential backoff | Retry budget + jitter + circuit-breaker window |
| Nonce stale/conflict | Service returns mismatch or conflicting tx state | Reconcile nonce, surface conflict UI, require re-propose | Stale nonce remediation workflow |
| Corrupted imported signature bundle | Schema/integrity validation failure | Quarantine import; show actionable error | Versioned envelope + hash integrity checks |
| WalletConnect request expires during signing | Compare `expires_at` with now | Persist local signing artifacts; mark request `Expired` | Resume/export path for completed signatures |
| App crash/refresh during deferred response | Missing in-memory context after restart | Rehydrate from persisted queue + transition log; replay pending side effects | Durable `PendingWalletConnectRequest` store |
| Preflight stale at execute time | Payload/account/chain revision changed since report | Invalidate report and rerun preflight | State revision binding on `PreflightReport` |
| EIP-1271 owner signature ambiguity | Owner type detection returns `Unknown` | Block threshold progression until owner type is resolved | Explicit `EOA`/`ContractOwner`/`Unknown` policy |

Error response contract:

```json
{
  "code": "STRING_ENUM",
  "message": "Human-readable summary",
  "retryable": true,
  "correlation_id": "corr-uuid",
  "details": {
    "flow_id": "tx:0xabc...",
    "state_revision": 14
  }
}
```

Required error enum families:

1. Provider: `PROVIDER_REJECTED`, `UNSUPPORTED_METHOD`, `CHAIN_MISMATCH`, `PROVIDER_DISCONNECTED`
2. Service: `SAFE_SERVICE_TIMEOUT`, `SAFE_SERVICE_RATE_LIMITED`, `SAFE_SERVICE_CONFLICT`
3. State: `ILLEGAL_TRANSITION`, `INTEGRITY_FAILURE`, `PRECONDITION_FAILED`
4. WalletConnect: `WC_EXPIRED`, `WC_UNSUPPORTED_METHOD`, `WC_RESPONSE_FAILED`

## Localsafe Parity Matrix

Baseline source:

- `prds/LOCALSAFE_SIGNING_FLOW_AND_CAPABILITIES.md:6`
- `prds/LOCALSAFE_SIGNING_FLOW_AND_CAPABILITIES.md:125`

Required parity set:

1. Build tx, compute hash, sign, execute.
2. Threshold-aware execution gating.
3. Manual signature entry for tx and messages.
4. Import/export/share tx and message signature payloads.
5. WalletConnect tx and message request handling with deferred tx response mode.

Parity-plus set (explicitly above baseline):

1. Deterministic multi-provider discovery and selection.
2. Idempotent propose/confirm/respond semantics with recovery-safe retries.
3. Mandatory pre-execute simulation and persisted risk report.
4. Crash-safe resume of deferred WalletConnect responses.

Parity acceptance rules:

1. Each capability above must be implemented with deterministic tests.
2. Any capability intentionally deferred must be flagged as `Parity Gap` in release checklist.
3. `sign-message` route must be full signing-capable (avoid hash-only behavior gap observed in localsafe baseline).

## 6. Integration Points

### External Dependencies

| Dependency | Purpose | Runtime Mode | Constraints |
|---|---|---|---|
| Injected wallet provider (`EIP-1193`) | Signing, transaction broadcast, account identity | Browser WASM | Varies by wallet capability |
| Safe Transaction Service | Proposal/confirmation/sync | HTTPS API | Service lag/conflict must be tolerated |
| `alloy` | Types + transport abstractions | Rust/WASM | Keep error taxonomy stable |
| `safe-hash-rs` (`safe-utils`, `safe-hash`) | Hashing + service model parity | Rust/WASM-safe subset | No CLI-only codepaths |
| `safers-cli` (reference only) | Behavioral parity vectors | Offline/reference | Do not pull native HID/runtime code |
| WalletConnect runtime | Session/request lifecycle | Browser WASM | Expiration/retry semantics required |

### Secret And Credential Handling

1. No private keys are stored by `rusty-safe`; signing happens in wallet/provider.
2. Safe service endpoints are public; optional auth headers must be loaded from runtime config only.
3. Any optional RPC API key used for simulation must be read from config and never persisted in queue exports.
4. Telemetry payloads must redact addresses/signatures unless explicit debug mode is enabled.

### Configuration Management

Config source order:

1. Compile-time defaults (`crates/rusty-safe/src/signing/config.rs`).
2. Runtime overrides from browser storage (`rusty_safe.signing.config.v1`).
3. Optional query-string/dev override in debug builds only.

Config keys:

| Key | Default | Purpose |
|---|---|---|
| `safe_service_base_url` | per-chain default | Safe service endpoint root |
| `preflight_required` | `true` | block execute when preflight unavailable/failed |
| `retry_max_attempts` | `5` | global retry budget per side effect |
| `retry_base_delay_ms` | `300` | backoff seed |
| `retry_max_delay_ms` | `5000` | backoff cap |
| `wc_deferred_ttl_ms` | `86400000` | deferred WC response retention |
| `diagnostics_enabled` | `false` | enable verbose flow diagnostics |

### Integration Requirements: Alloy

AR-1

Provider stack must run through Alloy abstractions with an EIP-1193 transport connector.

AR-2

Typed-data and signing payload preparation must use Alloy-compatible primitives and serde models end-to-end.

AR-3

Direct `alloy-signer-ledger` and `alloy-signer-trezor` integrations are optional and gated behind experimental feature flags after browser compatibility matrix is validated.

AR-4

Alloy transport boundary must expose a unified error taxonomy (`ProviderRejected`, `UnsupportedMethod`, `ChainMismatch`, `TransportFailure`, `RateLimited`) used across UI and retries.

### Integration Requirements: `safers-cli`

SRC-1

Port hash and Safe service serialization logic as pure Rust modules (no CLI runtime assumptions).

SRC-2

Do not port native hardware transport logic (`ledger-transport-hid`, stdin passphrase prompts) into WASM path.

SRC-3

Maintain parity tests between `rusty-safe` hash/service payload outputs and known `safers-cli` behavior for shared scenarios.

SRC-4

Port or recreate signature normalization fixtures for `eth_sign` and confirmation payload shapes to avoid subtle signature-semantic drift.

### Integration Requirements: `safe-hash-rs`

SHR-1

Keep `safe-utils` and `safe-hash` (`default-features = false`) as canonical hashing/API primitives to avoid logic drift between verify and sign paths.

SHR-2

Execution payload generation must use or remain byte-parity compatible with `safe-utils::FullTx::calldata()` for `execTransaction`.

SHR-3

Signature collection state should reuse `safe-hash` Safe API confirmation models where possible (`Confirmation`, `SafeTransaction`) instead of introducing divergent parallel types.

SHR-4

No CLI-only codepaths from `safe-hash-rs` may be pulled into WASM runtime modules.

SHR-5

Safe hash, calldata hash, and `execTransaction` calldata generation must be regression-tested against pinned `safe-hash-rs` commit vectors before each release.

## 7. Storage & Persistence

### Repository Directory Structure

```text
crates/rusty-safe/src/signing/
  domain.rs
  state_machine.rs
  orchestrator.rs
  eip1193.rs
  providers.rs
  safe_service.rs
  queue.rs
  wc.rs
  execute.rs
  preflight.rs
  telemetry.rs
  config.rs

crates/rusty-safe/tests/signing/
  unit/
  integration/
  e2e/
  fixtures/
```

### Runtime Persistence Layout (Browser)

Primary store: IndexedDB database `rusty_safe_signing_v1`.

Object stores:

1. `pending_safe_txs` keyed by `safe_tx_hash`
2. `pending_safe_messages` keyed by `message_hash`
3. `pending_wc_requests` keyed by `request_id`
4. `transition_log` keyed by `(flow_id, state_revision)`
5. `flow_metadata` keyed by `flow_id`

Secondary store: `localStorage`.

Keys:

1. `rusty_safe.signing.config.v1`
2. `rusty_safe.signing.active_provider.v1`
3. `rusty_safe.signing.ui_prefs.v1`

### File Formats

Export bundle format: JSON, versioned envelope.

```json
{
  "schema_version": 1,
  "exported_at": 1739750400000,
  "txs": [/* PendingSafeTx */],
  "messages": [/* PendingSafeMessage */],
  "wc_requests": [/* PendingWalletConnectRequest */],
  "integrity_hash": "0x..."
}
```

Import behavior:

1. Validate schema version and required fields.
2. Validate integrity hash.
3. Quarantine invalid objects (do not auto-merge).
4. Merge valid objects through deterministic dedupe rules.

### Caching Strategy

1. Safe service tx snapshot cache: TTL 15s, key `(chain_id, safe_tx_hash)`.
2. Safe nonce cache: TTL 10s, key `(chain_id, safe_address)`, invalidated after propose.
3. Preflight result cache: TTL 30s, key `(chain_id, safe_tx_hash, state_revision)`.
4. Provider capability cache: session-lifetime key `(provider_id, wallet_version)`.
5. Cache misses never bypass required guards (for example preflight-required policy).

## 8. Implementation Roadmap

1. Phase A (P0): Core state machine scaffolding + schema versioning + provider discovery (`EIP-6963` + fallback).
2. Phase B (P0): Signer abstraction + tx hash/sign/propose/confirm path + idempotent service interactions.
3. Phase C (P0): Execute pipeline with mandatory preflight simulation and deterministic `safe-utils::FullTx` calldata generation.
4. Phase D (P0): Message signing + threshold collection + combined signatures, including contract-owner compatibility checks.
5. Phase E (P0): WalletConnect request pipeline (tx + sign methods) with durable deferred response queue.
6. Phase F (P1): Collaboration hardening (integrity, conflict resolution, recovery UX, diagnostics panel).
7. Phase G (P2): Experimental direct hardware signers through Alloy, gated by compatibility matrix.
8. Phase H (P2): Canary rollout instrumentation, kill-switch wiring, and production runbook validation.

### Phase Dependencies, Complexity, and Parallelization

| Phase | Depends On | Key Tasks | Complexity | Parallelization Opportunities |
|---|---|---|---|---|
| A | none | `domain.rs`, `state_machine.rs`, `queue.rs` schema versioning, provider registry skeleton | L | state machine + provider discovery can run in parallel |
| B | A | `signer.rs`, `eip1193.rs`, `safe_service.rs`, tx propose/confirm flow | L | provider adapter + service adapter in parallel |
| C | B | `preflight.rs`, `execute.rs`, `FullTx::calldata()` parity vectors | M | preflight + execute orchestration split |
| D | A,B | message pipeline + signature aggregation + EIP-1271 owner handling | M | owner-resolution and message merge test tracks |
| E | A,B,D | `wc.rs` durable request/response queue + deferred completion | L | WC adapter and persistence replay testing |
| F | A-E | diagnostics panel + integrity hardening + import quarantine UX | M | diagnostics and quarantine flows parallel |
| G | B | experimental Alloy hardware signer adapters behind flags | M | ledger/trezor experiments parallel |
| H | A-F | canary metrics, kill-switch plumbing, runbook validation | S | rollout docs + instrumentation parallel |

## Risks And Mitigations

1. Provider fragmentation risk:
   - Mitigation: `EIP-6963` first, capability probing, deterministic fallback path.
2. Duplicate side effects (propose/confirm/respond):
   - Mitigation: idempotency keys + replay-safe transition model.
3. Signature semantic drift across wallets:
   - Mitigation: centralized normalization + cross-implementation golden vectors.
4. Runtime instability from async event races:
   - Mitigation: single-writer orchestrator and serialized transition application.
5. Service inconsistency or lag:
   - Mitigation: optimistic local state with reconciliation loop and explicit stale-state UX.

## Acceptance Criteria

1. User can complete tx signing and execution in-browser without leaving `rusty-safe`.
2. User can complete message signing and produce deterministic combined signatures.
3. WalletConnect supported methods produce deterministic approve/reject behavior.
4. Imported/manual signatures merge correctly and never bypass threshold gating.
5. Verify-before-sign gate is enforced for tx and message flows.
6. Feature parity checklist derived from `prds/LOCALSAFE_SIGNING_FLOW_AND_CAPABILITIES.md` is marked complete for required capabilities.
7. Duplicate propose/confirm/respond attempts do not create duplicate external side effects.
8. Crash/reload recovery restores in-flight tx/message/WC flows without signature loss.
9. P0 wallet compatibility matrix passes with deterministic behavior for all required methods.
10. Preflight simulation results are persisted and visible before execution.

## 9. Testing Strategy

1. Unit tests:
   - Safe tx/message hash vectors and signature merge ordering.
   - EIP-1193 request/response normalization and error mapping.
   - Service payload serialization and integrity checks.
   - `FullTx::calldata()` byte-parity vectors for `execTransaction` payload generation.
   - state-machine transition legality and invariant tests.
   - idempotency-key generation determinism.
2. Integration tests:
   - build -> sign -> propose -> confirm -> execute.
   - WalletConnect quick and deferred tx response flows.
   - message signing threshold progression with manual import.
   - crash/restart recovery with replayed pending side effects.
   - stale nonce reconciliation and conflict resolution.
3. Browser E2E tests:
   - Provider connect/disconnect, account switch, chain switch.
   - wallet matrix run (MetaMask + at least one alternate wallet) for all required methods.
   - multi-provider environment with deterministic provider selection.
4. Negative tests:
   - unsupported method, chain mismatch, stale request id, malformed signature import.
   - duplicate request replay, service timeout budget exhaustion, corrupted local queue envelope.
5. Differential tests:
   - compare hash/calldata/service payload outputs against pinned fixtures from `safe-hash-rs` and selected `safers-cli` vectors.

### Test Data Requirements

1. Canonical Safe tx/message fixtures for at least:
   - 2 chains (`1`, `11155111`)
   - 2 Safe versions
   - mixed signature methods (`typedData`, `personal_sign`, optional `eth_sign`)
2. Wallet capability fixtures per tested provider (`supports_eth_sign`, `supports_switch_chain`, typed-data quirks).
3. Safe service fixtures:
   - success propose/confirm
   - conflict (`409`)
   - timeout/rate-limit retry scenarios
4. WalletConnect fixtures:
   - quick response path
   - deferred response path
   - expired request path
5. Corruption fixtures:
   - malformed import envelope
   - integrity hash mismatch
   - illegal state transition replay

### Test File Path Plan

1. `crates/rusty-safe/tests/signing/unit/state_machine_transitions.rs`
2. `crates/rusty-safe/tests/signing/unit/hash_parity.rs`
3. `crates/rusty-safe/tests/signing/unit/idempotency.rs`
4. `crates/rusty-safe/tests/signing/integration/propose_confirm_execute.rs`
5. `crates/rusty-safe/tests/signing/integration/walletconnect_deferred.rs`
6. `crates/rusty-safe/tests/signing/e2e/provider_matrix_smoke.rs`
7. `crates/rusty-safe/tests/signing/fixtures/*.json`

## Execution Specification (Normative)

### Transaction State Transition Table (`PendingSafeTx`)

| Event | Allowed From | Guards | Side Effects | Next State | Failure Handling |
|---|---|---|---|---|---|
| `StartPreflight` | `Draft`, `Failed` | decode/hash computed; chain/account bound | run `eth_call` simulation + gas estimate; persist `PreflightReport` | `Preflighted` | set `last_error`; increment `retry_count`; move to `Failed` |
| `AddSignature` | `Preflighted`, `Proposed`, `Confirming` | signer is valid owner; signature parses and normalizes | merge/dedupe signature set; recalc threshold | same state or `ReadyToExecute` (if threshold met) | reject event; keep state unchanged |
| `ProposeTx` | `Preflighted`, `Failed` | nonce resolved; idempotency key available; not `Executed` | call Safe Service propose endpoint | `Proposed` | map error; keep deterministic retry metadata; move `Failed` |
| `ConfirmTx` | `Proposed`, `Confirming` | signer confirmation missing; threshold not already satisfied | call Safe Service confirmation endpoint | `Confirming` | map error; stay `Confirming` if retryable else `Failed` |
| `MarkThresholdMet` | `Proposed`, `Confirming` | signatures >= threshold | none | `ReadyToExecute` | reject event if guard fails |
| `ExecuteTx` | `ReadyToExecute` | preflight success; chain/account binding valid; tx not already executed | send execution tx through wallet/provider; persist tx hash | `Executed` | move `Failed`; keep retry-safe execute idempotency key |
| `ExternalError` | any non-terminal | none | persist error envelope + transition log | `Failed` | n/a |
| `Retry` | `Failed` | retry budget remaining; idempotency key stable | replay pending side effect only | previous intended state path | if budget exhausted, remain `Failed` |

### Message State Transition Table (`PendingSafeMessage`)

| Event | Allowed From | Guards | Side Effects | Next State | Failure Handling |
|---|---|---|---|---|---|
| `BeginCollect` | `Draft`, `Failed` | method resolved; hash computed | initialize collection metadata | `Collecting` | move `Failed` with `last_error` |
| `AddSignature` | `Collecting`, `ThresholdMet` | signer ownership and signature format valid | merge/dedupe signatures; threshold evaluation | `Collecting` or `ThresholdMet` | reject malformed/unauthorized signature |
| `RespondMessage` | `ThresholdMet` | linked WC request exists and not expired | send WC response payload | `Responding` then `Responded` | if transport error, set `Failed` and retain resumable context |
| `FinalizeWithoutWC` | `ThresholdMet` | no linked WC request | mark output complete for export/share | `Responded` | set `Failed` if serialization/export fails |
| `ExternalError` | any non-terminal | none | persist error envelope + transition log | `Failed` | n/a |
| `Retry` | `Failed` | retry budget remaining | replay pending response side effect | prior intended state path | remain `Failed` if non-retryable |

### WalletConnect Request State Transition Table (`PendingWalletConnectRequest`)

| Event | Allowed From | Guards | Side Effects | Next State | Failure Handling |
|---|---|---|---|---|---|
| `ReceiveRequest` | n/a (create) | payload parse success | persist request envelope | `Received` | create as `Failed` with parse reason |
| `ValidateRequest` | `Received` | method supported; chain matches policy; not expired | normalize params; bind context | `Validated` | transition `Rejected` with deterministic reason |
| `QueueFlow` | `Validated` | flow mapping available | create/link tx or message flow | `Queued` | transition `Failed` with recoverable mapping info |
| `RespondQuick` | `Queued` | quick mode policy satisfied | send immediate WC response (Safe tx hash) | `RespondingQuick` then `Responded` | transition `Failed`; allow retry |
| `RespondDeferredStart` | `Queued` | deferred mode required | persist deferred response handle | `RespondingDeferred` | transition `Failed`; allow retry |
| `RespondDeferredComplete` | `RespondingDeferred` | execute tx hash available | send deferred WC response | `Responded` | remain `RespondingDeferred` if retryable |
| `ExpireRequest` | any non-terminal | now >= `expires_at` | persist expiration metadata | `Expired` | n/a |
| `RejectRequest` | `Received`, `Validated`, `Queued` | user reject or policy reject | send reject response | `Rejected` | if send fails, remain `Rejected` and queue retry |

### State Machine Invariants

1. `state_revision` is monotonic and increments exactly once per accepted transition.
2. Transition application is single-writer per flow id.
3. Same `(flow_id, state_revision, event_id)` replay is idempotent no-op.
4. Terminal states (`Executed`, `Responded`, `Rejected`, `Expired`) cannot transition except for diagnostics-only metadata updates.
5. Any side effect must be emitted from transition output, never executed directly inside UI handlers.

## API Contracts (Normative)

### `signing/eip1193.rs`

```rust
pub struct ProviderDescriptor {
    pub provider_id: String,
    pub name: String,
    pub rdns: Option<String>,
    pub is_default: bool,
}

pub enum ProviderCapability {
    RequestAccounts,
    SwitchChain,
    SignTypedDataV4,
    PersonalSign,
    EthSign,
    SendTransaction,
}

pub struct RequestEnvelope {
    pub request_id: String,
    pub method: String,
    pub params: serde_json::Value,
    pub chain_id: Option<u64>,
    pub timeout_ms: u64,
}

pub enum ProviderEvent {
    AccountsChanged(Vec<String>),
    ChainChanged(u64),
    Disconnect,
    Message(String),
}

pub trait Eip1193Client {
    fn discover_providers(&self) -> Vec<ProviderDescriptor>; // EIP-6963 first, legacy fallback
    fn select_provider(&mut self, provider_id: &str) -> Result<(), ProviderError>;
    async fn request(&self, req: RequestEnvelope) -> Result<serde_json::Value, ProviderError>;
    fn subscribe_events(&self) -> ProviderEventStream;
}
```

Contract requirements:

1. Provider errors must map to stable taxonomy (`ProviderRejected`, `UnsupportedMethod`, `ChainMismatch`, `TransportFailure`, `RateLimited`).
2. Method/param normalization must be deterministic per wallet capability profile.
3. Event stream normalization must dedupe bursts and preserve order for the same provider.

### `signing/safe_service.rs`

```rust
pub struct ServiceRequestContext {
    pub idempotency_key: String,
    pub correlation_id: String,
    pub attempt: u32,
    pub deadline_ms: u64,
}

pub trait SafeServiceClient {
    async fn resolve_next_nonce(
        &self,
        chain_id: u64,
        safe: Address,
        ctx: ServiceRequestContext,
    ) -> Result<u64, SafeServiceError>;

    async fn propose_multisig_tx(
        &self,
        chain_id: u64,
        safe: Address,
        payload: ProposePayload,
        ctx: ServiceRequestContext,
    ) -> Result<SafeServiceProposeResult, SafeServiceError>;

    async fn confirm_multisig_tx(
        &self,
        chain_id: u64,
        safe_tx_hash: B256,
        payload: ConfirmPayload,
        ctx: ServiceRequestContext,
    ) -> Result<SafeServiceConfirmResult, SafeServiceError>;

    async fn get_multisig_tx(
        &self,
        chain_id: u64,
        safe_tx_hash: B256,
        ctx: ServiceRequestContext,
    ) -> Result<SafeTransaction, SafeServiceError>;
}
```

Contract requirements:

1. `ServiceRequestContext.idempotency_key` must remain stable across retries of the same logical action.
2. Retry policy applies only to retryable failures (timeout, rate limit, 5xx, transient network).
3. Error mapping must preserve retryability and user-facing reason.
4. Service payload serialization must stay byte-compatible with parity fixtures.

### `signing/state_machine.rs`

```rust
pub fn apply_tx_event(state: PendingSafeTx, event: TxEvent, now_ms: u64)
    -> TransitionOutcome<PendingSafeTx, SideEffect>;

pub fn apply_message_event(state: PendingSafeMessage, event: MessageEvent, now_ms: u64)
    -> TransitionOutcome<PendingSafeMessage, SideEffect>;

pub fn apply_wc_event(state: PendingWalletConnectRequest, event: WcEvent, now_ms: u64)
    -> TransitionOutcome<PendingWalletConnectRequest, SideEffect>;
```

Contract requirements:

1. Functions are pure and deterministic (no network/storage side effects).
2. Returned side effects are declarative work items for orchestrators.
3. Transition outcome includes structured guard failures for diagnostics.

## Wallet Compatibility Matrix Template (P0 Gate)

| Wallet | Discovery Path | Version | `eth_requestAccounts` | `eth_chainId` | `eth_signTypedData_v4` | `personal_sign` | `eth_sendTransaction` | `wallet_switchEthereumChain` | `eth_sign` | Ledger/Trezor Passthrough | Determinism Run (n=20) | Status | Blockers |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| MetaMask | EIP-6963 + fallback | TBD | TBD | TBD | TBD | TBD | TBD | TBD | TBD | TBD | TBD | Required | TBD |
| Rabby | EIP-6963 + fallback | TBD | TBD | TBD | TBD | TBD | TBD | TBD | TBD | TBD | TBD | Optional P0 | TBD |
| Coinbase Wallet Extension | EIP-6963 + fallback | TBD | TBD | TBD | TBD | TBD | TBD | TBD | TBD | TBD | TBD | Optional P0 | TBD |
| Brave Wallet | EIP-6963 + fallback | TBD | TBD | TBD | TBD | TBD | TBD | TBD | TBD | TBD | TBD | Optional P1 | TBD |

Matrix completion rules:

1. P0 release gate: MetaMask + at least one alternate wallet must pass all required methods.
2. Required methods: `eth_requestAccounts`, `eth_chainId`, `eth_signTypedData_v4`, `personal_sign`, `eth_sendTransaction`.
3. `eth_sign` may be unsupported; if unsupported, UX must show deterministic fallback path.
4. Determinism run must show zero signature/hash mismatches across repeated identical payload tests.
5. Results must record wallet version, browser version, OS, and test commit SHA.

## 10. Comparison & Trade-offs

### Why This Approach

| Approach | Strengths | Weaknesses | Decision |
|---|---|---|---|
| WASM + direct `EIP-1193` (selected) | Works in browser today, good wallet UX, no native bridge | Wallet capability variance | Adopt as P0 |
| Direct Ledger via `ledger-device-rust-sdk` | Deep hardware control | Not host/browser transport layer, firmware-oriented | Reject for this product path |
| Native HID via sidecar | Potential hardware coverage | Deployment complexity, trust boundary expansion | Defer, not P0 |
| Foundry browser-wallet server path reuse | Existing code reference | Localhost server model mismatches pure WASM deployment | Reference only |
| Full `safers-cli` runtime embed | Rich Safe ops today | Native transport and CLI assumptions incompatible with WASM | Reuse logic only |

### Trade-offs Acknowledged

1. Depending on browser wallets increases variability across providers.
2. Idempotent/replay-safe architecture adds complexity up front.
3. Mandatory preflight can add latency before execute.
4. Local-first persistence improves resilience but increases migration/versioning burden.

### Future Considerations

1. Add optional account-abstraction signer adapters after P0 stability.
2. Add policy engine for configurable risk thresholds per Safe.
3. Add remote collaboration sync mode with signed state deltas.
4. Re-evaluate direct hardware adapters once browser compatibility matrix is proven.

## Plan Quality Checklist Status

- [x] Every required section is present and detailed.
- [x] File paths and function/module names are specific.
- [x] Data schemas include concrete field names and types.
- [x] Commands/endpoints include usage and JSON examples.
- [x] Edge cases and recovery paths are explicit.
- [x] Tasks are execution-ready for implementation agents.

## References

Normative specs:

- EIP-1193
- EIP-6963
- EIP-1271

Current `rusty-safe` verification baseline:

- `crates/rusty-safe/src/app.rs:277`
- `crates/rusty-safe/src/app.rs:861`
- `crates/rusty-safe/src/state.rs:323`
- `crates/rusty-safe/src/hasher.rs:52`
- `crates/rusty-safe/src/hasher.rs:95`

Alloy integration points:

- `deps/alloy/crates/transport/src/trait.rs:5`
- `deps/alloy/crates/transport/src/connect.rs:17`
- `deps/alloy/crates/provider/src/builder.rs:466`
- `deps/alloy/crates/signer-ledger/src/signer.rs:91`
- `deps/alloy/crates/signer-trezor/src/signer.rs:42`

`safers-cli` reusable and non-reusable boundaries:

- `deps/safers-cli/src/utils.rs:160`
- `deps/safers-cli/src/types.rs:15`
- `deps/safers-cli/src/commands.rs:1072`
- `deps/safers-cli/src/commands.rs:1272`
- `deps/safers-cli/src/hardware_wallet.rs:130`
- `deps/safers-cli/src/hardware_wallet.rs:517`

`safe-hash-rs` reusable and non-reusable boundaries:

- `deps/safe-hash-rs/crates/safe-utils/src/hasher.rs:89`
- `deps/safe-hash-rs/crates/safe-utils/src/hasher.rs:127`
- `deps/safe-hash-rs/crates/safe-utils/src/eip712.rs:20`
- `deps/safe-hash-rs/crates/safe-hash/src/api.rs:24`
- `deps/safe-hash-rs/crates/safe-hash/src/api.rs:123`
- `deps/safe-hash-rs/crates/safe-hash/src/lib.rs:17`
- `deps/safe-hash-rs/crates/safe-hash/src/main.rs:25`
- `deps/safe-hash-rs/crates/safe-hash/Cargo.toml:18`

Localsafe parity baseline:

- `prds/LOCALSAFE_SIGNING_FLOW_AND_CAPABILITIES.md:6`
- `prds/LOCALSAFE_SIGNING_FLOW_AND_CAPABILITIES.md:59`
- `prds/LOCALSAFE_SIGNING_FLOW_AND_CAPABILITIES.md:76`
- `prds/LOCALSAFE_SIGNING_FLOW_AND_CAPABILITIES.md:125`
