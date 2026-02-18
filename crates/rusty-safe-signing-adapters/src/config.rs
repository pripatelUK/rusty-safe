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
        }
    }
}
