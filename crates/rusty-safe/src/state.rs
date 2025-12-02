//! Application state types
//!
//! UI state structs are rusty-safe specific.
//! API types are in api.rs (mirroring safe-hash which is a binary).

use crate::api::SafeTransaction;
use safe_utils::get_all_supported_chain_names;

/// Safe versions supported
pub const SAFE_VERSIONS: &[&str] = &[
    "1.4.1", "1.4.0", "1.3.0", "1.2.0", "1.1.1", "1.1.0", "1.0.0",
];

/// Transaction verification UI state
#[derive(Debug)]
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

    // Fetched from API (uses api::SafeTransaction)
    pub fetched_tx: Option<SafeTransaction>,

    // Computed hashes
    pub hashes: Option<ComputedHashes>,

    // Warnings
    pub warnings: Vec<Warning>,

    // Loading state
    pub is_loading: bool,

    // Error
    pub error: Option<String>,
}

/// Computed hash results
#[derive(Debug, Clone)]
pub struct ComputedHashes {
    pub domain_hash: String,
    pub message_hash: String,
    pub safe_tx_hash: String,
    pub matches_api: Option<bool>,
}

/// Security warnings (rusty-safe specific)
#[derive(Debug, Clone)]
pub enum Warning {
    DelegateCall,
    DangerousMethod(String),
    HashMismatch,
    NonceMismatch { expected: u64, actual: u64 },
    NonZeroGasToken,
    NonZeroRefundReceiver,
}

impl Warning {
    pub fn message(&self) -> String {
        match self {
            Warning::DelegateCall => "⚠️ DELEGATECALL - can modify Safe state!".to_string(),
            Warning::DangerousMethod(m) => format!("⚠️ Dangerous method: {}", m),
            Warning::HashMismatch => "⚠️ Computed hash doesn't match API!".to_string(),
            Warning::NonceMismatch { expected, actual } => {
                format!("Nonce mismatch: expected {}, got {}", expected, actual)
            }
            Warning::NonZeroGasToken => "Non-zero gas token".to_string(),
            Warning::NonZeroRefundReceiver => "Non-zero refund receiver".to_string(),
        }
    }

    pub fn severity(&self) -> Severity {
        match self {
            Warning::DelegateCall | Warning::HashMismatch => Severity::Critical,
            Warning::DangerousMethod(_) | Warning::NonceMismatch { .. } => Severity::High,
            Warning::NonZeroGasToken | Warning::NonZeroRefundReceiver => Severity::Medium,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn color(&self) -> egui::Color32 {
        match self {
            Severity::Medium => egui::Color32::from_rgb(220, 180, 50),
            Severity::High => egui::Color32::from_rgb(220, 120, 50),
            Severity::Critical => egui::Color32::from_rgb(220, 50, 50),
        }
    }
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
            fetched_tx: None,
            hashes: None,
            warnings: Vec::new(),
            is_loading: false,
            error: None,
        }
    }
}

impl TxVerifyState {
    pub fn clear_results(&mut self) {
        self.fetched_tx = None;
        self.hashes = None;
        self.warnings.clear();
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
