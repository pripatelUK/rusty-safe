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
}

#[derive(Debug, Clone)]
enum WalletConnectMode {
    InMemory(Arc<Mutex<WalletConnectState>>),
    #[cfg(not(target_arch = "wasm32"))]
    HttpBridge(HttpWalletConnectRuntime),
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
    pub fn in_memory() -> Self {
        Self {
            mode: WalletConnectMode::InMemory(Arc::new(Mutex::new(WalletConnectState::default()))),
        }
    }

    pub fn with_config(config: SigningAdapterConfig) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(base_url) = config.walletconnect_bridge_url {
                let timeout = std::time::Duration::from_millis(config.safe_service_timeout_ms);
                if let Ok(client) = reqwest::blocking::Client::builder()
                    .timeout(timeout)
                    .build()
                {
                    return Self {
                        mode: WalletConnectMode::HttpBridge(HttpWalletConnectRuntime {
                            base_url: base_url.trim_end_matches('/').to_owned(),
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
        f: impl FnOnce(&mut WalletConnectState) -> Result<T, PortError>,
    ) -> Result<T, PortError> {
        let mode = match &self.mode {
            WalletConnectMode::InMemory(state) => state,
            #[cfg(not(target_arch = "wasm32"))]
            WalletConnectMode::HttpBridge(_) => {
                return Err(PortError::NotImplemented("in-memory state unavailable"))
            }
        };
        let mut g = mode
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
}

impl WalletConnectPort for WalletConnectAdapter {
    fn pair(&self, uri: &str) -> Result<(), PortError> {
        #[cfg(not(target_arch = "wasm32"))]
        if let WalletConnectMode::HttpBridge(runtime) = &self.mode {
            let body = serde_json::json!({ "uri": uri });
            let _ = Self::call_http(runtime, reqwest::Method::POST, "/pair", Some(body))?;
            return Ok(());
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
        #[cfg(not(target_arch = "wasm32"))]
        if let WalletConnectMode::HttpBridge(runtime) = &self.mode {
            let body = serde_json::json!({ "topic": topic, "action": format!("{action:?}") });
            let path = format!("/session/{topic}/action");
            let _ = Self::call_http(runtime, reqwest::Method::POST, &path, Some(body))?;
            return Ok(());
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
        Ok(())
    }
}

fn keccak_topic(input: &str) -> String {
    let hash = alloy::primitives::keccak256(input.as_bytes());
    let short = &hash.as_slice()[..8];
    format!("0x{}", alloy::hex::encode(short))
}
