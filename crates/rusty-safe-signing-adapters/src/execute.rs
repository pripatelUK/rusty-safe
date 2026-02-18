use serde_json::Value;

use rusty_safe_signing_core::PortError;

#[derive(Debug, Clone, Default)]
pub struct ExecuteAdapter;

impl ExecuteAdapter {
    pub fn execute_transaction(&self, payload: &Value) -> Result<String, PortError> {
        let bytes = serde_json::to_vec(payload)
            .map_err(|e| PortError::Validation(format!("execute serialize failed: {e}")))?;
        Ok(format!(
            "0x{}",
            alloy::primitives::hex::encode(alloy::primitives::keccak256(bytes))
        ))
    }
}
