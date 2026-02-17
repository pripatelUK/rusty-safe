use serde_json::Value;

use rusty_safe_signing_core::PortError;

#[derive(Debug, Clone, Default)]
pub struct ExecuteAdapter;

impl ExecuteAdapter {
    pub fn execute_transaction(&self, _payload: &Value) -> Result<String, PortError> {
        Err(PortError::NotImplemented("execute.execute_transaction"))
    }
}
