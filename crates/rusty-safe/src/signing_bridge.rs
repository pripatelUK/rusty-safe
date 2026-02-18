//! Bridge between egui shell and signing workspace crates.
//! This must remain the only shell-facing boundary for signing operations.

use alloy::primitives::{Address, Bytes, B256};
use serde_json::Value;

use rusty_safe_signing_adapters::{
    AbiAdapter, Eip1193Adapter, HashingAdapter, QueueAdapter, SafeServiceAdapter,
    SystemClockAdapter, WalletConnectAdapter,
};
use rusty_safe_signing_core::{
    AppWriterLock, CommandEnvelope, CommandResult, MessageMethod, Orchestrator, PendingSafeMessage,
    PendingSafeTx, PendingWalletConnectRequest, PortError, QueuePort, SigningBundle,
    SigningCommand, TransitionLogRecord, UrlImportEnvelope, WalletConnectPort, WcSessionAction,
    WcSessionContext,
};

type SigningOrchestrator = Orchestrator<
    Eip1193Adapter,
    SafeServiceAdapter,
    WalletConnectAdapter,
    QueueAdapter,
    AbiAdapter,
    HashingAdapter,
    SystemClockAdapter,
>;

pub struct SigningBridge {
    orchestrator: SigningOrchestrator,
}

impl Default for SigningBridge {
    fn default() -> Self {
        Self {
            orchestrator: SigningOrchestrator::new(
                Eip1193Adapter::default(),
                SafeServiceAdapter::default(),
                WalletConnectAdapter::default(),
                QueueAdapter::default(),
                AbiAdapter,
                HashingAdapter,
                SystemClockAdapter,
            ),
        }
    }
}

impl SigningBridge {
    pub fn connect_provider(&self) -> Result<CommandResult, PortError> {
        self.orchestrator.handle(SigningCommand::ConnectProvider)
    }

    pub fn acquire_writer_lock(
        &self,
        tab_id: String,
        tab_nonce: B256,
        ttl_ms: u64,
    ) -> Result<CommandResult, PortError> {
        self.orchestrator.handle(SigningCommand::AcquireWriterLock {
            tab_id,
            tab_nonce,
            ttl_ms,
        })
    }

    pub fn load_writer_lock(&self) -> Result<Option<AppWriterLock>, PortError> {
        self.orchestrator.queue.load_writer_lock()
    }

    pub fn create_safe_tx(
        &self,
        chain_id: u64,
        safe_address: Address,
        nonce: u64,
        payload: Value,
    ) -> Result<CommandResult, PortError> {
        self.orchestrator.handle(SigningCommand::CreateSafeTx {
            chain_id,
            safe_address,
            nonce,
            payload,
        })
    }

    pub fn create_safe_tx_from_abi(
        &self,
        chain_id: u64,
        safe_address: Address,
        nonce: u64,
        to: Address,
        abi_json: String,
        method_signature: String,
        args: Vec<String>,
        value: String,
    ) -> Result<CommandResult, PortError> {
        self.orchestrator
            .handle(SigningCommand::CreateSafeTxFromAbi {
                chain_id,
                safe_address,
                nonce,
                to,
                abi_json,
                method_signature,
                args,
                value,
            })
    }

    pub fn add_tx_signature(
        &self,
        safe_tx_hash: B256,
        signer: Address,
        signature: Bytes,
    ) -> Result<CommandResult, PortError> {
        self.orchestrator.handle(SigningCommand::AddTxSignature {
            safe_tx_hash,
            signer,
            signature,
        })
    }

    pub fn propose_tx(&self, safe_tx_hash: B256) -> Result<CommandResult, PortError> {
        self.orchestrator
            .handle(SigningCommand::ProposeTx { safe_tx_hash })
    }

    pub fn confirm_tx(
        &self,
        safe_tx_hash: B256,
        signature: Bytes,
    ) -> Result<CommandResult, PortError> {
        self.orchestrator.handle(SigningCommand::ConfirmTx {
            safe_tx_hash,
            signature,
        })
    }

    pub fn execute_tx(&self, safe_tx_hash: B256) -> Result<CommandResult, PortError> {
        self.orchestrator
            .handle(SigningCommand::ExecuteTx { safe_tx_hash })
    }

    pub fn create_message(
        &self,
        chain_id: u64,
        safe_address: Address,
        method: MessageMethod,
        payload: Value,
    ) -> Result<CommandResult, PortError> {
        self.orchestrator.handle(SigningCommand::CreateMessage {
            chain_id,
            safe_address,
            method,
            payload,
        })
    }

    pub fn add_message_signature(
        &self,
        message_hash: B256,
        signer: Address,
        signature: Bytes,
    ) -> Result<CommandResult, PortError> {
        self.orchestrator
            .handle(SigningCommand::AddMessageSignature {
                message_hash,
                signer,
                signature,
            })
    }

    pub fn wc_session_action(
        &self,
        topic: String,
        action: WcSessionAction,
    ) -> Result<CommandResult, PortError> {
        self.orchestrator
            .handle(SigningCommand::WcSessionAction { topic, action })
    }

    pub fn respond_walletconnect(
        &self,
        request_id: String,
        result: Value,
        deferred: bool,
    ) -> Result<CommandResult, PortError> {
        self.orchestrator
            .handle(SigningCommand::RespondWalletConnect {
                request_id,
                result,
                deferred,
            })
    }

    pub fn import_bundle(&self, bundle: SigningBundle) -> Result<CommandResult, PortError> {
        self.orchestrator
            .handle(SigningCommand::ImportBundle { bundle })
    }

    pub fn export_bundle(&self, flow_ids: Vec<String>) -> Result<CommandResult, PortError> {
        self.orchestrator
            .handle(SigningCommand::ExportBundle { flow_ids })
    }

    pub fn import_url_payload(
        &self,
        envelope: UrlImportEnvelope,
    ) -> Result<CommandResult, PortError> {
        self.orchestrator
            .handle(SigningCommand::ImportUrlPayload { envelope })
    }

    pub fn list_txs(&self) -> Result<Vec<PendingSafeTx>, PortError> {
        self.orchestrator.queue.list_txs()
    }

    pub fn list_messages(&self) -> Result<Vec<PendingSafeMessage>, PortError> {
        self.orchestrator.queue.list_messages()
    }

    pub fn list_wc_requests(&self) -> Result<Vec<PendingWalletConnectRequest>, PortError> {
        self.orchestrator.queue.list_wc_requests()
    }

    pub fn load_tx(&self, hash: B256) -> Result<Option<PendingSafeTx>, PortError> {
        self.orchestrator.queue.load_tx(hash)
    }

    pub fn load_message(&self, hash: B256) -> Result<Option<PendingSafeMessage>, PortError> {
        self.orchestrator.queue.load_message(hash)
    }

    pub fn load_wc_request(
        &self,
        id: &str,
    ) -> Result<Option<PendingWalletConnectRequest>, PortError> {
        self.orchestrator.queue.load_wc_request(id)
    }

    pub fn load_transition_log(
        &self,
        flow_id: &str,
    ) -> Result<Vec<TransitionLogRecord>, PortError> {
        self.orchestrator.queue.load_transition_log(flow_id)
    }

    pub fn list_wc_sessions(&self) -> Result<Vec<WcSessionContext>, PortError> {
        self.orchestrator.walletconnect.list_sessions()
    }

    pub fn dispatch_with_envelope(
        &self,
        envelope: CommandEnvelope,
        command: SigningCommand,
    ) -> Result<CommandResult, PortError> {
        self.orchestrator.handle_with_envelope(envelope, command)
    }

    pub fn debug_seed_wc_session(&self, session: WcSessionContext) -> Result<(), PortError> {
        self.orchestrator.walletconnect.insert_session(session)
    }

    pub fn debug_seed_wc_request(
        &self,
        request: PendingWalletConnectRequest,
    ) -> Result<(), PortError> {
        self.orchestrator.walletconnect.insert_request(request)
    }

    pub fn debug_save_wc_request(
        &self,
        request: PendingWalletConnectRequest,
    ) -> Result<(), PortError> {
        self.orchestrator.queue.save_wc_request(&request)
    }
}
