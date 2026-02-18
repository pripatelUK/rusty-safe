use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use alloy::primitives::{keccak256, Address, Bytes, B256};
use serde::Serialize;
use serde_json::{json, Value};

use rusty_safe_signing_core::{PendingSafeTx, PortError, SafeServicePort};

use crate::SigningAdapterConfig;

#[derive(Debug, Clone)]
pub struct SafeServiceAdapter {
    mode: SafeServiceMode,
}

#[derive(Debug, Clone)]
enum SafeServiceMode {
    InMemory(Arc<Mutex<SafeServiceState>>),
    #[cfg(not(target_arch = "wasm32"))]
    Http(HttpSafeServiceRuntime),
}

#[derive(Debug, Default)]
struct SafeServiceState {
    txs: HashMap<B256, RemoteTxState>,
}

#[derive(Debug, Clone)]
struct RemoteTxState {
    chain_id: u64,
    safe_address: Address,
    proposed: bool,
    confirmations: Vec<Bytes>,
    executed_tx_hash: Option<B256>,
}

#[derive(Debug, Clone)]
#[cfg(not(target_arch = "wasm32"))]
struct HttpSafeServiceRuntime {
    base_url: String,
    retry_count: u32,
    client: reqwest::blocking::Client,
}

#[derive(Debug, Serialize)]
struct SafeTxServiceRequest {
    to: String,
    value: String,
    data: String,
    operation: u8,
    #[serde(rename = "safeTxGas")]
    safe_tx_gas: String,
    #[serde(rename = "baseGas")]
    base_gas: String,
    #[serde(rename = "gasPrice")]
    gas_price: String,
    #[serde(rename = "gasToken")]
    gas_token: String,
    #[serde(rename = "refundReceiver")]
    refund_receiver: String,
    nonce: String,
    #[serde(rename = "contractTransactionHash")]
    contract_transaction_hash: String,
    sender: String,
    signature: String,
    origin: String,
}

impl Default for SafeServiceAdapter {
    fn default() -> Self {
        Self::with_config(SigningAdapterConfig::from_env())
    }
}

impl SafeServiceAdapter {
    pub fn in_memory() -> Self {
        Self {
            mode: SafeServiceMode::InMemory(Arc::new(Mutex::new(SafeServiceState::default()))),
        }
    }

    pub fn with_config(config: SigningAdapterConfig) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if config.safe_service_http_enabled {
                let timeout = std::time::Duration::from_millis(config.safe_service_timeout_ms);
                if let Ok(client) = reqwest::blocking::Client::builder()
                    .timeout(timeout)
                    .build()
                {
                    return Self {
                        mode: SafeServiceMode::Http(HttpSafeServiceRuntime {
                            base_url: config
                                .safe_service_base_url
                                .trim_end_matches('/')
                                .to_owned(),
                            retry_count: config.safe_service_retry_count,
                            client,
                        }),
                    };
                }
            }
        }
        Self::in_memory()
    }

    fn with_state<T>(
        &self,
        f: impl FnOnce(&mut SafeServiceState) -> Result<T, PortError>,
    ) -> Result<T, PortError> {
        let mode = match &self.mode {
            SafeServiceMode::InMemory(state) => state,
            #[cfg(not(target_arch = "wasm32"))]
            SafeServiceMode::Http(_) => {
                return Err(PortError::NotImplemented("in-memory state unavailable"))
            }
        };
        let mut g = mode
            .lock()
            .map_err(|e| PortError::Transport(format!("safe service lock poisoned: {e}")))?;
        f(&mut g)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn call_http(
        runtime: &HttpSafeServiceRuntime,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
        idempotency_key: Option<&str>,
    ) -> Result<Value, PortError> {
        let url = format!("{}{}", runtime.base_url, path);
        let mut attempt = 0u32;
        loop {
            let mut req = runtime.client.request(method.clone(), &url);
            if let Some(key) = idempotency_key {
                req = req.header("X-Idempotency-Key", key);
            }
            if let Some(ref payload) = body {
                req = req.json(payload);
            }

            match req.send() {
                Ok(resp) => {
                    let status = resp.status();
                    let text = resp.text().map_err(|e| {
                        PortError::Transport(format!("safe service body read failed: {e}"))
                    })?;
                    let value: Value =
                        serde_json::from_str(&text).unwrap_or(Value::String(text.clone()));

                    if status.is_success() {
                        return Ok(value);
                    }

                    if status == reqwest::StatusCode::CONFLICT {
                        return Ok(value);
                    }

                    if status == reqwest::StatusCode::BAD_REQUEST {
                        let lowered = text.to_lowercase();
                        if lowered.contains("already exists")
                            || lowered.contains("exists")
                            || lowered.contains("duplicated")
                        {
                            return Ok(value);
                        }
                    }

                    let retryable = matches!(
                        status,
                        reqwest::StatusCode::TOO_MANY_REQUESTS
                            | reqwest::StatusCode::BAD_GATEWAY
                            | reqwest::StatusCode::SERVICE_UNAVAILABLE
                            | reqwest::StatusCode::GATEWAY_TIMEOUT
                            | reqwest::StatusCode::INTERNAL_SERVER_ERROR
                    );
                    if retryable && attempt < runtime.retry_count {
                        attempt = attempt.saturating_add(1);
                        std::thread::sleep(std::time::Duration::from_millis(
                            100u64.saturating_mul(1u64 << attempt.min(4)),
                        ));
                        continue;
                    }
                    return Err(PortError::Transport(format!(
                        "safe service {} {} failed with status {}: {}",
                        method, path, status, value
                    )));
                }
                Err(e) => {
                    if attempt < runtime.retry_count {
                        attempt = attempt.saturating_add(1);
                        std::thread::sleep(std::time::Duration::from_millis(
                            100u64.saturating_mul(1u64 << attempt.min(4)),
                        ));
                        continue;
                    }
                    return Err(PortError::Transport(format!(
                        "safe service request {} {} failed: {e}",
                        method, path
                    )));
                }
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn to_service_request(tx: &PendingSafeTx) -> Result<SafeTxServiceRequest, PortError> {
        let payload = &tx.payload;
        let to = get_address_string(payload, "to", tx.safe_address)?;
        let value = get_numeric_string(payload, "value", "0")?;
        let data = get_hex_string(payload, "data", "0x");
        let operation = payload
            .get("operation")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            .min(u8::MAX as u64) as u8;
        let safe_tx_gas = get_numeric_string(payload, "safeTxGas", "0")?;
        let base_gas = get_numeric_string(payload, "baseGas", "0")?;
        let gas_price = get_numeric_string(payload, "gasPrice", "0")?;
        let gas_token = get_address_string(payload, "gasToken", Address::ZERO)?;
        let refund_receiver = get_address_string(payload, "refundReceiver", Address::ZERO)?;
        let (sender, signature) = tx
            .signatures
            .first()
            .map(|s| (s.signer.to_string(), s.signature.to_string()))
            .unwrap_or_else(|| (Address::ZERO.to_string(), "0x".to_owned()));

        Ok(SafeTxServiceRequest {
            to,
            value,
            data,
            operation,
            safe_tx_gas,
            base_gas,
            gas_price,
            gas_token,
            refund_receiver,
            nonce: tx.nonce.to_string(),
            contract_transaction_hash: tx.safe_tx_hash.to_string(),
            sender,
            signature,
            origin: "rusty-safe".to_owned(),
        })
    }
}

impl SafeServicePort for SafeServiceAdapter {
    fn propose_tx(&self, tx: &PendingSafeTx) -> Result<(), PortError> {
        #[cfg(not(target_arch = "wasm32"))]
        if let SafeServiceMode::Http(runtime) = &self.mode {
            let request = Self::to_service_request(tx)?;
            let path = format!("/api/v1/safes/{}/multisig-transactions/", tx.safe_address);
            let body = serde_json::to_value(request).map_err(|e| {
                PortError::Validation(format!("serialize propose request failed: {e}"))
            })?;
            let _ = Self::call_http(
                runtime,
                reqwest::Method::POST,
                &path,
                Some(body),
                Some(&tx.idempotency_key),
            )?;
            return Ok(());
        }

        self.with_state(|g| {
            g.txs
                .entry(tx.safe_tx_hash)
                .and_modify(|state| state.proposed = true)
                .or_insert_with(|| RemoteTxState {
                    chain_id: tx.chain_id,
                    safe_address: tx.safe_address,
                    proposed: true,
                    confirmations: Vec::new(),
                    executed_tx_hash: None,
                });
            Ok(())
        })
    }

    fn confirm_tx(&self, safe_tx_hash: B256, signature: &Bytes) -> Result<(), PortError> {
        if signature.len() < 65 {
            return Err(PortError::Validation("INVALID_SIGNATURE_FORMAT".to_owned()));
        }

        #[cfg(not(target_arch = "wasm32"))]
        if let SafeServiceMode::Http(runtime) = &self.mode {
            let path = format!("/api/v1/multisig-transactions/{safe_tx_hash}/confirmations/");
            let body = json!({ "signature": signature.to_string() });
            let _ = Self::call_http(
                runtime,
                reqwest::Method::POST,
                &path,
                Some(body),
                Some(&format!("confirm-{safe_tx_hash}")),
            )?;
            return Ok(());
        }

        self.with_state(|g| {
            let state = g.txs.get_mut(&safe_tx_hash).ok_or_else(|| {
                PortError::NotFound(format!("remote tx not found: {safe_tx_hash}"))
            })?;
            if !state.proposed {
                return Err(PortError::Conflict(
                    "cannot confirm tx before propose".to_owned(),
                ));
            }
            if !state.confirmations.iter().any(|sig| sig == signature) {
                state.confirmations.push(signature.clone());
            }
            Ok(())
        })
    }

    fn execute_tx(&self, tx: &PendingSafeTx) -> Result<B256, PortError> {
        #[cfg(not(target_arch = "wasm32"))]
        if let SafeServiceMode::Http(runtime) = &self.mode {
            let path = format!("/api/v1/multisig-transactions/{}/", tx.safe_tx_hash);
            let status = Self::call_http(runtime, reqwest::Method::GET, &path, None, None)?;
            if let Some(hash) = status
                .get("transactionHash")
                .and_then(|v| v.as_str())
                .or_else(|| status.get("txHash").and_then(|v| v.as_str()))
                .or_else(|| status.get("ethereumTxHash").and_then(|v| v.as_str()))
            {
                let parsed = hash.parse().map_err(|e| {
                    PortError::Validation(format!("invalid execution tx hash from service: {e}"))
                })?;
                return Ok(parsed);
            }
            return Err(PortError::Policy(
                "SAFE_TX_NOT_EXECUTED_REMOTE; execute on-chain via wallet provider".to_owned(),
            ));
        }

        self.with_state(|g| {
            let state = g
                .txs
                .entry(tx.safe_tx_hash)
                .or_insert_with(|| RemoteTxState {
                    chain_id: tx.chain_id,
                    safe_address: tx.safe_address,
                    proposed: true,
                    confirmations: Vec::new(),
                    executed_tx_hash: None,
                });
            if state.executed_tx_hash.is_none() {
                let seed = format!("{}:{}:{}", tx.chain_id, tx.safe_address, tx.safe_tx_hash);
                state.executed_tx_hash = Some(keccak256(seed.as_bytes()));
            }
            Ok(state.executed_tx_hash.expect("set above"))
        })
    }

    fn fetch_status(&self, safe_tx_hash: B256) -> Result<Value, PortError> {
        #[cfg(not(target_arch = "wasm32"))]
        if let SafeServiceMode::Http(runtime) = &self.mode {
            let path = format!("/api/v1/multisig-transactions/{safe_tx_hash}/");
            return Self::call_http(runtime, reqwest::Method::GET, &path, None, None);
        }

        self.with_state(|g| {
            let state = g.txs.get(&safe_tx_hash).ok_or_else(|| {
                PortError::NotFound(format!("remote tx not found: {safe_tx_hash}"))
            })?;
            Ok(json!({
                "safeTxHash": safe_tx_hash,
                "chainId": state.chain_id,
                "safeAddress": state.safe_address,
                "proposed": state.proposed,
                "confirmations": state.confirmations.len(),
                "executedTxHash": state.executed_tx_hash,
            }))
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn get_numeric_string(payload: &Value, key: &str, fallback: &str) -> Result<String, PortError> {
    if let Some(v) = payload.get(key) {
        if let Some(s) = v.as_str() {
            return Ok(s.to_owned());
        }
        if let Some(n) = v.as_u64() {
            return Ok(n.to_string());
        }
        return Err(PortError::Validation(format!(
            "payload.{key} must be string/number"
        )));
    }
    Ok(fallback.to_owned())
}

#[cfg(not(target_arch = "wasm32"))]
fn get_hex_string(payload: &Value, key: &str, fallback: &str) -> String {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| {
            if s.starts_with("0x") {
                s.to_owned()
            } else {
                format!("0x{s}")
            }
        })
        .unwrap_or_else(|| fallback.to_owned())
}

#[cfg(not(target_arch = "wasm32"))]
fn get_address_string(payload: &Value, key: &str, fallback: Address) -> Result<String, PortError> {
    if let Some(v) = payload.get(key) {
        let raw = v.as_str().ok_or_else(|| {
            PortError::Validation(format!("payload.{key} must be address string"))
        })?;
        let parsed: Address = raw
            .parse()
            .map_err(|e| PortError::Validation(format!("invalid payload.{key} address: {e}")))?;
        return Ok(parsed.to_string());
    }
    Ok(fallback.to_string())
}
