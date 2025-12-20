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
/// Storage key for address book
const ADDRESS_BOOK_KEY: &str = "address_book";
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
    pub address_book: AddressBook,
}

/// Address book entry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct AddressBookEntry {
    pub address: String,
    pub name: String,
    pub chain_id: u64,
}

/// Result of address validation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressValidation {
    Valid,
    ChecksumMismatch,
    Invalid,
}

/// Check if a value looks like an Ethereum address and validate checksum
pub fn validate_address(value: &str) -> AddressValidation {
    if !value.starts_with("0x") || value.len() != 42 {
        return AddressValidation::Invalid;
    }

    if !value[2..].chars().all(|c| c.is_ascii_hexdigit()) {
        return AddressValidation::Invalid;
    }

    // Check if it's a valid checksummed address or all lower/upper
    match value.parse::<alloy::primitives::Address>() {
        Ok(addr) => {
            let checksummed = addr.to_checksum(None);
            // EIP-55: if it's all lowercase or all uppercase, it's valid (just not checksummed)
            if value == checksummed || value[2..] == value[2..].to_lowercase() || value[2..] == value[2..].to_uppercase() {
                AddressValidation::Valid
            } else {
                AddressValidation::ChecksumMismatch
            }
        }
        Err(_) => AddressValidation::Invalid,
    }
}

/// Normalize an address to EIP-55 checksummed format
pub fn normalize_address(value: &str) -> Option<String> {
    match value.parse::<alloy::primitives::Address>() {
        Ok(addr) => Some(addr.to_checksum(None)),
        Err(_) => None,
    }
}

/// Address book collection
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AddressBook {
    pub entries: Vec<AddressBookEntry>,
}

impl AddressBook {
    pub fn get_name(&self, address: &str, chain_id: u64) -> Option<String> {
        let addr_lower = address.to_lowercase();
        self.entries.iter()
            .find(|e| e.address.to_lowercase() == addr_lower && e.chain_id == chain_id)
            .map(|e| e.name.clone())
    }

    pub fn add_or_update(&mut self, mut entry: AddressBookEntry) {
        // Normalize address
        if let Some(normalized) = normalize_address(&entry.address) {
            entry.address = normalized;
        }

        let addr_lower = entry.address.to_lowercase();
        if let Some(existing) = self.entries.iter_mut()
            .find(|e| e.address.to_lowercase() == addr_lower && e.chain_id == entry.chain_id) 
        {
            existing.name = entry.name;
        } else {
            self.entries.push(entry);
        }
    }

    pub fn remove(&mut self, address: &str, chain_id: u64) {
        let addr_lower = address.to_lowercase();
        self.entries.retain(|e| e.address.to_lowercase() != addr_lower || e.chain_id != chain_id);
    }

    pub fn validate_entry(&self, entry: &AddressBookEntry) -> AddressValidation {
        validate_address(&entry.address)
    }

    /// Import from CSV: address,name,chainId
    pub fn import_csv(&mut self, csv_content: &str) -> Result<(usize, usize), String> {
        let mut count = 0;
        let mut skipped = 0;
        for (i, line) in csv_content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("address,") {
                continue;
            }
            
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 3 {
                skipped += 1;
                continue;
            }
            
            let address = parts[0].trim().to_string();
            let name = parts[1].trim().to_string();
            let chain_id = parts[2].trim().parse::<u64>().map_err(|_| format!("Invalid chainId on line {}", i + 1))?;
            
            if validate_address(&address) == AddressValidation::Invalid {
                skipped += 1;
                continue;
            }

            self.add_or_update(AddressBookEntry { address, name, chain_id });
            count += 1;
        }
        Ok((count, skipped))
    }

    /// Export to CSV: address,name,chainId
    pub fn export_csv(&self) -> String {
        let mut csv = String::from("address,name,chainId\n");
        for entry in &self.entries {
            csv.push_str(&format!("{},{},{}\n", entry.address, entry.name, entry.chain_id));
        }
        csv
    }
}

impl SafeContext {
    /// Load SafeContext from eframe storage
    pub fn load(storage: Option<&dyn eframe::Storage>) -> Self {
        let chains = get_all_supported_chain_names();
        let default_chain = chains.iter()
            .find(|c| *c == "ethereum")
            .cloned()
            .unwrap_or_else(|| chains.first().cloned().unwrap_or_default());
        
        let (safe_address, recent_addresses, address_book) = if let Some(storage) = storage {
            let addr = storage.get_string(SAFE_ADDRESS_KEY).unwrap_or_default();
            let recent: Vec<String> = storage.get_string(RECENT_ADDRESSES_KEY)
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            let book: AddressBook = storage.get_string(ADDRESS_BOOK_KEY)
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            (addr, recent, book)
        } else {
            (String::new(), Vec::new(), AddressBook::default())
        };
        
        Self {
            chain_name: default_chain,
            safe_address,
            safe_version: SAFE_VERSIONS[0].to_string(),
            recent_addresses,
            address_book,
        }
    }
    
    /// Save SafeContext to eframe storage
    pub fn save(&self, storage: &mut dyn eframe::Storage) {
        storage.set_string(SAFE_ADDRESS_KEY, self.safe_address.clone());
        if let Ok(json) = serde_json::to_string(&self.recent_addresses) {
            storage.set_string(RECENT_ADDRESSES_KEY, json);
        }
        if let Ok(json) = serde_json::to_string(&self.address_book) {
            storage.set_string(ADDRESS_BOOK_KEY, json);
        }
    }
    
    /// Clear all stored data
    pub fn clear(&mut self) {
        self.safe_address.clear();
        self.recent_addresses.clear();
        self.address_book.entries.clear();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_book_csv() {
        let mut book = AddressBook::default();
        // One valid checksummed, one valid unchecksummed (lowercase), one invalid
        let csv = "address,name,chainId\n0x4F2083f5fBede34C2714aFfb3105539775f7FE64,endowment.ensdao.eth,1\n0xfe89cc7abb2c4183683ab71653c4cdc9b02d44b7,test,1\n0xinvalid,bad,1";
        
        let (count, skipped) = book.import_csv(csv).unwrap();
        assert_eq!(count, 2);
        assert_eq!(skipped, 1);
        assert_eq!(book.entries.len(), 2);
        
        // Both should be checksummed now
        assert_eq!(book.entries[0].address, "0x4F2083f5fBede34C2714aFfb3105539775f7FE64");
        assert_eq!(book.entries[1].address, "0xFe89cc7aBB2C4183683ab71653C4cdc9B02D44b7");
        
        assert_eq!(book.get_name("0x4F2083f5fBede34C2714aFfb3105539775f7FE64", 1), Some("endowment.ensdao.eth".to_string()));
        assert_eq!(book.get_name("0xfe89cc7abb2c4183683ab71653c4cdc9b02d44b7", 1), Some("test".to_string()));
    }

    #[test]
    fn test_address_book_update() {
        let mut book = AddressBook::default();
        book.add_or_update(AddressBookEntry {
            address: "0x123".to_string(),
            name: "Old".to_string(),
            chain_id: 1,
        });
        book.add_or_update(AddressBookEntry {
            address: "0x123".to_string(),
            name: "New".to_string(),
            chain_id: 1,
        });
        
        assert_eq!(book.entries.len(), 1);
        assert_eq!(book.get_name("0x123", 1), Some("New".to_string()));
    }
}
