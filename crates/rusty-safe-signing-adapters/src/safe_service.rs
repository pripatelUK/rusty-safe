use alloy::primitives::B256;
use serde_json::Value;

use rusty_safe_signing_core::{PendingSafeTx, PortError, SafeServicePort};

#[derive(Debug, Clone, Default)]
pub struct SafeServiceAdapter;

impl SafeServicePort for SafeServiceAdapter {
    fn propose_tx(&self, _tx: &PendingSafeTx) -> Result<(), PortError> {
        Err(PortError::NotImplemented("safe_service.propose_tx"))
    }

    fn confirm_tx(&self, _safe_tx_hash: B256, _signature: &[u8]) -> Result<(), PortError> {
        Err(PortError::NotImplemented("safe_service.confirm_tx"))
    }

    fn fetch_status(&self, _safe_tx_hash: B256) -> Result<Value, PortError> {
        Err(PortError::NotImplemented("safe_service.fetch_status"))
    }
}
