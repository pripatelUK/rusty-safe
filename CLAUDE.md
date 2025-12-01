# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rusty-Safe is a Rust-native desktop GUI application for Safe{Wallet} transaction verification and management. It eliminates npm dependencies to reduce supply chain attack surface.

**Key Goals:**
- Pure Rust implementation (no JavaScript/npm)
- egui-based native GUI
- Safe transaction hash verification
- Hardware wallet support (Ledger/Trezor)

## Build and Development Commands

### Building
- `cargo build` - Debug build
- `cargo build --release` - Release build
- `cargo run` - Run the application

### Testing
- `cargo test` - Run all tests
- `cargo test --lib` - Run unit tests only
- `cargo nextest run` - Run tests with nextest (faster)

### Code Quality
- `cargo +nightly fmt --all` - Format code
- `cargo clippy --all-features` - Run linting
- `cargo clippy --fix --allow-dirty` - Auto-fix lint issues

### Checking
- `cargo check` - Quick compilation check
- `cargo doc --open` - Generate and view documentation

## Architecture Overview

```
rusty-safe/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── rusty-safe/         # Main GUI application
│   │   └── src/
│   │       ├── main.rs     # Entry point (eframe + tokio)
│   │       ├── app.rs      # Main App struct
│   │       └── ui/         # UI components
│   │
│   └── safe-core/          # Core logic library
│       └── src/
│           ├── lib.rs
│           ├── api.rs      # Safe Transaction Service client
│           ├── chains.rs   # Supported chains
│           └── decoder.rs  # Calldata decoding
```

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| `eframe/egui` | Native GUI framework |
| `alloy-*` | Ethereum primitives and ABI |
| `safe-utils` | Safe hash computation (from Cyfrin) |
| `tokio` | Async runtime |
| `reqwest` | HTTP client |

## Code Style Preferences

### General Principles

1. **Keep it simple** - Avoid over-engineering
2. **Explicit over implicit** - Clear variable names, no magic
3. **Fail fast** - Validate inputs early, return errors promptly

### Error Handling

Use `thiserror` for library errors, provide user-friendly messages:

```rust
// GOOD: Descriptive error with context
#[derive(thiserror::Error, Debug)]
pub enum SafeError {
    #[error("Failed to fetch transaction from Safe API: {0}")]
    ApiFetch(#[from] reqwest::Error),
    
    #[error("Invalid Safe address: {address}")]
    InvalidAddress { address: String },
}

// BAD: Generic errors
Err("something went wrong")
```

### Serialization

1. **Always use derive macros** for JSON serialization:
   - Prefer `#[derive(Serialize, Deserialize)]` over manual JSON construction
   - Use serde's attribute macros for field customization

2. **Use Alloy's built-in types**:
   - Don't manually serialize Ethereum types (Address, U256, B256)
   - Let alloy handle hex encoding/decoding

```rust
// GOOD: Using derives and Alloy types
#[derive(Serialize, Deserialize)]
struct SafeTransaction {
    to: Address,
    value: U256,
    data: Bytes,
    nonce: u64,
}

// BAD: Manual JSON with string conversion
let json = json!({
    "to": to.to_string(),
    "value": format!("{}", value),
});
```

### Async Patterns

For egui + tokio integration:

```rust
// Spawn async tasks, communicate via channels or shared state
fn spawn_fetch(&mut self, ctx: &egui::Context) {
    let ctx = ctx.clone();
    let (tx, rx) = oneshot::channel();
    
    self.pending_result = Some(rx);
    
    tokio::spawn(async move {
        let result = fetch_from_api().await;
        let _ = tx.send(result);
        ctx.request_repaint();
    });
}

// Check result in update loop
fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    if let Some(rx) = &mut self.pending_result {
        if let Ok(result) = rx.try_recv() {
            self.data = Some(result);
            self.pending_result = None;
        }
    }
}
```

### UI Components

Keep UI code clean and modular:

```rust
// GOOD: Separate rendering logic
impl TxVerifyTab {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.render_inputs(ui);
        self.render_results(ui);
        self.render_warnings(ui);
    }
    
    fn render_inputs(&mut self, ui: &mut egui::Ui) {
        // Input fields
    }
}

// BAD: Monolithic update function
fn update(&mut self, ctx: &egui::Context) {
    // 500 lines of mixed logic and UI
}
```

### Function Documentation

- Keep docstrings concise, focused on "what" and "why"
- Avoid verbose `# Arguments` sections unless truly complex
- Document public API, skip obvious internal functions

```rust
/// Computes the Safe transaction hash for the given parameters.
/// 
/// Uses EIP-712 structured hashing with the Safe domain separator.
pub fn compute_safe_tx_hash(tx: &SafeTransaction, chain_id: u64) -> B256 {
    // ...
}
```

## Testing Guidelines

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hash_computation_matches_known_vector() {
        // Test against known Safe transaction hash
        let expected = b256!("ad06b099...");
        let actual = compute_hash(&tx);
        assert_eq!(actual, expected);
    }
}
```

### Integration Tests

For API interactions, use mock servers:

```rust
#[tokio::test]
async fn test_api_fetch() {
    let mock_server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
        .mount(&mock_server)
        .await;
    
    let client = SafeApiClient::new(&mock_server.uri());
    let result = client.fetch_transaction(...).await;
    
    assert!(result.is_ok());
}
```

## Commit Convention

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>
```

### Types
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Formatting
- `refactor`: Code refactoring
- `test`: Tests
- `chore`: Maintenance

### Examples
- `feat(ui): add transaction verification tab`
- `fix(api): handle rate limiting from Safe service`
- `refactor(hash): simplify domain hash computation`
- `chore(deps): update egui to 0.29`

## Security Considerations

This is a wallet-adjacent application. Follow these principles:

1. **Never log sensitive data** - No private keys, signatures in logs
2. **Validate all inputs** - Address formats, hex strings, nonces
3. **Verify before display** - Compute hashes locally, don't trust API blindly
4. **Clear sensitive memory** - Use zeroize for any key material (Phase 2)

## Development Workflow

1. Check spec: `SPEC.md` for current phase and tasks
2. Implement feature in smallest working increment
3. Write tests for new functionality
4. Format and lint: `cargo +nightly fmt && cargo clippy`
5. Commit with conventional message
6. Update `SPEC.md` task status if applicable

## External Dependencies

### safe-utils (git dependency)

From Cyfrin's safe-hash-rs. Provides:
- `DomainHasher`, `TxMessageHasher`, `SafeHasher`
- Chain ID mappings
- EIP-712 hashing

If it causes issues, we can vendor at a specific commit.

### egui/eframe

GUI framework. Key patterns:
- Immediate mode rendering
- `ctx.request_repaint()` for async updates
- Use `egui_extras` for tables, images

## Minimum Supported Rust Version

- MSRV: 1.80.0
- Edition: 2021
- Use nightly only for `rustfmt`
