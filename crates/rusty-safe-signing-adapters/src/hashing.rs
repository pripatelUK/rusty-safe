use alloy::primitives::{keccak256, Address, B256, U256};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;

use rusty_safe_signing_core::{HashingPort, MessageMethod, PortError};
use safe_hash::{tx_signing_hashes, TxInput};
use safe_utils::{DomainHasher, MessageHasher, SafeHasher, SafeWalletVersion};

#[derive(Debug, Clone, Default)]
pub struct HashingAdapter;

impl HashingPort for HashingAdapter {
    fn safe_tx_hash(
        &self,
        chain_id: u64,
        safe_address: Address,
        nonce: u64,
        payload: &Value,
    ) -> Result<B256, PortError> {
        if let Ok(hash) = safe_tx_hash_via_safe_hash(chain_id, safe_address, nonce, payload) {
            return Ok(hash);
        }

        // Fallback keeps deterministic behavior for incomplete drafts.
        let canonical = canonical_json_bytes(payload)?;
        let mut bytes = Vec::with_capacity(128 + canonical.len());
        bytes.extend_from_slice(&chain_id.to_be_bytes());
        bytes.extend_from_slice(safe_address.as_slice());
        bytes.extend_from_slice(&nonce.to_be_bytes());
        bytes.extend_from_slice(&canonical);
        Ok(keccak256(bytes))
    }

    fn message_hash(
        &self,
        chain_id: u64,
        safe_address: Address,
        _method: MessageMethod,
        payload: &Value,
    ) -> Result<B256, PortError> {
        let safe_version = payload
            .get("safeVersion")
            .and_then(|v| v.as_str())
            .unwrap_or("1.3.0");
        let safe_version = SafeWalletVersion::parse(safe_version)
            .map_err(|e| PortError::Validation(format!("invalid safeVersion: {e}")))?;
        let domain_hash = DomainHasher::new(safe_version, chain_id, safe_address).hash();

        let message_string = extract_message(payload);
        let message_hash = MessageHasher::new(message_string).hash();
        Ok(SafeHasher::new(domain_hash, message_hash).hash())
    }

    fn integrity_mac(&self, payload: &[u8], key_id: &str) -> Result<B256, PortError> {
        type HmacSha256 = Hmac<Sha256>;
        let secret = std::env::var("RUSTY_SAFE_MAC_SECRET")
            .unwrap_or_else(|_| "rusty-safe-mac-dev-secret".to_owned());
        let hk = Hkdf::<Sha256>::new(None, secret.as_bytes());
        let mut mac_key = [0u8; 32];
        hk.expand(key_id.as_bytes(), &mut mac_key).map_err(|_| {
            PortError::Validation("hkdf expand failed for integrity mac".to_owned())
        })?;
        let mut mac = <HmacSha256 as Mac>::new_from_slice(&mac_key)
            .map_err(|e| PortError::Validation(format!("hmac init failed: {e}")))?;
        mac.update(payload);
        let out = mac.finalize().into_bytes();
        Ok(B256::from_slice(&out))
    }
}

fn safe_tx_hash_via_safe_hash(
    chain_id: u64,
    safe_address: Address,
    nonce: u64,
    payload: &Value,
) -> Result<B256, PortError> {
    let to = payload_address(payload, "to")?;
    let value = payload_u256(payload, "value")?;
    let data = payload_data(payload)?;
    let operation = payload
        .get("operation")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u8;
    let safe_tx_gas = payload_u256(payload, "safeTxGas").unwrap_or(U256::ZERO);
    let base_gas = payload_u256(payload, "baseGas").unwrap_or(U256::ZERO);
    let gas_price = payload_u256(payload, "gasPrice").unwrap_or(U256::ZERO);
    let gas_token = payload_address(payload, "gasToken").unwrap_or(Address::ZERO);
    let refund_receiver = payload_address(payload, "refundReceiver").unwrap_or(Address::ZERO);
    let safe_version = payload
        .get("safeVersion")
        .and_then(|v| v.as_str())
        .unwrap_or("1.3.0");
    let safe_version = SafeWalletVersion::parse(safe_version)
        .map_err(|e| PortError::Validation(format!("invalid safeVersion: {e}")))?;

    let tx = TxInput::new(
        to,
        value,
        data,
        operation,
        safe_tx_gas,
        base_gas,
        gas_price,
        gas_token,
        refund_receiver,
        String::new(),
    );

    Ok(tx_signing_hashes(&tx, safe_address, nonce, chain_id, safe_version).safe_tx_hash)
}

fn payload_u256(payload: &Value, key: &str) -> Result<U256, PortError> {
    let value = payload
        .get(key)
        .ok_or_else(|| PortError::Validation(format!("missing payload.{key}")))?;
    match value {
        Value::String(s) => parse_u256(s),
        Value::Number(num) => parse_u256(&num.to_string()),
        _ => Err(PortError::Validation(format!(
            "invalid numeric payload.{key}"
        ))),
    }
}

fn payload_address(payload: &Value, key: &str) -> Result<Address, PortError> {
    let s = payload
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| PortError::Validation(format!("missing payload.{key}")))?;
    s.parse()
        .map_err(|e| PortError::Validation(format!("invalid payload.{key}: {e}")))
}

fn payload_data(payload: &Value) -> Result<String, PortError> {
    let raw = payload
        .get("data")
        .and_then(|v| v.as_str())
        .ok_or_else(|| PortError::Validation("missing payload.data".to_owned()))?;
    if raw.starts_with("0x") {
        return Ok(raw.to_owned());
    }
    Ok(format!("0x{raw}"))
}

fn parse_u256(raw: &str) -> Result<U256, PortError> {
    if raw.starts_with("0x") || raw.starts_with("0X") {
        U256::from_str_radix(raw.trim_start_matches("0x").trim_start_matches("0X"), 16)
            .map_err(|e| PortError::Validation(format!("invalid hex integer: {e}")))
    } else {
        raw.parse()
            .map_err(|e| PortError::Validation(format!("invalid integer: {e}")))
    }
}

fn extract_message(payload: &Value) -> String {
    if let Some(msg) = payload.get("message").and_then(|v| v.as_str()) {
        return msg.replace("\r\n", "\n");
    }
    if let Some(msg) = payload.as_str() {
        return msg.replace("\r\n", "\n");
    }
    serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_owned())
}

fn canonical_json_bytes(value: &Value) -> Result<Vec<u8>, PortError> {
    let s = serde_json::to_string(value)
        .map_err(|e| PortError::Validation(format!("serialize: {e}")))?;
    Ok(s.into_bytes())
}
