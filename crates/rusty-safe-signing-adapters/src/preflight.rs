use serde_json::Value;

use rusty_safe_signing_core::PortError;

#[derive(Debug, Clone, Default)]
pub struct PreflightAdapter;

impl PreflightAdapter {
    pub fn run(&self, _payload: &Value) -> Result<Value, PortError> {
        Err(PortError::NotImplemented("preflight.run"))
    }
}
