//! Safe Transaction Service API client
//!
//! Re-exports types from safe-hash library.

// Re-export API types from safe-hash
pub use safe_hash::{
    SafeTransaction, SafeApiResponse, Confirmation, DataDecoded, Parameter, Mismatch,
    get_safe_transaction_async,
    validate_safe_tx_hash,
};

// Re-export hash types
pub use safe_hash::{TxInput, tx_signing_hashes, SafeHashes};

// Re-export warning check
pub use safe_hash::{check_suspicious_content, SafeWarnings};
