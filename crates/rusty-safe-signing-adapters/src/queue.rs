use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use alloy::primitives::{keccak256, Address, Bytes, PrimitiveSignature, B256};
use alloy::signers::{local::PrivateKeySigner, SignerSync};
use serde_json::Value;

use rusty_safe_signing_core::{
    AppWriterLock, BundleCryptoEnvelope, MacAlgorithm, MergeResult, PendingSafeMessage,
    PendingSafeTx, PendingWalletConnectRequest, PortError, QueuePort, SigningBundle, TimestampMs,
    TransitionLogRecord, UrlImportEnvelope, UrlImportKey,
};

use crate::crypto::{
    canonical_json_bytes, decrypt_aes_gcm, derive_crypto, encrypt_aes_gcm, generate_nonce,
    generate_salt, hmac_sha256_b256,
};
use crate::SigningAdapterConfig;

const EXPORT_SIGNER_PRIVATE_KEY_ENV: &str = "RUSTY_SAFE_EXPORT_SIGNER_PRIVATE_KEY";
const DEFAULT_EXPORT_SIGNER_PRIVATE_KEY: &str =
    "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

#[derive(Debug, Clone)]
pub struct QueueAdapter {
    inner: Arc<Mutex<QueueState>>,
    config: SigningAdapterConfig,
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
    pub fn with_config(config: SigningAdapterConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(QueueState::default())),
            config,
        }
    }

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
            config: SigningAdapterConfig::from_env(),
        }
    }

    fn export_passphrase(&self) -> String {
        std::env::var(&self.config.export_passphrase_env)
            .unwrap_or_else(|_| "rusty-safe-dev-passphrase".to_owned())
    }

    fn export_signer(&self) -> Result<PrivateKeySigner, PortError> {
        let key = std::env::var(EXPORT_SIGNER_PRIVATE_KEY_ENV)
            .unwrap_or_else(|_| DEFAULT_EXPORT_SIGNER_PRIVATE_KEY.to_owned());
        key.trim()
            .trim_start_matches("0x")
            .parse::<PrivateKeySigner>()
            .map_err(|e| {
                PortError::Validation(format!(
                    "invalid export signer private key in {EXPORT_SIGNER_PRIVATE_KEY_ENV}: {e}"
                ))
            })
    }
}

impl Default for QueueAdapter {
    fn default() -> Self {
        Self::with_config(SigningAdapterConfig::from_env())
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
        let (bundle_txs, bundle_messages, bundle_wc_requests) = if let Some(crypto) =
            &bundle.crypto_envelope
        {
            let passphrase = self.export_passphrase();
            let salt_bytes = decode_base64(&crypto.kdf_salt_base64)?;
            if salt_bytes.len() != 16 {
                return Err(PortError::Validation(
                    "invalid bundle kdf salt length".to_owned(),
                ));
            }
            let mut salt = [0u8; 16];
            salt.copy_from_slice(&salt_bytes);
            let derived = derive_crypto(passphrase.as_bytes(), salt)?;
            let nonce_bytes = decode_base64(&crypto.enc_nonce_base64)?;
            if nonce_bytes.len() != 12 {
                return Err(PortError::Validation(
                    "invalid bundle encryption nonce length".to_owned(),
                ));
            }
            let mut nonce = [0u8; 12];
            nonce.copy_from_slice(&nonce_bytes);
            let ciphertext = decode_base64(&crypto.ciphertext_base64)?;
            let computed_mac = hmac_sha256_b256(&derived.mac_key, &ciphertext)?;
            if computed_mac != bundle.integrity_mac {
                return Err(PortError::Validation(
                    "bundle integrity mac mismatch".to_owned(),
                ));
            }
            let plaintext = decrypt_aes_gcm(&derived.enc_key, nonce, &ciphertext)?;
            let payload: Value = serde_json::from_slice(&plaintext).map_err(|e| {
                PortError::Validation(format!("decrypted bundle payload invalid: {e}"))
            })?;
            let txs = payload
                .get("txs")
                .cloned()
                .ok_or_else(|| PortError::Validation("decrypted bundle missing txs".to_owned()))
                .and_then(|v| {
                    serde_json::from_value(v).map_err(|e| {
                        PortError::Validation(format!("decrypted bundle tx decode failed: {e}"))
                    })
                })?;
            let messages = payload
                .get("messages")
                .cloned()
                .ok_or_else(|| {
                    PortError::Validation("decrypted bundle missing messages".to_owned())
                })
                .and_then(|v| {
                    serde_json::from_value(v).map_err(|e| {
                        PortError::Validation(format!(
                            "decrypted bundle message decode failed: {e}"
                        ))
                    })
                })?;
            let wc_requests = payload
                .get("wc_requests")
                .cloned()
                .ok_or_else(|| {
                    PortError::Validation("decrypted bundle missing wc_requests".to_owned())
                })
                .and_then(|v| {
                    serde_json::from_value(v).map_err(|e| {
                        PortError::Validation(format!("decrypted bundle wc decode failed: {e}"))
                    })
                })?;
            (txs, messages, wc_requests)
        } else {
            if bundle.txs.is_empty() && bundle.messages.is_empty() && bundle.wc_requests.is_empty()
            {
                return Err(PortError::Validation("empty bundle".to_owned()));
            }
            let payload = serde_json::json!({
                "txs": bundle.txs,
                "messages": bundle.messages,
                "wc_requests": bundle.wc_requests,
            });
            let canonical = canonical_json_bytes(&payload)?;
            let fallback_salt = [0u8; 16];
            let derived = derive_crypto(self.export_passphrase().as_bytes(), fallback_salt)?;
            let computed_mac = hmac_sha256_b256(&derived.mac_key, &canonical)?;
            if computed_mac != bundle.integrity_mac {
                return Err(PortError::Validation(
                    "bundle integrity mac mismatch".to_owned(),
                ));
            }
            (
                bundle.txs.clone(),
                bundle.messages.clone(),
                bundle.wc_requests.clone(),
            )
        };
        let digest = bundle_payload_digest(&bundle_txs, &bundle_messages, &bundle_wc_requests)?;
        verify_bundle_authenticity(bundle, digest)?;

        let mut g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("queue lock poisoned: {e}")))?;
        let mut merge = MergeResult::empty();
        for tx in &bundle_txs {
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
        for msg in &bundle_messages {
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
        for req in &bundle_wc_requests {
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
        let plaintext_payload = serde_json::json!({
            "schema_version": 1,
            "txs": txs,
            "messages": messages,
            "wc_requests": wc_requests,
        });
        let canonical_plaintext = canonical_json_bytes(&plaintext_payload)?;
        let digest = keccak256(canonical_plaintext.clone());
        let signer = self.export_signer()?;
        let signature = signer
            .sign_hash_sync(&digest)
            .map_err(|e| PortError::Transport(format!("bundle signing failed: {e}")))?;
        let salt = generate_salt()?;
        let nonce = generate_nonce()?;
        let derived = derive_crypto(self.export_passphrase().as_bytes(), salt)?;
        let ciphertext = encrypt_aes_gcm(&derived.enc_key, nonce, &canonical_plaintext)?;
        let integrity_mac = hmac_sha256_b256(&derived.mac_key, &ciphertext)?;
        let crypto_envelope = BundleCryptoEnvelope {
            kdf_algorithm: derived.kdf_algorithm,
            kdf_salt_base64: encode_base64(&salt),
            enc_nonce_base64: encode_base64(&nonce),
            ciphertext_base64: encode_base64(&ciphertext),
        };

        let txs = plaintext_payload
            .get("txs")
            .cloned()
            .ok_or_else(|| PortError::Validation("bundle txs serialization failed".to_owned()))
            .and_then(|v| {
                serde_json::from_value(v)
                    .map_err(|e| PortError::Validation(format!("bundle tx decode failed: {e}")))
            })?;
        let messages = plaintext_payload
            .get("messages")
            .cloned()
            .ok_or_else(|| PortError::Validation("bundle messages serialization failed".to_owned()))
            .and_then(|v| {
                serde_json::from_value(v).map_err(|e| {
                    PortError::Validation(format!("bundle message decode failed: {e}"))
                })
            })?;
        let wc_requests = plaintext_payload
            .get("wc_requests")
            .cloned()
            .ok_or_else(|| PortError::Validation("bundle wc serialization failed".to_owned()))
            .and_then(|v| {
                serde_json::from_value(v)
                    .map_err(|e| PortError::Validation(format!("bundle wc decode failed: {e}")))
            })?;

        Ok(SigningBundle {
            schema_version: 1,
            exported_at_ms: TimestampMs(0),
            exporter: signer.address(),
            bundle_digest: digest,
            bundle_signature: Bytes::copy_from_slice(&signature.as_bytes()),
            txs,
            messages,
            wc_requests,
            crypto_envelope: Some(crypto_envelope),
            mac_algorithm: MacAlgorithm::HmacSha256V1,
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
        let recovered = recover_signature_signer(&sig, tx.safe_tx_hash)?;
        if recovered != signer {
            return Err(PortError::Validation(
                "importSig signer mismatch for txHash".to_owned(),
            ));
        }
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
                recovered_signer: Some(recovered),
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

fn bundle_payload_digest(
    txs: &[PendingSafeTx],
    messages: &[PendingSafeMessage],
    wc_requests: &[PendingWalletConnectRequest],
) -> Result<B256, PortError> {
    let payload = serde_json::json!({
        "schema_version": 1,
        "txs": txs,
        "messages": messages,
        "wc_requests": wc_requests,
    });
    let canonical = canonical_json_bytes(&payload)?;
    Ok(keccak256(canonical))
}

fn verify_bundle_authenticity(
    bundle: &SigningBundle,
    expected_digest: B256,
) -> Result<(), PortError> {
    if bundle.bundle_digest != expected_digest {
        return Err(PortError::Validation("bundle digest mismatch".to_owned()));
    }
    let sig = PrimitiveSignature::from_raw(bundle.bundle_signature.as_ref())
        .map_err(|e| PortError::Validation(format!("invalid bundle signature: {e}")))?;
    let recovered = sig
        .recover_address_from_prehash(&bundle.bundle_digest)
        .map_err(|e| PortError::Validation(format!("bundle signature recovery failed: {e}")))?;
    if recovered != bundle.exporter {
        return Err(PortError::Validation(
            "bundle exporter/signature mismatch".to_owned(),
        ));
    }
    Ok(())
}

fn recover_signature_signer(signature: &Bytes, payload_hash: B256) -> Result<Address, PortError> {
    let sig = PrimitiveSignature::from_raw(signature.as_ref())
        .map_err(|e| PortError::Validation(format!("invalid signature format: {e}")))?;
    sig.recover_address_from_prehash(&payload_hash)
        .map_err(|e| PortError::Validation(format!("signature recovery failed: {e}")))
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

fn decode_base64(input: &str) -> Result<Vec<u8>, PortError> {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(input)
        .map_err(|e| PortError::Validation(format!("base64 decode failed: {e}")))
}

fn encode_base64(input: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(input)
}
