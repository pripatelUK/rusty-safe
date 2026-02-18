use alloy::primitives::{Address, B256, Bytes};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimestampMs(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureMethod {
    SafeTxHash,
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
    pub capabilities_json: Option<String>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureSource {
    InjectedProvider,
    WalletConnect,
    ImportedBundle,
    ManualEntry,
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
    pub chain_id: u64,
    pub safe_address: Address,
    pub safe_tx_hash: B256,
    pub build_source: TxBuildSource,
    pub abi_context: Option<AbiMethodContext>,
    pub state_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingSafeMessage {
    pub chain_id: u64,
    pub safe_address: Address,
    pub message_hash: B256,
    pub method: SignatureMethod,
    pub state_revision: u64,
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
