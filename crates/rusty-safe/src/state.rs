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
/// LocalStorage key for recent addresses
const RECENT_ADDRESSES_KEY: &str = "rusty-safe-recent-v1";
/// Max recent addresses to keep
const MAX_RECENT_ADDRESSES: usize = 10;

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

/// Load recent addresses from LocalStorage (WASM only)
#[cfg(target_arch = "wasm32")]
pub fn load_recent_addresses() -> Vec<String> {
    use gloo_storage::{LocalStorage, Storage};
    LocalStorage::get::<Vec<String>>(RECENT_ADDRESSES_KEY).unwrap_or_default()
}

/// Load recent addresses - returns empty on native
#[cfg(not(target_arch = "wasm32"))]
pub fn load_recent_addresses() -> Vec<String> {
    Vec::new()
}

/// Save recent addresses to LocalStorage (WASM only)
#[cfg(target_arch = "wasm32")]
pub fn save_recent_addresses(addresses: &[String]) {
    use gloo_storage::{LocalStorage, Storage};
    let _ = LocalStorage::set(RECENT_ADDRESSES_KEY, addresses);
}

/// Save recent addresses - no-op on native
#[cfg(not(target_arch = "wasm32"))]
pub fn save_recent_addresses(_addresses: &[String]) {
    // No-op
}

/// Add address to recent list (most recent first, deduped, capped)
pub fn add_recent_address(addresses: &mut Vec<String>, address: &str) {
    if address.is_empty() || !address.starts_with("0x") || address.len() != 42 {
        return;
    }
    // Remove if already exists (will re-add at front)
    addresses.retain(|a| a.to_lowercase() != address.to_lowercase());
    // Insert at front
    addresses.insert(0, address.to_string());
    // Cap at max
    addresses.truncate(MAX_RECENT_ADDRESSES);
    // Persist
    save_recent_addresses(addresses);
}

/// Safe versions supported
pub const SAFE_VERSIONS: &[&str] = &[
    "1.4.1", "1.4.0", "1.3.0", "1.2.0", "1.1.1", "1.1.0", "1.0.0",
];

// =============================================================================
// SHARED SAFE CONTEXT (used by sidebar, shared across all tabs)
// =============================================================================

/// Shared Safe context - displayed in sidebar, used by all tabs
pub struct SafeContext {
    pub chain_name: String,
    pub safe_address: String,
    pub safe_version: String,
    pub recent_addresses: Vec<String>,
}

impl Default for SafeContext {
    fn default() -> Self {
        let chains = get_all_supported_chain_names();
        let default_chain = chains.iter()
            .find(|c| *c == "ethereum")
            .cloned()
            .unwrap_or_else(|| chains.first().cloned().unwrap_or_default());
        
        Self {
            chain_name: default_chain,
            safe_address: load_cached_safe_address().unwrap_or_default(),
            safe_version: SAFE_VERSIONS[0].to_string(),
            recent_addresses: load_recent_addresses(),
        }
    }
}

/// Sidebar UI state
#[derive(Default)]
pub struct SidebarState {
    pub collapsed: bool,
}

// =============================================================================
// TAB-SPECIFIC STATES (no longer contain chain/address/version - use SafeContext)
// =============================================================================

/// Transaction verification UI state (Verify Safe API tab)
#[derive(Default)]
pub struct TxVerifyState {
    pub nonce: String,
    pub expected: ExpectedState,
    pub decode: Option<DecodedTransaction>,
    pub show_full_data: bool,
    pub fetched_tx: Option<SafeTransaction>,
    pub hashes: Option<ComputedHashes>,
    pub warnings: SafeWarnings,
    pub is_loading: bool,
    pub error: Option<String>,
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

/// Computed hash results (display strings)
#[derive(Debug, Clone)]
pub struct ComputedHashes {
    pub domain_hash: String,
    pub message_hash: String,
    pub safe_tx_hash: String,
    pub matches_api: Option<bool>,
}

/// Message verification UI state
#[derive(Debug, Default)]
pub struct MsgVerifyState {
    pub message: String,
    pub is_hex: bool,
    pub hashes: Option<MsgHashes>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MsgHashes {
    pub raw_hash: String,
    pub message_hash: String,
    pub safe_msg_hash: String,
}

/// EIP-712 verification UI state
#[derive(Debug, Default)]
pub struct Eip712State {
    pub json_input: String,
    pub standalone: bool,
    pub hashes: Option<Eip712Hashes>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Eip712Hashes {
    // Raw EIP-712 hashes (from the typed data itself)
    pub eip712_hash: String,
    pub eip712_domain_hash: String,
    pub eip712_message_hash: String,
    // Safe-wrapped hashes (when not standalone)
    pub safe_domain_hash: Option<String>,
    pub safe_message_hash: Option<String>,
    pub safe_hash: Option<String>,
}

// =============================================================================
// OFFLINE MODE STATE
// =============================================================================

use crate::decode::OfflineDecodeResult;

/// Offline verification UI state (manual transaction input)
pub struct OfflineState {
    // Transaction inputs
    pub to: String,
    pub value: String,
    pub data: String,
    pub operation: u8,
    pub nonce: String,
    pub safe_tx_gas: String,
    pub base_gas: String,
    pub gas_price: String,
    pub gas_token: String,
    pub refund_receiver: String,
    
    // Results
    pub decode_result: Option<OfflineDecodeResult>,
    pub hashes: Option<ComputedHashes>,
    pub warnings: SafeWarnings,
    
    // State
    pub is_loading: bool,
    pub error: Option<String>,
}

impl Default for OfflineState {
    fn default() -> Self {
        Self {
            to: String::new(),
            value: "0".to_string(),
            data: String::new(),
            operation: 0,
            nonce: "0".to_string(),
            safe_tx_gas: "0".to_string(),
            base_gas: "0".to_string(),
            gas_price: "0".to_string(),
            gas_token: "0x0000000000000000000000000000000000000000".to_string(),
            refund_receiver: "0x0000000000000000000000000000000000000000".to_string(),
            decode_result: None,
            hashes: None,
            warnings: SafeWarnings::new(),
            is_loading: false,
            error: None,
        }
    }
}

impl OfflineState {
    pub fn clear_results(&mut self) {
        self.decode_result = None;
        self.hashes = None;
        self.warnings = SafeWarnings::new();
        self.error = None;
    }
}
