# Test Coverage Requirements

This document specifies the test coverage requirements for rusty-safe to achieve production readiness. Given the wallet-adjacent nature of this application, comprehensive testing is essential.

## Current Test Coverage Status

**Existing Tests:**
- `crates/rusty-safe/src/state.rs` - Address book CSV import/export tests
- `crates/rusty-safe/src/decode/parser.rs` - Basic calldata decoding tests
- `crates/rusty-safe/src/decode/sourcify.rs` - Selector normalization test

**Coverage Gaps:**
- No hash computation tests
- No API response parsing tests
- No expected value validation tests
- No MultiSend parsing/verification tests
- No UI component tests
- No integration tests
- No adversarial input tests

---

## Required Test Suites

### 1. Hash Computation Tests (`crates/rusty-safe/src/hasher.rs`)

**Priority:** CRITICAL

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Known test vectors from actual Safe transactions
    #[test]
    fn test_hash_computation_mainnet_v1_3() {
        // Use known Safe transaction with verified hash
        let hashes = compute_hashes(
            "ethereum",
            "0x...",  // Known Safe address
            "1.3.0",
            "0x...",  // to
            "0",      // value
            "0x...",  // data
            0,        // operation
            "0", "0", "0",
            "0x0000000000000000000000000000000000000000",
            "0x0000000000000000000000000000000000000000",
            "0",
        ).unwrap();

        assert_eq!(hashes.safe_tx_hash, "0x...");  // Expected hash
    }

    #[test]
    fn test_hash_computation_all_safe_versions() {
        for version in ["1.0.0", "1.1.0", "1.1.1", "1.2.0", "1.3.0", "1.4.0", "1.4.1"] {
            // Test each version produces valid (different) hashes
        }
    }

    #[test]
    fn test_hash_computation_all_supported_chains() {
        // Test that each chain in safe_utils produces valid hashes
    }

    #[test]
    fn test_invalid_chain_returns_error() {
        let result = compute_hashes("invalid_chain", ...);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_address_returns_error() {
        let result = compute_hashes("ethereum", "not_an_address", ...);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_hex_data_returns_error() {
        let result = compute_hashes(..., "0xGGGG", ...);  // Invalid hex
        assert!(result.is_err());
    }

    #[test]
    fn test_odd_length_hex_data_returns_error() {
        let result = compute_hashes(..., "0xabc", ...);  // Odd length
        assert!(result.is_err());
    }

    #[test]
    fn test_large_value_handling() {
        // Test with value > u128::MAX
        let result = compute_hashes(
            ...,
            "340282366920938463463374607431768211456",  // 2^128
            ...
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_delegatecall_warning() {
        let warnings = get_warnings_for_tx(..., 1, ...);  // operation = 1
        assert!(warnings.delegatecall);
    }

    #[test]
    fn test_dangerous_method_warning() {
        // Test addOwnerWithThreshold, removeOwner, swapOwner, changeThreshold
    }
}
```

### 2. Calldata Decoding Tests (`crates/rusty-safe/src/decode/`)

**Priority:** CRITICAL

```rust
// parser.rs tests
#[cfg(test)]
mod tests {
    use super::*;

    // ERC20 Functions
    #[test]
    fn test_decode_erc20_transfer() {
        let data = "0xa9059cbb000000000000000000000000...";
        let result = decode_with_signature(data, "transfer(address,uint256)");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().method, "transfer");
    }

    #[test]
    fn test_decode_erc20_approve() {
        let data = "0x095ea7b3...";
        let result = decode_with_signature(data, "approve(address,uint256)");
        assert!(result.is_ok());
    }

    // MultiSend
    #[test]
    fn test_multisend_unpack_single_tx() {
        let packed = hex::decode("00...").unwrap();
        let txs = unpack_multisend_transactions(&packed).unwrap();
        assert_eq!(txs.len(), 1);
    }

    #[test]
    fn test_multisend_unpack_multiple_txs() {
        // Known MultiSend with 5 transactions
        let packed = hex::decode("...").unwrap();
        let txs = unpack_multisend_transactions(&packed).unwrap();
        assert_eq!(txs.len(), 5);
    }

    #[test]
    fn test_multisend_truncated_data_error() {
        let packed = hex::decode("00").unwrap();  // Incomplete
        let result = unpack_multisend_transactions(&packed);
        assert!(result.is_err());
    }

    #[test]
    fn test_multisend_empty_nested_calldata() {
        // Native ETH transfer within MultiSend
    }

    // Edge cases
    #[test]
    fn test_decode_empty_calldata() {
        let decoded = parse_initial("0x", None);
        assert!(matches!(decoded.kind, TransactionKind::Empty));
    }

    #[test]
    fn test_decode_short_calldata() {
        let decoded = parse_initial("0xabcd", None);  // < 4 bytes
        assert!(matches!(decoded.kind, TransactionKind::Unknown));
    }

    #[test]
    fn test_selector_collision_handling() {
        // Test selector that has multiple valid signatures
        // Verify behavior is deterministic
    }

    #[test]
    fn test_complex_tuple_decoding() {
        // Zodiac scopeFunction with nested arrays
        let sig = "scopeFunction(uint16,address,bytes4,bool[],uint8[],uint8[],bytes[],uint8)";
        let data = "0x33a0480c...";
        let result = decode_with_signature(data, sig);
        assert!(result.is_ok());
    }

    #[test]
    fn test_bytes4_formatting() {
        // Ensure bytes4 is formatted as 0x + 8 chars, not full 32 bytes
    }
}

// compare.rs tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_matching_decodes() {
        let api = ApiDecode { method: "transfer".into(), params: vec![...] };
        let local = LocalDecode { method: "transfer".into(), params: vec![...] };
        let result = compare_decodes(Some(&api), Some(&local));
        assert!(matches!(result, ComparisonResult::Match));
    }

    #[test]
    fn test_compare_method_mismatch() {
        let api = ApiDecode { method: "transfer".into(), ... };
        let local = LocalDecode { method: "approve".into(), ... };
        let result = compare_decodes(Some(&api), Some(&local));
        assert!(matches!(result, ComparisonResult::MethodMismatch { .. }));
    }

    #[test]
    fn test_compare_param_mismatch() {
        // Different parameter values
    }

    #[test]
    fn test_compare_large_uint256_values() {
        // Values > u128::MAX should still compare correctly
        let api_val = "340282366920938463463374607431768211456";  // 2^128
        let local_val = "340282366920938463463374607431768211456";
        // Should match, not truncate
    }

    #[test]
    fn test_compare_address_case_insensitive() {
        // 0xAbC... should equal 0xabc...
    }

    #[test]
    fn test_compare_only_api() {
        let result = compare_decodes(Some(&api), None);
        assert!(matches!(result, ComparisonResult::OnlyApi));
    }

    #[test]
    fn test_compare_only_local() {
        let result = compare_decodes(None, Some(&local));
        assert!(matches!(result, ComparisonResult::OnlyLocal));
    }
}
```

### 3. Expected Value Validation Tests (`crates/rusty-safe/src/expected.rs`)

**Priority:** HIGH

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_matching_to_address() {
        let tx = mock_safe_transaction("0xAbC...", "0", "0x", 0);
        let expected = ExpectedState { to: "0xabc...".into(), ..Default::default() };
        let result = validate_against_api(&tx, &expected);
        assert!(result.mismatches.is_empty());
    }

    #[test]
    fn test_validate_mismatched_to_address() {
        let tx = mock_safe_transaction("0xAbC...", "0", "0x", 0);
        let expected = ExpectedState { to: "0xDEF...".into(), ..Default::default() };
        let result = validate_against_api(&tx, &expected);
        assert!(!result.mismatches.is_empty());
    }

    #[test]
    fn test_validate_invalid_expected_address_is_error() {
        // Invalid expected address should produce an error, not silent skip
        let expected = ExpectedState { to: "not_valid".into(), ..Default::default() };
        // Should not silently return "Match"
    }

    #[test]
    fn test_validate_value_wei() {
        let tx = mock_safe_transaction(..., "1000000000000000000", ...);  // 1 ETH
        let expected = ExpectedState { value: "1000000000000000000".into(), ... };
        // Should match
    }

    #[test]
    fn test_validate_value_mismatch() {
        let tx = mock_safe_transaction(..., "1000000000000000000", ...);
        let expected = ExpectedState { value: "2000000000000000000".into(), ... };
        // Should report mismatch
    }

    #[test]
    fn test_validate_data_prefix_match() {
        let tx = mock_safe_transaction(..., "0xa9059cbb...", ...);
        let expected = ExpectedState { data_prefix: "0xa9059cbb".into(), ... };
        // Should match (prefix only)
    }

    #[test]
    fn test_validate_operation_call() {
        let tx = mock_safe_transaction(..., 0);  // Call
        let expected = ExpectedState { operation: "0".into(), ... };
        // Should match
    }

    #[test]
    fn test_validate_operation_delegatecall() {
        let tx = mock_safe_transaction(..., 1);  // DelegateCall
        let expected = ExpectedState { operation: "1".into(), ... };
        // Should match
    }
}
```

### 4. Address Validation Tests (`crates/rusty-safe/src/state.rs`)

**Priority:** HIGH

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // EIP-55 Checksum Tests
    #[test]
    fn test_valid_checksummed_address() {
        let result = validate_address("0x4F2083f5fBede34C2714aFfb3105539775f7FE64");
        assert_eq!(result, AddressValidation::Valid);
    }

    #[test]
    fn test_valid_lowercase_address() {
        let result = validate_address("0x4f2083f5fbede34c2714affb3105539775f7fe64");
        assert_eq!(result, AddressValidation::Valid);
    }

    #[test]
    fn test_valid_uppercase_address() {
        let result = validate_address("0x4F2083F5FBEDE34C2714AFFB3105539775F7FE64");
        assert_eq!(result, AddressValidation::Valid);
    }

    #[test]
    fn test_invalid_checksum() {
        // Wrong case that doesn't match EIP-55
        let result = validate_address("0x4F2083f5fbede34c2714affb3105539775f7fe64");
        assert_eq!(result, AddressValidation::ChecksumMismatch);
    }

    #[test]
    fn test_invalid_too_short() {
        let result = validate_address("0x4F2083f5");
        assert_eq!(result, AddressValidation::Invalid);
    }

    #[test]
    fn test_invalid_too_long() {
        let result = validate_address("0x4F2083f5fBede34C2714aFfb3105539775f7FE6400");
        assert_eq!(result, AddressValidation::Invalid);
    }

    #[test]
    fn test_invalid_no_prefix() {
        let result = validate_address("4F2083f5fBede34C2714aFfb3105539775f7FE64");
        assert_eq!(result, AddressValidation::Invalid);
    }

    #[test]
    fn test_invalid_non_hex() {
        let result = validate_address("0xGGGGGGf5fBede34C2714aFfb3105539775f7FE64");
        assert_eq!(result, AddressValidation::Invalid);
    }

    #[test]
    fn test_normalize_lowercase_to_checksummed() {
        let normalized = normalize_address("0x4f2083f5fbede34c2714affb3105539775f7fe64");
        assert_eq!(normalized, Some("0x4F2083f5fBede34C2714aFfb3105539775f7FE64".to_string()));
    }

    // Recent Addresses Tests
    #[test]
    fn test_recent_addresses_deduplication() {
        let mut recent = vec!["0xAAA...".into()];
        add_recent_address(&mut recent, "0xaaa...");  // Same address, different case
        assert_eq!(recent.len(), 1);
    }

    #[test]
    fn test_recent_addresses_max_limit() {
        let mut recent = vec![];
        for i in 0..15 {
            add_recent_address(&mut recent, &format!("0x{:040x}", i));
        }
        assert_eq!(recent.len(), 10);  // MAX_RECENT_ADDRESSES
    }

    #[test]
    fn test_recent_addresses_invalid_ignored() {
        let mut recent = vec![];
        add_recent_address(&mut recent, "invalid");
        assert!(recent.is_empty());
    }
}
```

### 5. API Response Parsing Tests

**Priority:** HIGH

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_safe_info_response() {
        let json = r#"{
            "address": "0x...",
            "nonce": "42",
            "threshold": 2,
            "owners": ["0x...", "0x..."],
            "modules": [],
            "version": "1.3.0"
        }"#;
        let info: SafeInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.nonce, 42);
    }

    #[test]
    fn test_parse_safe_transaction_response() {
        // Full Safe Transaction Service response
    }

    #[test]
    fn test_parse_data_decoded_nested() {
        // MultiSend with nested dataDecoded
    }

    #[test]
    fn test_malformed_nonce_string() {
        // nonce: "not_a_number" should fail gracefully
    }

    #[test]
    fn test_missing_optional_fields() {
        // Response without dataDecoded should still parse
    }
}
```

### 6. Signature Cache Tests (`crates/rusty-safe/src/decode/sourcify.rs`)

**Priority:** MEDIUM

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_hit() {
        let lookup = SignatureLookup::new();
        // Pre-populate cache
        lookup.cache.lock().unwrap().insert(
            "0xa9059cbb".into(),
            vec!["transfer(address,uint256)".into()],
        );

        let result = lookup.lookup("0xa9059cbb").await.unwrap();
        assert_eq!(result, vec!["transfer(address,uint256)"]);
    }

    #[test]
    fn test_spurious_detection() {
        let lookup = SignatureLookup::new();
        // Simulate 3 failures
        for _ in 0..3 {
            lookup.on_failure(&mock_timeout_error());
        }
        assert!(lookup.is_spurious());
    }

    #[test]
    fn test_spurious_reset() {
        let lookup = SignatureLookup::new();
        lookup.is_spurious.store(true, Ordering::Relaxed);
        lookup.reset_spurious();
        assert!(!lookup.is_spurious());
    }

    #[test]
    fn test_selector_normalization() {
        assert_eq!(normalize_selector("0xA9059CBB"), "0xa9059cbb");
        assert_eq!(normalize_selector("a9059cbb"), "0xa9059cbb");
    }

    #[test]
    fn test_cache_max_size_enforcement() {
        // Adding > MAX_CACHED_SELECTORS should truncate on save
    }
}
```

### 7. Integration Tests

**Priority:** HIGH

```rust
// tests/integration_tests.rs

#[tokio::test]
async fn test_full_verification_flow_mainnet() {
    // 1. Fetch known Safe transaction from API (use mock)
    // 2. Compute hashes
    // 3. Verify hash matches API
    // 4. Decode calldata
    // 5. Compare API vs local decode
}

#[tokio::test]
async fn test_multisend_verification_flow() {
    // Full MultiSend batch verification
}

#[tokio::test]
async fn test_offline_mode_flow() {
    // Manual input -> hash computation -> decode
}

#[tokio::test]
async fn test_eip712_flow() {
    // JSON input -> parse -> Safe wrap -> hash
}

#[tokio::test]
async fn test_message_signing_flow() {
    // Message -> hash -> Safe message hash
}
```

### 8. Adversarial Input Tests

**Priority:** HIGH (Security)

```rust
#[cfg(test)]
mod adversarial_tests {
    use super::*;

    // Malformed Input Tests
    #[test]
    fn test_unicode_in_address() {
        let result = validate_address("0x4F2083f5fBede34C2714aFfb3105539775f7FE6\u{200B}4");
        assert_eq!(result, AddressValidation::Invalid);
    }

    #[test]
    fn test_null_bytes_in_input() {
        let data = "0xa9059cbb\0\0\0\0...";
        // Should handle gracefully
    }

    #[test]
    fn test_extremely_long_calldata() {
        // 1MB of calldata
        let data = format!("0x{}", "aa".repeat(500_000));
        // Should not crash or hang
    }

    #[test]
    fn test_deeply_nested_multisend() {
        // MultiSend containing MultiSend
    }

    // Selector Collision Tests
    #[test]
    fn test_known_selector_collision() {
        // transfer(address,uint256) collides with some other function
        // Test that we handle ambiguity correctly
    }

    // Value Overflow Tests
    #[test]
    fn test_value_overflow_max_uint256() {
        let max = "115792089237316195423570985008687907853269984665640564039457584007913129639935";
        // Should handle without panic
    }

    // JSON Injection Tests
    #[test]
    fn test_malicious_json_in_eip712() {
        let json = r#"{"types": {}, "__proto__": {"polluted": true}}"#;
        // Should not cause issues
    }

    // CSV Injection Tests
    #[test]
    fn test_csv_formula_injection() {
        let csv = "=CMD|'/C calc'!A0,name,1\n0x...,safe,1";
        let mut book = AddressBook::default();
        let result = book.import_csv(csv);
        // Formula should be treated as invalid address
    }
}
```

---

## Test Infrastructure Requirements

### Mock Server Setup

```rust
// tests/common/mod.rs

use wiremock::{MockServer, Mock, matchers::*, ResponseTemplate};

pub async fn setup_mock_safe_api() -> MockServer {
    let mock_server = MockServer::start().await;

    // Mock Safe info endpoint
    Mock::given(method("GET"))
        .and(path_regex(r"/api/v1/safes/0x[a-fA-F0-9]{40}/"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(mock_safe_info()))
        .mount(&mock_server)
        .await;

    // Mock transactions endpoint
    Mock::given(method("GET"))
        .and(path_regex(r"/api/v1/safes/0x[a-fA-F0-9]{40}/multisig-transactions/"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(mock_transactions()))
        .mount(&mock_server)
        .await;

    mock_server
}

pub async fn setup_mock_sourcify_api() -> MockServer {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/signature-database/v1/lookup"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(mock_signatures()))
        .mount(&mock_server)
        .await;

    mock_server
}
```

### Test Data Fixtures

```rust
// tests/fixtures/mod.rs

/// Known Safe transaction with verified hash (Mainnet)
pub const KNOWN_TX_MAINNET: &str = r#"{
    "safe": "0x...",
    "to": "0x...",
    "value": "0",
    "data": "0xa9059cbb...",
    "operation": 0,
    "safeTxGas": 0,
    "baseGas": 0,
    "gasPrice": "0",
    "gasToken": "0x0000000000000000000000000000000000000000",
    "refundReceiver": "0x0000000000000000000000000000000000000000",
    "nonce": 42,
    "safeTxHash": "0x..."
}"#;

/// MultiSend with 5 nested transactions
pub const MULTISEND_5_TXS: &str = "0x8d80ff0a...";

/// EIP-712 typed data example
pub const EIP712_PERMIT: &str = r#"{
    "types": {...},
    "primaryType": "Permit",
    "domain": {...},
    "message": {...}
}"#;
```

---

## Coverage Targets

| Module | Current | Target | Priority |
|--------|---------|--------|----------|
| `hasher.rs` | 0% | 90% | CRITICAL |
| `decode/parser.rs` | 20% | 85% | CRITICAL |
| `decode/compare.rs` | 0% | 90% | CRITICAL |
| `expected.rs` | 0% | 85% | HIGH |
| `state.rs` (validation) | 30% | 80% | HIGH |
| `decode/sourcify.rs` | 5% | 70% | MEDIUM |
| `decode/verify.rs` | 0% | 75% | HIGH |
| `decode/offline.rs` | 0% | 70% | MEDIUM |
| Integration tests | 0% | N/A | HIGH |
| Adversarial tests | 0% | N/A | HIGH |

**Overall Target:** 80% line coverage, 100% coverage of security-critical paths

---

## CI/CD Integration

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Run tests
        run: cargo test --all-features

      - name: Run tests with coverage
        run: cargo llvm-cov --all-features --lcov --output-path lcov.info

      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          files: lcov.info
          fail_ci_if_error: true

      - name: Check coverage threshold
        run: |
          COVERAGE=$(cargo llvm-cov --all-features --summary-only | grep -oP '\d+\.\d+(?=%)')
          if (( $(echo "$COVERAGE < 80" | bc -l) )); then
            echo "Coverage $COVERAGE% is below 80% threshold"
            exit 1
          fi

  fuzz:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install cargo-fuzz
        run: cargo install cargo-fuzz

      - name: Run fuzzer (limited)
        run: cargo fuzz run calldata_parser -- -max_total_time=60
```

---

## Fuzzing Targets

```rust
// fuzz/fuzz_targets/calldata_parser.rs
#![no_main]
use libfuzzer_sys::fuzz_target;
use rusty_safe::decode::parser::*;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = parse_initial(s, None);
    }
});

// fuzz/fuzz_targets/address_validation.rs
fuzz_target!(|data: &str| {
    let _ = validate_address(data);
});

// fuzz/fuzz_targets/multisend_unpack.rs
fuzz_target!(|data: &[u8]| {
    let _ = unpack_multisend_transactions(data);
});
```
