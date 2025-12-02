//! Hash computation - uses safe_hash library

use crate::api::{SafeTransaction, TxInput, tx_signing_hashes, check_suspicious_content, get_safe_transaction_async, validate_safe_tx_hash};
use crate::state::ComputedHashes;
use alloy::primitives::{Address, FixedBytes, ChainId, U256, hex};
use safe_hash::{SafeHashes, SafeWarnings, Mismatch};
use safe_utils::{Of, SafeWalletVersion};

/// Fetch transaction from Safe API (async - works on WASM)
pub async fn fetch_transaction(
    chain_name: &str,
    safe_address: &str,
    nonce: u64,
) -> Result<SafeTransaction, String> {
    let chain_id = ChainId::of(chain_name)
        .map_err(|e| format!("Invalid chain '{}': {}", chain_name, e))?;
    
    let addr: Address = safe_address.trim().parse()
        .map_err(|e| format!("Invalid Safe address: {}", e))?;
    
    get_safe_transaction_async(chain_id, addr, nonce).await
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
) -> Result<ComputedHashes, String> {
    let chain_id = ChainId::of(chain_name)
        .map_err(|e| format!("Invalid chain '{}': {}", chain_name, e))?;

    let safe_version = SafeWalletVersion::parse(version)
        .map_err(|e| format!("Invalid version: {}", e))?;

    let safe_addr: Address = safe_address
        .trim()
        .parse()
        .map_err(|e| format!("Invalid Safe address: {}", e))?;

    let to_addr: Address = to
        .trim()
        .parse()
        .map_err(|e| format!("Invalid 'to' address: {}", e))?;

    let value_u256 = parse_u256(value)?;
    let safe_tx_gas_u256 = parse_u256(safe_tx_gas)?;
    let base_gas_u256 = parse_u256(base_gas)?;
    let gas_price_u256 = parse_u256(gas_price)?;
    let nonce_u64: u64 = nonce.trim().parse().map_err(|e| format!("Invalid nonce: {}", e))?;

    let gas_token_addr: Address = gas_token
        .trim()
        .parse()
        .map_err(|e| format!("Invalid gas token address: {}", e))?;

    let refund_receiver_addr: Address = refund_receiver
        .trim()
        .parse()
        .map_err(|e| format!("Invalid refund receiver address: {}", e))?;

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
) -> Result<(ComputedHashes, Option<Mismatch>), String> {
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
        .map_err(|e| format!("Failed to decode computed hash: {}", e))?;
    let computed_fixed: FixedBytes<32> = FixedBytes::from_slice(&computed_hash_bytes);
    
    let mismatch = match validate_safe_tx_hash(tx, &computed_fixed) {
        Ok(()) => None,
        Err(m) => Some(m),
    };

    let mut final_hashes = hashes;
    final_hashes.matches_api = Some(mismatch.is_none());

    Ok((final_hashes, mismatch))
}

fn parse_u256(value: &str) -> Result<U256, String> {
    let value = value.trim();
    if value.is_empty() || value == "0" {
        return Ok(U256::ZERO);
    }
    if value.starts_with("0x") || value.starts_with("0X") {
        U256::from_str_radix(&value[2..], 16)
            .map_err(|e| format!("Invalid hex value '{}': {}", value, e))
    } else {
        value
            .parse()
            .map_err(|e| format!("Invalid value '{}': {}", value, e))
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
