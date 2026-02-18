//! Bridge between egui shell and signing workspace crates.
//! This is intentionally thin for Phase A0 and should remain UI-facing only.

use alloy::primitives::{Address, B256};

use rusty_safe_signing_adapters::{
    Eip1193Adapter, QueueAdapter, SafeServiceAdapter, WalletConnectAdapter,
};
use rusty_safe_signing_core::{
    Orchestrator, PortError, SigningCommand, UrlImportEnvelope, WcSessionAction,
};

pub struct SigningBridge {
    orchestrator: Orchestrator<Eip1193Adapter, SafeServiceAdapter, WalletConnectAdapter, QueueAdapter>,
}

impl Default for SigningBridge {
    fn default() -> Self {
        Self {
            orchestrator: Orchestrator::new(
                Eip1193Adapter,
                SafeServiceAdapter,
                WalletConnectAdapter,
                QueueAdapter,
            ),
        }
    }
}

impl SigningBridge {
    pub fn connect_provider(&self) -> Result<(), PortError> {
        self.orchestrator.handle(SigningCommand::ConnectProvider)
    }

    pub fn create_safe_tx_from_abi(
        &self,
        to: Address,
        abi_json: String,
        method_signature: String,
        args: Vec<String>,
    ) -> Result<(), PortError> {
        self.orchestrator.handle(SigningCommand::CreateSafeTxFromAbi {
            to,
            abi_json,
            method_signature,
            args,
        })
    }

    pub fn add_tx_signature(
        &self,
        safe_tx_hash: B256,
        signer: Address,
        signature: Vec<u8>,
    ) -> Result<(), PortError> {
        self.orchestrator.handle(SigningCommand::AddTxSignature {
            safe_tx_hash,
            signer,
            signature,
        })
    }

    pub fn propose_tx(&self, safe_tx_hash: B256) -> Result<(), PortError> {
        self.orchestrator
            .handle(SigningCommand::ProposeTx { safe_tx_hash })
    }

    pub fn wc_session_action(&self, topic: String, action: WcSessionAction) -> Result<(), PortError> {
        self.orchestrator
            .handle(SigningCommand::WalletConnectSessionAction { topic, action })
    }

    pub fn respond_walletconnect(&self, request_id: String) -> Result<(), PortError> {
        self.orchestrator
            .handle(SigningCommand::RespondWalletConnect { request_id })
    }

    pub fn import_url_payload(&self, envelope: UrlImportEnvelope) -> Result<(), PortError> {
        self.orchestrator
            .handle(SigningCommand::ImportUrlPayload { envelope })
    }
}
