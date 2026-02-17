use serde_json::Value;

use rusty_safe_signing_core::{PortError, WalletConnectPort};

#[derive(Debug, Clone, Default)]
pub struct WalletConnectAdapter;

impl WalletConnectPort for WalletConnectAdapter {
    fn respond_success(&self, _request_id: &str, _result: Value) -> Result<(), PortError> {
        Err(PortError::NotImplemented("walletconnect.respond_success"))
    }

    fn respond_error(
        &self,
        _request_id: &str,
        _code: i64,
        _message: &str,
    ) -> Result<(), PortError> {
        Err(PortError::NotImplemented("walletconnect.respond_error"))
    }
}
