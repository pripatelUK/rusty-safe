//! Offline mode calldata decoding
//!
//! Decodes calldata using 4byte signature lookup only (no Safe API comparison).
//! Used when manually inputting transaction data.

use std::collections::HashSet;

use super::parser;
use super::sourcify::SignatureLookup;
use super::types::*;
use super::decode_log;

/// Decode calldata for offline mode (4byte lookup only, no API comparison)
pub async fn decode_offline(
    raw_data: &str,
    lookup: &SignatureLookup,
) -> OfflineDecodeResult {
    let raw_data = raw_data.trim();
    
    // Empty calldata = native ETH transfer
    if raw_data.is_empty() || raw_data == "0x" {
        return OfflineDecodeResult::Empty;
    }
    
    // Need at least selector (4 bytes = 8 chars + 0x)
    if raw_data.len() < 10 {
        return OfflineDecodeResult::RawHex(raw_data.to_string());
    }
    
    let selector = raw_data[..10].to_lowercase();
    
    // Check if MultiSend
    if selector == parser::MULTISEND_SELECTOR {
        match decode_offline_multisend(raw_data, lookup).await {
            Ok(txs) => return OfflineDecodeResult::MultiSend(txs),
            Err(e) => {
                decode_log!("Failed to decode MultiSend: {}", e);
                return OfflineDecodeResult::RawHex(raw_data.to_string());
            }
        }
    }
    
    // Single function call
    decode_offline_single(raw_data, &selector, lookup).await
}

/// Decode a single function call for offline mode
async fn decode_offline_single(
    raw_data: &str,
    selector: &str,
    lookup: &SignatureLookup,
) -> OfflineDecodeResult {
    // Lookup signature
    let sigs = match lookup.lookup(selector).await {
        Ok(s) => s,
        Err(_) => vec![],
    };
    
    if sigs.is_empty() {
        return OfflineDecodeResult::Single {
            local: LocalDecode {
                signature: String::new(),
                method: format!("Unknown function {}", selector),
                params: vec![],
            },
            status: OfflineDecodeStatus::Unknown(selector.to_string()),
        };
    }
    
    // Try each signature until one decodes
    for sig in &sigs {
        match parser::decode_with_signature(raw_data, sig) {
            Ok(decoded) => {
                return OfflineDecodeResult::Single {
                    local: decoded,
                    status: OfflineDecodeStatus::Decoded,
                };
            }
            Err(e) => {
                decode_log!("Failed to decode with {}: {}", sig, e);
            }
        }
    }
    
    // All signatures failed
    OfflineDecodeResult::Single {
        local: LocalDecode {
            signature: sigs.first().cloned().unwrap_or_default(),
            method: format!("Failed to decode {}", selector),
            params: vec![],
        },
        status: OfflineDecodeStatus::Failed("ABI decode failed".to_string()),
    }
}

/// Decode MultiSend for offline mode
async fn decode_offline_multisend(
    raw_data: &str,
    lookup: &SignatureLookup,
) -> eyre::Result<Vec<OfflineMultiSendTx>> {
    // Unpack the MultiSend bytes
    let bytes = parser::decode_multisend_bytes(raw_data)?;
    let online_txs = parser::unpack_multisend_transactions(&bytes)?;
    
    // Collect unique selectors
    let selectors: Vec<String> = online_txs
        .iter()
        .filter(|tx| tx.data.len() >= 10 && tx.data != "0x")
        .map(|tx| tx.data[..10].to_lowercase())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    
    // Batch fetch signatures
    let signatures = lookup.lookup_batch(&selectors).await;
    
    // Convert to offline format and decode each
    let mut result = Vec::with_capacity(online_txs.len());
    
    for tx in online_txs {
        let (local_decode, status) = if tx.data.len() < 10 || tx.data == "0x" {
            // Empty calldata
            (None, OfflineDecodeStatus::Decoded)
        } else {
            let selector = tx.data[..10].to_lowercase();
            
            match signatures.get(&selector) {
                Some(sigs) if !sigs.is_empty() => {
                    // Try to decode with available signatures
                    let mut decoded = None;
                    for sig in sigs {
                        if let Ok(d) = parser::decode_with_signature(&tx.data, sig) {
                            decoded = Some(d);
                            break;
                        }
                    }
                    
                    match decoded {
                        Some(d) => (Some(d), OfflineDecodeStatus::Decoded),
                        None => (None, OfflineDecodeStatus::Failed("ABI decode failed".to_string())),
                    }
                }
                _ => (None, OfflineDecodeStatus::Unknown(selector)),
            }
        };
        
        result.push(OfflineMultiSendTx {
            index: tx.index,
            operation: tx.operation,
            to: tx.to,
            value: tx.value,
            data: tx.data,
            local_decode,
            status,
            is_expanded: false,
        });
    }
    
    Ok(result)
}

