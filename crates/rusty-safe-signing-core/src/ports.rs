use alloy::primitives::{Address, Bytes, B256};
use serde_json::Value;
use thiserror::Error;

use crate::domain::{
    AppWriterLock, MergeResult, MessageMethod, PendingSafeMessage, PendingSafeTx,
    PendingWalletConnectRequest, SigningBundle, TransitionLogRecord, UrlImportEnvelope,
    WcSessionAction, WcSessionContext,
};

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PortError {
    #[error("port not implemented: {0}")]
    NotImplemented(&'static str),
    #[error("transport error: {0}")]
    Transport(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("policy violation: {0}")]
    Policy(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderEventKind {
    AccountsChanged,
    ChainChanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderEvent {
    pub sequence: u64,
    pub kind: ProviderEventKind,
    pub value: String,
}

pub trait ClockPort {
    fn now_ms(&self) -> Result<u64, PortError>;
}

pub trait ProviderPort {
    fn request_accounts(&self) -> Result<Vec<Address>, PortError>;
    fn chain_id(&self) -> Result<u64, PortError>;
    fn wallet_get_capabilities(&self) -> Result<Option<Value>, PortError>;
    fn sign_payload(
        &self,
        method: MessageMethod,
        payload: &[u8],
        expected_signer: Address,
    ) -> Result<Bytes, PortError>;
    fn send_transaction(&self, tx_payload: &Value) -> Result<B256, PortError>;
    fn drain_events(&self) -> Result<Vec<ProviderEvent>, PortError> {
        Ok(Vec::new())
    }
}

pub trait SafeServicePort {
    fn propose_tx(&self, tx: &PendingSafeTx) -> Result<(), PortError>;
    fn confirm_tx(&self, safe_tx_hash: B256, signature: &Bytes) -> Result<(), PortError>;
    fn execute_tx(&self, tx: &PendingSafeTx) -> Result<B256, PortError>;
    fn fetch_status(&self, safe_tx_hash: B256) -> Result<Value, PortError>;
}

pub trait WalletConnectPort {
    fn pair(&self, _uri: &str) -> Result<(), PortError> {
        Err(PortError::NotImplemented("walletconnect pair"))
    }
    fn session_action(&self, topic: &str, action: WcSessionAction) -> Result<(), PortError>;
    fn list_sessions(&self) -> Result<Vec<WcSessionContext>, PortError>;
    fn list_pending_requests(&self) -> Result<Vec<PendingWalletConnectRequest>, PortError>;
    fn respond_success(&self, request_id: &str, result: Value) -> Result<(), PortError>;
    fn respond_error(&self, request_id: &str, code: i64, message: &str) -> Result<(), PortError>;
    fn sync(&self) -> Result<(), PortError> {
        Ok(())
    }
}

pub trait QueuePort {
    fn acquire_writer_lock(&self, lock: AppWriterLock) -> Result<AppWriterLock, PortError>;
    fn load_writer_lock(&self) -> Result<Option<AppWriterLock>, PortError>;
    fn release_writer_lock(&self, holder_tab_id: &str) -> Result<(), PortError>;

    fn save_tx(&self, tx: &PendingSafeTx) -> Result<(), PortError>;
    fn save_message(&self, message: &PendingSafeMessage) -> Result<(), PortError>;
    fn save_wc_request(&self, request: &PendingWalletConnectRequest) -> Result<(), PortError>;

    fn load_tx(&self, safe_tx_hash: B256) -> Result<Option<PendingSafeTx>, PortError>;
    fn load_message(&self, message_hash: B256) -> Result<Option<PendingSafeMessage>, PortError>;
    fn load_wc_request(
        &self,
        request_id: &str,
    ) -> Result<Option<PendingWalletConnectRequest>, PortError>;

    fn list_txs(&self) -> Result<Vec<PendingSafeTx>, PortError>;
    fn list_messages(&self) -> Result<Vec<PendingSafeMessage>, PortError>;
    fn list_wc_requests(&self) -> Result<Vec<PendingWalletConnectRequest>, PortError>;

    fn append_transition_log(&self, record: TransitionLogRecord) -> Result<(), PortError>;
    fn load_transition_log(&self, flow_id: &str) -> Result<Vec<TransitionLogRecord>, PortError>;

    fn import_bundle(&self, bundle: &SigningBundle) -> Result<MergeResult, PortError>;
    fn export_bundle(&self, flow_ids: &[String]) -> Result<SigningBundle, PortError>;
    fn import_url_payload(&self, envelope: &UrlImportEnvelope) -> Result<MergeResult, PortError>;
}

pub trait AbiPort {
    fn encode_calldata(
        &self,
        abi_json: &str,
        method_signature: &str,
        args: &[String],
    ) -> Result<(Bytes, [u8; 4]), PortError>;
    fn selector_from_method_signature(&self, method_signature: &str) -> Result<[u8; 4], PortError>;
}

pub trait HashingPort {
    fn safe_tx_hash(
        &self,
        chain_id: u64,
        safe_address: Address,
        nonce: u64,
        payload: &Value,
    ) -> Result<B256, PortError>;
    fn message_hash(
        &self,
        chain_id: u64,
        safe_address: Address,
        method: MessageMethod,
        payload: &Value,
    ) -> Result<B256, PortError>;
    fn integrity_mac(&self, payload: &[u8], key_id: &str) -> Result<B256, PortError>;
}
