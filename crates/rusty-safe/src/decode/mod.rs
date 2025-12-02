//! Calldata decoding with independent verification
//!
//! Provides side-by-side comparison of:
//! - Safe API's decoded calldata
//! - Independent decode via 4byte signature lookup
//!
//! Supports nested calls (MultiSend batches).

mod compare;
pub mod parser;
mod sourcify;
pub mod types;
pub mod ui;

pub use compare::compare_decodes;
pub use parser::{parse_initial, decode_with_signature, get_selector};
pub use sourcify::SignatureLookup;
pub use types::*;
pub use ui::{render_decode_section, render_single_comparison};

