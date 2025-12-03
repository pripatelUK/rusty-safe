//! Calldata parsing and decoding

use alloy::dyn_abi::DynSolType;
use alloy::primitives::{hex, U256};

use super::types::*;
use crate::api::DataDecoded;

/// MultiSend function selector
pub const MULTISEND_SELECTOR: &str = "0x8d80ff0a";

/// Parse calldata and API decode into initial structure
pub fn parse_initial(
    raw_data: &str,
    api_decoded: Option<&DataDecoded>,
) -> DecodedTransaction {
    let raw_data = raw_data.trim();

    // Empty calldata
    if raw_data.is_empty() || raw_data == "0x" {
        return DecodedTransaction {
            raw_data: raw_data.to_string(),
            selector: String::new(),
            kind: TransactionKind::Empty,
            status: OverallStatus::AllMatch,
        };
    }

    // Need at least selector (4 bytes = 8 chars + 0x)
    if raw_data.len() < 10 {
        return DecodedTransaction {
            raw_data: raw_data.to_string(),
            selector: raw_data.to_string(),
            kind: TransactionKind::Unknown,
            status: OverallStatus::Failed,
        };
    }

    let selector = raw_data[..10].to_lowercase();
    let api_decode = api_decoded.map(convert_api_decode);

    // Check if MultiSend
    if selector == MULTISEND_SELECTOR {
        match parse_multisend(raw_data, api_decoded) {
            Ok(multi) => DecodedTransaction {
                raw_data: raw_data.to_string(),
                selector,
                kind: TransactionKind::MultiSend(multi),
                status: OverallStatus::Pending,
            },
            Err(_) => DecodedTransaction {
                raw_data: raw_data.to_string(),
                selector,
                kind: TransactionKind::Unknown,
                status: OverallStatus::Failed,
            },
        }
    } else {
        // Single function call
        DecodedTransaction {
            raw_data: raw_data.to_string(),
            selector: selector.clone(),
            kind: TransactionKind::Single(SingleDecode {
                api: api_decode,
                local: None,
                comparison: ComparisonResult::Pending,
            }),
            status: OverallStatus::Pending,
        }
    }
}

/// Convert Safe API DataDecoded to our ApiDecode type
fn convert_api_decode(decoded: &DataDecoded) -> ApiDecode {
    ApiDecode {
        method: decoded.method.clone(),
        params: decoded
            .parameters
            .iter()
            .map(|p| ApiParam {
                name: p.name.clone(),
                typ: p.r#type.clone(),
                value: p.value_as_string(),
            })
            .collect(),
    }
}

/// Parse MultiSend calldata
fn parse_multisend(
    raw_data: &str,
    api_decoded: Option<&DataDecoded>,
) -> Result<MultiSendDecode, String> {
    // Decode the outer multiSend(bytes) call
    let bytes_data = decode_multisend_bytes(raw_data)?;

    // Get nested decodes from API if available (used later for comparison)
    let _api_nested_count = api_decoded
        .and_then(|d| d.parameters.first())
        .and_then(|p| p.value_decoded.as_ref())
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0);

    // Parse packed transactions
    let transactions = unpack_multisend_transactions(&bytes_data)?;

    let mut multi = MultiSendDecode {
        transactions,
        summary: MultiSendSummary::default(),
    };
    multi.summary.update(&multi.transactions);

    Ok(multi)
}

/// Decode multiSend(bytes) ABI encoding to get the packed bytes
fn decode_multisend_bytes(raw_data: &str) -> Result<Vec<u8>, String> {
    // Skip selector (4 bytes = 8 hex chars + 2 for "0x")
    let encoded = raw_data
        .strip_prefix("0x")
        .unwrap_or(raw_data)
        .get(8..)
        .ok_or("Data too short")?;

    let bytes = hex::decode(encoded).map_err(|e| format!("Hex decode error: {}", e))?;

    // ABI decode: bytes is (offset, length, data)
    // offset is at position 0 (32 bytes)
    // length is at offset position (32 bytes)
    // data follows

    if bytes.len() < 64 {
        return Err("Data too short for ABI bytes".into());
    }

    // Read offset (should be 32 = 0x20)
    let offset = U256::from_be_slice(&bytes[0..32]);
    let offset_usize = offset.to::<usize>();

    if offset_usize + 32 > bytes.len() {
        return Err("Invalid offset".into());
    }

    // Read length
    let length = U256::from_be_slice(&bytes[offset_usize..offset_usize + 32]);
    let length_usize = length.to::<usize>();

    let data_start = offset_usize + 32;
    if data_start + length_usize > bytes.len() {
        return Err("Invalid length".into());
    }

    Ok(bytes[data_start..data_start + length_usize].to_vec())
}

/// Unpack MultiSend packed transactions
fn unpack_multisend_transactions(packed: &[u8]) -> Result<Vec<MultiSendTx>, String> {
    let mut transactions = Vec::new();
    let mut offset = 0;

    while offset < packed.len() {
        // operation: 1 byte
        if offset >= packed.len() {
            break;
        }
        let operation = packed[offset];
        offset += 1;

        // to: 20 bytes
        if offset + 20 > packed.len() {
            return Err("Incomplete transaction: missing 'to' address".into());
        }
        let to = format!("0x{}", hex::encode(&packed[offset..offset + 20]));
        offset += 20;

        // value: 32 bytes
        if offset + 32 > packed.len() {
            return Err("Incomplete transaction: missing 'value'".into());
        }
        let value = U256::from_be_slice(&packed[offset..offset + 32]);
        offset += 32;

        // dataLength: 32 bytes
        if offset + 32 > packed.len() {
            return Err("Incomplete transaction: missing 'dataLength'".into());
        }
        let data_length = U256::from_be_slice(&packed[offset..offset + 32]);
        let data_length_usize = data_length.to::<usize>();
        offset += 32;

        // data: dataLength bytes
        if offset + data_length_usize > packed.len() {
            return Err("Incomplete transaction: missing 'data'".into());
        }
        let data = if data_length_usize > 0 {
            format!("0x{}", hex::encode(&packed[offset..offset + data_length_usize]))
        } else {
            "0x".to_string()
        };
        offset += data_length_usize;

        transactions.push(MultiSendTx {
            index: transactions.len(),
            operation,
            to,
            value: value.to_string(),
            data,
            decode: None,
            is_expanded: false,
            is_loading: false,
        });
    }

    Ok(transactions)
}

/// Log to console (works in both WASM and native)
macro_rules! decode_log {
    ($($arg:tt)*) => {
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::console::log_1(&format!($($arg)*).into());
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            eprintln!("[decode] {}", format!($($arg)*));
        }
    };
}

/// Decode calldata using a function signature
pub fn decode_with_signature(
    data: &str,
    signature: &str,
) -> Result<LocalDecode, String> {
    decode_log!("Decoding with signature: {}", signature);

    // Parse method name from signature
    let method = signature
        .split('(')
        .next()
        .unwrap_or(signature)
        .to_string();

    // Parse parameter types from signature
    let types = match parse_signature_types(signature) {
        Ok(t) => {
            decode_log!("Parsed {} types: {:?}", t.len(), t);
            t
        }
        Err(e) => {
            decode_log!("Failed to parse signature types: {}", e);
            return Err(e);
        }
    };

    // Skip selector (first 4 bytes = 8 hex chars)
    let data_normalized = data.strip_prefix("0x").unwrap_or(data);
    if data_normalized.len() < 8 {
        return Err("Data too short".into());
    }
    let params_hex = &data_normalized[8..];

    // Empty params case
    if params_hex.is_empty() && types.is_empty() {
        return Ok(LocalDecode {
            signature: signature.to_string(),
            method,
            params: vec![],
        });
    }

    let params_bytes = hex::decode(params_hex).map_err(|e| format!("Hex decode: {}", e))?;
    decode_log!("Params bytes length: {}", params_bytes.len());

    // Build tuple type for decoding
    let tuple_type = DynSolType::Tuple(types.clone());

    // Try abi_decode_params first (handles function params encoding)
    // Fall back to abi_decode if that fails
    let decoded = tuple_type
        .abi_decode_params(&params_bytes)
        .or_else(|e1| {
            decode_log!("abi_decode_params failed, trying abi_decode: {}", e1);
            tuple_type.abi_decode(&params_bytes).map_err(|e2| {
                decode_log!("Both decode methods failed for '{}': params={}, decode={}", signature, e1, e2);
                format!("ABI decode failed: {} / {}", e1, e2)
            })
        })?;

    // Extract values
    let params = match decoded {
        alloy::dyn_abi::DynSolValue::Tuple(values) => values
            .iter()
            .zip(types.iter())
            .map(|(val, typ)| LocalParam {
                typ: format_type(typ),
                value: format_value(val),
            })
            .collect(),
        _ => return Err("Expected tuple from decode".into()),
    };

    Ok(LocalDecode {
        signature: signature.to_string(),
        method,
        params,
    })
}

/// Parse function signature to extract parameter types
fn parse_signature_types(sig: &str) -> Result<Vec<DynSolType>, String> {
    let start = sig.find('(').ok_or("Invalid signature: no '('")?;
    let end = sig.rfind(')').ok_or("Invalid signature: no ')'")?;
    let params_str = &sig[start + 1..end];

    if params_str.is_empty() {
        return Ok(vec![]);
    }

    // Handle nested tuples by tracking parenthesis depth
    let mut types = Vec::new();
    let mut current = String::new();
    let mut depth = 0;

    for ch in params_str.chars() {
        match ch {
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                let typ = current.trim();
                if !typ.is_empty() {
                    types.push(
                        DynSolType::parse(typ).map_err(|e| format!("Invalid type '{}': {}", typ, e))?,
                    );
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    // Don't forget the last type
    let typ = current.trim();
    if !typ.is_empty() {
        types.push(DynSolType::parse(typ).map_err(|e| format!("Invalid type '{}': {}", typ, e))?);
    }

    Ok(types)
}

/// Format a DynSolType for display
fn format_type(typ: &DynSolType) -> String {
    format!("{}", typ)
}

/// Format a decoded value for display
fn format_value(val: &alloy::dyn_abi::DynSolValue) -> String {
    use alloy::dyn_abi::DynSolValue;

    match val {
        DynSolValue::Bool(b) => b.to_string(),
        DynSolValue::Int(i, _) => i.to_string(),
        DynSolValue::Uint(u, _) => u.to_string(),
        // FixedBytes: word is 32 bytes, size is actual length (e.g., 4 for bytes4)
        // bytesN is right-padded, so take first `size` bytes
        DynSolValue::FixedBytes(word, size) => {
            let bytes = word.as_slice();
            format!("0x{}", hex::encode(&bytes[..*size]))
        }
        DynSolValue::Address(a) => format!("{}", a),
        DynSolValue::Function(f) => format!("0x{}", hex::encode(f)),
        DynSolValue::Bytes(b) => format!("0x{}", hex::encode(b)),
        DynSolValue::String(s) => s.clone(),
        DynSolValue::Array(arr) | DynSolValue::FixedArray(arr) => {
            let items: Vec<std::string::String> = arr.iter().map(format_value).collect();
            format!("[{}]", items.join(", "))
        }
        DynSolValue::Tuple(items) => {
            let items: Vec<std::string::String> = items.iter().map(format_value).collect();
            format!("({})", items.join(", "))
        }
        DynSolValue::CustomStruct { name, tuple, .. } => {
            let items: Vec<std::string::String> = tuple.iter().map(format_value).collect();
            format!("{}({})", name, items.join(", "))
        }
    }
}

/// Get selector from calldata
pub fn get_selector(data: &str) -> String {
    let data = data.strip_prefix("0x").unwrap_or(data);
    if data.len() >= 8 {
        format!("0x{}", &data[..8].to_lowercase())
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_signature_types() {
        let types = parse_signature_types("transfer(address,uint256)").unwrap();
        assert_eq!(types.len(), 2);

        let types = parse_signature_types("multiSend(bytes)").unwrap();
        assert_eq!(types.len(), 1);

        let types = parse_signature_types("noParams()").unwrap();
        assert_eq!(types.len(), 0);
    }

    #[test]
    fn test_get_selector() {
        assert_eq!(get_selector("0xa9059cbb1234"), "0xa9059cbb");
        assert_eq!(get_selector("a9059cbb1234"), "0xa9059cbb");
    }

    #[test]
    fn test_decode_scope_function() {
        let sig = "scopeFunction(uint16,address,bytes4,bool[],uint8[],uint8[],bytes[],uint8)";
        let data = "0x33a0480c000000000000000000000000000000000000000000000000000000000000000100000000000000000000000068b3465833fb72a70ecdf485e0e4c7bd8665fc45472b43f300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000001a0000000000000000000000000000000000000000000000000000000000000024000000000000000000000000000000000000000000000000000000000000002e000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000000e000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000200000000000000000000000004f2083f5fbede34c2714affb3105539775f7fe64";

        // First verify signature parsing works
        let types = parse_signature_types(sig).unwrap();
        assert_eq!(types.len(), 8, "Should have 8 params");
        println!("Types parsed: {:?}", types);

        // Now try decoding
        let result = decode_with_signature(data, sig);
        match result {
            Ok(decoded) => {
                println!("Decoded successfully!");
                println!("Method: {}", decoded.method);
                for (i, p) in decoded.params.iter().enumerate() {
                    println!("  Param {}: {} = {}", i, p.typ, p.value);
                }
                assert_eq!(decoded.params.len(), 8, "Should decode 8 params");
            }
            Err(e) => {
                panic!("Failed to decode: {}", e);
            }
        }
    }
}

