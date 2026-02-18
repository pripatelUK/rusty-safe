use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::Value;

use rusty_safe_signing_core::{
    PendingWalletConnectRequest, PortError, WalletConnectPort, WcSessionAction, WcSessionContext,
    WcSessionStatus,
};

use crate::SigningAdapterConfig;

#[derive(Debug, Clone)]
pub struct WalletConnectAdapter {
    mode: WalletConnectMode,
    state: Arc<Mutex<WalletConnectState>>,
}

#[derive(Debug, Clone)]
enum WalletConnectMode {
    Disabled(String),
    InMemory,
    #[cfg(not(target_arch = "wasm32"))]
    HttpBridge(HttpWalletConnectRuntime),
    #[cfg(target_arch = "wasm32")]
    BrowserRuntime,
}

#[derive(Debug, Default)]
struct WalletConnectState {
    sessions: HashMap<String, WcSessionContext>,
    requests: HashMap<String, PendingWalletConnectRequest>,
}

#[derive(Debug, Clone)]
#[cfg(not(target_arch = "wasm32"))]
struct HttpWalletConnectRuntime {
    base_url: String,
    retry_count: u32,
    client: reqwest::blocking::Client,
}

impl Default for WalletConnectAdapter {
    fn default() -> Self {
        Self::with_config(SigningAdapterConfig::from_env())
    }
}

impl WalletConnectAdapter {
    fn with_mode(mode: WalletConnectMode) -> Self {
        Self {
            mode,
            state: Arc::new(Mutex::new(WalletConnectState::default())),
        }
    }

    pub fn in_memory() -> Self {
        Self::with_mode(WalletConnectMode::InMemory)
    }

    pub fn with_config(config: SigningAdapterConfig) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(ref base_url) = config.walletconnect_bridge_url {
                let timeout = std::time::Duration::from_millis(config.safe_service_timeout_ms);
                if let Ok(client) = reqwest::blocking::Client::builder()
                    .timeout(timeout)
                    .build()
                {
                    return Self::with_mode(WalletConnectMode::HttpBridge(
                        HttpWalletConnectRuntime {
                            base_url: base_url.trim_end_matches('/').to_owned(),
                            retry_count: config.safe_service_retry_count,
                            client,
                        },
                    ));
                }
                if config.strict_runtime_required() {
                    return Self::with_mode(WalletConnectMode::Disabled(
                        "failed to initialize WalletConnect bridge runtime in production profile"
                            .to_owned(),
                    ));
                }
            } else if config.strict_runtime_required() {
                return Self::with_mode(WalletConnectMode::Disabled(
                    "WalletConnect runtime bridge URL is required in production profile".to_owned(),
                ));
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            if browser_runtime_available() {
                return Self::with_mode(WalletConnectMode::BrowserRuntime);
            }
            if config.strict_runtime_required() {
                return Self::with_mode(WalletConnectMode::Disabled(
                    "WalletConnect browser runtime missing in production profile".to_owned(),
                ));
            }
        }
        Self::in_memory()
    }

    fn check_mode(&self) -> Result<(), PortError> {
        if let WalletConnectMode::Disabled(reason) = &self.mode {
            return Err(PortError::Policy(reason.clone()));
        }
        Ok(())
    }

    fn with_state<T>(
        &self,
        f: impl FnOnce(&mut WalletConnectState) -> Result<T, PortError>,
    ) -> Result<T, PortError> {
        self.check_mode()?;
        let mut g = self
            .state
            .lock()
            .map_err(|e| PortError::Transport(format!("wc lock poisoned: {e}")))?;
        f(&mut g)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn call_http(
        runtime: &HttpWalletConnectRuntime,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, PortError> {
        let url = format!("{}{}", runtime.base_url, path);
        let mut attempt = 0u32;
        loop {
            let mut req = runtime.client.request(method.clone(), &url);
            if let Some(ref payload) = body {
                req = req.json(payload);
            }
            match req.send() {
                Ok(resp) => {
                    let status = resp.status();
                    let text = resp.text().map_err(|e| {
                        PortError::Transport(format!("walletconnect body read failed: {e}"))
                    })?;
                    let value: Value =
                        serde_json::from_str(&text).unwrap_or(Value::String(text.clone()));
                    if status.is_success() {
                        return Ok(value);
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
                        "walletconnect bridge {} {} failed with status {}: {}",
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
                        "walletconnect bridge {} {} request failed: {e}",
                        method, path
                    )));
                }
            }
        }
    }

    pub fn insert_session(&self, session: WcSessionContext) -> Result<(), PortError> {
        self.with_state(|g| {
            g.sessions.insert(session.topic.clone(), session);
            Ok(())
        })
    }

    pub fn insert_request(&self, req: PendingWalletConnectRequest) -> Result<(), PortError> {
        self.with_state(|g| {
            g.requests.insert(req.request_id.clone(), req);
            Ok(())
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn wasm_pair_async(&self, uri: &str) -> Result<(), PortError> {
        self.check_mode()?;
        let value = self
            .wasm_request("pair", serde_json::json!({ "uri": uri }))
            .await?;
        if let Some(topic) = value.get("topic").and_then(|v| v.as_str()) {
            self.with_state(|g| {
                g.sessions
                    .entry(topic.to_owned())
                    .or_insert(WcSessionContext {
                        topic: topic.to_owned(),
                        status: WcSessionStatus::Proposed,
                        dapp_name: None,
                        dapp_url: None,
                        dapp_icons: Vec::new(),
                        capability_snapshot: None,
                        updated_at_ms: rusty_safe_signing_core::TimestampMs(0),
                    });
                Ok(())
            })?;
        }
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn wasm_session_action_async(
        &self,
        topic: &str,
        action: WcSessionAction,
    ) -> Result<(), PortError> {
        self.check_mode()?;
        let _ = self
            .wasm_request(
                "session_action",
                serde_json::json!({
                    "topic": topic,
                    "action": format!("{action:?}")
                }),
            )
            .await?;
        self.with_state(|g| {
            if let Some(session) = g.sessions.get_mut(topic) {
                session.status = match action {
                    WcSessionAction::Approve => WcSessionStatus::Approved,
                    WcSessionAction::Reject => WcSessionStatus::Rejected,
                    WcSessionAction::Disconnect => WcSessionStatus::Disconnected,
                };
            }
            Ok(())
        })?;
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn wasm_list_sessions_async(&self) -> Result<Vec<WcSessionContext>, PortError> {
        self.check_mode()?;
        let value = self
            .wasm_request("list_sessions", serde_json::json!({}))
            .await?;
        let sessions: Vec<WcSessionContext> = serde_json::from_value(value).map_err(|e| {
            PortError::Transport(format!("walletconnect runtime sessions decode failed: {e}"))
        })?;
        self.with_state(|g| {
            g.sessions = sessions
                .iter()
                .cloned()
                .map(|x| (x.topic.clone(), x))
                .collect();
            Ok(())
        })?;
        Ok(sessions)
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn wasm_list_pending_requests_async(
        &self,
    ) -> Result<Vec<PendingWalletConnectRequest>, PortError> {
        self.check_mode()?;
        let value = self
            .wasm_request("list_pending_requests", serde_json::json!({}))
            .await?;
        let requests: Vec<PendingWalletConnectRequest> =
            serde_json::from_value(value).map_err(|e| {
                PortError::Transport(format!("walletconnect runtime requests decode failed: {e}"))
            })?;
        self.with_state(|g| {
            g.requests = requests
                .iter()
                .cloned()
                .map(|x| (x.request_id.clone(), x))
                .collect();
            Ok(())
        })?;
        Ok(requests)
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn wasm_respond_success_async(
        &self,
        request_id: &str,
        result: Value,
    ) -> Result<(), PortError> {
        self.check_mode()?;
        let _ = self
            .wasm_request(
                "respond_success",
                serde_json::json!({
                    "request_id": request_id,
                    "result": result
                }),
            )
            .await?;
        self.with_state(|g| {
            g.requests.remove(request_id);
            Ok(())
        })?;
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn wasm_respond_error_async(
        &self,
        request_id: &str,
        code: i64,
        message: &str,
    ) -> Result<(), PortError> {
        self.check_mode()?;
        let _ = self
            .wasm_request(
                "respond_error",
                serde_json::json!({
                    "request_id": request_id,
                    "code": code,
                    "message": message
                }),
            )
            .await?;
        self.with_state(|g| {
            g.requests.remove(request_id);
            Ok(())
        })?;
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn wasm_sync_async(&self) -> Result<(), PortError> {
        self.check_mode()?;
        let _ = self.wasm_request("sync", serde_json::json!({})).await?;
        let _ = self.wasm_list_sessions_async().await?;
        let _ = self.wasm_list_pending_requests_async().await?;
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    async fn wasm_request(&self, method: &str, params: Value) -> Result<Value, PortError> {
        use wasm_bindgen::JsCast;

        if !matches!(self.mode, WalletConnectMode::BrowserRuntime) {
            return Err(PortError::NotImplemented(
                "walletconnect wasm runtime unavailable; use in-memory adapter",
            ));
        }

        let runtime = browser_runtime()?;
        let request_fn = get_prop(&runtime, "request")
            .ok()
            .and_then(|v| v.dyn_into::<js_sys::Function>().ok())
            .ok_or(PortError::NotImplemented(
                "walletconnect runtime request() unavailable",
            ))?;
        let payload = serde_json::json!({
            "method": method,
            "params": params,
        });
        let payload_js = serde_wasm_bindgen::to_value(&payload).map_err(|e| {
            PortError::Transport(format!("walletconnect payload encode failed: {e}"))
        })?;
        let promise_js = request_fn.call1(&runtime, &payload_js).map_err(|e| {
            PortError::Transport(format!("walletconnect runtime dispatch failed: {e:?}"))
        })?;
        let promise = promise_js.dyn_into::<js_sys::Promise>().map_err(|_| {
            PortError::Transport("walletconnect runtime did not return Promise".to_owned())
        })?;
        let result_js = wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(|e| PortError::Transport(format!("walletconnect runtime rejected: {e:?}")))?;
        serde_wasm_bindgen::from_value(result_js)
            .map_err(|e| PortError::Transport(format!("walletconnect response decode failed: {e}")))
    }
}

impl WalletConnectPort for WalletConnectAdapter {
    fn pair(&self, uri: &str) -> Result<(), PortError> {
        self.check_mode()?;

        #[cfg(not(target_arch = "wasm32"))]
        if let WalletConnectMode::HttpBridge(runtime) = &self.mode {
            let body = serde_json::json!({ "uri": uri });
            let _ = Self::call_http(runtime, reqwest::Method::POST, "/pair", Some(body))?;
            return Ok(());
        }

        #[cfg(target_arch = "wasm32")]
        if matches!(self.mode, WalletConnectMode::BrowserRuntime) {
            return Err(PortError::NotImplemented(
                "wasm sync pair is unavailable; use wasm_pair_async",
            ));
        }

        // Deterministic in-memory fallback emulates a proposed session for parity tests.
        self.with_state(|g| {
            let topic = format!("wc-{}", keccak_topic(uri));
            g.sessions.entry(topic.clone()).or_insert(WcSessionContext {
                topic,
                status: WcSessionStatus::Proposed,
                dapp_name: Some("WalletConnect dApp".to_owned()),
                dapp_url: None,
                dapp_icons: Vec::new(),
                capability_snapshot: None,
                updated_at_ms: rusty_safe_signing_core::TimestampMs(0),
            });
            Ok(())
        })
    }

    fn session_action(&self, topic: &str, action: WcSessionAction) -> Result<(), PortError> {
        self.check_mode()?;

        #[cfg(not(target_arch = "wasm32"))]
        if let WalletConnectMode::HttpBridge(runtime) = &self.mode {
            let body = serde_json::json!({ "topic": topic, "action": format!("{action:?}") });
            let path = format!("/session/{topic}/action");
            let _ = Self::call_http(runtime, reqwest::Method::POST, &path, Some(body))?;
            return Ok(());
        }

        #[cfg(target_arch = "wasm32")]
        if matches!(self.mode, WalletConnectMode::BrowserRuntime) {
            return Err(PortError::NotImplemented(
                "wasm sync session_action is unavailable; use wasm_session_action_async",
            ));
        }

        self.with_state(|g| {
            let session = g
                .sessions
                .get_mut(topic)
                .ok_or_else(|| PortError::NotFound(format!("wc session missing: {topic}")))?;
            session.status = match action {
                WcSessionAction::Approve => WcSessionStatus::Approved,
                WcSessionAction::Reject => WcSessionStatus::Rejected,
                WcSessionAction::Disconnect => WcSessionStatus::Disconnected,
            };
            Ok(())
        })
    }

    fn list_sessions(&self) -> Result<Vec<WcSessionContext>, PortError> {
        self.check_mode()?;

        #[cfg(not(target_arch = "wasm32"))]
        if let WalletConnectMode::HttpBridge(runtime) = &self.mode {
            let value = Self::call_http(runtime, reqwest::Method::GET, "/sessions", None)?;
            return serde_json::from_value(value).map_err(|e| {
                PortError::Transport(format!("walletconnect bridge sessions decode failed: {e}"))
            });
        }

        self.with_state(|g| Ok(g.sessions.values().cloned().collect()))
    }

    fn list_pending_requests(&self) -> Result<Vec<PendingWalletConnectRequest>, PortError> {
        self.check_mode()?;

        #[cfg(not(target_arch = "wasm32"))]
        if let WalletConnectMode::HttpBridge(runtime) = &self.mode {
            let value = Self::call_http(runtime, reqwest::Method::GET, "/requests/pending", None)?;
            return serde_json::from_value(value).map_err(|e| {
                PortError::Transport(format!("walletconnect bridge request decode failed: {e}"))
            });
        }

        self.with_state(|g| Ok(g.requests.values().cloned().collect()))
    }

    fn respond_success(&self, request_id: &str, result: Value) -> Result<(), PortError> {
        self.check_mode()?;
        #[cfg(target_arch = "wasm32")]
        let _ = &result;

        #[cfg(not(target_arch = "wasm32"))]
        if let WalletConnectMode::HttpBridge(runtime) = &self.mode {
            let path = format!("/requests/{request_id}/response");
            let body = serde_json::json!({
                "request_id": request_id,
                "result": result,
            });
            let _ = Self::call_http(runtime, reqwest::Method::POST, &path, Some(body))?;
            return Ok(());
        }

        #[cfg(target_arch = "wasm32")]
        if matches!(self.mode, WalletConnectMode::BrowserRuntime) {
            return Err(PortError::NotImplemented(
                "wasm sync respond_success is unavailable; use wasm_respond_success_async",
            ));
        }

        self.with_state(|g| {
            if g.requests.remove(request_id).is_none() {
                return Err(PortError::NotFound(format!(
                    "wc request missing for success response: {request_id}"
                )));
            }
            Ok(())
        })
    }

    fn respond_error(&self, request_id: &str, code: i64, message: &str) -> Result<(), PortError> {
        self.check_mode()?;
        #[cfg(target_arch = "wasm32")]
        let _ = (&code, &message);

        #[cfg(not(target_arch = "wasm32"))]
        if let WalletConnectMode::HttpBridge(runtime) = &self.mode {
            let path = format!("/requests/{request_id}/error");
            let body = serde_json::json!({
                "request_id": request_id,
                "code": code,
                "message": message,
            });
            let _ = Self::call_http(runtime, reqwest::Method::POST, &path, Some(body))?;
            return Ok(());
        }

        #[cfg(target_arch = "wasm32")]
        if matches!(self.mode, WalletConnectMode::BrowserRuntime) {
            return Err(PortError::NotImplemented(
                "wasm sync respond_error is unavailable; use wasm_respond_error_async",
            ));
        }

        self.with_state(|g| {
            if g.requests.remove(request_id).is_none() {
                return Err(PortError::NotFound(format!(
                    "wc request missing for error response: {request_id}"
                )));
            }
            Ok(())
        })
    }

    fn sync(&self) -> Result<(), PortError> {
        self.check_mode()?;

        #[cfg(not(target_arch = "wasm32"))]
        if let WalletConnectMode::HttpBridge(runtime) = &self.mode {
            let _ = Self::call_http(
                runtime,
                reqwest::Method::POST,
                "/sync",
                Some(serde_json::json!({})),
            )?;
            return Ok(());
        }

        #[cfg(target_arch = "wasm32")]
        if matches!(self.mode, WalletConnectMode::BrowserRuntime) {
            return Err(PortError::NotImplemented(
                "wasm sync runtime sync is unavailable; use wasm_sync_async",
            ));
        }
        Ok(())
    }
}

fn keccak_topic(input: &str) -> String {
    let hash = alloy::primitives::keccak256(input.as_bytes());
    let short = &hash.as_slice()[..8];
    format!("0x{}", alloy::hex::encode(short))
}

#[cfg(target_arch = "wasm32")]
fn browser_runtime_available() -> bool {
    browser_runtime().is_ok()
}

#[cfg(target_arch = "wasm32")]
fn browser_runtime() -> Result<wasm_bindgen::JsValue, PortError> {
    let window =
        web_sys::window().ok_or_else(|| PortError::Transport("missing window".to_owned()))?;
    let runtime = get_prop(&window.into(), "__rustySafeWalletConnect")?;
    if runtime.is_null() || runtime.is_undefined() {
        return Err(PortError::NotFound(
            "window.__rustySafeWalletConnect missing".to_owned(),
        ));
    }
    Ok(runtime)
}

#[cfg(target_arch = "wasm32")]
fn get_prop(target: &wasm_bindgen::JsValue, key: &str) -> Result<wasm_bindgen::JsValue, PortError> {
    js_sys::Reflect::get(target, &wasm_bindgen::JsValue::from_str(key)).map_err(|e| {
        PortError::Transport(format!(
            "read walletconnect runtime property {key} failed: {e:?}"
        ))
    })
}
