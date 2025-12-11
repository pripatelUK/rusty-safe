//! Calldata parsing and decoding
//!
//! Uses `alloy_json_abi::Function::parse()` for signature parsing,
//! following the same pattern as Foundry's `abi_decode_calldata`.

use alloy::dyn_abi::JsonAbiExt;
use alloy::json_abi::Function;
use alloy::primitives::{hex, U256};
use eyre::{Result, WrapErr};

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
) -> Result<MultiSendDecode> {
    // Decode the outer multiSend(bytes) call
    let bytes_data = decode_multisend_bytes(raw_data)?;

    // Get nested decodes from API if available
    let api_nested_decodes: Vec<Option<ApiDecode>> = api_decoded
        .and_then(|d| d.parameters.first())
        .and_then(|p| p.value_decoded.as_ref())
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|item| {
                    // Each item has dataDecoded which contains method + params
                    item.get("dataDecoded")
                        .and_then(|dd| serde_json::from_value::<DataDecoded>(dd.clone()).ok())
                        .map(|d| convert_api_decode(&d))
                })
                .collect()
        })
        .unwrap_or_default();

    // Parse packed transactions and attach API decodes
    let mut transactions = unpack_multisend_transactions(&bytes_data)?;
    
    // Attach API decode data to each transaction
    for (i, tx) in transactions.iter_mut().enumerate() {
        tx.api_decode = api_nested_decodes.get(i).cloned().flatten();
    }

    let mut multi = MultiSendDecode {
        transactions,
        summary: MultiSendSummary::default(),
        verification_state: VerificationState::Pending,
    };
    multi.summary.update(&multi.transactions);

    Ok(multi)
}

/// Decode multiSend(bytes) ABI encoding to get the packed bytes
pub fn decode_multisend_bytes(raw_data: &str) -> Result<Vec<u8>> {
    // Skip selector (4 bytes = 8 hex chars + 2 for "0x")
    let encoded = raw_data
        .strip_prefix("0x")
        .unwrap_or(raw_data)
        .get(8..)
        .ok_or_else(|| eyre::eyre!("Data too short"))?;

    let bytes = hex::decode(encoded).wrap_err("Failed to decode hex")?;

    // ABI decode: bytes is (offset, length, data)
    // offset is at position 0 (32 bytes)
    // length is at offset position (32 bytes)
    // data follows

    eyre::ensure!(bytes.len() >= 64, "Data too short for ABI bytes");

    // Read offset (should be 32 = 0x20)
    let offset = U256::from_be_slice(&bytes[0..32]);
    let offset_usize = offset.to::<usize>();

    eyre::ensure!(offset_usize + 32 <= bytes.len(), "Invalid offset");

    // Read length
    let length = U256::from_be_slice(&bytes[offset_usize..offset_usize + 32]);
    let length_usize = length.to::<usize>();

    let data_start = offset_usize + 32;
    eyre::ensure!(data_start + length_usize <= bytes.len(), "Invalid length");

    Ok(bytes[data_start..data_start + length_usize].to_vec())
}

/// Unpack MultiSend packed transactions
pub fn unpack_multisend_transactions(packed: &[u8]) -> Result<Vec<MultiSendTx>> {
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
        eyre::ensure!(
            offset + 20 <= packed.len(),
            "Incomplete transaction: missing 'to' address"
        );
        let to = format!("0x{}", hex::encode(&packed[offset..offset + 20]));
        offset += 20;

        // value: 32 bytes
        eyre::ensure!(
            offset + 32 <= packed.len(),
            "Incomplete transaction: missing 'value'"
        );
        let value = U256::from_be_slice(&packed[offset..offset + 32]);
        offset += 32;

        // dataLength: 32 bytes
        eyre::ensure!(
            offset + 32 <= packed.len(),
            "Incomplete transaction: missing 'dataLength'"
        );
        let data_length = U256::from_be_slice(&packed[offset..offset + 32]);
        let data_length_usize = data_length.to::<usize>();
        offset += 32;

        // data: dataLength bytes
        eyre::ensure!(
            offset + data_length_usize <= packed.len(),
            "Incomplete transaction: missing 'data'"
        );
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
            api_decode: None, // Will be filled in by parse_multisend
            decode: None,
            is_expanded: false,
        });
    }

    Ok(transactions)
}

use super::decode_log;

/// Decode calldata using a function signature
///
/// Uses `alloy_json_abi::Function::parse()` for robust signature parsing,
/// following the same pattern as Foundry's `abi_decode_calldata`.
pub fn decode_with_signature(
    data: &str,
    signature: &str,
) -> Result<LocalDecode> {
    decode_log!("Decoding with signature: {}", signature);

    // Parse function signature using alloy-json-abi (same as Foundry)
    let func = Function::parse(signature)
        .wrap_err_with(|| format!("Invalid signature '{}'", signature))?;

    decode_log!("Parsed function: {} with {} inputs", func.name, func.inputs.len());

    // Decode the calldata bytes
    let data_bytes = hex::decode(data.strip_prefix("0x").unwrap_or(data))
        .wrap_err("Failed to decode hex calldata")?;

    // Need at least 4 bytes for selector
    eyre::ensure!(
        data_bytes.len() >= 4,
        "Data too short (need at least 4 bytes for selector)"
    );

    // Empty params case
    if data_bytes.len() == 4 && func.inputs.is_empty() {
        return Ok(LocalDecode {
            signature: signature.to_string(),
            method: func.name.clone(),
            params: vec![],
        });
    }

    // Use Function::abi_decode_input which handles the selector automatically
    let decoded = func
        .abi_decode_input(&data_bytes[4..], true)
        .wrap_err_with(|| format!("ABI decode failed for '{}'", signature))?;

    // Ensure we decoded something (same check as Foundry)
    eyre::ensure!(
        !decoded.is_empty() || func.inputs.is_empty(),
        "No data was decoded"
    );

    // Build params with type info from the function definition
    let params = decoded
        .iter()
        .zip(func.inputs.iter())
        .map(|(val, input)| LocalParam {
            typ: input.ty.clone(),
            value: format_value(val),
        })
        .collect();

    Ok(LocalDecode {
        signature: signature.to_string(),
        method: func.name.clone(),
        params,
    })
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
    fn test_get_selector() {
        assert_eq!(get_selector("0xa9059cbb1234"), "0xa9059cbb");
        assert_eq!(get_selector("a9059cbb1234"), "0xa9059cbb");
    }

    #[test]
    fn test_decode_transfer() {
        // Standard ERC20 transfer(address,uint256)
        let sig = "transfer(address,uint256)";
        let data = "0xa9059cbb000000000000000000000000d8da6bf26964af9d7eed9e03e53415d37aa960450000000000000000000000000000000000000000000000000de0b6b3a7640000";

        let result = decode_with_signature(data, sig).unwrap();
        assert_eq!(result.method, "transfer");
        assert_eq!(result.params.len(), 2);
        assert_eq!(result.params[0].typ, "address");
        assert_eq!(result.params[1].typ, "uint256");
    }

    #[test]
    fn test_decode_no_params() {
        let sig = "pause()";
        // Just the selector, no params
        let data = "0x8456cb59";

        let result = decode_with_signature(data, sig).unwrap();
        assert_eq!(result.method, "pause");
        assert_eq!(result.params.len(), 0);
    }

    #[test]
    fn test_decode_scope_function() {
        let sig = "scopeFunction(uint16,address,bytes4,bool[],uint8[],uint8[],bytes[],uint8)";
        let data = "0x33a0480c000000000000000000000000000000000000000000000000000000000000000100000000000000000000000068b3465833fb72a70ecdf485e0e4c7bd8665fc45472b43f300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000001a0000000000000000000000000000000000000000000000000000000000000024000000000000000000000000000000000000000000000000000000000000002e000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000000e000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000200000000000000000000000004f2083f5fbede34c2714affb3105539775f7fe64";

        let result = decode_with_signature(data, sig);
        match result {
            Ok(decoded) => {
                println!("Decoded successfully!");
                println!("Method: {}", decoded.method);
                for (i, p) in decoded.params.iter().enumerate() {
                    println!("  Param {}: {} = {}", i, p.typ, p.value);
                }
                assert_eq!(decoded.method, "scopeFunction");
                assert_eq!(decoded.params.len(), 8, "Should decode 8 params");
            }
            Err(e) => {
                panic!("Failed to decode: {}", e);
            }
        }
    }
}

