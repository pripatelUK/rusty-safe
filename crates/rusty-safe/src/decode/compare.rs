//! Comparison logic for API vs Local decode

use alloy::primitives::U256;

use super::types::*;

/// Compare API decode against Local decode
pub fn compare_decodes(
    api: Option<&ApiDecode>,
    local: Option<&LocalDecode>,
) -> ComparisonResult {
    match (api, local) {
        (None, None) => ComparisonResult::Failed("No decode available".into()),
        (Some(_), None) => ComparisonResult::OnlyApi,
        (None, Some(_)) => ComparisonResult::OnlyLocal,
        (Some(a), Some(l)) => compare_both(a, l),
    }
}

/// Compare when both decodes are available
fn compare_both(api: &ApiDecode, local: &LocalDecode) -> ComparisonResult {
    // Compare method names (normalize)
    let api_method = normalize_method(&api.method);
    let local_method = normalize_method(&local.method);

    if api_method != local_method {
        return ComparisonResult::MethodMismatch {
            api: api.method.clone(),
            local: local.method.clone(),
        };
    }

    // Compare parameter counts
    if api.params.len() != local.params.len() {
        return ComparisonResult::ParamMismatch(vec![ParamDiff {
            index: 0,
            typ: "count".into(),
            api_value: format!("{} params", api.params.len()),
            local_value: format!("{} params", local.params.len()),
        }]);
    }

    // Compare parameter values
    let mut diffs = Vec::new();

    for (i, (ap, lp)) in api.params.iter().zip(local.params.iter()).enumerate() {
        if !values_match(&ap.value, &lp.value, &lp.typ) {
            diffs.push(ParamDiff {
                index: i,
                typ: lp.typ.clone(),
                api_value: ap.value.clone(),
                local_value: lp.value.clone(),
            });
        }
    }

    if diffs.is_empty() {
        ComparisonResult::Match
    } else {
        ComparisonResult::ParamMismatch(diffs)
    }
}

/// Normalize method name for comparison
fn normalize_method(method: &str) -> String {
    method.trim().to_lowercase()
}

/// Check if two values match (with type-aware normalization)
fn values_match(api_val: &str, local_val: &str, typ: &str) -> bool {
    let api_norm = normalize_value(api_val, typ);
    let local_norm = normalize_value(local_val, typ);
    api_norm == local_norm
}

/// Normalize a value based on its type
fn normalize_value(value: &str, typ: &str) -> String {
    let value = value.trim();

    // Address: lowercase, ensure 0x prefix
    if typ.contains("address") {
        return normalize_address(value);
    }

    // Integers: parse and compare numerically (must come before bytes check
    // because uint256 values can be hex like "0x3e8")
    if typ.contains("int") {
        return normalize_int(value);
    }

    // Bytes/hex: lowercase, ensure 0x prefix
    if typ.contains("bytes") || value.starts_with("0x") || value.starts_with("0X") {
        return normalize_hex(value);
    }

    // Bool: normalize to lowercase
    if typ.contains("bool") {
        return value.to_lowercase();
    }

    // String: as-is
    value.to_string()
}

/// Normalize address to checksummed or lowercase
fn normalize_address(addr: &str) -> String {
    let addr = addr.trim().to_lowercase();
    if addr.starts_with("0x") {
        addr
    } else {
        format!("0x{}", addr)
    }
}

/// Normalize hex value
fn normalize_hex(hex: &str) -> String {
    let hex = hex.trim().to_lowercase();
    if hex.starts_with("0x") {
        hex
    } else {
        format!("0x{}", hex)
    }
}

/// Normalize integer value (handle decimal and hex up to uint256)
fn normalize_int(value: &str) -> String {
    let value = value.trim();

    // Try parsing as hex (U256 handles full uint256 range)
    if value.starts_with("0x") || value.starts_with("0X") {
        if let Ok(n) = U256::from_str_radix(&value[2..], 16) {
            return n.to_string();
        }
        // Invalid hex - fall through to string comparison
    }

    // Try parsing as decimal (U256 handles full uint256 range)
    if let Ok(n) = value.parse::<U256>() {
        return n.to_string();
    }

    // Unparseable value - normalize case for string comparison
    // This preserves the original value for error context in mismatches
    value.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_address() {
        assert_eq!(
            normalize_address("0xAbCdEf1234567890AbCdEf1234567890AbCdEf12"),
            "0xabcdef1234567890abcdef1234567890abcdef12"
        );
    }

    #[test]
    fn test_normalize_int() {
        assert_eq!(normalize_int("1000000"), "1000000");
        assert_eq!(normalize_int("0x3e8"), "1000");
        assert_eq!(normalize_int("0x0"), "0");

        // Test large uint256 values (> u128::MAX)
        // 2^128 = 340282366920938463463374607431768211456
        let large_dec = "340282366920938463463374607431768211456";
        let large_hex = "0x100000000000000000000000000000000";
        assert_eq!(normalize_int(large_dec), large_dec);
        assert_eq!(normalize_int(large_hex), large_dec);

        // Test max uint256
        let max_u256 = "115792089237316195423570985008687907853269984665640564039457584007913129639935";
        assert_eq!(normalize_int(max_u256), max_u256);
    }

    #[test]
    fn test_values_match() {
        assert!(values_match(
            "0xAbCd",
            "0xabcd",
            "address"
        ));
        assert!(values_match("1000", "0x3e8", "uint256"));
        assert!(!values_match("1000", "2000", "uint256"));
    }
}


