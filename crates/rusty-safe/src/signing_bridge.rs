//! Bridge between egui shell and signing workspace crates.
//! This must remain the only shell-facing boundary for signing operations.

use alloy::primitives::{Address, Bytes, B256};
use serde_json::Value;
use std::sync::Arc;

use rusty_safe_signing_adapters::{
    AbiAdapter, Eip1193Adapter, HashingAdapter, QueueAdapter, SafeServiceAdapter,
    SystemClockAdapter, WalletConnectAdapter,
};
#[cfg(target_arch = "wasm32")]
use rusty_safe_signing_core::TxStatus;
use rusty_safe_signing_core::{
    AppWriterLock, ClockPort, CommandEnvelope, CommandResult, MessageMethod, Orchestrator,
    PendingSafeMessage, PendingSafeTx, PendingWalletConnectRequest, PortError, QueuePort,
    SigningBundle, SigningCommand, TransitionLogRecord, UrlImportEnvelope, WalletConnectPort,
    WcSessionAction, WcSessionContext,
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

#[derive(Clone)]
pub struct SigningBridge {
    orchestrator: Arc<SigningOrchestrator>,
}

impl Default for SigningBridge {
    fn default() -> Self {
        Self {
            orchestrator: Arc::new(SigningOrchestrator::new(
                Eip1193Adapter::default(),
                SafeServiceAdapter::default(),
                WalletConnectAdapter::default(),
                QueueAdapter::default(),
                AbiAdapter,
                HashingAdapter,
                SystemClockAdapter,
            )),
        }
    }
}

impl SigningBridge {
    pub fn connect_provider(&self) -> Result<CommandResult, PortError> {
        self.orchestrator.handle(SigningCommand::ConnectProvider)
    }

    pub fn recover_provider_events(
        &self,
        expected_chain_id: u64,
    ) -> Result<CommandResult, PortError> {
        self.orchestrator
            .handle(SigningCommand::RecoverProviderEvents { expected_chain_id })
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn connect_provider_runtime_async(&self) -> Result<CommandResult, PortError> {
        let _ = self
            .orchestrator
            .provider
            .wasm_request_accounts_async()
            .await?;
        let _ = self.orchestrator.provider.wasm_chain_id_async().await?;
        let _ = self
            .orchestrator
            .provider
            .wasm_wallet_get_capabilities_async()
            .await?;
        self.connect_provider()
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn sign_message_with_provider_async(
        &self,
        message_hash: B256,
    ) -> Result<CommandResult, PortError> {
        let msg = self
            .orchestrator
            .queue
            .load_message(message_hash)?
            .ok_or_else(|| PortError::NotFound(format!("message not found: {message_hash}")))?;
        let signer = self
            .orchestrator
            .provider
            .wasm_request_accounts_async()
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| PortError::Policy("NO_CONNECTED_ACCOUNT".to_owned()))?;
        let payload = canonical_message_payload(&msg)?;
        let signature = self
            .orchestrator
            .provider
            .wasm_sign_payload_async(msg.method, &payload, signer)
            .await?;
        self.add_message_signature(message_hash, signer, signature)
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn sign_tx_with_provider_async(
        &self,
        safe_tx_hash: B256,
    ) -> Result<CommandResult, PortError> {
        let tx = self
            .orchestrator
            .queue
            .load_tx(safe_tx_hash)?
            .ok_or_else(|| PortError::NotFound(format!("tx not found: {safe_tx_hash}")))?;
        let signer = self
            .orchestrator
            .provider
            .wasm_request_accounts_async()
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| PortError::Policy("NO_CONNECTED_ACCOUNT".to_owned()))?;
        let payload = tx.safe_tx_hash.as_slice().to_vec();
        let signature = self
            .orchestrator
            .provider
            .wasm_sign_payload_async(MessageMethod::EthSign, &payload, signer)
            .await?;
        self.add_tx_signature(tx.safe_tx_hash, signer, signature)
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn send_tx_with_provider_async(
        &self,
        safe_tx_hash: B256,
    ) -> Result<CommandResult, PortError> {
        let mut tx = self
            .orchestrator
            .queue
            .load_tx(safe_tx_hash)?
            .ok_or_else(|| PortError::NotFound(format!("tx not found: {safe_tx_hash}")))?;

        let threshold = threshold_from_payload(&tx.payload);
        if tx.signatures.len() < threshold {
            return Err(PortError::Validation(
                "tx is not ready to execute (threshold not met)".to_owned(),
            ));
        }
        let provider_chain_id = self.orchestrator.provider.wasm_chain_id_async().await?;
        if provider_chain_id != tx.chain_id {
            return Err(PortError::Policy(format!(
                "CHAIN_MISMATCH: expected {}, got {}",
                tx.chain_id, provider_chain_id
            )));
        }

        let state_before = format!("{:?}", tx.status);
        tx.status = TxStatus::Executing;
        tx.state_revision = tx.state_revision.saturating_add(1);
        tx.updated_at_ms = rusty_safe_signing_core::TimestampMs(self.orchestrator.clock.now_ms()?);
        self.orchestrator.queue.save_tx(&tx)?;

        let tx_hash = self
            .orchestrator
            .provider
            .wasm_send_transaction_async(&tx.payload)
            .await?;
        tx.status = TxStatus::Executed;
        tx.executed_tx_hash = Some(tx_hash);
        tx.state_revision = tx.state_revision.saturating_add(1);
        tx.updated_at_ms = rusty_safe_signing_core::TimestampMs(self.orchestrator.clock.now_ms()?);
        self.orchestrator.queue.save_tx(&tx)?;

        let rec = self.bridge_transition_record(
            format!("tx:{}", tx.safe_tx_hash),
            state_before,
            format!("{:?}", tx.status),
            Some(tx.idempotency_key.clone()),
            Some(format!("executed:{tx_hash}")),
        )?;
        self.orchestrator.queue.append_transition_log(rec.clone())?;
        Ok(CommandResult {
            transition: Some(rec),
            merge: None,
            exported_bundle: None,
            provider_recovery: None,
        })
    }

    pub fn wc_pair(&self, uri: String) -> Result<CommandResult, PortError> {
        self.orchestrator.handle(SigningCommand::WcPair { uri })
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn wc_pair_runtime_async(&self, uri: String) -> Result<(), PortError> {
        self.orchestrator
            .walletconnect
            .wasm_pair_async(&uri)
            .await?;
        self.wc_runtime_sync_async().await
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn wc_session_action_runtime_async(
        &self,
        topic: String,
        action: WcSessionAction,
    ) -> Result<(), PortError> {
        self.orchestrator
            .walletconnect
            .wasm_session_action_async(&topic, action)
            .await?;
        self.wc_runtime_sync_async().await
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn wc_runtime_sync_async(&self) -> Result<(), PortError> {
        self.orchestrator.walletconnect.wasm_sync_async().await?;
        let requests = self
            .orchestrator
            .walletconnect
            .wasm_list_pending_requests_async()
            .await?;
        for request in requests {
            self.orchestrator.queue.save_wc_request(&request)?;
        }
        Ok(())
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

    fn bridge_transition_record(
        &self,
        flow_id: String,
        state_before: String,
        state_after: String,
        side_effect_key: Option<String>,
        side_effect_outcome: Option<String>,
    ) -> Result<TransitionLogRecord, PortError> {
        let existing = self.orchestrator.queue.load_transition_log(&flow_id)?;
        let next_seq = existing.last().map(|x| x.event_seq + 1).unwrap_or(1);
        let now = rusty_safe_signing_core::TimestampMs(self.orchestrator.clock.now_ms()?);
        Ok(TransitionLogRecord {
            event_seq: next_seq,
            command_id: format!("bridge-async-{}", now.0),
            flow_id,
            state_before,
            state_after,
            side_effect_key,
            side_effect_dispatched: true,
            side_effect_outcome,
            recorded_at_ms: now,
        })
    }
}

fn threshold_from_payload(payload: &Value) -> usize {
    payload
        .get("threshold")
        .and_then(|v| v.as_u64())
        .map(|x| x.max(1) as usize)
        .unwrap_or(1)
}

fn canonical_message_payload(message: &PendingSafeMessage) -> Result<Vec<u8>, PortError> {
    if let Some(raw) = message.payload.get("message").and_then(|v| v.as_str()) {
        return Ok(raw.as_bytes().to_vec());
    }
    serde_json::to_vec(&message.payload)
        .map_err(|e| PortError::Validation(format!("message payload serialization failed: {e}")))
}
