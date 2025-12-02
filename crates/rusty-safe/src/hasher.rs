//! Hash computation - direct use of safe_utils hashers

use crate::api::SafeTransaction;
use crate::state::{ComputedHashes, Warning};
use alloy::primitives::{Address, ChainId, U256};
use safe_utils::{CallDataHasher, DomainHasher, Of, SafeHasher, SafeWalletVersion, TxMessageHasher};

/// Compute hashes for a transaction using safe_utils (offline mode)
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
    let nonce_u256 = parse_u256(nonce)?;

    let gas_token_addr: Address = gas_token
        .trim()
        .parse()
        .map_err(|e| format!("Invalid gas token address: {}", e))?;

    let refund_receiver_addr: Address = refund_receiver
        .trim()
        .parse()
        .map_err(|e| format!("Invalid refund receiver address: {}", e))?;

    // Use safe_utils::CallDataHasher
    let data_clean = data.strip_prefix("0x").unwrap_or(data);
    let data_hash = CallDataHasher::new(data_clean.to_string())
        .hash()
        .map_err(|e| format!("Failed to hash data: {}", e))?;

    // Use safe_utils::DomainHasher
    let domain_hasher = DomainHasher::new(safe_version.clone(), chain_id, safe_addr);
    let domain_hash = domain_hasher.hash();

    // Use safe_utils::TxMessageHasher
    let msg_hasher = TxMessageHasher::new(
        safe_version,
        to_addr,
        value_u256,
        data_hash,
        operation,
        safe_tx_gas_u256,
        base_gas_u256,
        gas_price_u256,
        gas_token_addr,
        refund_receiver_addr,
        nonce_u256,
    );
    let message_hash = msg_hasher.hash();

    // Use safe_utils::SafeHasher
    let safe_hasher = SafeHasher::new(domain_hash, message_hash);
    let safe_tx_hash = safe_hasher.hash();

    Ok(ComputedHashes {
        domain_hash: format!("{:?}", domain_hash),
        message_hash: format!("{:?}", message_hash),
        safe_tx_hash: format!("{:?}", safe_tx_hash),
        matches_api: None,
    })
}

/// Compute hashes from a SafeTransaction (fetched from API)
pub fn compute_hashes_from_api_tx(
    chain_name: &str,
    safe_address: &str,
    version: &str,
    tx: &SafeTransaction,
) -> Result<ComputedHashes, String> {
    let data = tx.data.as_deref().unwrap_or("");
    
    let mut hashes = compute_hashes(
        chain_name,
        safe_address,
        version,
        &format!("{}", tx.to),
        &tx.value,
        data,
        tx.operation,
        &tx.safe_tx_gas.to_string(),
        &tx.base_gas.to_string(),
        &tx.gas_price,
        &format!("{}", tx.gas_token),
        &format!("{}", tx.refund_receiver),
        &tx.nonce.to_string(),
    )?;

    // Compare with API hash
    let api_hash = tx.safe_tx_hash.to_lowercase();
    let computed_hash = hashes.safe_tx_hash.to_lowercase();
    hashes.matches_api = Some(api_hash == computed_hash);

    Ok(hashes)
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

/// Generate warnings for a transaction (from strings - offline mode)
pub fn check_warnings(
    _to: &str,
    _value: &str,
    _data: &str,
    operation: u8,
    gas_token: &str,
    refund_receiver: &str,
) -> Vec<Warning> {
    let mut warnings = Vec::new();
    let zero_addr = "0x0000000000000000000000000000000000000000";

    if operation == 1 {
        warnings.push(Warning::DelegateCall);
    }

    if gas_token != zero_addr && !gas_token.is_empty() {
        warnings.push(Warning::NonZeroGasToken);
    }

    if refund_receiver != zero_addr && !refund_receiver.is_empty() {
        warnings.push(Warning::NonZeroRefundReceiver);
    }

    warnings
}

/// Generate warnings from a SafeTransaction (from API)
pub fn check_warnings_from_api_tx(tx: &SafeTransaction) -> Vec<Warning> {
    let mut warnings = Vec::new();

    if tx.operation == 1 {
        warnings.push(Warning::DelegateCall);
    }

    if tx.gas_token != Address::ZERO {
        warnings.push(Warning::NonZeroGasToken);
    }

    if tx.refund_receiver != Address::ZERO {
        warnings.push(Warning::NonZeroRefundReceiver);
    }

    // Check for dangerous methods from decoded data
    if let Some(decoded) = &tx.data_decoded {
        let method = decoded.method.as_str();
        if matches!(
            method,
            "addOwnerWithThreshold" | "removeOwner" | "swapOwner" | "changeThreshold"
        ) {
            warnings.push(Warning::DangerousMethod(method.to_string()));
        }
    }

    warnings
}
