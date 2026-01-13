# Production Hardening Requirements

This document outlines security findings and production hardening requirements for rusty-safe, a wallet-adjacent application for Safe{Wallet} transaction verification.

**Current Security Posture:** HIGH RISK - Not Production Ready (6 issues fixed, 6 remaining)

## Critical Issues (Must Fix Before Production)

### 1. 4byte Signature Trust Model is Fundamentally Broken

**Severity:** HIGH
**Location:** `crates/rusty-safe/src/app.rs:1084-1093`, `crates/rusty-safe/src/decode/ui.rs:197,597`

**Problem:** The "independent" calldata decoding is not actually independent - it trusts 4byte signatures from Sourcify and accepts the first signature that happens to decode successfully. This is vulnerable to:
- **Selector collisions:** Different functions can have the same 4-byte selector
- **Misleading ABIs:** An attacker could register a misleading signature for a malicious contract
- **False confidence:** The UI presents this as "verified" when it's actually just a guess

**Attack Vector:** A malicious actor could create a contract where a function like `transfer(address,uint256)` has a selector collision with `stealAllFunds(address,uint256)`. The 4byte lookup would return the benign-looking signature.

**Remediation:**
1. Treat 4byte signatures as **untrusted hints**, not verification
2. Show ALL candidate signatures when multiple exist (with explicit ambiguity warnings)
3. Require user acknowledgment when multiple signatures decode successfully
4. Integrate with Sourcify full contract verification API for verified contracts
5. Update UI labels from "Verified" to "Decoded (unverified)" or similar

### ~~2. Message Hash Ignores Hex Flag~~ FIXED

**Severity:** HIGH
**Location:** `crates/rusty-safe/src/app.rs:780`
**Status:** FIXED in commit `8186a2b`

**Problem:** The `compute_message_hash` function ignored `self.msg_state.is_hex` flag, always treating input as UTF-8 string.

**Resolution:** Now checks `is_hex` flag and parses input as hex bytes when enabled, using `MessageHasher::new_from_bytes()` with proper keccak256 hashing. Invalid hex input returns a descriptive error.

### ~~3. Missing UI Repaint After Decode Lookup~~ FIXED

**Severity:** MEDIUM
**Location:** `crates/rusty-safe/src/app.rs:1023-1051`
**Status:** FIXED in commit `2029448`

**Problem:** `trigger_decode_lookup` spawned async work but never called `ctx.request_repaint()`.

**Resolution:** Added `ctx: &egui::Context` parameter, cloned context into async blocks, and added `ctx.request_repaint()` after setting results in both WASM and native paths.

---

## Medium Severity Issues

### 4. Signature Cache Has No Integrity Checking

**Severity:** MEDIUM
**Location:** `crates/rusty-safe/src/decode/sourcify.rs:97-136`

**Problem:** The signature cache is loaded from eframe storage (localStorage in WASM) without any integrity or provenance checks. A compromised browser extension or XSS attack could inject malicious signatures.

**Attack Vector:** Attacker injects cache entry mapping a benign selector to a misleading signature, causing users to misinterpret transaction intent.

**Remediation:**
1. Add HMAC or signature verification for cache entries
2. Store source, timestamp, and hash for cache entries
3. Clear cache on version changes
4. Consider disabling persistent caching in production builds
5. Add "cache provenance unknown" warning in UI

### ~~5. Expected Value Validation Silently Skips Invalid Inputs~~ FIXED

**Severity:** MEDIUM
**Location:** `crates/rusty-safe/src/expected.rs:170,184`
**Status:** FIXED in commit `d12e26a`

**Problem:** When user entered invalid expected values, validation silently skipped and could return false "Match".

**Resolution:** Added `ValidationResult::ParseErrors` variant. Validation now reports all parse errors explicitly in UI with yellow warning styling. Never returns "Match" if any field failed to parse.

### ~~6. Warning Computation Coerces Parse Failures to Zero~~ FIXED

**Severity:** MEDIUM
**Location:** `crates/rusty-safe/src/hasher.rs:245-251,280-285`
**Status:** FIXED in commit `d12e26a`

**Problem:** `get_warnings_for_tx` and `get_warnings_from_api_tx` used `.unwrap_or(...)` to coerce parse failures to zero values, suppressing warnings.

**Resolution:** Both functions now return `Result<SafeWarnings>`. Parse errors are propagated and tracked via `warnings_error` field in UI state. Callers display appropriate error messages instead of computing warnings on invalid data.

### 7. Supply Chain Risk: Unpinned Git Dependencies

**Severity:** MEDIUM
**Location:** `Cargo.toml:23-24`

**Problem:** `safe-utils` and `safe-hash` are pulled from `main` branch without commit pinning. Hash computation behavior could change unexpectedly across builds.

**Impact:** Builds at different times could produce different hash results, or a compromised upstream could inject malicious code.

**Remediation:**
```toml
# Pin to specific commit
safe-utils = { git = "https://github.com/pripatelUK/safe-hash-rs", rev = "abc123..." }
safe-hash = { git = "https://github.com/pripatelUK/safe-hash-rs", rev = "abc123...", default-features = false }
```

Also:
1. Audit and vendor dependencies in release builds
2. Enforce `Cargo.lock` in production pipelines
3. Set up dependency update notifications

### ~~8. Integer Comparison Truncates Large Values~~ FIXED

**Severity:** MEDIUM
**Location:** `crates/rusty-safe/src/decode/compare.rs:125-145`
**Status:** FIXED in commit `88edf2a`

**Problem:** `normalize_int` used `u128` which truncated values > 2^128, causing false mismatch reports.

**Resolution:** Updated `normalize_int` to use `alloy::primitives::U256` for full uint256 range support. Unparseable values fall through to lowercase string comparison, preserving error context in mismatch reports. Added tests for 2^128 and max uint256 values.

---

## Low Severity Issues

### 9. MultiSend Header Shows Unverified API Data

**Severity:** LOW
**Location:** `crates/rusty-safe/src/decode/ui.rs:381,385`

**Problem:** MultiSend transaction headers display API-decoded method/params before verification completes. Could mislead users.

**Remediation:** Label API-derived fields as "(unverified)" until local decode succeeds; prefer local decode display when available.

### ~~10. Mutex Unwraps Can Panic~~ FIXED

**Severity:** LOW
**Location:** `crates/rusty-safe/src/app.rs`, `crates/rusty-safe/src/decode/sourcify.rs`
**Status:** FIXED in commit `6d97a84`

**Problem:** Multiple `.lock().unwrap()` calls would panic on poisoned locks.

**Resolution:** Added `lock_or_recover!` macro that handles poisoned mutexes gracefully. If a thread panicked while holding a lock, the app now recovers the inner data via `poisoned.into_inner()` and logs a warning. Replaced 20 occurrences across app.rs (14) and sourcify.rs (6).

### 11. Non-Checksummed Addresses Accepted Without Warning

**Severity:** LOW
**Location:** `crates/rusty-safe/src/state.rs:93-106`

**Problem:** All-lowercase addresses are treated as valid. Typos in lowercase addresses won't be caught by checksum validation.

**Remediation:** Show warning when address is not checksummed but could be (i.e., contains letters). Recommend using checksummed addresses.

### 12. Empty Nested Calldata Shows "Pending" Instead of "Verified"

**Severity:** LOW
**Location:** `crates/rusty-safe/src/decode/verify.rs`

**Problem:** Transactions with empty calldata (native ETH transfers) in MultiSend show "Pending" status instead of being marked as verified.

**Remediation:** Explicitly handle empty calldata case and mark as "Verified - Native Transfer".

---

## Edge Cases Requiring Special Handling

### Hash Computation Edge Cases
- **Safe version mismatch:** User-selected version doesn't match actual Safe contract version
- **Unsupported chain:** Chain not in safe-utils list
- **API timeout during verification:** Should show "verification incomplete" not "verified"

### Decoding Edge Cases
- **Multiple valid signatures for selector:** Currently picks first; should show all
- **Very long calldata:** May cause UI performance issues
- **Malformed ABI encoding:** Should fail gracefully, not crash
- **Recursive MultiSend:** MultiSend containing MultiSend calls

### Input Validation Edge Cases
- **Unicode in addresses:** Should be rejected
- **Leading/trailing whitespace:** Currently handled inconsistently
- **Scientific notation in values:** May parse incorrectly

---

## Security Architecture Recommendations

### 1. Defense in Depth for Calldata Verification
```
Layer 1: Parse calldata structure (MultiSend vs single)
Layer 2: 4byte signature lookup (HINT ONLY)
Layer 3: Sourcify verified contract ABI (if available)
Layer 4: User-provided expected values
Layer 5: Visual diff highlighting
```

### 2. Trust Boundaries
```
UNTRUSTED:
- Safe Transaction Service API responses
- 4byte.directory / Sourcify signature database
- User inputs
- localStorage / eframe storage

TRUSTED (after verification):
- Computed hashes (from safe-utils)
- EIP-712 structured data
- Chain configuration (from safe-utils)
```

### 3. Error Handling Strategy
```rust
// Principle: Fail loudly, never silently succeed
pub enum VerificationResult {
    Verified { confidence: Confidence },
    Mismatch { details: Vec<Mismatch> },
    VerificationFailed { reason: String },  // <- Use this, don't silently pass
    Pending,
}

pub enum Confidence {
    High,    // Sourcify verified ABI + expected values match
    Medium,  // 4byte decode matches API
    Low,     // 4byte only, no API comparison
    Unknown, // Decode failed
}
```

---

## Pre-Production Checklist

- [ ] Fix all CRITICAL and HIGH severity issues (2/3 done: #2, #3 fixed; #1 remains)
- [x] Fix MEDIUM severity issues #5, #6 (validation/warnings parse error handling)
- [x] Fix MEDIUM severity issue #8 (uint256 comparison truncation)
- [x] Fix LOW severity issue #10 (mutex unwraps can panic)
- [ ] Address MEDIUM severity issues or document accepted risks
- [ ] Pin all git dependencies to specific commits
- [ ] Add comprehensive input validation
- [ ] Implement cache integrity verification
- [ ] Add "verification confidence" indicators to UI
- [ ] Update all "verified" labels to accurately reflect trust level
- [ ] Add security-focused user documentation
- [ ] Conduct external security audit
- [ ] Set up automated dependency vulnerability scanning
- [ ] Implement telemetry for error tracking (opt-in)
- [ ] Add rate limiting awareness for API calls
- [ ] Test with adversarial inputs (fuzzing)
