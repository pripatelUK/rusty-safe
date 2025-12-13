//! Application state types
//!
//! Uses types from safe-hash library where possible.
//! UI state structs are rusty-safe specific.
//!
//! Storage is handled via eframe's built-in persistence (works on both WASM and native).

use crate::api::SafeTransaction;
use crate::decode::DecodedTransaction;
use crate::expected::ExpectedState;
use safe_hash::SafeWarnings;
use safe_utils::get_all_supported_chain_names;

/// Storage key for cached Safe address
const SAFE_ADDRESS_KEY: &str = "safe_address";
/// Storage key for recent addresses  
const RECENT_ADDRESSES_KEY: &str = "recent_addresses";
/// Max recent addresses to keep
const MAX_RECENT_ADDRESSES: usize = 10;

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

impl SafeContext {
    /// Load SafeContext from eframe storage
    pub fn load(storage: Option<&dyn eframe::Storage>) -> Self {
        let chains = get_all_supported_chain_names();
        let default_chain = chains.iter()
            .find(|c| *c == "ethereum")
            .cloned()
            .unwrap_or_else(|| chains.first().cloned().unwrap_or_default());
        
        let (safe_address, recent_addresses) = if let Some(storage) = storage {
            let addr = storage.get_string(SAFE_ADDRESS_KEY).unwrap_or_default();
            let recent: Vec<String> = storage.get_string(RECENT_ADDRESSES_KEY)
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            (addr, recent)
        } else {
            (String::new(), Vec::new())
        };
        
        Self {
            chain_name: default_chain,
            safe_address,
            safe_version: SAFE_VERSIONS[0].to_string(),
            recent_addresses,
        }
    }
    
    /// Save SafeContext to eframe storage
    pub fn save(&self, storage: &mut dyn eframe::Storage) {
        storage.set_string(SAFE_ADDRESS_KEY, self.safe_address.clone());
        if let Ok(json) = serde_json::to_string(&self.recent_addresses) {
            storage.set_string(RECENT_ADDRESSES_KEY, json);
        }
    }
    
    /// Clear all stored data
    pub fn clear(&mut self) {
        self.safe_address.clear();
        self.recent_addresses.clear();
    }
}

impl Default for SafeContext {
    fn default() -> Self {
        Self::load(None)
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
