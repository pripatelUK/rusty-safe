//! Hash computation - uses safe_hash library

use crate::api::{SafeTransaction, TxInput, tx_signing_hashes, check_suspicious_content, get_safe_transaction_async, validate_safe_tx_hash};
use crate::state::ComputedHashes;
use alloy::primitives::{Address, FixedBytes, ChainId, U256, hex};
use eyre::{Result, WrapErr};
use safe_hash::{SafeHashes, SafeWarnings, Mismatch};
use safe_utils::{get_safe_api, Of, SafeWalletVersion};
use serde::Deserialize;

/// Safe info response from API
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeInfo {
    pub address: Address,
    #[serde(deserialize_with = "deserialize_string_to_u64")]
    pub nonce: u64,
    pub threshold: u64,
    pub owners: Vec<Address>,
    pub modules: Vec<Address>,
    pub version: String,
}

/// Deserialize a string number to u64
fn deserialize_string_to_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let s: String = String::deserialize(deserializer)?;
    s.parse().map_err(|_| D::Error::custom(format!("Failed to parse '{}' as u64", s)))
}

/// Fetch Safe info from API (async - works on WASM)
pub async fn fetch_safe_info(
    chain_name: &str,
    safe_address: &str,
) -> Result<SafeInfo> {
    let chain_id = ChainId::of(chain_name)
        .map_err(|e| eyre::eyre!("Invalid chain '{}': {}", chain_name, e))?;
    
    let addr: Address = safe_address.trim().parse()
        .wrap_err("Invalid Safe address")?;
    
    let api_url = get_safe_api(chain_id)
        .map_err(|e| eyre::eyre!("Failed to get API URL: {}", e))?;
    
    let url = format!("{}/api/v1/safes/{}/", api_url, addr);
    
    let response = reqwest::get(&url)
        .await
        .wrap_err("Network error")?;
    
    if !response.status().is_success() {
        eyre::bail!("API error: {}", response.status());
    }
    
    let safe_info: SafeInfo = response
        .json()
        .await
        .wrap_err("Failed to parse Safe info")?;
    
    Ok(safe_info)
}

/// Fetch transaction from Safe API (async - works on WASM)
pub async fn fetch_transaction(
    chain_name: &str,
    safe_address: &str,
    nonce: u64,
) -> Result<SafeTransaction> {
    let chain_id = ChainId::of(chain_name)
        .map_err(|e| eyre::eyre!("Invalid chain '{}': {}", chain_name, e))?;
    
    let addr: Address = safe_address.trim().parse()
        .wrap_err("Invalid Safe address")?;
    
    get_safe_transaction_async(chain_id, addr, nonce)
        .await
        .map_err(|e| eyre::eyre!(e))
}

/// Compute hashes for a transaction using safe_hash::tx_signing_hashes
pub fn compute_hashes(
    chain_name: &str,
    safe_address: &str,
    version: &str,
    to: &str,
    value: &str,
    data: &str,
    operation: u8,
    safe_tx_gas: &str,
    base_gas: &str,
    gas_price: &str,
    gas_token: &str,
    refund_receiver: &str,
    nonce: &str,
) -> Result<ComputedHashes> {
    let chain_id = ChainId::of(chain_name)
        .map_err(|e| eyre::eyre!("Invalid chain '{}': {}", chain_name, e))?;

    let safe_version = SafeWalletVersion::parse(version)
        .map_err(|e| eyre::eyre!("Invalid Safe version '{}': {}", version, e))?;

    let safe_addr: Address = safe_address
        .trim()
        .parse()
        .wrap_err("Invalid Safe address")?;

    let to_addr: Address = to
        .trim()
        .parse()
        .wrap_err("Invalid 'to' address")?;

    let value_u256 = parse_u256(value).wrap_err("Invalid value")?;
    let safe_tx_gas_u256 = parse_u256(safe_tx_gas).wrap_err("Invalid safeTxGas")?;
    let base_gas_u256 = parse_u256(base_gas).wrap_err("Invalid baseGas")?;
    let gas_price_u256 = parse_u256(gas_price).wrap_err("Invalid gasPrice")?;
    let nonce_u64: u64 = nonce.trim().parse().wrap_err("Invalid nonce")?;

    let gas_token_addr: Address = gas_token
        .trim()
        .parse()
        .wrap_err("Invalid gas token address")?;

    let refund_receiver_addr: Address = refund_receiver
        .trim()
        .parse()
        .wrap_err("Invalid refund receiver address")?;

    // Normalize data - remove 0x prefix if present
    let data_normalized = data.strip_prefix("0x").unwrap_or(data);
    let data_with_prefix = if data_normalized.is_empty() {
        "0x".to_string()
    } else {
        format!("0x{}", data_normalized)
    };

    // Create TxInput for safe_hash::tx_signing_hashes
    let tx_input = TxInput::new(
        to_addr,
        value_u256,
        data_with_prefix,
        operation,
        safe_tx_gas_u256,
        base_gas_u256,
        gas_price_u256,
        gas_token_addr,
        refund_receiver_addr,
        String::new(), // signatures not needed for hash computation
    );

    // Use safe_hash::tx_signing_hashes
    let hashes: SafeHashes = tx_signing_hashes(
        &tx_input,
        safe_addr,
        nonce_u64,
        chain_id,
        safe_version,
    );

    Ok(ComputedHashes {
        domain_hash: format!("0x{}", hex::encode(hashes.domain_hash)),
        message_hash: format!("0x{}", hex::encode(hashes.message_hash)),
        safe_tx_hash: format!("0x{}", hex::encode(hashes.safe_tx_hash)),
        matches_api: None,
    })
}

/// Compute hashes from a SafeTransaction (fetched from API)
/// Returns (hashes, optional_mismatch)
pub fn compute_hashes_from_api_tx(
    chain_name: &str,
    safe_address: &str,
    version: &str,
    tx: &SafeTransaction,
) -> Result<(ComputedHashes, Option<Mismatch>)> {
    let hashes = compute_hashes(
        chain_name,
        safe_address,
        version,
        &format!("{}", tx.to),
        &tx.value,
        &tx.data,
        tx.operation,
        &tx.safe_tx_gas.to_string(),
        &tx.base_gas.to_string(),
        &tx.gas_price,
        &format!("{}", tx.gas_token),
        &format!("{}", tx.refund_receiver),
        &tx.nonce.to_string(),
    )?;

    // Use validate_safe_tx_hash from safe-hash
    let computed_hash_bytes = hex::decode(hashes.safe_tx_hash.strip_prefix("0x").unwrap_or(&hashes.safe_tx_hash))
        .wrap_err("Failed to decode computed hash")?;
    let computed_fixed: FixedBytes<32> = FixedBytes::from_slice(&computed_hash_bytes);
    
    let mismatch = match validate_safe_tx_hash(tx, &computed_fixed) {
        Ok(()) => None,
        Err(m) => Some(m),
    };

    let mut final_hashes = hashes;
    final_hashes.matches_api = Some(mismatch.is_none());

    Ok((final_hashes, mismatch))
}

fn parse_u256(value: &str) -> Result<U256> {
    let value = value.trim();
    if value.is_empty() || value == "0" {
        return Ok(U256::ZERO);
    }
    if value.starts_with("0x") || value.starts_with("0X") {
        U256::from_str_radix(&value[2..], 16)
            .wrap_err_with(|| format!("Invalid hex value '{}'", value))
    } else {
        value
            .parse()
            .wrap_err_with(|| format!("Invalid value '{}'", value))
    }
}

/// Generate warnings using safe_hash::check_suspicious_content
pub fn get_warnings_for_tx(
    to: &str,
    value: &str,
    data: &str,
    operation: u8,
    safe_tx_gas: &str,
    base_gas: &str,
    gas_price: &str,
    gas_token: &str,
    refund_receiver: &str,
) -> SafeWarnings {
    let to_addr: Address = to.trim().parse().unwrap_or(Address::ZERO);
    let value_u256 = parse_u256(value).unwrap_or(U256::ZERO);
    let safe_tx_gas_u256 = parse_u256(safe_tx_gas).unwrap_or(U256::ZERO);
    let base_gas_u256 = parse_u256(base_gas).unwrap_or(U256::ZERO);
    let gas_price_u256 = parse_u256(gas_price).unwrap_or(U256::ZERO);
    let gas_token_addr: Address = gas_token.trim().parse().unwrap_or(Address::ZERO);
    let refund_receiver_addr: Address = refund_receiver.trim().parse().unwrap_or(Address::ZERO);

    let data_normalized = data.strip_prefix("0x").unwrap_or(data);
    let data_with_prefix = if data_normalized.is_empty() {
        "0x".to_string()
    } else {
        format!("0x{}", data_normalized)
    };

    let tx_input = TxInput::new(
        to_addr,
        value_u256,
        data_with_prefix,
        operation,
        safe_tx_gas_u256,
        base_gas_u256,
        gas_price_u256,
        gas_token_addr,
        refund_receiver_addr,
        String::new(),
    );

    check_suspicious_content(&tx_input, None)
}

/// Generate warnings from a SafeTransaction (from API)
pub fn get_warnings_from_api_tx(tx: &SafeTransaction, chain_id: Option<ChainId>) -> SafeWarnings {
    let tx_input = TxInput::new(
        tx.to,
        U256::from_str_radix(&tx.value, 10).unwrap_or(U256::ZERO),
        tx.data.clone(),
        tx.operation,
        U256::from(tx.safe_tx_gas),
        U256::from(tx.base_gas),
        U256::from_str_radix(&tx.gas_price, 10).unwrap_or(U256::ZERO),
        tx.gas_token,
        tx.refund_receiver,
        String::new(),
    );

    let mut warnings = check_suspicious_content(&tx_input, chain_id);

    // Check for dangerous methods from decoded data
    if let Some(decoded) = &tx.data_decoded {
        let dangerous_methods = ["addOwnerWithThreshold", "removeOwner", "swapOwner", "changeThreshold"];
        if dangerous_methods.iter().any(|m| *m == decoded.method) {
            warnings.dangerous_methods = true;
        }
    }

    warnings
}
