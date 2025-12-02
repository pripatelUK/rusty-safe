//! Safe Transaction Service API client
//! 
//! Uses safe_utils for chain/API mapping.
//! Struct definitions mirror safe-hash/src/api.rs (which is a binary, not importable).

use alloy::primitives::{Address, ChainId};
use safe_utils::{get_safe_api, Of};
use serde::Deserialize;

/// API response wrapper
#[derive(Debug, Deserialize)]
pub struct SafeApiResponse {
    pub count: u64,
    pub results: Vec<SafeTransaction>,
}

/// Transaction from Safe API - mirrors safe-hash/src/api.rs
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeTransaction {
    pub safe: Address,
    pub to: Address,
    pub value: String,
    #[serde(default)]
    pub data: Option<String>,
    pub data_decoded: Option<DataDecoded>,
    pub operation: u8,
    pub gas_token: Address,
    pub safe_tx_gas: u64,
    pub base_gas: u64,
    pub gas_price: String,
    pub refund_receiver: Address,
    pub nonce: u64,
    pub safe_tx_hash: String,
    pub confirmations_required: u64,
    #[serde(default)]
    pub confirmations: Vec<Confirmation>,
    pub is_executed: bool,
}

/// Decoded calldata
#[derive(Debug, Clone, Deserialize)]
pub struct DataDecoded {
    pub method: String,
    pub parameters: Vec<Parameter>,
}

/// Decoded parameter
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parameter {
    pub name: String,
    pub r#type: String,
    pub value: serde_json::Value,
}

/// Confirmation from a signer
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Confirmation {
    pub owner: Address,
    pub signature: String,
    pub signature_type: String,
}

/// Fetch transaction from Safe API
pub async fn fetch_transaction(
    chain_name: &str,
    safe_address: &str,
    nonce: u64,
) -> Result<SafeTransaction, String> {
    let chain_id = ChainId::of(chain_name)
        .map_err(|e| format!("Unsupported chain '{}': {}", chain_name, e))?;

    let api_base = get_safe_api(chain_id)
        .map_err(|e| format!("No API for chain {}: {}", chain_name, e))?;

    let url = format!(
        "{}/api/v1/safes/{}/multisig-transactions/?nonce={}",
        api_base, safe_address, nonce
    );

    let response = reqwest::get(&url)
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API error: {} - {}", response.status(), url));
    }

    let data: SafeApiResponse = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    if data.count == 0 {
        return Err(format!("No transaction found for nonce {}", nonce));
    }

    Ok(data.results.into_iter().next().unwrap())
}
