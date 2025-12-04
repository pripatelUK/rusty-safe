//! Calldata decoding with independent verification
//!
//! Provides side-by-side comparison of:
//! - Safe API's decoded calldata
//! - Independent decode via 4byte signature lookup
//!
//! Supports nested calls (MultiSend batches).

mod compare;
pub mod parser;
mod sourcify;
pub mod types;
pub mod ui;

use std::collections::HashSet;

pub use compare::compare_decodes;
pub use parser::{parse_initial, decode_with_signature, get_selector};
pub use sourcify::SignatureLookup;
pub use types::*;
pub use ui::{render_decode_section, render_single_comparison};

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

/// Bulk verify all transactions in a MultiSend batch
/// 
/// 1. Collects all unique selectors from transactions
/// 2. Batch fetches signatures from Sourcify (uses cache)
/// 3. Decodes each transaction locally
/// 4. Compares with API decode
/// 5. Updates summary
pub async fn verify_multisend_batch(
    multi: &mut MultiSendDecode,
    lookup: &SignatureLookup,
) {
    decode_log!("Starting bulk verification for {} transactions", multi.transactions.len());
    
    // 1. Collect unique selectors from all transactions with calldata
    let selectors: Vec<String> = multi.transactions
        .iter()
        .filter(|tx| tx.data.len() >= 10 && tx.data != "0x")
        .map(|tx| tx.data[..10].to_lowercase())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    
    decode_log!("Found {} unique selectors to lookup", selectors.len());
    
    // 2. Batch fetch from Sourcify (handles cache internally)
    let signatures = lookup.lookup_batch(&selectors).await;
    decode_log!("Fetched signatures for {} selectors", signatures.len());
    
    // 3. Decode each transaction
    for tx in &mut multi.transactions {
        // Skip empty calldata
        if tx.data.len() < 10 || tx.data == "0x" {
            decode_log!("TX #{}: skipping (no calldata)", tx.index);
            continue;
        }
        
        let selector = tx.data[..10].to_lowercase();
        
        // Get signatures for this selector
        let sigs = match signatures.get(&selector) {
            Some(s) if !s.is_empty() => s,
            _ => {
                decode_log!("TX #{}: no signatures found for {}", tx.index, selector);
                // No signatures available - mark as unavailable
                tx.decode = Some(SingleDecode {
                    api: tx.api_decode.clone(),
                    local: None,
                    comparison: if tx.api_decode.is_some() {
                        ComparisonResult::OnlyApi
                    } else {
                        ComparisonResult::Failed("No signature found".to_string())
                    },
                });
                continue;
            }
        };
        
        decode_log!("TX #{}: trying {} signatures for {}", tx.index, sigs.len(), selector);
        
        // Try each signature until one decodes successfully
        let mut local_decode = None;
        for sig in sigs {
            match parser::decode_with_signature(&tx.data, sig) {
                Ok(decoded) => {
                    decode_log!("TX #{}: decoded with {}", tx.index, sig);
                    local_decode = Some(decoded);
                    break;
                }
                Err(e) => {
                    decode_log!("TX #{}: failed to decode with {}: {}", tx.index, sig, e);
                }
            }
        }
        
        // 4. Compare with API decode
        let comparison = compare::compare_decodes(tx.api_decode.as_ref(), local_decode.as_ref());
        decode_log!("TX #{}: comparison result: {:?}", tx.index, comparison);
        
        tx.decode = Some(SingleDecode {
            api: tx.api_decode.clone(),
            local: local_decode,
            comparison,
        });
    }
    
    // 5. Update summary and mark complete
    multi.summary.update(&multi.transactions);
    multi.verification_state = VerificationState::Complete;
    
    decode_log!("Bulk verification complete: {} verified, {} mismatched, {} pending",
        multi.summary.verified, multi.summary.mismatched, multi.summary.pending);
}

