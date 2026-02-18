use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use alloy::primitives::{keccak256, Address, Bytes, B256};
use serde_json::Value;

use rusty_safe_signing_core::{
    AppWriterLock, MergeResult, PendingSafeMessage, PendingSafeTx, PendingWalletConnectRequest,
    PortError, QueuePort, SigningBundle, TimestampMs, TransitionLogRecord, UrlImportEnvelope,
    UrlImportKey,
};

#[derive(Debug, Clone, Default)]
pub struct QueueAdapter {
    inner: Arc<Mutex<QueueState>>,
}

#[derive(Debug, Default)]
struct QueueState {
    writer_lock: Option<AppWriterLock>,
    txs: HashMap<B256, PendingSafeTx>,
    messages: HashMap<B256, PendingSafeMessage>,
    wc_requests: HashMap<String, PendingWalletConnectRequest>,
    logs: HashMap<String, Vec<TransitionLogRecord>>,
}

impl QueueAdapter {
    pub fn with_seed(seed: QueueSeed) -> Self {
        let mut state = QueueState::default();
        for tx in seed.txs {
            state.txs.insert(tx.safe_tx_hash, tx);
        }
        for msg in seed.messages {
            state.messages.insert(msg.message_hash, msg);
        }
        for req in seed.wc_requests {
            state.wc_requests.insert(req.request_id.clone(), req);
        }
        Self {
            inner: Arc::new(Mutex::new(state)),
        }
    }
}

#[derive(Debug, Default)]
pub struct QueueSeed {
    pub txs: Vec<PendingSafeTx>,
    pub messages: Vec<PendingSafeMessage>,
    pub wc_requests: Vec<PendingWalletConnectRequest>,
}

impl QueuePort for QueueAdapter {
    fn acquire_writer_lock(&self, lock: AppWriterLock) -> Result<AppWriterLock, PortError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("queue lock poisoned: {e}")))?;
        if let Some(existing) = &g.writer_lock {
            if existing.holder_tab_id != lock.holder_tab_id
                && existing.expires_at_ms > lock.acquired_at_ms
            {
                return Err(PortError::Conflict("WRITER_LOCK_CONFLICT".to_owned()));
            }
        }
        g.writer_lock = Some(lock.clone());
        Ok(lock)
    }

    fn load_writer_lock(&self) -> Result<Option<AppWriterLock>, PortError> {
        let g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("queue lock poisoned: {e}")))?;
        Ok(g.writer_lock.clone())
    }

    fn release_writer_lock(&self, holder_tab_id: &str) -> Result<(), PortError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("queue lock poisoned: {e}")))?;
        if g.writer_lock
            .as_ref()
            .is_some_and(|lock| lock.holder_tab_id == holder_tab_id)
        {
            g.writer_lock = None;
        }
        Ok(())
    }

    fn save_tx(&self, tx: &PendingSafeTx) -> Result<(), PortError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("queue lock poisoned: {e}")))?;
        if let Some(existing) = g.txs.get(&tx.safe_tx_hash) {
            if tx.state_revision < existing.state_revision {
                return Err(PortError::Conflict(
                    "state revision regression for tx".to_owned(),
                ));
            }
        }
        g.txs.insert(tx.safe_tx_hash, tx.clone());
        Ok(())
    }

    fn save_message(&self, message: &PendingSafeMessage) -> Result<(), PortError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("queue lock poisoned: {e}")))?;
        if let Some(existing) = g.messages.get(&message.message_hash) {
            if message.state_revision < existing.state_revision {
                return Err(PortError::Conflict(
                    "state revision regression for message".to_owned(),
                ));
            }
        }
        g.messages.insert(message.message_hash, message.clone());
        Ok(())
    }

    fn save_wc_request(&self, request: &PendingWalletConnectRequest) -> Result<(), PortError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("queue lock poisoned: {e}")))?;
        if let Some(existing) = g.wc_requests.get(&request.request_id) {
            if request.state_revision < existing.state_revision {
                return Err(PortError::Conflict(
                    "state revision regression for wc request".to_owned(),
                ));
            }
        }
        g.wc_requests
            .insert(request.request_id.clone(), request.clone());
        Ok(())
    }

    fn load_tx(&self, safe_tx_hash: B256) -> Result<Option<PendingSafeTx>, PortError> {
        let g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("queue lock poisoned: {e}")))?;
        Ok(g.txs.get(&safe_tx_hash).cloned())
    }

    fn load_message(&self, message_hash: B256) -> Result<Option<PendingSafeMessage>, PortError> {
        let g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("queue lock poisoned: {e}")))?;
        Ok(g.messages.get(&message_hash).cloned())
    }

    fn load_wc_request(
        &self,
        request_id: &str,
    ) -> Result<Option<PendingWalletConnectRequest>, PortError> {
        let g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("queue lock poisoned: {e}")))?;
        Ok(g.wc_requests.get(request_id).cloned())
    }

    fn list_txs(&self) -> Result<Vec<PendingSafeTx>, PortError> {
        let g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("queue lock poisoned: {e}")))?;
        Ok(g.txs.values().cloned().collect())
    }

    fn list_messages(&self) -> Result<Vec<PendingSafeMessage>, PortError> {
        let g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("queue lock poisoned: {e}")))?;
        Ok(g.messages.values().cloned().collect())
    }

    fn list_wc_requests(&self) -> Result<Vec<PendingWalletConnectRequest>, PortError> {
        let g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("queue lock poisoned: {e}")))?;
        Ok(g.wc_requests.values().cloned().collect())
    }

    fn append_transition_log(&self, record: TransitionLogRecord) -> Result<(), PortError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("queue lock poisoned: {e}")))?;
        let list = g.logs.entry(record.flow_id.clone()).or_default();
        let expected = list.last().map(|x| x.event_seq + 1).unwrap_or(1);
        if record.event_seq != expected {
            return Err(PortError::Conflict(format!(
                "event_seq gap for {}: got {}, expected {}",
                record.flow_id, record.event_seq, expected
            )));
        }
        list.push(record);
        Ok(())
    }

    fn load_transition_log(&self, flow_id: &str) -> Result<Vec<TransitionLogRecord>, PortError> {
        let g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("queue lock poisoned: {e}")))?;
        Ok(g.logs.get(flow_id).cloned().unwrap_or_default())
    }

    fn import_bundle(&self, bundle: &SigningBundle) -> Result<MergeResult, PortError> {
        if bundle.schema_version != 1 {
            return Err(PortError::Validation(
                "unsupported bundle schema_version".to_owned(),
            ));
        }
        if bundle.txs.is_empty() && bundle.messages.is_empty() && bundle.wc_requests.is_empty() {
            return Err(PortError::Validation("empty bundle".to_owned()));
        }
        let mut g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("queue lock poisoned: {e}")))?;
        let mut merge = MergeResult::empty();
        for tx in &bundle.txs {
            match g.txs.get(&tx.safe_tx_hash) {
                None => {
                    g.txs.insert(tx.safe_tx_hash, tx.clone());
                    merge.tx_added += 1;
                }
                Some(existing) if existing.state_revision < tx.state_revision => {
                    g.txs.insert(tx.safe_tx_hash, tx.clone());
                    merge.tx_updated += 1;
                }
                Some(existing) if existing.state_revision == tx.state_revision => {
                    merge.tx_skipped += 1;
                }
                Some(_) => {
                    merge.tx_conflicted += 1;
                }
            }
        }
        for msg in &bundle.messages {
            match g.messages.get(&msg.message_hash) {
                None => {
                    g.messages.insert(msg.message_hash, msg.clone());
                    merge.message_added += 1;
                }
                Some(existing) if existing.state_revision < msg.state_revision => {
                    g.messages.insert(msg.message_hash, msg.clone());
                    merge.message_updated += 1;
                }
                Some(existing) if existing.state_revision == msg.state_revision => {
                    merge.message_skipped += 1;
                }
                Some(_) => {
                    merge.message_conflicted += 1;
                }
            }
        }
        for req in &bundle.wc_requests {
            g.wc_requests
                .entry(req.request_id.clone())
                .or_insert_with(|| req.clone());
        }
        Ok(merge)
    }

    fn export_bundle(&self, flow_ids: &[String]) -> Result<SigningBundle, PortError> {
        let g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("queue lock poisoned: {e}")))?;
        let mut txs = Vec::new();
        let mut messages = Vec::new();
        let mut wc_requests = Vec::new();
        for id in flow_ids {
            if let Some(hash) = id.strip_prefix("tx:") {
                let parsed: B256 = hash
                    .parse()
                    .map_err(|e| PortError::Validation(format!("invalid tx hash id {id}: {e}")))?;
                if let Some(tx) = g.txs.get(&parsed) {
                    txs.push(tx.clone());
                }
            } else if let Some(hash) = id.strip_prefix("msg:") {
                let parsed: B256 = hash.parse().map_err(|e| {
                    PortError::Validation(format!("invalid message hash id {id}: {e}"))
                })?;
                if let Some(msg) = g.messages.get(&parsed) {
                    messages.push(msg.clone());
                }
            } else if let Some(req_id) = id.strip_prefix("wc:") {
                if let Some(req) = g.wc_requests.get(req_id) {
                    wc_requests.push(req.clone());
                }
            }
        }
        let digest_payload = serde_json::json!({
            "schema_version": 1,
            "txs": txs,
            "messages": messages,
            "wc_requests": wc_requests,
        });
        let digest_bytes = serde_json::to_vec(&digest_payload).map_err(|e| {
            PortError::Validation(format!("bundle digest serialization failed: {e}"))
        })?;
        let digest = keccak256(digest_bytes);
        let integrity_mac = keccak256([b"bundle-mac-v1".as_slice(), digest.as_slice()].concat());

        let txs = digest_payload
            .get("txs")
            .and_then(|v| v.as_array())
            .cloned()
            .ok_or_else(|| PortError::Validation("bundle txs serialization failed".to_owned()))?
            .into_iter()
            .map(serde_json::from_value)
            .collect::<Result<Vec<PendingSafeTx>, _>>()
            .map_err(|e| PortError::Validation(format!("bundle tx decode failed: {e}")))?;
        let messages = digest_payload
            .get("messages")
            .and_then(|v| v.as_array())
            .cloned()
            .ok_or_else(|| {
                PortError::Validation("bundle messages serialization failed".to_owned())
            })?
            .into_iter()
            .map(serde_json::from_value)
            .collect::<Result<Vec<PendingSafeMessage>, _>>()
            .map_err(|e| PortError::Validation(format!("bundle message decode failed: {e}")))?;
        let wc_requests = digest_payload
            .get("wc_requests")
            .and_then(|v| v.as_array())
            .cloned()
            .ok_or_else(|| PortError::Validation("bundle wc serialization failed".to_owned()))?
            .into_iter()
            .map(serde_json::from_value)
            .collect::<Result<Vec<PendingWalletConnectRequest>, _>>()
            .map_err(|e| PortError::Validation(format!("bundle wc decode failed: {e}")))?;

        Ok(SigningBundle {
            schema_version: 1,
            exported_at_ms: TimestampMs(0),
            exporter: Address::ZERO,
            bundle_digest: digest,
            bundle_signature: Bytes::from(vec![0x1b; 65]),
            txs,
            messages,
            wc_requests,
            mac_algorithm: rusty_safe_signing_core::MacAlgorithm::HmacSha256V1,
            mac_key_id: "mac-key-v1".to_owned(),
            integrity_mac,
        })
    }

    fn import_url_payload(&self, envelope: &UrlImportEnvelope) -> Result<MergeResult, PortError> {
        if envelope.schema_version != 1 {
            return Err(PortError::Validation(
                "URL_IMPORT_SCHEMA_INVALID".to_owned(),
            ));
        }
        let raw = decode_base64_url(&envelope.payload_base64url)?;
        let payload: Value = serde_json::from_slice(&raw)
            .map_err(|e| PortError::Validation(format!("invalid url payload json: {e}")))?;
        match envelope.key {
            UrlImportKey::ImportTx => self.import_url_tx(payload),
            UrlImportKey::ImportSig => self.import_url_tx_sig(payload),
            UrlImportKey::ImportMsg => self.import_url_message(payload),
            UrlImportKey::ImportMsgSig => self.import_url_message_sig(payload),
        }
    }
}

impl QueueAdapter {
    fn import_url_tx(&self, payload: Value) -> Result<MergeResult, PortError> {
        let tx: PendingSafeTx = serde_json::from_value(payload)
            .map_err(|e| PortError::Validation(format!("importTx payload invalid: {e}")))?;
        let mut g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("queue lock poisoned: {e}")))?;
        let mut merge = MergeResult::empty();
        match g.txs.entry(tx.safe_tx_hash) {
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(tx);
                merge.tx_added = 1;
            }
            std::collections::hash_map::Entry::Occupied(mut e) => {
                if e.get().state_revision <= tx.state_revision {
                    e.insert(tx);
                    merge.tx_updated = 1;
                } else {
                    merge.tx_conflicted = 1;
                }
            }
        }
        Ok(merge)
    }

    fn import_url_tx_sig(&self, payload: Value) -> Result<MergeResult, PortError> {
        let tx_hash: B256 = payload
            .get("txHash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PortError::Validation("missing txHash".to_owned()))?
            .parse()
            .map_err(|e| PortError::Validation(format!("invalid txHash: {e}")))?;
        let signer: Address = payload
            .get("signature")
            .and_then(|x| x.get("signer"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| PortError::Validation("missing signature.signer".to_owned()))?
            .parse()
            .map_err(|e| PortError::Validation(format!("invalid signer: {e}")))?;
        let sig: Bytes = payload
            .get("signature")
            .and_then(|x| x.get("data"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| PortError::Validation("missing signature.data".to_owned()))?
            .parse()
            .map_err(|e| PortError::Validation(format!("invalid signature data: {e}")))?;

        let mut g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("queue lock poisoned: {e}")))?;
        let tx = g
            .txs
            .get_mut(&tx_hash)
            .ok_or_else(|| PortError::NotFound("tx for signature import not found".to_owned()))?;
        tx.signatures
            .push(rusty_safe_signing_core::CollectedSignature {
                signer,
                signature: sig,
                source: rusty_safe_signing_core::SignatureSource::ImportedBundle,
                method: rusty_safe_signing_core::SignatureMethod::SafeTxHash,
                chain_id: tx.chain_id,
                safe_address: tx.safe_address,
                payload_hash: tx.safe_tx_hash,
                expected_signer: signer,
                recovered_signer: Some(signer),
                added_at_ms: TimestampMs(0),
            });
        tx.state_revision = tx.state_revision.saturating_add(1);
        let mut merge = MergeResult::empty();
        merge.tx_updated = 1;
        Ok(merge)
    }

    fn import_url_message(&self, payload: Value) -> Result<MergeResult, PortError> {
        let msg: PendingSafeMessage = serde_json::from_value(payload)
            .map_err(|e| PortError::Validation(format!("importMsg payload invalid: {e}")))?;
        let mut g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("queue lock poisoned: {e}")))?;
        let mut merge = MergeResult::empty();
        match g.messages.entry(msg.message_hash) {
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(msg);
                merge.message_added = 1;
            }
            std::collections::hash_map::Entry::Occupied(mut e) => {
                if e.get().state_revision <= msg.state_revision {
                    e.insert(msg);
                    merge.message_updated = 1;
                } else {
                    merge.message_conflicted = 1;
                }
            }
        }
        Ok(merge)
    }

    fn import_url_message_sig(&self, payload: Value) -> Result<MergeResult, PortError> {
        let msg_hash: B256 = payload
            .get("messageHash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PortError::Validation("missing messageHash".to_owned()))?
            .parse()
            .map_err(|e| PortError::Validation(format!("invalid messageHash: {e}")))?;
        let signer: Address = payload
            .get("signature")
            .and_then(|x| x.get("signer"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| PortError::Validation("missing signature.signer".to_owned()))?
            .parse()
            .map_err(|e| PortError::Validation(format!("invalid signer: {e}")))?;
        let sig: Bytes = payload
            .get("signature")
            .and_then(|x| x.get("data"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| PortError::Validation("missing signature.data".to_owned()))?
            .parse()
            .map_err(|e| PortError::Validation(format!("invalid signature data: {e}")))?;
        let mut g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("queue lock poisoned: {e}")))?;
        let msg = g.messages.get_mut(&msg_hash).ok_or_else(|| {
            PortError::NotFound("message for signature import not found".to_owned())
        })?;
        msg.signatures
            .push(rusty_safe_signing_core::CollectedSignature {
                signer,
                signature: sig,
                source: rusty_safe_signing_core::SignatureSource::ImportedBundle,
                method: match msg.method {
                    rusty_safe_signing_core::MessageMethod::PersonalSign => {
                        rusty_safe_signing_core::SignatureMethod::PersonalSign
                    }
                    rusty_safe_signing_core::MessageMethod::EthSign => {
                        rusty_safe_signing_core::SignatureMethod::EthSign
                    }
                    rusty_safe_signing_core::MessageMethod::EthSignTypedData => {
                        rusty_safe_signing_core::SignatureMethod::EthSignTypedData
                    }
                    rusty_safe_signing_core::MessageMethod::EthSignTypedDataV4 => {
                        rusty_safe_signing_core::SignatureMethod::EthSignTypedDataV4
                    }
                },
                chain_id: msg.chain_id,
                safe_address: msg.safe_address,
                payload_hash: msg.message_hash,
                expected_signer: signer,
                recovered_signer: Some(signer),
                added_at_ms: TimestampMs(0),
            });
        msg.state_revision = msg.state_revision.saturating_add(1);
        let mut merge = MergeResult::empty();
        merge.message_updated = 1;
        Ok(merge)
    }
}

fn decode_base64_url(input: &str) -> Result<Vec<u8>, PortError> {
    let mut s = input.replace('-', "+").replace('_', "/");
    while s.len() % 4 != 0 {
        s.push('=');
    }
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| PortError::Validation(format!("base64url decode failed: {e}")))
}
