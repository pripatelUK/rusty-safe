//! Calldata decoding types

use std::collections::HashMap;

/// Top-level decoded transaction
#[derive(Debug, Clone, Default)]
pub struct DecodedTransaction {
    pub raw_data: String,
    pub selector: String,
    pub kind: TransactionKind,
    pub status: OverallStatus,
}

/// Type of transaction calldata
#[derive(Debug, Clone, Default)]
pub enum TransactionKind {
    #[default]
    Empty,
    Single(SingleDecode),
    MultiSend(MultiSendDecode),
    Unknown,
}

/// Single function call decode (both sources)
#[derive(Debug, Clone, Default)]
pub struct SingleDecode {
    pub api: Option<ApiDecode>,
    pub local: Option<LocalDecode>,
    pub comparison: ComparisonResult,
}

/// MultiSend batch decode
#[derive(Debug, Clone, Default)]
pub struct MultiSendDecode {
    pub transactions: Vec<MultiSendTx>,
    pub summary: MultiSendSummary,
    pub verification_state: VerificationState,
}

/// Verification state for bulk operations
#[derive(Debug, Clone, Default)]
pub enum VerificationState {
    #[default]
    Pending,
    InProgress {
        total: usize,
    },
    Complete,
}

/// Single transaction within a MultiSend batch
#[derive(Debug, Clone)]
pub struct MultiSendTx {
    pub index: usize,
    pub operation: u8,
    pub to: String,
    pub value: String,
    pub data: String,
    /// API decode from Safe Transaction Service (available immediately)
    pub api_decode: Option<ApiDecode>,
    /// Full decode comparison (populated after bulk verification)
    pub decode: Option<SingleDecode>,
    /// UI-only: whether this item is expanded for viewing details
    pub is_expanded: bool,
}

/// Summary counts for MultiSend
#[derive(Debug, Clone, Default)]
pub struct MultiSendSummary {
    pub total: usize,
    pub verified: usize,
    pub mismatched: usize,
    pub pending: usize,
}

impl MultiSendSummary {
    pub fn update(&mut self, transactions: &[MultiSendTx]) {
        self.total = transactions.len();
        self.verified = 0;
        self.mismatched = 0;
        self.pending = 0;

        for tx in transactions {
            match &tx.decode {
                Some(d) => match &d.comparison {
                    ComparisonResult::Match => self.verified += 1,
                    ComparisonResult::MethodMismatch { .. }
                    | ComparisonResult::ParamMismatch(_) => self.mismatched += 1,
                    // OnlyApi/OnlyLocal = no independent verification possible
                    ComparisonResult::OnlyApi
                    | ComparisonResult::OnlyLocal
                    | ComparisonResult::Pending
                    | ComparisonResult::Failed(_) => self.pending += 1,
                },
                None => self.pending += 1,
            }
        }
    }
}

// --- API Decode (from Safe Transaction Service) ---

/// Decode provided by Safe API
#[derive(Debug, Clone)]
pub struct ApiDecode {
    pub method: String,
    pub params: Vec<ApiParam>,
}

/// Parameter from API decode
#[derive(Debug, Clone)]
pub struct ApiParam {
    pub name: String,
    pub typ: String,
    pub value: String,
}

// --- Local Decode (from 4byte + alloy) ---

/// Decode from local 4byte lookup + ABI decoding
#[derive(Debug, Clone)]
pub struct LocalDecode {
    pub signature: String,
    pub method: String,
    pub params: Vec<LocalParam>,
    /// Whether this signature comes from a verified contract on Sourcify
    pub verified: bool,
}

/// Parameter from local decode (no names, just types)
#[derive(Debug, Clone)]
pub struct LocalParam {
    pub typ: String,
    pub value: String,
}

// --- Comparison ---

/// Result of comparing API vs Local decode
#[derive(Debug, Clone, Default)]
pub enum ComparisonResult {
    #[default]
    Pending,
    Match,
    MethodMismatch {
        api: String,
        local: String,
    },
    ParamMismatch(Vec<ParamDiff>),
    OnlyApi,
    OnlyLocal,
    Failed(String),
}

impl ComparisonResult {
    pub fn is_match(&self) -> bool {
        matches!(self, ComparisonResult::Match)
    }

    pub fn is_mismatch(&self) -> bool {
        matches!(
            self,
            ComparisonResult::MethodMismatch { .. } | ComparisonResult::ParamMismatch(_)
        )
    }
}

/// Difference in a single parameter
#[derive(Debug, Clone)]
pub struct ParamDiff {
    pub index: usize,
    pub typ: String,
    pub api_value: String,
    pub local_value: String,
}

/// Overall status for the transaction
#[derive(Debug, Clone, Default)]
pub enum OverallStatus {
    #[default]
    Pending,
    AllMatch,
    HasMismatches,
    PartiallyVerified,
    Failed,
}

// --- Signature Cache ---

/// Cached signature lookups
#[derive(Debug, Clone, Default)]
pub struct SignatureCache {
    pub cache: HashMap<String, Vec<String>>,
}

impl SignatureCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn get(&self, selector: &str) -> Option<&Vec<String>> {
        self.cache.get(selector)
    }

    pub fn insert(&mut self, selector: String, signatures: Vec<String>) {
        self.cache.insert(selector, signatures);
    }

    pub fn contains(&self, selector: &str) -> bool {
        self.cache.contains_key(selector)
    }
}

// =============================================================================
// OFFLINE MODE TYPES
// =============================================================================

/// Status of offline decode (no API comparison, just 4byte lookup result)
#[derive(Debug, Clone)]
pub enum OfflineDecodeStatus {
    /// Successfully decoded via 4byte lookup (green ✅)
    Decoded,
    /// Selector not found in 4byte database (red ❌)
    Unknown(String),
    /// Decode failed with error (red ❌)
    Failed(String),
}

impl Default for OfflineDecodeStatus {
    fn default() -> Self {
        Self::Unknown(String::new())
    }
}

impl OfflineDecodeStatus {
    pub fn is_decoded(&self) -> bool {
        matches!(self, OfflineDecodeStatus::Decoded)
    }

    pub fn is_error(&self) -> bool {
        matches!(
            self,
            OfflineDecodeStatus::Unknown(_) | OfflineDecodeStatus::Failed(_)
        )
    }
}

/// Single transaction within an offline MultiSend batch
#[derive(Debug, Clone)]
pub struct OfflineMultiSendTx {
    pub index: usize,
    pub operation: u8,
    pub to: String,
    pub value: String,
    pub data: String,
    /// Local decode from 4byte lookup
    pub local_decode: Option<LocalDecode>,
    /// Decode status
    pub status: OfflineDecodeStatus,
    /// UI-only: whether this item is expanded
    pub is_expanded: bool,
}

/// Result of offline calldata decode
#[derive(Debug, Clone)]
pub enum OfflineDecodeResult {
    /// Empty calldata (native ETH transfer)
    Empty,
    /// Single function call
    Single {
        local: LocalDecode,
        status: OfflineDecodeStatus,
    },
    /// MultiSend batch
    MultiSend(Vec<OfflineMultiSendTx>),
    /// Could not parse calldata (shows raw hex)
    RawHex(String),
}

impl Default for OfflineDecodeResult {
    fn default() -> Self {
        Self::Empty
    }
}
