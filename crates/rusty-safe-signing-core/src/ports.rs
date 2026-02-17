use alloy::primitives::{Address, B256};
use serde_json::Value;
use thiserror::Error;

use crate::domain::{PendingSafeMessage, PendingSafeTx, SignatureMethod};

#[derive(Debug, Error)]
pub enum PortError {
    #[error("port not implemented: {0}")]
    NotImplemented(&'static str),
    #[error("transport error: {0}")]
    Transport(String),
    #[error("validation error: {0}")]
    Validation(String),
}

pub trait ProviderPort {
    fn request_accounts(&self) -> Result<Vec<Address>, PortError>;
    fn chain_id(&self) -> Result<u64, PortError>;
    fn sign_payload(
        &self,
        method: SignatureMethod,
        payload: &[u8],
        expected_signer: Address,
    ) -> Result<Vec<u8>, PortError>;
    fn send_transaction(&self, tx_payload: &Value) -> Result<B256, PortError>;
}

pub trait SafeServicePort {
    fn propose_tx(&self, tx: &PendingSafeTx) -> Result<(), PortError>;
    fn confirm_tx(&self, safe_tx_hash: B256, signature: &[u8]) -> Result<(), PortError>;
    fn fetch_status(&self, safe_tx_hash: B256) -> Result<Value, PortError>;
}

pub trait WalletConnectPort {
    fn respond_success(&self, request_id: &str, result: Value) -> Result<(), PortError>;
    fn respond_error(
        &self,
        request_id: &str,
        code: i64,
        message: &str,
    ) -> Result<(), PortError>;
}

pub trait QueuePort {
    fn save_tx(&self, tx: &PendingSafeTx) -> Result<(), PortError>;
    fn save_message(&self, message: &PendingSafeMessage) -> Result<(), PortError>;
    fn load_tx(&self, safe_tx_hash: B256) -> Result<Option<PendingSafeTx>, PortError>;
    fn load_message(&self, message_hash: B256) -> Result<Option<PendingSafeMessage>, PortError>;
}
