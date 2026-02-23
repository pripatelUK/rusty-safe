#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeProfile {
    Development,
    Production,
}

#[derive(Debug, Clone)]
pub struct SigningAdapterConfig {
    pub allow_eth_sign: bool,
    pub provider_capability_cache_ttl_ms: u64,
    pub writer_lock_ttl_ms: u64,
    pub safe_service_timeout_ms: u64,
    pub wc_request_poll_interval_ms: u64,
    pub import_max_bundle_bytes: usize,
    pub import_max_object_count: usize,
    pub abi_max_bytes: usize,
    pub url_import_max_payload_bytes: usize,
    pub wc_session_idle_timeout_ms: u64,
    pub command_latency_budget_ms: u64,
    pub rehydration_budget_ms: u64,
    pub safe_service_base_url: String,
    pub safe_service_retry_count: u32,
    pub safe_service_http_enabled: bool,
    pub eip1193_proxy_url: Option<String>,
    pub walletconnect_bridge_url: Option<String>,
    pub export_passphrase_env: String,
    pub runtime_profile: RuntimeProfile,
}

impl Default for SigningAdapterConfig {
    fn default() -> Self {
        Self {
            allow_eth_sign: false,
            provider_capability_cache_ttl_ms: 60_000,
            writer_lock_ttl_ms: 15_000,
            safe_service_timeout_ms: 15_000,
            wc_request_poll_interval_ms: 1_000,
            import_max_bundle_bytes: 4 * 1024 * 1024,
            import_max_object_count: 2_000,
            abi_max_bytes: 512 * 1024,
            url_import_max_payload_bytes: 256 * 1024,
            wc_session_idle_timeout_ms: 30 * 60 * 1000,
            command_latency_budget_ms: 150,
            rehydration_budget_ms: 1_500,
            safe_service_base_url: "https://safe-transaction-mainnet.safe.global".to_owned(),
            safe_service_retry_count: 2,
            safe_service_http_enabled: false,
            eip1193_proxy_url: None,
            walletconnect_bridge_url: None,
            export_passphrase_env: "RUSTY_SAFE_EXPORT_PASSPHRASE".to_owned(),
            runtime_profile: if cfg!(debug_assertions) {
                RuntimeProfile::Development
            } else {
                RuntimeProfile::Production
            },
        }
    }
}

impl SigningAdapterConfig {
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Ok(v) = std::env::var("RUSTY_SAFE_SAFE_SERVICE_BASE_URL") {
            if !v.trim().is_empty() {
                cfg.safe_service_base_url = v;
            }
        }
        if let Ok(v) = std::env::var("RUSTY_SAFE_SAFE_SERVICE_TIMEOUT_MS") {
            if let Ok(parsed) = v.parse::<u64>() {
                cfg.safe_service_timeout_ms = parsed;
            }
        }
        if let Ok(v) = std::env::var("RUSTY_SAFE_SAFE_SERVICE_RETRY_COUNT") {
            if let Ok(parsed) = v.parse::<u32>() {
                cfg.safe_service_retry_count = parsed;
            }
        }
        if let Ok(v) = std::env::var("RUSTY_SAFE_SAFE_SERVICE_HTTP_ENABLED") {
            cfg.safe_service_http_enabled = matches!(v.as_str(), "1" | "true" | "TRUE" | "yes");
        }
        if let Ok(v) = std::env::var("RUSTY_SAFE_EIP1193_PROXY_URL") {
            if !v.trim().is_empty() {
                cfg.eip1193_proxy_url = Some(v);
            }
        }
        if let Ok(v) = std::env::var("RUSTY_SAFE_WALLETCONNECT_BRIDGE_URL") {
            if !v.trim().is_empty() {
                cfg.walletconnect_bridge_url = Some(v);
            }
        }
        if let Ok(v) = std::env::var("RUSTY_SAFE_EXPORT_PASSPHRASE_ENV") {
            if !v.trim().is_empty() {
                cfg.export_passphrase_env = v;
            }
        }
        if let Ok(v) = std::env::var("RUSTY_SAFE_RUNTIME_PROFILE") {
            cfg.runtime_profile = match v.trim().to_ascii_lowercase().as_str() {
                "prod" | "production" => RuntimeProfile::Production,
                _ => RuntimeProfile::Development,
            };
        }
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsValue;

            if let Some(window) = web_sys::window() {
                let window_ref = window.as_ref();
                if let Ok(value) = js_sys::Reflect::get(
                    window_ref,
                    &JsValue::from_str("__RUSTY_SAFE_RUNTIME_PROFILE"),
                ) {
                    if let Some(profile) = value.as_string() {
                        cfg.runtime_profile = match profile.trim().to_ascii_lowercase().as_str() {
                            "prod" | "production" => RuntimeProfile::Production,
                            _ => RuntimeProfile::Development,
                        };
                    }
                }
            }
        }
        cfg
    }

    pub fn strict_runtime_required(&self) -> bool {
        matches!(self.runtime_profile, RuntimeProfile::Production)
    }
}
