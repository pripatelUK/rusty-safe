//! Application state types
//!
//! Uses types from safe-hash library where possible.
//! UI state structs are rusty-safe specific.

use crate::api::SafeTransaction;
use crate::decode::DecodedTransaction;
use crate::expected::ExpectedState;
use safe_hash::SafeWarnings;
use safe_utils::get_all_supported_chain_names;

/// LocalStorage key for cached Safe address
const SAFE_ADDRESS_KEY: &str = "rusty-safe-address-v1";

/// Load cached Safe address from LocalStorage (WASM only)
#[cfg(target_arch = "wasm32")]
pub fn load_cached_safe_address() -> Option<String> {
    use gloo_storage::{LocalStorage, Storage};
    LocalStorage::get::<String>(SAFE_ADDRESS_KEY).ok().filter(|s| !s.is_empty())
}

/// Load cached Safe address - returns None on native
#[cfg(not(target_arch = "wasm32"))]
pub fn load_cached_safe_address() -> Option<String> {
    None
}

/// Save Safe address to LocalStorage (WASM only)
#[cfg(target_arch = "wasm32")]
pub fn save_safe_address(address: &str) {
    use gloo_storage::{LocalStorage, Storage};
    if !address.is_empty() {
        let _ = LocalStorage::set(SAFE_ADDRESS_KEY, address);
    }
}

/// Save Safe address - no-op on native
#[cfg(not(target_arch = "wasm32"))]
pub fn save_safe_address(_address: &str) {
    // No-op
}

/// Safe versions supported
pub const SAFE_VERSIONS: &[&str] = &[
    "1.4.1", "1.4.0", "1.3.0", "1.2.0", "1.1.1", "1.1.0", "1.0.0",
];

/// Transaction verification UI state
pub struct TxVerifyState {
    /// Selected chain name (from safe_utils)
    pub chain_name: String,
    /// Safe address input
    pub safe_address: String,
    /// Safe version
    pub safe_version: String,
    /// Transaction nonce
    pub nonce: String,
    /// Offline mode
    pub offline_mode: bool,

    // Offline mode inputs
    pub to: String,
    pub value: String,
    pub data: String,
    pub operation: u8,
    pub safe_tx_gas: String,
    pub base_gas: String,
    pub gas_price: String,
    pub gas_token: String,
    pub refund_receiver: String,

    // Expected values for API validation
    pub expected: ExpectedState,

    // Calldata decode state
    pub decode: Option<DecodedTransaction>,

    // UI toggle for showing full data
    pub show_full_data: bool,

    // Fetched from API
    pub fetched_tx: Option<SafeTransaction>,

    // Computed hashes (display strings)
    pub hashes: Option<ComputedHashes>,

    // Warnings from safe_hash
    pub warnings: SafeWarnings,

    // Loading state
    pub is_loading: bool,

    // Error
    pub error: Option<String>,
}

/// Computed hash results (display strings)
#[derive(Debug, Clone)]
pub struct ComputedHashes {
    pub domain_hash: String,
    pub message_hash: String,
    pub safe_tx_hash: String,
    pub matches_api: Option<bool>,
}

impl Default for TxVerifyState {
    fn default() -> Self {
        let chains = get_all_supported_chain_names();
        let default_chain = chains
            .iter()
            .find(|c| *c == "ethereum")
            .cloned()
            .unwrap_or_else(|| chains.first().cloned().unwrap_or_default());

        Self {
            chain_name: default_chain,
            safe_address: String::new(),
            safe_version: SAFE_VERSIONS[0].to_string(),
            nonce: String::new(),
            offline_mode: false,
            to: String::new(),
            value: "0".to_string(),
            data: String::new(),
            operation: 0,
            safe_tx_gas: "0".to_string(),
            base_gas: "0".to_string(),
            gas_price: "0".to_string(),
            gas_token: "0x0000000000000000000000000000000000000000".to_string(),
            refund_receiver: "0x0000000000000000000000000000000000000000".to_string(),
            expected: ExpectedState::default(),
            decode: None,
            show_full_data: false,
            fetched_tx: None,
            hashes: None,
            warnings: SafeWarnings::new(),
            is_loading: false,
            error: None,
        }
    }
}

impl TxVerifyState {
    pub fn clear_results(&mut self) {
        self.fetched_tx = None;
        self.hashes = None;
        self.warnings = SafeWarnings::new();
        self.expected.clear_result();
        self.decode = None;
        self.error = None;
    }
}

/// Message verification UI state
#[derive(Debug)]
pub struct MsgVerifyState {
    pub chain_name: String,
    pub safe_address: String,
    pub safe_version: String,
    pub message: String,
    pub is_hex: bool,
    pub hashes: Option<MsgHashes>,
    pub error: Option<String>,
}

impl Default for MsgVerifyState {
    fn default() -> Self {
        let chains = get_all_supported_chain_names();
        let default_chain = chains
            .iter()
            .find(|c| *c == "ethereum")
            .cloned()
            .unwrap_or_else(|| chains.first().cloned().unwrap_or_default());

        Self {
            chain_name: default_chain,
            safe_address: String::new(),
            safe_version: SAFE_VERSIONS[0].to_string(),
            message: String::new(),
            is_hex: false,
            hashes: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MsgHashes {
    pub raw_hash: String,
    pub message_hash: String,
    pub safe_msg_hash: String,
}

/// EIP-712 verification UI state
#[derive(Debug)]
pub struct Eip712State {
    pub chain_name: String,
    pub safe_address: String,
    pub safe_version: String,
    pub json_input: String,
    pub hashes: Option<Eip712Hashes>,
    pub error: Option<String>,
}

impl Default for Eip712State {
    fn default() -> Self {
        let chains = get_all_supported_chain_names();
        let default_chain = chains
            .iter()
            .find(|c| *c == "ethereum")
            .cloned()
            .unwrap_or_else(|| chains.first().cloned().unwrap_or_default());

        Self {
            chain_name: default_chain,
            safe_address: String::new(),
            safe_version: SAFE_VERSIONS[0].to_string(),
            json_input: String::new(),
            hashes: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Eip712Hashes {
    pub domain_hash: String,
    pub message_hash: String,
    pub full_hash: String,
}
