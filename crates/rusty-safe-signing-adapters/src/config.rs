#[derive(Debug, Clone)]
pub struct SigningAdapterConfig {
    pub safe_service_base_url: String,
    pub request_timeout_ms: u64,
}

impl Default for SigningAdapterConfig {
    fn default() -> Self {
        Self {
            safe_service_base_url: "https://safe-transaction-mainnet.safe.global".to_owned(),
            request_timeout_ms: 15_000,
        }
    }
}
