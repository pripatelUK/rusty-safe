use std::sync::{Arc, Mutex};

use alloy::primitives::{keccak256, Address, Bytes, B256};
use serde_json::Value;

use rusty_safe_signing_core::{
    MessageMethod, PortError, ProviderEvent, ProviderEventKind, ProviderPort,
};

use crate::SigningAdapterConfig;

#[derive(Debug, Clone)]
pub struct Eip1193Adapter {
    mode: ProviderMode,
    state: Arc<Mutex<ProviderState>>,
    #[cfg(target_arch = "wasm32")]
    hooks: Arc<Mutex<BrowserHooks>>,
}

#[derive(Debug, Clone)]
enum ProviderMode {
    Disabled(String),
    Deterministic,
    #[cfg(not(target_arch = "wasm32"))]
    Proxy(ProxyRuntime),
    #[cfg(target_arch = "wasm32")]
    Browser,
}

#[derive(Debug, Clone)]
#[cfg(not(target_arch = "wasm32"))]
struct ProxyRuntime {
    base_url: String,
    client: reqwest::blocking::Client,
}

#[derive(Debug, Clone)]
struct ProviderState {
    accounts: Vec<Address>,
    chain_id: u64,
    capabilities: Option<Value>,
    event_seq: u64,
    events: Vec<ProviderEvent>,
}

impl Default for ProviderState {
    fn default() -> Self {
        Self {
            accounts: vec!["0x1000000000000000000000000000000000000001"
                .parse()
                .expect("valid built-in deterministic account")],
            chain_id: 1,
            capabilities: Some(serde_json::json!({
                "wallet_getCapabilities": true,
                "signMethods": [
                    "personal_sign",
                    "eth_signTypedData",
                    "eth_signTypedData_v4",
                    "eth_sendTransaction"
                ],
                "note": "deterministic fallback adapter"
            })),
            event_seq: 0,
            events: Vec::new(),
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug)]
struct BrowserHooks {
    accounts_changed: Option<wasm_bindgen::closure::Closure<dyn FnMut(wasm_bindgen::JsValue)>>,
    chain_changed: Option<wasm_bindgen::closure::Closure<dyn FnMut(wasm_bindgen::JsValue)>>,
}

#[cfg(target_arch = "wasm32")]
impl Default for BrowserHooks {
    fn default() -> Self {
        Self {
            accounts_changed: None,
            chain_changed: None,
        }
    }
}

impl Default for Eip1193Adapter {
    fn default() -> Self {
        Self::with_config(SigningAdapterConfig::from_env())
    }
}

impl Eip1193Adapter {
    pub fn with_config(config: SigningAdapterConfig) -> Self {
        #[cfg(target_arch = "wasm32")]
        let mode = if browser_provider_available() {
            ProviderMode::Browser
        } else if config.strict_runtime_required() {
            ProviderMode::Disabled(
                "EIP-1193 browser provider not found in production runtime profile".to_owned(),
            )
        } else {
            ProviderMode::Deterministic
        };

        #[cfg(not(target_arch = "wasm32"))]
        let mode = if let Some(ref base_url) = config.eip1193_proxy_url {
            let timeout = std::time::Duration::from_millis(config.safe_service_timeout_ms);
            match reqwest::blocking::Client::builder()
                .timeout(timeout)
                .build()
            {
                Ok(client) => ProviderMode::Proxy(ProxyRuntime {
                    base_url: base_url.clone(),
                    client,
                }),
                Err(e) => {
                    if config.strict_runtime_required() {
                        ProviderMode::Disabled(format!(
                            "failed to initialize EIP-1193 proxy client in production profile: {e}"
                        ))
                    } else {
                        ProviderMode::Deterministic
                    }
                }
            }
        } else if config.strict_runtime_required() {
            ProviderMode::Disabled(
                "EIP-1193 proxy URL not configured in production runtime profile".to_owned(),
            )
        } else {
            ProviderMode::Deterministic
        };

        let adapter = Self {
            mode,
            state: Arc::new(Mutex::new(ProviderState::default())),
            #[cfg(target_arch = "wasm32")]
            hooks: Arc::new(Mutex::new(BrowserHooks::default())),
        };

        #[cfg(target_arch = "wasm32")]
        if matches!(adapter.mode, ProviderMode::Browser) {
            // Avoid eager provider.on(...) registration at startup.
            // In MetaMask 13.13.1 this can leave eth_requestAccounts pending with no notification popup
            // in our Chromium automation runtime; snapshot reads remain safe for initial state.
            let _ = adapter.refresh_browser_snapshot();
        }

        adapter
    }

    fn check_mode(&self) -> Result<(), PortError> {
        if let ProviderMode::Disabled(reason) = &self.mode {
            return Err(PortError::Policy(reason.clone()));
        }
        Ok(())
    }

    fn deterministic_signature(
        &self,
        method: MessageMethod,
        payload: &[u8],
        expected_signer: Address,
    ) -> Bytes {
        let mut seed = Vec::new();
        seed.extend_from_slice(format!("{method:?}").as_bytes());
        seed.extend_from_slice(expected_signer.as_slice());
        seed.extend_from_slice(payload);
        let hash = keccak256(seed);
        let mut sig = Vec::with_capacity(65);
        sig.extend_from_slice(hash.as_slice());
        sig.extend_from_slice(hash.as_slice());
        sig.push(27);
        Bytes::from(sig)
    }

    fn record_event(&self, kind: ProviderEventKind, value: String) -> Result<(), PortError> {
        let mut g = self
            .state
            .lock()
            .map_err(|e| PortError::Transport(format!("provider lock poisoned: {e}")))?;
        g.event_seq = g.event_seq.saturating_add(1);
        let seq = g.event_seq;
        g.events.push(ProviderEvent {
            sequence: seq,
            kind,
            value,
        });
        Ok(())
    }

    pub fn debug_inject_accounts_changed(&self, accounts: Vec<Address>) -> Result<(), PortError> {
        let payload = serde_json::json!(accounts.iter().map(|a| a.to_string()).collect::<Vec<_>>())
            .to_string();
        let mut g = self
            .state
            .lock()
            .map_err(|e| PortError::Transport(format!("provider lock poisoned: {e}")))?;
        g.accounts = accounts;
        g.event_seq = g.event_seq.saturating_add(1);
        let seq = g.event_seq;
        g.events.push(ProviderEvent {
            sequence: seq,
            kind: ProviderEventKind::AccountsChanged,
            value: payload,
        });
        Ok(())
    }

    pub fn debug_inject_chain_changed(&self, chain_id: u64) -> Result<(), PortError> {
        let mut g = self
            .state
            .lock()
            .map_err(|e| PortError::Transport(format!("provider lock poisoned: {e}")))?;
        g.chain_id = chain_id;
        g.event_seq = g.event_seq.saturating_add(1);
        let seq = g.event_seq;
        g.events.push(ProviderEvent {
            sequence: seq,
            kind: ProviderEventKind::ChainChanged,
            value: chain_id.to_string(),
        });
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn proxy_call(&self, method: &str, params: Value) -> Result<Value, PortError> {
        let proxy = match &self.mode {
            ProviderMode::Proxy(proxy) => proxy,
            ProviderMode::Disabled(reason) => return Err(PortError::Policy(reason.clone())),
            _ => {
                return Err(PortError::NotImplemented(
                    "eip1193 proxy runtime not enabled",
                ))
            }
        };

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });
        let response = proxy
            .client
            .post(&proxy.base_url)
            .json(&payload)
            .send()
            .map_err(|e| PortError::Transport(format!("eip1193 proxy request failed: {e}")))?;
        let status = response.status();
        let body: Value = response
            .json()
            .map_err(|e| PortError::Transport(format!("eip1193 proxy json decode failed: {e}")))?;
        if !status.is_success() {
            return Err(PortError::Transport(format!(
                "eip1193 proxy status {}: {}",
                status, body
            )));
        }
        if let Some(err) = body.get("error") {
            return Err(PortError::Transport(format!(
                "eip1193 proxy returned error: {err}"
            )));
        }
        body.get("result")
            .cloned()
            .ok_or_else(|| PortError::Transport("eip1193 proxy missing result".to_owned()))
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn wasm_request_accounts_async(&self) -> Result<Vec<Address>, PortError> {
        self.check_mode()?;
        let result = self
            .wasm_request("eth_requestAccounts", serde_json::json!([]))
            .await?;
        let arr = result.as_array().ok_or_else(|| {
            PortError::Transport("eth_requestAccounts result must be array".to_owned())
        })?;
        let mut accounts = Vec::with_capacity(arr.len());
        for item in arr {
            let raw = item.as_str().ok_or_else(|| {
                PortError::Transport("eth_requestAccounts item must be string".to_owned())
            })?;
            let parsed: Address = raw
                .parse()
                .map_err(|e| PortError::Validation(format!("invalid account: {e}")))?;
            accounts.push(parsed);
        }

        let serialized = serde_json::to_string(
            &accounts
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>(),
        )
        .unwrap_or_else(|_| "[]".to_owned());
        {
            let mut g = self
                .state
                .lock()
                .map_err(|e| PortError::Transport(format!("provider lock poisoned: {e}")))?;
            g.accounts = accounts.clone();
        }
        self.record_event(ProviderEventKind::AccountsChanged, serialized)?;
        Ok(accounts)
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn wasm_chain_id_async(&self) -> Result<u64, PortError> {
        self.check_mode()?;
        let result = self
            .wasm_request("eth_chainId", serde_json::json!([]))
            .await?;
        let chain_id = json_chain_id_to_u64(&result)?;
        {
            let mut g = self
                .state
                .lock()
                .map_err(|e| PortError::Transport(format!("provider lock poisoned: {e}")))?;
            g.chain_id = chain_id;
        }
        self.record_event(ProviderEventKind::ChainChanged, chain_id.to_string())?;
        Ok(chain_id)
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn wasm_wallet_get_capabilities_async(&self) -> Result<Option<Value>, PortError> {
        self.check_mode()?;
        let account = self
            .request_accounts()
            .ok()
            .and_then(|mut x| x.drain(..).next())
            .map(|x| x.to_string());
        let params = account
            .map(|x| serde_json::json!([x]))
            .unwrap_or_else(|| serde_json::json!([]));
        let result = self
            .wasm_request("wallet_getCapabilities", params)
            .await
            .ok();
        if let Some(ref capabilities) = result {
            let mut g = self
                .state
                .lock()
                .map_err(|e| PortError::Transport(format!("provider lock poisoned: {e}")))?;
            g.capabilities = Some(capabilities.clone());
        }
        Ok(result)
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn wasm_sign_payload_async(
        &self,
        method: MessageMethod,
        payload: &[u8],
        expected_signer: Address,
    ) -> Result<Bytes, PortError> {
        self.check_mode()?;
        let method_name = method_rpc_name(method);
        let payload_hex = format!("0x{}", alloy::hex::encode(payload));
        let payload_text = String::from_utf8_lossy(payload).to_string();
        let params = match method {
            MessageMethod::PersonalSign => {
                serde_json::json!([payload_hex, expected_signer.to_string()])
            }
            MessageMethod::EthSign => serde_json::json!([expected_signer.to_string(), payload_hex]),
            MessageMethod::EthSignTypedData | MessageMethod::EthSignTypedDataV4 => {
                serde_json::json!([expected_signer.to_string(), payload_text])
            }
        };
        let result = self.wasm_request(method_name, params).await?;
        let sig_raw = result.as_str().ok_or_else(|| {
            PortError::Transport("signature response must be hex string".to_owned())
        })?;
        sig_raw
            .parse()
            .map_err(|e| PortError::Validation(format!("invalid signature hex: {e}")))
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn wasm_send_transaction_async(&self, tx_payload: &Value) -> Result<B256, PortError> {
        self.check_mode()?;
        let result = self
            .wasm_request("eth_sendTransaction", serde_json::json!([tx_payload]))
            .await?;
        let hash = result.as_str().ok_or_else(|| {
            PortError::Transport("eth_sendTransaction must return tx hash".to_owned())
        })?;
        hash.parse()
            .map_err(|e| PortError::Validation(format!("invalid tx hash: {e}")))
    }

    #[cfg(target_arch = "wasm32")]
    async fn wasm_request(&self, method: &str, params: Value) -> Result<Value, PortError> {
        use wasm_bindgen::JsCast;

        let provider = browser_provider()?;
        let request_fn = get_prop(&provider, "request")
            .ok()
            .and_then(|v| v.dyn_into::<js_sys::Function>().ok())
            .ok_or(PortError::NotImplemented(
                "window.ethereum.request is unavailable",
            ))?;

        let request = serde_json::json!({
            "method": method,
            "params": params,
        });
        let request_js = serde_wasm_bindgen::to_value(&request)
            .map_err(|e| PortError::Transport(format!("failed to encode wasm request: {e}")))?;
        let promise_js = request_fn.call1(&provider, &request_js).map_err(|e| {
            PortError::Transport(format!("provider request dispatch failed: {e:?}"))
        })?;
        let promise = promise_js.dyn_into::<js_sys::Promise>().map_err(|_| {
            PortError::Transport("provider request did not return Promise".to_owned())
        })?;
        let result_js = wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(|e| PortError::Transport(format!("provider request rejected: {e:?}")))?;
        serde_wasm_bindgen::from_value(result_js)
            .map_err(|e| PortError::Transport(format!("failed to decode wasm response: {e}")))
    }

    #[cfg(target_arch = "wasm32")]
    fn refresh_browser_snapshot(&self) -> Result<(), PortError> {
        use wasm_bindgen::JsValue;

        let provider = browser_provider()?;
        let selected = get_prop(&provider, "selectedAddress").unwrap_or(JsValue::NULL);
        let chain = get_prop(&provider, "chainId").unwrap_or(JsValue::NULL);

        let mut g = self
            .state
            .lock()
            .map_err(|e| PortError::Transport(format!("provider lock poisoned: {e}")))?;

        if let Some(s) = selected.as_string() {
            let parsed: Address = s
                .parse()
                .map_err(|e| PortError::Validation(format!("invalid selectedAddress: {e}")))?;
            if g.accounts.first().copied() != Some(parsed) {
                g.accounts = vec![parsed];
                g.event_seq = g.event_seq.saturating_add(1);
                let seq = g.event_seq;
                g.events.push(ProviderEvent {
                    sequence: seq,
                    kind: ProviderEventKind::AccountsChanged,
                    value: serde_json::json!([s]).to_string(),
                });
            }
        }

        if !chain.is_null() && !chain.is_undefined() {
            let parsed = js_chain_id_to_u64(chain)?;
            if g.chain_id != parsed {
                g.chain_id = parsed;
                g.event_seq = g.event_seq.saturating_add(1);
                let seq = g.event_seq;
                g.events.push(ProviderEvent {
                    sequence: seq,
                    kind: ProviderEventKind::ChainChanged,
                    value: parsed.to_string(),
                });
            }
        }

        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    fn register_browser_hooks(&self) -> Result<(), PortError> {
        use wasm_bindgen::{closure::Closure, JsCast, JsValue};

        let provider = browser_provider()?;
        let on_fn = get_prop(&provider, "on")
            .ok()
            .and_then(|v| v.dyn_into::<js_sys::Function>().ok())
            .or_else(|| {
                get_prop(&provider, "addListener")
                    .ok()
                    .and_then(|v| v.dyn_into::<js_sys::Function>().ok())
            })
            .ok_or(PortError::NotImplemented(
                "provider does not expose on/addListener",
            ))?;

        let mut hooks = self
            .hooks
            .lock()
            .map_err(|e| PortError::Transport(format!("provider hooks lock poisoned: {e}")))?;
        if hooks.accounts_changed.is_some() && hooks.chain_changed.is_some() {
            return Ok(());
        }

        let state_for_accounts = Arc::clone(&self.state);
        let accounts_cb = Closure::<dyn FnMut(JsValue)>::new(move |value: JsValue| {
            let mut accounts = Vec::new();
            let mut account_strings = Vec::new();
            if js_sys::Array::is_array(&value) {
                let arr = js_sys::Array::from(&value);
                for item in arr.iter() {
                    if let Some(raw) = item.as_string() {
                        account_strings.push(raw.clone());
                        if let Ok(addr) = raw.parse::<Address>() {
                            accounts.push(addr);
                        }
                    }
                }
            }
            if let Ok(mut g) = state_for_accounts.lock() {
                g.accounts = accounts;
                g.event_seq = g.event_seq.saturating_add(1);
                let seq = g.event_seq;
                g.events.push(ProviderEvent {
                    sequence: seq,
                    kind: ProviderEventKind::AccountsChanged,
                    value: serde_json::to_string(&account_strings)
                        .unwrap_or_else(|_| "[]".to_owned()),
                });
            }
        });

        let state_for_chain = Arc::clone(&self.state);
        let chain_cb = Closure::<dyn FnMut(JsValue)>::new(move |value: JsValue| {
            if let Ok(chain_id) = js_chain_id_to_u64(value) {
                if let Ok(mut g) = state_for_chain.lock() {
                    g.chain_id = chain_id;
                    g.event_seq = g.event_seq.saturating_add(1);
                    let seq = g.event_seq;
                    g.events.push(ProviderEvent {
                        sequence: seq,
                        kind: ProviderEventKind::ChainChanged,
                        value: chain_id.to_string(),
                    });
                }
            }
        });

        on_fn
            .call2(
                &provider,
                &JsValue::from_str("accountsChanged"),
                accounts_cb.as_ref().unchecked_ref(),
            )
            .map_err(|e| PortError::Transport(format!("register accountsChanged failed: {e:?}")))?;
        on_fn
            .call2(
                &provider,
                &JsValue::from_str("chainChanged"),
                chain_cb.as_ref().unchecked_ref(),
            )
            .map_err(|e| PortError::Transport(format!("register chainChanged failed: {e:?}")))?;

        hooks.accounts_changed = Some(accounts_cb);
        hooks.chain_changed = Some(chain_cb);
        Ok(())
    }
}

impl ProviderPort for Eip1193Adapter {
    fn request_accounts(&self) -> Result<Vec<Address>, PortError> {
        self.check_mode()?;

        #[cfg(target_arch = "wasm32")]
        if matches!(self.mode, ProviderMode::Browser) {
            self.refresh_browser_snapshot()?;
            let g = self
                .state
                .lock()
                .map_err(|e| PortError::Transport(format!("provider lock poisoned: {e}")))?;
            if g.accounts.is_empty() {
                return Err(PortError::Policy(
                    "no provider accounts available; unlock/connect wallet".to_owned(),
                ));
            }
            return Ok(g.accounts.clone());
        }

        #[cfg(not(target_arch = "wasm32"))]
        if matches!(self.mode, ProviderMode::Proxy(_)) {
            let result = self.proxy_call("eth_requestAccounts", serde_json::json!([]))?;
            let arr = result.as_array().ok_or_else(|| {
                PortError::Transport("eth_requestAccounts: array expected".to_owned())
            })?;
            let mut accounts = Vec::with_capacity(arr.len());
            for item in arr {
                let raw = item.as_str().ok_or_else(|| {
                    PortError::Transport("eth_requestAccounts: string expected".to_owned())
                })?;
                let parsed: Address = raw
                    .parse()
                    .map_err(|e| PortError::Validation(format!("invalid account address: {e}")))?;
                accounts.push(parsed);
            }
            let mut g = self
                .state
                .lock()
                .map_err(|e| PortError::Transport(format!("provider lock poisoned: {e}")))?;
            if g.accounts != accounts {
                drop(g);
                let serialized = serde_json::to_string(
                    &accounts
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>(),
                )
                .unwrap_or_else(|_| "[]".to_owned());
                self.record_event(ProviderEventKind::AccountsChanged, serialized)?;
                g = self
                    .state
                    .lock()
                    .map_err(|e| PortError::Transport(format!("provider lock poisoned: {e}")))?;
            }
            g.accounts = accounts.clone();
            return Ok(accounts);
        }

        let g = self
            .state
            .lock()
            .map_err(|e| PortError::Transport(format!("provider lock poisoned: {e}")))?;
        Ok(g.accounts.clone())
    }

    fn chain_id(&self) -> Result<u64, PortError> {
        self.check_mode()?;

        #[cfg(target_arch = "wasm32")]
        if matches!(self.mode, ProviderMode::Browser) {
            self.refresh_browser_snapshot()?;
            let g = self
                .state
                .lock()
                .map_err(|e| PortError::Transport(format!("provider lock poisoned: {e}")))?;
            return Ok(g.chain_id);
        }

        #[cfg(not(target_arch = "wasm32"))]
        if matches!(self.mode, ProviderMode::Proxy(_)) {
            let result = self.proxy_call("eth_chainId", serde_json::json!([]))?;
            let chain_id = json_chain_id_to_u64(&result)?;
            let mut g = self
                .state
                .lock()
                .map_err(|e| PortError::Transport(format!("provider lock poisoned: {e}")))?;
            if g.chain_id != chain_id {
                drop(g);
                self.record_event(ProviderEventKind::ChainChanged, chain_id.to_string())?;
                g = self
                    .state
                    .lock()
                    .map_err(|e| PortError::Transport(format!("provider lock poisoned: {e}")))?;
            }
            g.chain_id = chain_id;
            return Ok(chain_id);
        }

        let g = self
            .state
            .lock()
            .map_err(|e| PortError::Transport(format!("provider lock poisoned: {e}")))?;
        Ok(g.chain_id)
    }

    fn wallet_get_capabilities(&self) -> Result<Option<Value>, PortError> {
        self.check_mode()?;

        #[cfg(not(target_arch = "wasm32"))]
        if matches!(self.mode, ProviderMode::Proxy(_)) {
            let account = self
                .request_accounts()
                .ok()
                .and_then(|mut x| x.drain(..).next())
                .map(|a| a.to_string());
            let params = account
                .map(|a| serde_json::json!([a]))
                .unwrap_or_else(|| serde_json::json!([]));
            let result = self.proxy_call("wallet_getCapabilities", params).ok();
            let mut g = self
                .state
                .lock()
                .map_err(|e| PortError::Transport(format!("provider lock poisoned: {e}")))?;
            if result.is_some() {
                g.capabilities = result.clone();
            }
            return Ok(result);
        }

        let g = self
            .state
            .lock()
            .map_err(|e| PortError::Transport(format!("provider lock poisoned: {e}")))?;
        Ok(g.capabilities.clone())
    }

    fn sign_payload(
        &self,
        method: MessageMethod,
        payload: &[u8],
        expected_signer: Address,
    ) -> Result<Bytes, PortError> {
        self.check_mode()?;

        #[cfg(not(target_arch = "wasm32"))]
        if matches!(self.mode, ProviderMode::Proxy(_)) {
            let method_name = method_rpc_name(method);
            let payload_hex = format!("0x{}", alloy::hex::encode(payload));
            let payload_text = String::from_utf8_lossy(payload).to_string();
            let params = match method {
                MessageMethod::PersonalSign => {
                    serde_json::json!([payload_hex, expected_signer.to_string()])
                }
                MessageMethod::EthSign => {
                    serde_json::json!([expected_signer.to_string(), payload_hex])
                }
                MessageMethod::EthSignTypedData | MessageMethod::EthSignTypedDataV4 => {
                    serde_json::json!([expected_signer.to_string(), payload_text])
                }
            };
            let result = self.proxy_call(method_name, params)?;
            let sig_raw = result.as_str().ok_or_else(|| {
                PortError::Transport("sign response must be hex string".to_owned())
            })?;
            let parsed: Bytes = sig_raw
                .parse()
                .map_err(|e| PortError::Validation(format!("invalid signature hex: {e}")))?;
            return Ok(parsed);
        }

        #[cfg(target_arch = "wasm32")]
        if matches!(self.mode, ProviderMode::Browser) {
            return Err(PortError::NotImplemented(
                "wasm sync sign_payload is unavailable; use wasm_sign_payload_async",
            ));
        }

        Ok(self.deterministic_signature(method, payload, expected_signer))
    }

    fn send_transaction(&self, tx_payload: &Value) -> Result<B256, PortError> {
        self.check_mode()?;

        #[cfg(not(target_arch = "wasm32"))]
        if matches!(self.mode, ProviderMode::Proxy(_)) {
            let result = self.proxy_call("eth_sendTransaction", serde_json::json!([tx_payload]))?;
            let hash = result.as_str().ok_or_else(|| {
                PortError::Transport("eth_sendTransaction must return hash".to_owned())
            })?;
            let parsed = hash
                .parse()
                .map_err(|e| PortError::Validation(format!("invalid tx hash: {e}")))?;
            return Ok(parsed);
        }

        #[cfg(target_arch = "wasm32")]
        if matches!(self.mode, ProviderMode::Browser) {
            return Err(PortError::NotImplemented(
                "wasm sync send_transaction is unavailable; use wasm_send_transaction_async",
            ));
        }

        let canonical = serde_json::to_vec(tx_payload)
            .map_err(|e| PortError::Validation(format!("tx payload serialization failed: {e}")))?;
        Ok(keccak256(canonical))
    }

    fn drain_events(&self) -> Result<Vec<ProviderEvent>, PortError> {
        self.check_mode()?;
        let mut g = self
            .state
            .lock()
            .map_err(|e| PortError::Transport(format!("provider lock poisoned: {e}")))?;
        let events = std::mem::take(&mut g.events);
        Ok(events)
    }
}

fn method_rpc_name(method: MessageMethod) -> &'static str {
    match method {
        MessageMethod::PersonalSign => "personal_sign",
        MessageMethod::EthSign => "eth_sign",
        MessageMethod::EthSignTypedData => "eth_signTypedData",
        MessageMethod::EthSignTypedDataV4 => "eth_signTypedData_v4",
    }
}

fn json_chain_id_to_u64(value: &Value) -> Result<u64, PortError> {
    if let Some(n) = value.as_u64() {
        return Ok(n);
    }
    let s = value
        .as_str()
        .ok_or_else(|| PortError::Validation("chain id must be string or number".to_owned()))?;
    parse_chain_id_str(s)
}

fn parse_chain_id_str(raw: &str) -> Result<u64, PortError> {
    if raw.starts_with("0x") || raw.starts_with("0X") {
        u64::from_str_radix(raw.trim_start_matches("0x").trim_start_matches("0X"), 16)
            .map_err(|e| PortError::Validation(format!("invalid hex chain id: {e}")))
    } else {
        raw.parse()
            .map_err(|e| PortError::Validation(format!("invalid chain id: {e}")))
    }
}

#[cfg(target_arch = "wasm32")]
fn browser_provider_available() -> bool {
    browser_provider().is_ok()
}

#[cfg(target_arch = "wasm32")]
fn browser_provider() -> Result<wasm_bindgen::JsValue, PortError> {
    let window =
        web_sys::window().ok_or_else(|| PortError::Transport("missing window".to_owned()))?;
    let provider = get_prop(&window.into(), "ethereum")?;
    if provider.is_null() || provider.is_undefined() {
        return Err(PortError::NotFound("window.ethereum missing".to_owned()));
    }
    Ok(provider)
}

#[cfg(target_arch = "wasm32")]
fn get_prop(target: &wasm_bindgen::JsValue, key: &str) -> Result<wasm_bindgen::JsValue, PortError> {
    js_sys::Reflect::get(target, &wasm_bindgen::JsValue::from_str(key))
        .map_err(|e| PortError::Transport(format!("read provider property {key} failed: {e:?}")))
}

#[cfg(target_arch = "wasm32")]
fn js_chain_id_to_u64(value: wasm_bindgen::JsValue) -> Result<u64, PortError> {
    if let Some(s) = value.as_string() {
        return parse_chain_id_str(&s);
    }
    if let Some(num) = value.as_f64() {
        return Ok(num as u64);
    }
    Err(PortError::Validation("invalid JS chain id".to_owned()))
}
