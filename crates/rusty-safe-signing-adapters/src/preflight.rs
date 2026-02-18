use serde_json::Value;

use rusty_safe_signing_core::PortError;

#[derive(Debug, Clone, Default)]
pub struct PreflightAdapter;

impl PreflightAdapter {
    pub fn run(&self, payload: &Value) -> Result<Value, PortError> {
        Ok(serde_json::json!({
            "ok": true,
            "warnings": [],
            "payloadDigest": alloy::primitives::keccak256(
                serde_json::to_vec(payload)
                    .map_err(|e| PortError::Validation(format!("preflight serialize failed: {e}")))?,
            ),
        }))
    }
}
