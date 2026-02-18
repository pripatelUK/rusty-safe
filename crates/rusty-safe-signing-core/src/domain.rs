use alloy::primitives::{Address, Bytes, B256};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TimestampMs(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageMethod {
    PersonalSign,
    EthSign,
    EthSignTypedData,
    EthSignTypedDataV4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WcMethod {
    EthSendTransaction,
    PersonalSign,
    EthSign,
    EthSignTypedData,
    EthSignTypedDataV4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxStatus {
    Draft,
    Signing,
    Proposed,
    Confirming,
    ReadyToExecute,
    Executing,
    Executed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageStatus {
    Draft,
    Signing,
    AwaitingThreshold,
    ThresholdMet,
    Responded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WcStatus {
    Pending,
    Routed,
    AwaitingThreshold,
    RespondingImmediate,
    RespondingDeferred,
    Responded,
    Expired,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WcSessionStatus {
    Proposed,
    Approved,
    Rejected,
    Disconnected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WcSessionAction {
    Approve,
    Reject,
    Disconnect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureSource {
    InjectedProvider,
    WalletConnect,
    ImportedBundle,
    ManualEntry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureMethod {
    SafeTxHash,
    PersonalSign,
    EthSign,
    EthSignTypedData,
    EthSignTypedDataV4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MacAlgorithm {
    HmacSha256V1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KdfAlgorithm {
    Argon2idV1,
    Pbkdf2HmacSha256V1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxBuildSource {
    RawCalldata,
    AbiMethodForm,
    UrlImport,
}

impl Default for TxBuildSource {
    fn default() -> Self {
        Self::RawCalldata
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbiMethodContext {
    pub abi_digest: B256,
    pub method_signature: String,
    pub method_selector: [u8; 4],
    pub encoded_args: Bytes,
    pub raw_calldata_override: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCapabilitySnapshot {
    pub wallet_get_capabilities_supported: bool,
    pub capabilities_json: Option<Value>,
    pub collected_at_ms: TimestampMs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WcSessionContext {
    pub topic: String,
    pub status: WcSessionStatus,
    pub dapp_name: Option<String>,
    pub dapp_url: Option<String>,
    pub dapp_icons: Vec<String>,
    pub capability_snapshot: Option<ProviderCapabilitySnapshot>,
    pub updated_at_ms: TimestampMs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UrlImportKey {
    #[serde(rename = "importTx")]
    ImportTx,
    #[serde(rename = "importSig")]
    ImportSig,
    #[serde(rename = "importMsg")]
    ImportMsg,
    #[serde(rename = "importMsgSig")]
    ImportMsgSig,
}

impl UrlImportKey {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ImportTx => "importTx",
            Self::ImportSig => "importSig",
            Self::ImportMsg => "importMsg",
            Self::ImportMsgSig => "importMsgSig",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlImportEnvelope {
    pub key: UrlImportKey,
    pub schema_version: u16,
    pub payload_base64url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectedSignature {
    pub signer: Address,
    pub signature: Bytes,
    pub source: SignatureSource,
    pub method: SignatureMethod,
    pub chain_id: u64,
    pub safe_address: Address,
    pub payload_hash: B256,
    pub expected_signer: Address,
    pub recovered_signer: Option<Address>,
    pub added_at_ms: TimestampMs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingSafeTx {
    pub schema_version: u16,
    pub chain_id: u64,
    pub safe_address: Address,
    pub nonce: u64,
    pub payload: Value,
    pub build_source: TxBuildSource,
    pub abi_context: Option<AbiMethodContext>,
    pub safe_tx_hash: B256,
    pub signatures: Vec<CollectedSignature>,
    pub status: TxStatus,
    pub state_revision: u64,
    pub idempotency_key: String,
    pub created_at_ms: TimestampMs,
    pub updated_at_ms: TimestampMs,
    pub executed_tx_hash: Option<B256>,
    pub mac_algorithm: MacAlgorithm,
    pub mac_key_id: String,
    pub integrity_mac: B256,
}

impl PendingSafeTx {
    pub fn signature_count(&self) -> usize {
        self.signatures.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingSafeMessage {
    pub schema_version: u16,
    pub chain_id: u64,
    pub safe_address: Address,
    pub method: MessageMethod,
    pub payload: Value,
    pub message_hash: B256,
    pub signatures: Vec<CollectedSignature>,
    pub status: MessageStatus,
    pub state_revision: u64,
    pub idempotency_key: String,
    pub created_at_ms: TimestampMs,
    pub updated_at_ms: TimestampMs,
    pub mac_algorithm: MacAlgorithm,
    pub mac_key_id: String,
    pub integrity_mac: B256,
}

impl PendingSafeMessage {
    pub fn signature_count(&self) -> usize {
        self.signatures.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingWalletConnectRequest {
    pub request_id: String,
    pub topic: String,
    pub session_status: WcSessionStatus,
    pub chain_id: u64,
    pub method: WcMethod,
    pub status: WcStatus,
    pub linked_safe_tx_hash: Option<B256>,
    pub linked_message_hash: Option<B256>,
    pub created_at_ms: TimestampMs,
    pub updated_at_ms: TimestampMs,
    pub expires_at_ms: Option<TimestampMs>,
    pub state_revision: u64,
    pub correlation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppWriterLock {
    pub holder_tab_id: String,
    pub tab_nonce: B256,
    pub lock_epoch: u64,
    pub acquired_at_ms: TimestampMs,
    pub expires_at_ms: TimestampMs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandEnvelope {
    pub command_id: String,
    pub correlation_id: String,
    pub parity_capability_id: String,
    pub idempotency_key: String,
    pub issued_at_ms: TimestampMs,
    pub command_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionLogRecord {
    pub event_seq: u64,
    pub command_id: String,
    pub flow_id: String,
    pub state_before: String,
    pub state_after: String,
    pub side_effect_key: Option<String>,
    pub side_effect_dispatched: bool,
    pub side_effect_outcome: Option<String>,
    pub recorded_at_ms: TimestampMs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SigningBundle {
    pub schema_version: u16,
    pub exported_at_ms: TimestampMs,
    pub exporter: Address,
    pub bundle_digest: B256,
    pub bundle_signature: Bytes,
    pub txs: Vec<PendingSafeTx>,
    pub messages: Vec<PendingSafeMessage>,
    pub wc_requests: Vec<PendingWalletConnectRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crypto_envelope: Option<BundleCryptoEnvelope>,
    pub mac_algorithm: MacAlgorithm,
    pub mac_key_id: String,
    pub integrity_mac: B256,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleCryptoEnvelope {
    pub kdf_algorithm: KdfAlgorithm,
    pub kdf_salt_base64: String,
    pub enc_nonce_base64: String,
    pub ciphertext_base64: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeResult {
    pub tx_added: usize,
    pub tx_updated: usize,
    pub tx_skipped: usize,
    pub tx_conflicted: usize,
    pub message_added: usize,
    pub message_updated: usize,
    pub message_skipped: usize,
    pub message_conflicted: usize,
}

impl MergeResult {
    pub fn empty() -> Self {
        Self {
            tx_added: 0,
            tx_updated: 0,
            tx_skipped: 0,
            tx_conflicted: 0,
            message_added: 0,
            message_updated: 0,
            message_skipped: 0,
            message_conflicted: 0,
        }
    }
}
