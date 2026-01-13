//! Safe Transaction Service API client
//!
//! Re-exports types from safe-hash library.

// Re-export API types from safe-hash
pub use safe_hash::{
    get_safe_transaction_async, validate_safe_tx_hash, Confirmation, DataDecoded, Mismatch,
    Parameter, SafeApiResponse, SafeTransaction,
};

// Re-export hash types
pub use safe_hash::{tx_signing_hashes, SafeHashes, TxInput};

// Re-export warning check
pub use safe_hash::{check_suspicious_content, SafeWarnings};
