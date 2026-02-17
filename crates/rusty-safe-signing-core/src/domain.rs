use alloy::primitives::{Address, B256};
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingSafeTx {
    pub chain_id: u64,
    pub safe_address: Address,
    pub safe_tx_hash: B256,
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
