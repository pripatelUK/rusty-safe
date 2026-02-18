use alloy::primitives::{Address, Bytes, B256};
use serde_json::Value;

use crate::domain::{
    AppWriterLock, CollectedSignature, CommandEnvelope, MacAlgorithm, MergeResult, MessageMethod,
    MessageStatus, PendingSafeMessage, PendingSafeTx, SignatureMethod, SignatureSource,
    TimestampMs, TransitionLogRecord, TxBuildSource, TxStatus, UrlImportEnvelope, WcSessionAction,
    WcSessionStatus, WcStatus,
};
use crate::ports::{
    AbiPort, ClockPort, HashingPort, PortError, ProviderPort, QueuePort, SafeServicePort,
    WalletConnectPort,
};
use crate::state_machine::{
    message_transition, tx_transition, wc_transition, MessageAction, TxAction, WcAction,
};

#[derive(Debug, Clone)]
pub enum SigningCommand {
    ConnectProvider,
    AcquireWriterLock {
        tab_id: String,
        tab_nonce: B256,
        ttl_ms: u64,
    },
    CreateSafeTx {
        chain_id: u64,
        safe_address: Address,
        nonce: u64,
        payload: Value,
    },
    CreateSafeTxFromAbi {
        chain_id: u64,
        safe_address: Address,
        nonce: u64,
        to: Address,
        abi_json: String,
        method_signature: String,
        args: Vec<String>,
        value: String,
    },
    AddTxSignature {
        safe_tx_hash: B256,
        signer: Address,
        signature: Bytes,
    },
    ProposeTx {
        safe_tx_hash: B256,
    },
    ConfirmTx {
        safe_tx_hash: B256,
        signature: Bytes,
    },
    ExecuteTx {
        safe_tx_hash: B256,
    },
    CreateMessage {
        chain_id: u64,
        safe_address: Address,
        method: MessageMethod,
        payload: Value,
    },
    AddMessageSignature {
        message_hash: B256,
        signer: Address,
        signature: Bytes,
    },
    WcSessionAction {
        topic: String,
        action: WcSessionAction,
    },
    RespondWalletConnect {
        request_id: String,
        result: Value,
        deferred: bool,
    },
    ImportBundle {
        bundle: crate::domain::SigningBundle,
    },
    ExportBundle {
        flow_ids: Vec<String>,
    },
    ImportUrlPayload {
        envelope: UrlImportEnvelope,
    },
}

#[derive(Debug, Clone)]
pub struct CommandResult {
    pub transition: Option<TransitionLogRecord>,
    pub merge: Option<MergeResult>,
    pub exported_bundle: Option<crate::domain::SigningBundle>,
}

impl CommandResult {
    fn empty() -> Self {
        Self {
            transition: None,
            merge: None,
            exported_bundle: None,
        }
    }
}

pub struct Orchestrator<P, S, W, Q, A, H, C>
where
    P: ProviderPort,
    S: SafeServicePort,
    W: WalletConnectPort,
    Q: QueuePort,
    A: AbiPort,
    H: HashingPort,
    C: ClockPort,
{
    pub provider: P,
    pub safe_service: S,
    pub walletconnect: W,
    pub queue: Q,
    pub abi: A,
    pub hashing: H,
    pub clock: C,
}

impl<P, S, W, Q, A, H, C> Orchestrator<P, S, W, Q, A, H, C>
where
    P: ProviderPort,
    S: SafeServicePort,
    W: WalletConnectPort,
    Q: QueuePort,
    A: AbiPort,
    H: HashingPort,
    C: ClockPort,
{
    pub fn new(
        provider: P,
        safe_service: S,
        walletconnect: W,
        queue: Q,
        abi: A,
        hashing: H,
        clock: C,
    ) -> Self {
        Self {
            provider,
            safe_service,
            walletconnect,
            queue,
            abi,
            hashing,
            clock,
        }
    }

    pub fn handle_with_envelope(
        &self,
        envelope: CommandEnvelope,
        command: SigningCommand,
    ) -> Result<CommandResult, PortError> {
        match command {
            SigningCommand::ConnectProvider => {
                let _ = self.provider.request_accounts()?;
                let _ = self.provider.chain_id()?;
                let _ = self.provider.wallet_get_capabilities();
                Ok(CommandResult::empty())
            }
            SigningCommand::AcquireWriterLock {
                tab_id,
                tab_nonce,
                ttl_ms,
            } => {
                let now = TimestampMs(self.clock.now_ms()?);
                let lock = AppWriterLock {
                    holder_tab_id: tab_id,
                    tab_nonce,
                    lock_epoch: now.0,
                    acquired_at_ms: now,
                    expires_at_ms: TimestampMs(now.0.saturating_add(ttl_ms)),
                };
                let _ = self.queue.acquire_writer_lock(lock)?;
                Ok(CommandResult::empty())
            }
            SigningCommand::CreateSafeTx {
                chain_id,
                safe_address,
                nonce,
                payload,
            } => {
                self.ensure_writer_lock()?;
                self.create_safe_tx_internal(
                    envelope,
                    chain_id,
                    safe_address,
                    nonce,
                    payload,
                    TxBuildSource::RawCalldata,
                    None,
                )
            }
            SigningCommand::CreateSafeTxFromAbi {
                chain_id,
                safe_address,
                nonce,
                to,
                abi_json,
                method_signature,
                args,
                value,
            } => {
                self.ensure_writer_lock()?;
                let (data, selector) =
                    self.abi
                        .encode_calldata(&abi_json, &method_signature, &args)?;
                let payload = serde_json::json!({
                    "to": to,
                    "value": value,
                    "data": data,
                    "operation": 0u8,
                    "safeTxGas": "0",
                    "baseGas": "0",
                    "gasPrice": "0",
                    "gasToken": Address::ZERO,
                    "refundReceiver": Address::ZERO,
                    "threshold": 1,
                    "safeVersion": "1.3.0",
                });
                let abi_digest = self
                    .hashing
                    .integrity_mac(abi_json.as_bytes(), "abi-digest")?;
                let ctx = crate::domain::AbiMethodContext {
                    abi_digest,
                    method_signature,
                    method_selector: selector,
                    encoded_args: data,
                    raw_calldata_override: false,
                };
                self.create_safe_tx_internal(
                    envelope,
                    chain_id,
                    safe_address,
                    nonce,
                    payload,
                    TxBuildSource::AbiMethodForm,
                    Some(ctx),
                )
            }
            SigningCommand::AddTxSignature {
                safe_tx_hash,
                signer,
                signature,
            } => {
                self.ensure_writer_lock()?;
                self.add_tx_signature_internal(envelope, safe_tx_hash, signer, signature)
            }
            SigningCommand::ProposeTx { safe_tx_hash } => {
                self.ensure_writer_lock()?;
                self.propose_tx_internal(envelope, safe_tx_hash)
            }
            SigningCommand::ConfirmTx {
                safe_tx_hash,
                signature,
            } => {
                self.ensure_writer_lock()?;
                self.confirm_tx_internal(envelope, safe_tx_hash, signature)
            }
            SigningCommand::ExecuteTx { safe_tx_hash } => {
                self.ensure_writer_lock()?;
                self.execute_tx_internal(envelope, safe_tx_hash)
            }
            SigningCommand::CreateMessage {
                chain_id,
                safe_address,
                method,
                payload,
            } => {
                self.ensure_writer_lock()?;
                self.create_message_internal(envelope, chain_id, safe_address, method, payload)
            }
            SigningCommand::AddMessageSignature {
                message_hash,
                signer,
                signature,
            } => {
                self.ensure_writer_lock()?;
                self.add_message_signature_internal(envelope, message_hash, signer, signature)
            }
            SigningCommand::WcSessionAction { topic, action } => {
                self.ensure_writer_lock()?;
                self.walletconnect.session_action(&topic, action)?;
                Ok(CommandResult::empty())
            }
            SigningCommand::RespondWalletConnect {
                request_id,
                result,
                deferred,
            } => {
                self.ensure_writer_lock()?;
                self.respond_wc_internal(envelope, &request_id, result, deferred)
            }
            SigningCommand::ImportBundle { bundle } => {
                self.ensure_writer_lock()?;
                let merge = self.queue.import_bundle(&bundle)?;
                Ok(CommandResult {
                    transition: None,
                    merge: Some(merge),
                    exported_bundle: None,
                })
            }
            SigningCommand::ExportBundle { flow_ids } => {
                let bundle = self.queue.export_bundle(&flow_ids)?;
                Ok(CommandResult {
                    transition: None,
                    merge: None,
                    exported_bundle: Some(bundle),
                })
            }
            SigningCommand::ImportUrlPayload { envelope } => {
                self.ensure_writer_lock()?;
                let merge = self.queue.import_url_payload(&envelope)?;
                Ok(CommandResult {
                    transition: None,
                    merge: Some(merge),
                    exported_bundle: None,
                })
            }
        }
    }

    pub fn handle(&self, command: SigningCommand) -> Result<CommandResult, PortError> {
        let now = self.clock.now_ms()?;
        let envelope = CommandEnvelope {
            command_id: format!("cmd-{now}"),
            correlation_id: format!("corr-{now}"),
            parity_capability_id: parity_id_for_command(&command).to_owned(),
            idempotency_key: format!("idem-{now}"),
            issued_at_ms: TimestampMs(now),
            command_kind: command_kind(&command),
        };
        self.handle_with_envelope(envelope, command)
    }

    fn ensure_writer_lock(&self) -> Result<(), PortError> {
        let lock = self
            .queue
            .load_writer_lock()?
            .ok_or_else(|| PortError::Conflict("WRITER_LOCK_CONFLICT".to_owned()))?;
        let now = self.clock.now_ms()?;
        if lock.expires_at_ms.0 <= now {
            return Err(PortError::Conflict("WRITER_LOCK_CONFLICT".to_owned()));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn create_safe_tx_internal(
        &self,
        envelope: CommandEnvelope,
        chain_id: u64,
        safe_address: Address,
        nonce: u64,
        payload: Value,
        build_source: TxBuildSource,
        abi_context: Option<crate::domain::AbiMethodContext>,
    ) -> Result<CommandResult, PortError> {
        if let Some(ctx) = abi_context.as_ref() {
            if !ctx.raw_calldata_override {
                let payload_data = payload
                    .get("data")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| PortError::Validation("missing payload.data".to_owned()))?;
                let bytes: Bytes = payload_data
                    .parse()
                    .map_err(|e| PortError::Validation(format!("invalid payload.data: {e}")))?;
                if bytes.len() < 4 || bytes[0..4] != ctx.method_selector {
                    return Err(PortError::Validation("ABI_SELECTOR_MISMATCH".to_owned()));
                }
            }
        }

        let now = TimestampMs(self.clock.now_ms()?);
        let safe_tx_hash = self
            .hashing
            .safe_tx_hash(chain_id, safe_address, nonce, &payload)?;
        let mac_payload = serde_json::to_vec(&payload)
            .map_err(|e| PortError::Validation(format!("payload serialization failed: {e}")))?;
        let mac = self.hashing.integrity_mac(&mac_payload, "mac-key-v1")?;
        let tx = PendingSafeTx {
            schema_version: 1,
            chain_id,
            safe_address,
            nonce,
            payload,
            build_source,
            abi_context,
            safe_tx_hash,
            signatures: Vec::new(),
            status: TxStatus::Draft,
            state_revision: 0,
            idempotency_key: envelope.idempotency_key.clone(),
            created_at_ms: now,
            updated_at_ms: now,
            executed_tx_hash: None,
            mac_algorithm: MacAlgorithm::HmacSha256V1,
            mac_key_id: "mac-key-v1".to_owned(),
            integrity_mac: mac,
        };
        self.queue.save_tx(&tx)?;
        let rec = self.transition_record(
            &envelope,
            format!("tx:{}", tx.safe_tx_hash),
            "None",
            format!("{:?}", tx.status),
            Some(tx.idempotency_key.clone()),
            false,
            None,
        )?;
        self.queue.append_transition_log(rec.clone())?;
        Ok(CommandResult {
            transition: Some(rec),
            merge: None,
            exported_bundle: None,
        })
    }

    fn add_tx_signature_internal(
        &self,
        envelope: CommandEnvelope,
        safe_tx_hash: B256,
        signer: Address,
        signature: Bytes,
    ) -> Result<CommandResult, PortError> {
        validate_signature_bytes(&signature)?;
        let mut tx = self
            .queue
            .load_tx(safe_tx_hash)?
            .ok_or_else(|| PortError::NotFound(format!("tx not found: {safe_tx_hash}")))?;

        let state_before = format!("{:?}", tx.status);
        if tx
            .signatures
            .iter()
            .any(|x| x.signer == signer && x.signature == signature)
        {
            let rec = self.transition_record(
                &envelope,
                format!("tx:{}", tx.safe_tx_hash),
                state_before.clone(),
                state_before,
                Some(tx.idempotency_key.clone()),
                false,
                Some("duplicate_signature_skipped".to_owned()),
            )?;
            self.queue.append_transition_log(rec.clone())?;
            return Ok(CommandResult {
                transition: Some(rec),
                merge: None,
                exported_bundle: None,
            });
        }

        let (mut status, transition) = tx_transition(tx.status, TxAction::Sign)?;
        let now = TimestampMs(self.clock.now_ms()?);
        tx.signatures.push(CollectedSignature {
            signer,
            signature,
            source: SignatureSource::ManualEntry,
            method: SignatureMethod::SafeTxHash,
            chain_id: tx.chain_id,
            safe_address: tx.safe_address,
            payload_hash: tx.safe_tx_hash,
            expected_signer: signer,
            recovered_signer: Some(signer),
            added_at_ms: now,
        });

        let threshold = threshold_from_payload(&tx.payload);
        if tx.signature_count() >= threshold
            && matches!(status, TxStatus::Confirming | TxStatus::Proposed)
        {
            status = tx_transition(status, TxAction::ThresholdMet)?.0;
        }

        tx.status = status;
        tx.state_revision = tx.state_revision.saturating_add(1);
        tx.updated_at_ms = now;
        self.queue.save_tx(&tx)?;

        let rec = self.transition_record(
            &envelope,
            format!("tx:{}", tx.safe_tx_hash),
            transition.from,
            format!("{:?}", tx.status),
            Some(tx.idempotency_key.clone()),
            false,
            None,
        )?;
        self.queue.append_transition_log(rec.clone())?;
        Ok(CommandResult {
            transition: Some(rec),
            merge: None,
            exported_bundle: None,
        })
    }

    fn propose_tx_internal(
        &self,
        envelope: CommandEnvelope,
        safe_tx_hash: B256,
    ) -> Result<CommandResult, PortError> {
        let mut tx = self
            .queue
            .load_tx(safe_tx_hash)?
            .ok_or_else(|| PortError::NotFound(format!("tx not found: {safe_tx_hash}")))?;
        if tx.signatures.is_empty() {
            return Err(PortError::Validation(
                "cannot propose tx without signatures".to_owned(),
            ));
        }

        let (_, transition) = tx_transition(tx.status, TxAction::Propose)?;
        self.safe_service.propose_tx(&tx)?;

        tx.status = TxStatus::Proposed;
        if tx.signature_count() >= threshold_from_payload(&tx.payload) {
            tx.status = tx_transition(tx.status, TxAction::ThresholdMet)?.0;
        }
        tx.state_revision = tx.state_revision.saturating_add(1);
        tx.updated_at_ms = TimestampMs(self.clock.now_ms()?);
        self.queue.save_tx(&tx)?;
        let rec = self.transition_record(
            &envelope,
            format!("tx:{}", tx.safe_tx_hash),
            transition.from,
            format!("{:?}", tx.status),
            Some(tx.idempotency_key.clone()),
            true,
            Some("proposed".to_owned()),
        )?;
        self.queue.append_transition_log(rec.clone())?;
        Ok(CommandResult {
            transition: Some(rec),
            merge: None,
            exported_bundle: None,
        })
    }

    fn confirm_tx_internal(
        &self,
        envelope: CommandEnvelope,
        safe_tx_hash: B256,
        signature: Bytes,
    ) -> Result<CommandResult, PortError> {
        validate_signature_bytes(&signature)?;
        let mut tx = self
            .queue
            .load_tx(safe_tx_hash)?
            .ok_or_else(|| PortError::NotFound(format!("tx not found: {safe_tx_hash}")))?;

        let (_, transition) = tx_transition(tx.status, TxAction::Confirm)?;
        self.safe_service.confirm_tx(tx.safe_tx_hash, &signature)?;

        let signer = self
            .provider
            .request_accounts()
            .ok()
            .and_then(|mut x| x.drain(..).next())
            .unwrap_or(Address::ZERO);

        let now = TimestampMs(self.clock.now_ms()?);
        tx.signatures.push(CollectedSignature {
            signer,
            signature,
            source: SignatureSource::InjectedProvider,
            method: SignatureMethod::SafeTxHash,
            chain_id: tx.chain_id,
            safe_address: tx.safe_address,
            payload_hash: tx.safe_tx_hash,
            expected_signer: signer,
            recovered_signer: Some(signer),
            added_at_ms: now,
        });

        tx.status = TxStatus::Confirming;
        if tx.signature_count() >= threshold_from_payload(&tx.payload) {
            tx.status = tx_transition(tx.status, TxAction::ThresholdMet)?.0;
        }
        tx.state_revision = tx.state_revision.saturating_add(1);
        tx.updated_at_ms = now;
        self.queue.save_tx(&tx)?;
        let rec = self.transition_record(
            &envelope,
            format!("tx:{}", tx.safe_tx_hash),
            transition.from,
            format!("{:?}", tx.status),
            Some(tx.idempotency_key.clone()),
            true,
            Some("confirmed".to_owned()),
        )?;
        self.queue.append_transition_log(rec.clone())?;
        Ok(CommandResult {
            transition: Some(rec),
            merge: None,
            exported_bundle: None,
        })
    }

    fn execute_tx_internal(
        &self,
        envelope: CommandEnvelope,
        safe_tx_hash: B256,
    ) -> Result<CommandResult, PortError> {
        let mut tx = self
            .queue
            .load_tx(safe_tx_hash)?
            .ok_or_else(|| PortError::NotFound(format!("tx not found: {safe_tx_hash}")))?;
        if tx.status != TxStatus::ReadyToExecute {
            if tx.signature_count() >= threshold_from_payload(&tx.payload)
                && matches!(tx.status, TxStatus::Confirming | TxStatus::Proposed)
            {
                tx.status = tx_transition(tx.status, TxAction::ThresholdMet)?.0;
                tx.state_revision = tx.state_revision.saturating_add(1);
                tx.updated_at_ms = TimestampMs(self.clock.now_ms()?);
                self.queue.save_tx(&tx)?;
            } else {
                return Err(PortError::Validation(
                    "tx is not ready to execute".to_owned(),
                ));
            }
        }

        let (_, transition) = tx_transition(tx.status, TxAction::ExecuteStart)?;
        tx.status = TxStatus::Executing;
        tx.state_revision = tx.state_revision.saturating_add(1);
        tx.updated_at_ms = TimestampMs(self.clock.now_ms()?);
        self.queue.save_tx(&tx)?;

        let tx_hash = self.safe_service.execute_tx(&tx)?;
        let (_, transition2) = tx_transition(tx.status, TxAction::ExecuteSuccess)?;
        tx.status = TxStatus::Executed;
        tx.executed_tx_hash = Some(tx_hash);
        tx.state_revision = tx.state_revision.saturating_add(1);
        tx.updated_at_ms = TimestampMs(self.clock.now_ms()?);
        self.queue.save_tx(&tx)?;
        let rec = self.transition_record(
            &envelope,
            format!("tx:{}", tx.safe_tx_hash),
            transition.from,
            transition2.to,
            Some(tx.idempotency_key.clone()),
            true,
            Some(format!("executed:{tx_hash}")),
        )?;
        self.queue.append_transition_log(rec.clone())?;
        Ok(CommandResult {
            transition: Some(rec),
            merge: None,
            exported_bundle: None,
        })
    }

    fn create_message_internal(
        &self,
        envelope: CommandEnvelope,
        chain_id: u64,
        safe_address: Address,
        method: MessageMethod,
        payload: Value,
    ) -> Result<CommandResult, PortError> {
        let now = TimestampMs(self.clock.now_ms()?);
        let message_hash = self
            .hashing
            .message_hash(chain_id, safe_address, method, &payload)?;
        let mac_payload = serde_json::to_vec(&payload)
            .map_err(|e| PortError::Validation(format!("payload serialization failed: {e}")))?;
        let mac = self.hashing.integrity_mac(&mac_payload, "mac-key-v1")?;
        let message = PendingSafeMessage {
            schema_version: 1,
            chain_id,
            safe_address,
            method,
            payload,
            message_hash,
            signatures: Vec::new(),
            status: MessageStatus::Draft,
            state_revision: 0,
            idempotency_key: envelope.idempotency_key.clone(),
            created_at_ms: now,
            updated_at_ms: now,
            mac_algorithm: MacAlgorithm::HmacSha256V1,
            mac_key_id: "mac-key-v1".to_owned(),
            integrity_mac: mac,
        };
        self.queue.save_message(&message)?;
        let rec = self.transition_record(
            &envelope,
            format!("msg:{}", message.message_hash),
            "None",
            format!("{:?}", message.status),
            Some(message.idempotency_key.clone()),
            false,
            None,
        )?;
        self.queue.append_transition_log(rec.clone())?;
        Ok(CommandResult {
            transition: Some(rec),
            merge: None,
            exported_bundle: None,
        })
    }

    fn add_message_signature_internal(
        &self,
        envelope: CommandEnvelope,
        message_hash: B256,
        signer: Address,
        signature: Bytes,
    ) -> Result<CommandResult, PortError> {
        validate_signature_bytes(&signature)?;
        let mut msg = self
            .queue
            .load_message(message_hash)?
            .ok_or_else(|| PortError::NotFound(format!("message not found: {message_hash}")))?;

        let state_before = format!("{:?}", msg.status);
        if msg
            .signatures
            .iter()
            .any(|x| x.signer == signer && x.signature == signature)
        {
            let rec = self.transition_record(
                &envelope,
                format!("msg:{}", msg.message_hash),
                state_before.clone(),
                state_before,
                Some(msg.idempotency_key.clone()),
                false,
                Some("duplicate_signature_skipped".to_owned()),
            )?;
            self.queue.append_transition_log(rec.clone())?;
            return Ok(CommandResult {
                transition: Some(rec),
                merge: None,
                exported_bundle: None,
            });
        }

        let (_, transition) = message_transition(msg.status, MessageAction::Sign)?;
        let now = TimestampMs(self.clock.now_ms()?);
        msg.signatures.push(CollectedSignature {
            signer,
            signature,
            source: SignatureSource::ManualEntry,
            method: match msg.method {
                MessageMethod::PersonalSign => SignatureMethod::PersonalSign,
                MessageMethod::EthSign => SignatureMethod::EthSign,
                MessageMethod::EthSignTypedData => SignatureMethod::EthSignTypedData,
                MessageMethod::EthSignTypedDataV4 => SignatureMethod::EthSignTypedDataV4,
            },
            chain_id: msg.chain_id,
            safe_address: msg.safe_address,
            payload_hash: msg.message_hash,
            expected_signer: signer,
            recovered_signer: Some(signer),
            added_at_ms: now,
        });

        msg.status = match msg.status {
            MessageStatus::Draft => MessageStatus::Signing,
            MessageStatus::Signing => MessageStatus::Signing,
            MessageStatus::AwaitingThreshold => MessageStatus::AwaitingThreshold,
            MessageStatus::ThresholdMet => MessageStatus::ThresholdMet,
            MessageStatus::Responded | MessageStatus::Failed | MessageStatus::Cancelled => {
                return Err(PortError::Validation(
                    "cannot add signature to finalized message state".to_owned(),
                ))
            }
        };

        let threshold = threshold_from_payload(&msg.payload);
        if msg.signature_count() >= threshold {
            msg.status = message_transition(msg.status, MessageAction::ThresholdMet)?.0;
        } else if msg.status == MessageStatus::Signing {
            msg.status = message_transition(msg.status, MessageAction::AwaitThreshold)?.0;
        }

        msg.state_revision = msg.state_revision.saturating_add(1);
        msg.updated_at_ms = now;
        self.queue.save_message(&msg)?;
        let rec = self.transition_record(
            &envelope,
            format!("msg:{}", msg.message_hash),
            transition.from,
            format!("{:?}", msg.status),
            Some(msg.idempotency_key.clone()),
            false,
            None,
        )?;
        self.queue.append_transition_log(rec.clone())?;
        Ok(CommandResult {
            transition: Some(rec),
            merge: None,
            exported_bundle: None,
        })
    }

    fn respond_wc_internal(
        &self,
        envelope: CommandEnvelope,
        request_id: &str,
        result: Value,
        deferred: bool,
    ) -> Result<CommandResult, PortError> {
        let mut req = self
            .queue
            .load_wc_request(request_id)?
            .ok_or_else(|| PortError::NotFound(format!("wc request not found: {request_id}")))?;

        if let Some(expires_at) = req.expires_at_ms {
            if self.clock.now_ms()? >= expires_at.0 {
                req.status = WcStatus::Expired;
                req.state_revision = req.state_revision.saturating_add(1);
                req.updated_at_ms = TimestampMs(self.clock.now_ms()?);
                self.queue.save_wc_request(&req)?;
                return Err(PortError::Validation("WC_REQUEST_EXPIRED".to_owned()));
            }
        }

        if req.session_status != WcSessionStatus::Approved {
            return Err(PortError::Policy("WC_SESSION_NOT_APPROVED".to_owned()));
        }
        if deferred && req.linked_safe_tx_hash.is_none() {
            return Err(PortError::Validation(
                "deferred response requires linked tx".to_owned(),
            ));
        }

        let action = if deferred {
            WcAction::RespondDeferred
        } else {
            WcAction::RespondImmediate
        };
        let (_, transition) = wc_transition(req.status, action)?;
        req.status = if deferred {
            WcStatus::RespondingDeferred
        } else {
            WcStatus::RespondingImmediate
        };
        req.state_revision = req.state_revision.saturating_add(1);
        req.updated_at_ms = TimestampMs(self.clock.now_ms()?);
        self.queue.save_wc_request(&req)?;
        self.walletconnect.respond_success(request_id, result)?;
        let (_, transition2) = wc_transition(req.status, WcAction::RespondSuccess)?;
        req.status = WcStatus::Responded;
        req.state_revision = req.state_revision.saturating_add(1);
        req.updated_at_ms = TimestampMs(self.clock.now_ms()?);
        self.queue.save_wc_request(&req)?;
        let rec = self.transition_record(
            &envelope,
            format!("wc:{request_id}"),
            transition.from,
            transition2.to,
            Some(envelope.idempotency_key.clone()),
            true,
            Some("wc_responded".to_owned()),
        )?;
        self.queue.append_transition_log(rec.clone())?;
        Ok(CommandResult {
            transition: Some(rec),
            merge: None,
            exported_bundle: None,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn transition_record(
        &self,
        envelope: &CommandEnvelope,
        flow_id: String,
        state_before: impl Into<String>,
        state_after: impl Into<String>,
        side_effect_key: Option<String>,
        side_effect_dispatched: bool,
        side_effect_outcome: Option<String>,
    ) -> Result<TransitionLogRecord, PortError> {
        let existing = self.queue.load_transition_log(&flow_id)?;
        let next_seq = existing.last().map(|x| x.event_seq + 1).unwrap_or(1);
        Ok(TransitionLogRecord {
            event_seq: next_seq,
            command_id: envelope.command_id.clone(),
            flow_id,
            state_before: state_before.into(),
            state_after: state_after.into(),
            side_effect_key,
            side_effect_dispatched,
            side_effect_outcome,
            recorded_at_ms: TimestampMs(self.clock.now_ms()?),
        })
    }
}

fn validate_signature_bytes(signature: &Bytes) -> Result<(), PortError> {
    // 65-byte ECDSA signature (r,s,v) is the minimum accepted format for parity wave.
    if signature.len() < 65 {
        return Err(PortError::Validation("INVALID_SIGNATURE_FORMAT".to_owned()));
    }
    Ok(())
}

fn threshold_from_payload(payload: &Value) -> usize {
    payload
        .get("threshold")
        .and_then(|v| v.as_u64())
        .map(|v| v.max(1) as usize)
        .unwrap_or(1)
}

fn parity_id_for_command(command: &SigningCommand) -> &'static str {
    match command {
        SigningCommand::ConnectProvider | SigningCommand::WcSessionAction { .. } => "PARITY-WC-01",
        SigningCommand::AcquireWriterLock { .. }
        | SigningCommand::ImportBundle { .. }
        | SigningCommand::ExportBundle { .. }
        | SigningCommand::ImportUrlPayload { .. } => "PARITY-COLLAB-01",
        SigningCommand::CreateSafeTx { .. }
        | SigningCommand::AddTxSignature { .. }
        | SigningCommand::ProposeTx { .. }
        | SigningCommand::ConfirmTx { .. }
        | SigningCommand::ExecuteTx { .. } => "PARITY-TX-01",
        SigningCommand::CreateSafeTxFromAbi { .. } => "PARITY-ABI-01",
        SigningCommand::CreateMessage { .. } | SigningCommand::AddMessageSignature { .. } => {
            "PARITY-MSG-01"
        }
        SigningCommand::RespondWalletConnect { .. } => "PARITY-WC-01",
    }
}

fn command_kind(command: &SigningCommand) -> String {
    match command {
        SigningCommand::ConnectProvider => "connect_provider",
        SigningCommand::AcquireWriterLock { .. } => "acquire_writer_lock",
        SigningCommand::CreateSafeTx { .. } => "create_safe_tx",
        SigningCommand::CreateSafeTxFromAbi { .. } => "create_safe_tx_from_abi",
        SigningCommand::AddTxSignature { .. } => "add_tx_signature",
        SigningCommand::ProposeTx { .. } => "propose_tx",
        SigningCommand::ConfirmTx { .. } => "confirm_tx",
        SigningCommand::ExecuteTx { .. } => "execute_tx",
        SigningCommand::CreateMessage { .. } => "create_message",
        SigningCommand::AddMessageSignature { .. } => "add_message_signature",
        SigningCommand::WcSessionAction { .. } => "wc_session_action",
        SigningCommand::RespondWalletConnect { .. } => "respond_walletconnect",
        SigningCommand::ImportBundle { .. } => "import_bundle",
        SigningCommand::ExportBundle { .. } => "export_bundle",
        SigningCommand::ImportUrlPayload { .. } => "import_url_payload",
    }
    .to_owned()
}
