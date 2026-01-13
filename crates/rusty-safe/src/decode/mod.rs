//! Calldata decoding with independent verification
//!
//! Provides side-by-side comparison of:
//! - Safe API's decoded calldata
//! - Independent decode via 4byte signature lookup
//!
//! Supports nested calls (MultiSend batches).

mod compare;
mod offline;
pub mod parser;
mod sourcify;
pub mod types;
pub mod ui;
mod verify;

// Re-exports
pub use compare::compare_decodes;
pub use offline::decode_offline;
pub use parser::{
    decode_multisend_bytes, decode_with_signature, get_selector, parse_initial,
    unpack_multisend_transactions, MULTISEND_SELECTOR,
};
pub use sourcify::{SignatureInfo, SignatureLookup};
pub use types::*;
pub use ui::{render_decode_section, render_offline_decode_section, render_single_comparison};
pub use verify::verify_multisend_batch;

/// Log to console (works in both WASM and native)
///
/// Used throughout the decode module for debugging.
macro_rules! decode_log {
    ($($arg:tt)*) => {
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::console::log_1(&format!($($arg)*).into());
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            eprintln!("[decode] {}", format!($($arg)*));
        }
    };
}

// Make macro available to submodules
pub(crate) use decode_log;
