# Rusty-Safe: A Rust-Native Safe Wallet GUI

## Motivation

The primary motivation is **reducing attack surface by eliminating npm dependencies**. JavaScript supply chain attacks (event-stream, ua-parser-js, node-ipc, etc.) pose significant risks to wallet applications handling private keys and signing transactions.

By building a pure Rust application:
- All dependencies are auditable Rust crates from crates.io
- Single compiled binary with no runtime dependencies
- No node_modules with thousands of transitive dependencies
- Cryptographic operations use audited Rust libraries (alloy, k256, etc.)

## Vision

A cross-platform GUI application (desktop + web) that provides:
1. **Transaction Verification** (Phase 1) - safe-hash-rs features with a visual interface
2. **Safe Wallet Management** (Phase 2) - localsafe.eth features ported to Rust

### Platform Support

| Platform | Build Target | Notes |
|----------|--------------|-------|
| Desktop (Linux/macOS/Windows) | Native binary | Full feature support |
| Web Browser | WASM + WebGL | No X11/OpenGL dependencies, runs anywhere |

The same Rust codebase compiles to both native and WASM targets.

## Reference Projects (External)

These are external projects used as reference implementations:

| Project | Purpose | Link |
|---------|---------|------|
| safe-hash-rs | CLI hash verification (Cyfrin) | https://github.com/Cyfrin/safe-hash-rs |
| localsafe.eth | Web-based Safe UI | Reference for Phase 2 features |
| Zeus | egui wallet example | Reference for Rust GUI patterns |

---

## Phase 1: Transaction Verification UI

Port the functionality of `safe-hash-rs` into an egui-based GUI.

### Features

| Feature | Description | Priority |
|---------|-------------|----------|
| Transaction Hash Verification | Compute and display domain/message/safe tx hashes | P0 |
| API Fetch Mode | Fetch transaction details from Safe Transaction Service | P0 |
| Offline Mode | Manual entry of all transaction parameters | P0 |
| Message Hash Verification | Verify Safe message signing hashes | P0 |
| EIP-712 Typed Data | Hash and verify EIP-712 structures | P1 |
| Nested Safe Support | Handle multisig-of-multisig setups | P1 |
| Security Warnings | Visual alerts for dangerous patterns | P0 |
| Calldata Decoding | Decode known methods (transfer, approve, etc.) | P0 |

### UI Mockup (ASCII)

```
┌────────────────────────────────────────────────────────────────────┐
│ Rusty-Safe                                              [─] [□] [×]│
├────────────────────────────────────────────────────────────────────┤
│  [Transaction]  [Message]  [EIP-712]                               │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│  Chain:        [Ethereum      ▼]     Safe Version: [1.4.1  ▼]     │
│                                                                    │
│  Safe Address: [0x1c694Fc3006D81ff4a56F97E1b99529066a23725    ]   │
│                                                                    │
│  Nonce:        [63                                            ]   │
│                                                                    │
│  ☐ Offline Mode (manually provide all parameters)                 │
│                                                                    │
│  ┌─ Transaction Details ─────────────────────────────────────────┐│
│  │ To:        0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48         ││
│  │ Value:     0 ETH                                              ││
│  │ Data:      0xa9059cbb00000000000000000000000036bffa...        ││
│  │ Operation: Call (0)                                           ││
│  └───────────────────────────────────────────────────────────────┘│
│                                                                    │
│  ┌─ Decoded Calldata ────────────────────────────────────────────┐│
│  │ Method: transfer(address,uint256)                             ││
│  │   recipient: 0x36bffa3048d89fad48509c83fdb6a3410232f3d3       ││
│  │   amount:    1000000 (1.0 USDC)                               ││
│  └───────────────────────────────────────────────────────────────┘│
│                                                                    │
│  ⚠️  WARNINGS                                                     │
│  ┌───────────────────────────────────────────────────────────────┐│
│  │ • Zero value transaction                                      ││
│  └───────────────────────────────────────────────────────────────┘│
│                                                                    │
│                              [Verify Hashes]                       │
│                                                                    │
│  ┌─ Results ─────────────────────────────────────────────────────┐│
│  │ Domain Hash:           0x1655e94a9bcc5a957daa1acae692...      ││
│  │ Message Hash:          0xf22754eba5a2b230714534b4657...      ││
│  │ Safe Transaction Hash: 0xad06b099fca34e51e4886643d95...  [📋]││
│  │                                                               ││
│  │ ✅ Matches API data                                           ││
│  └───────────────────────────────────────────────────────────────┘│
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
```

### Architecture

```
rusty-safe/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── rusty-safe/         # Main GUI application
│   │   ├── index.html      # WASM entry point (trunk)
│   │   ├── assets/
│   │   │   └── favicon.ico
│   │   ├── dist/           # WASM build output (trunk build)
│   │   └── src/
│   │       ├── main.rs     # Entry point (native + WASM)
│   │       ├── app.rs      # Main App struct
│   │       └── ui/
│   │           ├── tx_verify.rs    # Verify Safe API tab
│   │           ├── msg_verify.rs   # Message verification tab
│   │           ├── eip712.rs       # EIP-712 tab
│   │           └── components/     # Reusable widgets
│   │
│   └── safe-core/          # Core logic (can reuse safe-utils or embed)
│       └── src/
│           ├── lib.rs
│           ├── hasher.rs   # Hash computation
│           ├── chains.rs   # Supported chains
│           ├── api.rs      # Safe Transaction Service client
│           └── decoder.rs  # Calldata decoding
```

### Dependencies

| Crate | Purpose | Platform |
|-------|---------|----------|
| `eframe` / `egui` | GUI framework (native + WASM) | All |
| `alloy-primitives` | Ethereum primitives (Address, U256, B256) | All |
| `alloy-sol-types` | ABI encoding/decoding | All |
| `alloy-dyn-abi` | Dynamic ABI for calldata decoding | All |
| `reqwest` | HTTP client for Safe API | All |
| `serde` / `serde_json` | JSON serialization | All |
| `semver` | Safe version handling | All |
| `tokio` | Async runtime | Native only |
| `arboard` | Clipboard access | Native only |
| `tracing-subscriber` | Logging to stdout | Native only |
| `tracing-wasm` | Logging to console.log | WASM only |
| `wasm-bindgen-futures` | Async/await in WASM | WASM only |
| `web-sys` | Browser APIs | WASM only |

### Building for Web (WASM)

The application compiles to WebAssembly for browser deployment. This eliminates X11/OpenGL dependencies and allows running anywhere with a modern browser.

**Prerequisites:**
```bash
# Install WASM target and trunk bundler
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
```

**Build Commands:**
```bash
cd crates/rusty-safe

# Development build with hot-reload
trunk serve --open

# Production build (outputs to dist/)
trunk build --release

# Serve production build
trunk serve --release --address 0.0.0.0 --port 8080
```

**WASM-specific Dependencies:**
```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = ["Window", "Document", "Element", "HtmlCanvasElement"] }
tracing-wasm = "0.2"
getrandom = { version = "0.2", features = ["js"] }  # Required for crypto in WASM
```

**Native-only Dependencies:**
```toml
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
tokio = { workspace = true }
arboard = { workspace = true }          # Clipboard (not available in WASM)
tracing-subscriber = { workspace = true }
```

### Integration with safe-hash-rs

**Decision: Depend on `safe-utils` crate from safe-hash-rs**

```toml
[dependencies]
safe-utils = { git = "https://github.com/Cyfrin/safe-hash-rs" }
```

This gives us:
- `DomainHasher` - EIP-712 domain separator computation
- `TxMessageHasher` - Safe transaction message hash
- `MessageHasher` - Safe message hash for off-chain signing
- `SafeHasher` - Final Safe transaction hash
- `CallDataHasher` - Calldata hashing
- `Eip712Hasher` - Generic EIP-712 typed data hashing
- Chain ID mappings and Safe API endpoints

If upstream changes break compatibility, we can vendor a specific commit or fork.

### Supported Chains (Day One)

All chains from safe-hash-rs:

| Chain | Chain ID | Safe API Endpoint |
|-------|----------|-------------------|
| Ethereum | 1 | safe-transaction-mainnet.safe.global |
| Arbitrum | 42161 | safe-transaction-arbitrum.safe.global |
| Aurora | 1313161554 | safe-transaction-aurora.safe.global |
| Avalanche | 43114 | safe-transaction-avalanche.safe.global |
| Base | 8453 | safe-transaction-base.safe.global |
| Blast | 81457 | safe-transaction-blast.safe.global |
| BSC | 56 | safe-transaction-bsc.safe.global |
| Celo | 42220 | safe-transaction-celo.safe.global |
| Gnosis | 100 | safe-transaction-gnosis-chain.safe.global |
| Linea | 59144 | safe-transaction-linea.safe.global |
| Mantle | 5000 | safe-transaction-mantle.safe.global |
| Optimism | 10 | safe-transaction-optimism.safe.global |
| Polygon | 137 | safe-transaction-polygon.safe.global |
| Scroll | 534352 | safe-transaction-scroll.safe.global |
| Sepolia | 11155111 | safe-transaction-sepolia.safe.global |
| World Chain | 480 | safe-transaction-worldchain.safe.global |
| X Layer | 196 | safe-transaction-xlayer.safe.global |
| zkSync | 324 | safe-transaction-zksync.safe.global |
| Base Sepolia | 84532 | safe-transaction-base-sepolia.safe.global |
| Gnosis Chiado | 10200 | safe-transaction-chiado.safe.global |
| Polygon zkEVM | 1101 | safe-transaction-zkevm.safe.global |
| Monad | 143 | safe-transaction-monad-testnet.safe.global |

---

## Phase 2: Safe Wallet Management

Port localsafe.eth features to Rust. This is a larger undertaking.

### Features

| Feature | Description | Priority |
|---------|-------------|----------|
| Create New Safe | Predict address, deploy counterfactual Safe | P0 |
| Connect Existing Safe | Load Safe by address, fetch owners/threshold | P0 |
| Transaction Creation | Build Safe transactions with calldata | P0 |
| Transaction Signing | Sign with connected wallet (hardware wallet support) | P0 |
| Signature Collection | Import/export signatures for offline signing | P1 |
| Transaction Execution | Broadcast signed transactions | P0 |
| Owner Management | Add/remove owners, change threshold | P1 |
| WalletConnect | Connect to dApps as the Safe | P2 |
| Address Book | Store named addresses | P1 |

### Wallet Connectivity

For Phase 2, we need wallet connectivity for signing. Hardware wallet support is **high priority**.

#### Implementation Strategy (Ordered)

| Phase | Approach | Description | Crates |
|-------|----------|-------------|--------|
| 2.0 | Manual Signature | User signs externally, pastes signature | - |
| 2.1 | Ledger USB | Direct Ledger communication via USB HID | `coins-ledger` |
| 2.2 | Trezor USB | Direct Trezor communication | `trezor-client` |
| 2.3 | WalletConnect | Connect to mobile/browser wallets | `walletconnect-rs` |

#### Hardware Wallet Crates

**Ledger:**
```toml
# coins-ledger - maintained, used by Foundry
coins-ledger = "0.12"
```

Provides:
- USB HID transport
- Ethereum app communication
- Sign transaction / Sign message
- Get address with derivation path

**Trezor:**
```toml
# trezor-client - official Trezor crate
trezor-client = "0.1"
```

#### Signing Flow (Hardware Wallet)

```
┌─────────────────────────────────────────────────────────────────┐
│                     Hardware Wallet Signing                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. User clicks "Sign Transaction"                              │
│                                                                 │
│  2. App detects connected hardware wallet                       │
│     ┌─────────────────────────────────────────────────────┐    │
│     │  🔌 Ledger Nano S Plus detected                     │    │
│     │     Path: m/44'/60'/0'/0/0                          │    │
│     │     Address: 0x1234...5678                          │    │
│     └─────────────────────────────────────────────────────┘    │
│                                                                 │
│  3. App sends EIP-712 typed data to device                     │
│     ┌─────────────────────────────────────────────────────┐    │
│     │  ⏳ Please confirm on your Ledger device...         │    │
│     │                                                     │    │
│     │  Safe Transaction Hash:                             │    │
│     │  0xad06b099fca34e51e4886643d95d9a19ace2cd02...      │    │
│     └─────────────────────────────────────────────────────┘    │
│                                                                 │
│  4. User confirms on device → Signature returned               │
│                                                                 │
│  5. Signature added to transaction, saved locally              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

#### Manual Signature Entry (Fallback)

For air-gapped setups or unsupported wallets:

```
┌─────────────────────────────────────────────────────────────────┐
│  Manual Signature Entry                                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Safe Transaction Hash (copy this):                             │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ 0xad06b099fca34e51e4886643d95d9a19ace2cd024065efb6...   │📋 │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  Paste signature from external signer:                          │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ 0x...                                                   │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  Signer Address: [0x...                                    ]   │
│                                                                 │
│                              [Add Signature]                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Data Storage

Local JSON files (like Zeus approach):
```
~/.rusty-safe/
├── config.json          # App configuration
├── address_book.json    # Named addresses
├── safes/
│   ├── 1/               # Chain ID
│   │   └── 0x1234...json  # Safe data + pending txs
│   └── 10/
└── sessions/            # WalletConnect sessions
```

---

## Technical Decisions

### GUI Framework: egui

Using `egui` via `eframe` because:
- Pure Rust, cross-platform
- Immediate mode = simple state management
- Already proven in Zeus wallet
- Good performance, responsive UI
- Supports custom rendering if needed

### Async Runtime: tokio

For network requests (Safe API, RPC calls):
```rust
// Spawn async tasks from egui
let ctx = ctx.clone();
tokio::spawn(async move {
    let result = fetch_transaction(&safe_address, nonce).await;
    ctx.request_repaint(); // Trigger UI update
});
```

### Error Handling

Use `thiserror` for error types, display user-friendly messages in UI:
```rust
#[derive(thiserror::Error, Debug)]
pub enum SafeError {
    #[error("Failed to fetch transaction: {0}")]
    ApiFetch(#[from] reqwest::Error),
    
    #[error("Invalid Safe address")]
    InvalidAddress,
    
    #[error("Nonce mismatch: expected {expected}, got {actual}")]
    NonceMismatch { expected: u64, actual: u64 },
}
```

### State Management

```rust
pub struct App {
    // Current tab
    active_tab: Tab,
    
    // Transaction verification state
    tx_state: TxVerifyState,
    
    // Message verification state  
    msg_state: MsgVerifyState,
    
    // Async task results
    pending_tasks: Vec<Task>,
    
    // Persistent data
    config: AppConfig,
    address_book: AddressBook,
}

pub struct TxVerifyState {
    chain: Chain,
    safe_address: String,
    safe_version: SafeVersion,
    nonce: String,
    offline_mode: bool,
    
    // Fetched/computed data
    tx_details: Option<TxDetails>,
    hashes: Option<SafeHashes>,
    warnings: Vec<Warning>,
    errors: Vec<String>,
    
    // Loading state
    is_loading: bool,
}
```

---

## Development Phases

### Phase 1.0: Core Verification (MVP)

**Goal**: Functional transaction hash verification with GUI

| Task | Description | Est. Time |
|------|-------------|-----------|
| 1.0.1 | Cargo workspace setup | 1h |
| 1.0.2 | Add dependencies (eframe, alloy, safe-utils, reqwest) | 30m |
| 1.0.3 | Basic eframe app scaffold with window | 1h |
| 1.0.4 | Tab navigation (Verify Safe API / Message / EIP-712) | 1h |
| 1.0.5 | Verify Safe API tab UI layout | 2h |
| 1.0.6 | Chain dropdown with all supported chains | 1h |
| 1.0.7 | Safe version dropdown (1.0.0 - 1.4.1) | 30m |
| 1.0.8 | Input fields (Safe address, nonce) | 1h |
| 1.0.9 | Async API fetch with loading state | 2h |
| 1.0.10 | Display fetched transaction details | 1h |
| 1.0.11 | Hash computation using safe-utils | 1h |
| 1.0.12 | Results display with copy buttons | 1h |
| 1.0.13 | Offline mode toggle + manual inputs | 2h |

**Deliverable**: Can verify any Safe transaction hash via GUI

### Phase 1.1: Enhanced Verification

| Task | Description | Est. Time |
|------|-------------|-----------|
| 1.1.1 | Message signing tab | 2h |
| 1.1.2 | File picker for message input | 1h |
| 1.1.3 | Message hash computation | 1h |
| 1.1.4 | EIP-712 tab with JSON input | 2h |
| 1.1.5 | EIP-712 parsing and hashing | 2h |
| 1.1.6 | Calldata decoder integration | 3h |
| 1.1.7 | Decoded calldata display panel | 2h |
| 1.1.8 | Warning detection system | 2h |
| 1.1.9 | Warning display with severity colors | 1h |
| 1.1.10 | Nested Safe support (UI + logic) | 3h |
| 1.1.11 | API vs user input comparison | 2h |

**Deliverable**: Full parity with safe-hash-rs CLI features

### Phase 1.2: Polish

| Task | Description | Est. Time |
|------|-------------|-----------|
| 1.2.1 | Dark/light theme toggle | 2h |
| 1.2.2 | Theme persistence (config file) | 1h |
| 1.2.3 | Chain selector with network icons | 2h |
| 1.2.4 | Recent verifications history | 3h |
| 1.2.5 | Keyboard shortcuts (Ctrl+V paste, etc.) | 1h |
| 1.2.6 | Better error messages in UI | 2h |
| 1.2.7 | Input validation (address format, etc.) | 2h |
| 1.2.8 | Tooltips and help text | 1h |

**Deliverable**: Production-ready Phase 1 release

### Phase 2.0: Safe Management (Foundation)

| Task | Description | Est. Time |
|------|-------------|-----------|
| 2.0.1 | Safe info fetching (owners, threshold, nonce) | 3h |
| 2.0.2 | Connect existing Safe flow | 2h |
| 2.0.3 | Safe dashboard view | 3h |
| 2.0.4 | Transaction builder UI | 4h |
| 2.0.5 | Multi-send (batch transactions) | 3h |
| 2.0.6 | Signature collection data structure | 2h |
| 2.0.7 | Export transaction + signatures as JSON | 2h |
| 2.0.8 | Import transaction JSON | 2h |
| 2.0.9 | Manual signature entry | 2h |
| 2.0.10 | Signature validation | 2h |

**Deliverable**: Can create, sign (manually), and export Safe transactions

### Phase 2.1: Hardware Wallet Integration

| Task | Description | Est. Time |
|------|-------------|-----------|
| 2.1.1 | Ledger detection via USB | 3h |
| 2.1.2 | Ledger address derivation | 2h |
| 2.1.3 | Ledger EIP-712 signing | 4h |
| 2.1.4 | Ledger error handling | 2h |
| 2.1.5 | Trezor detection | 3h |
| 2.1.6 | Trezor signing flow | 4h |
| 2.1.7 | Device selection UI | 2h |
| 2.1.8 | Derivation path configuration | 2h |

**Deliverable**: Sign Safe transactions with Ledger/Trezor

### Phase 2.2: Full Wallet

| Task | Description | Est. Time |
|------|-------------|-----------|
| 2.2.1 | Safe creation (predict address) | 3h |
| 2.2.2 | Safe deployment flow | 4h |
| 2.2.3 | Owner management UI | 3h |
| 2.2.4 | Threshold change UI | 2h |
| 2.2.5 | Address book (CRUD) | 3h |
| 2.2.6 | Multiple Safes management | 3h |
| 2.2.7 | WalletConnect v2 integration | 8h |
| 2.2.8 | dApp session handling | 4h |

**Deliverable**: Full Safe wallet functionality in Rust

---

## Decisions Made

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Repository | Standalone repo | Independent project, not tied to safe-hash-rs |
| Chain Support | All safe-hash-rs chains | No artificial limitations |
| safe-utils | Git dependency | Leverage existing tested code |
| Hardware Wallets | High priority (Phase 2) | Critical for Safe users |
| UI Framework | egui/eframe | Pure Rust, cross-platform |
| WASM Support | Yes, via trunk | Run in browser without native dependencies |
| Build Tool | trunk | Standard WASM bundler for Rust web apps |

## Open Decisions

| Topic | Options | Notes |
|-------|---------|-------|
| Project Name | "Rusty-Safe" / other | Can decide later |
| Multi-chain UX | Single selector / Dashboard | Decide during Phase 2 |
| Theme | Dark / Light / System | Can support all three |
| Distribution | GitHub releases + cargo install | Standard approach |

---

## Implementation Details

### Core Hashing Logic

The EIP-712 domain and message hashing follows Safe's specification:

```rust
// Domain separator for Safe >= 1.3.0
struct EIP712Domain {
    chain_id: U256,
    verifying_contract: Address,
}

// Domain separator for Safe < 1.3.0  
struct EIP712DomainLegacy {
    verifying_contract: Address,
}

// Safe transaction structure
struct SafeTx {
    to: Address,
    value: U256,
    data: Bytes,
    operation: u8,           // 0 = Call, 1 = DelegateCall
    safe_tx_gas: U256,
    base_gas: U256,
    gas_price: U256,
    gas_token: Address,
    refund_receiver: Address,
    nonce: U256,
}
```

**Hash Computation:**
```
domain_hash = keccak256(encode(EIP712Domain))
message_hash = keccak256(encode(SafeTx))
safe_tx_hash = keccak256(0x1901 || domain_hash || message_hash)
```

### Safe Transaction Service API

**Fetch pending transaction:**
```
GET https://safe-transaction-{chain}.safe.global/api/v1/safes/{address}/multisig-transactions/?nonce={nonce}
```

**Response structure:**
```json
{
  "results": [{
    "safe": "0x...",
    "to": "0x...",
    "value": "0",
    "data": "0x...",
    "operation": 0,
    "safeTxGas": 0,
    "baseGas": 0,
    "gasPrice": "0",
    "gasToken": "0x0000000000000000000000000000000000000000",
    "refundReceiver": "0x0000000000000000000000000000000000000000",
    "nonce": 63,
    "safeTxHash": "0x...",
    "confirmations": [...],
    "confirmationsRequired": 2
  }]
}
```

### Calldata Decoding

Common Safe transaction methods to decode:

| Selector | Method | Description |
|----------|--------|-------------|
| `0xa9059cbb` | `transfer(address,uint256)` | ERC20 transfer |
| `0x095ea7b3` | `approve(address,uint256)` | ERC20 approve |
| `0x23b872dd` | `transferFrom(address,address,uint256)` | ERC20 transferFrom |
| `0x42842e0e` | `safeTransferFrom(address,address,uint256)` | ERC721 transfer |
| `0x0d582f13` | `addOwnerWithThreshold(address,uint256)` | Safe: add owner |
| `0xf8dc5dd9` | `removeOwner(address,address,uint256)` | Safe: remove owner |
| `0xe318b52b` | `swapOwner(address,address,address)` | Safe: swap owner |
| `0x694e80c3` | `changeThreshold(uint256)` | Safe: change threshold |

Use `alloy-dyn-abi` for dynamic decoding with ABI:
```rust
use alloy_dyn_abi::{DynSolType, DynSolValue};

fn decode_transfer(data: &[u8]) -> Option<(Address, U256)> {
    let types = vec![DynSolType::Address, DynSolType::Uint(256)];
    let decoded = DynSolType::Tuple(types).abi_decode(&data[4..])?;
    // Extract address and amount
}
```

### Security Warnings System

```rust
#[derive(Debug, Clone)]
pub enum Warning {
    ZeroAddress,           // to == 0x0
    ZeroValue,             // value == 0
    EmptyData,             // data == 0x
    DelegateCall,          // operation == 1
    NonZeroGasToken,       // gas_token != 0x0
    NonZeroRefundReceiver, // refund_receiver != 0x0
    DangerousMethod(String), // addOwner, removeOwner, etc.
    NonceMismatch { expected: u64, actual: u64 },
    DataMismatch,          // User input differs from API
}

impl Warning {
    pub fn severity(&self) -> Severity {
        match self {
            Warning::DelegateCall => Severity::Critical,
            Warning::DangerousMethod(_) => Severity::High,
            Warning::DataMismatch => Severity::High,
            Warning::NonZeroGasToken => Severity::Medium,
            _ => Severity::Low,
        }
    }
}
```

## Testing Strategy

### Unit Tests
- Hash computation matches known vectors
- Calldata decoding correctness
- Warning detection logic

### Integration Tests
- API fetch with mock server
- Full verification flow
- Offline mode with manual input

### Manual Testing
- Verify against live Safe transactions
- Compare hashes with safe-hash-rs CLI
- Test all supported chains

---

## Quick Start

**Native Build:**
```bash
cargo build --release
./target/release/rusty-safe

# For X11 forwarding (software rendering):
LIBGL_ALWAYS_SOFTWARE=1 ./target/release/rusty-safe
```

**Web/WASM Build:**
```bash
cd crates/rusty-safe
trunk serve --release --address 0.0.0.0 --port 8080
# Open http://localhost:8080 in browser
```

---

## Next Steps

To continue implementation:

1. **Start with Phase 1.0.5 - 1.0.13** (transaction verification UI)

2. **Verify safe-utils integration** works before building UI

3. **Iterate**: Build minimal UI → test → expand

---

## Cargo.toml (Current)

**Workspace root (`Cargo.toml`):**
```toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.dependencies]
# GUI - configured for both native and WASM
eframe = { version = "0.29", default-features = false, features = [
    "default_fonts",
    "glow",
    "persistence",
] }
egui = "0.29"

# Safe hash computation
safe-utils = { git = "https://github.com/Cyfrin/safe-hash-rs" }

# Async + HTTP
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync"] }
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Utils
thiserror = "2"
semver = "1"
arboard = "3"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[profile.release]
lto = true
strip = true
```

**Crate (`crates/rusty-safe/Cargo.toml`):**
```toml
[package]
name = "rusty-safe"
version = "0.1.0"
edition = "2021"

[dependencies]
eframe.workspace = true
egui.workspace = true
safe-utils.workspace = true
reqwest.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
semver.workspace = true
tracing.workspace = true

# Native-only
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
tokio.workspace = true
arboard.workspace = true
tracing-subscriber.workspace = true

# WASM-only
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = ["Window", "Document", "Element", "HtmlCanvasElement"] }
tracing-wasm = "0.2"
getrandom = { version = "0.2", features = ["js"] }
```

---

## References

| Resource | Description |
|----------|-------------|
| [safe-hash-rs](https://github.com/Cyfrin/safe-hash-rs) | CLI hash verification tool (core logic) |
| [Safe Protocol Kit](https://github.com/safe-global/safe-core-sdk) | Official Safe SDK (JS reference) |
| [Safe Contracts](https://github.com/safe-global/safe-contracts) | Safe smart contracts |
| [egui docs](https://docs.rs/egui) | GUI framework documentation |
| [eframe docs](https://docs.rs/eframe) | egui native shell |
| [trunk](https://trunkrs.dev/) | WASM bundler for Rust web apps |
| [alloy-rs](https://github.com/alloy-rs/alloy) | Ethereum primitives |
| [coins-ledger](https://docs.rs/coins-ledger) | Ledger hardware wallet |
| [EIP-712](https://eips.ethereum.org/EIPS/eip-712) | Typed structured data hashing |
| [Safe Transaction Service API](https://safe-transaction-mainnet.safe.global/) | API documentation |

