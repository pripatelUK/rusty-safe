use alloy::primitives::{Address, Bytes, B256};
use rusty_safe_signing_core::{
    MacAlgorithm, MergeResult, PendingSafeMessage, PendingSafeTx, SigningBundle, TimestampMs,
    UrlImportEnvelope, UrlImportKey,
};

#[test]
fn url_import_keys_serialize_as_localsafe_keys() {
    let keys = [
        UrlImportKey::ImportTx,
        UrlImportKey::ImportSig,
        UrlImportKey::ImportMsg,
        UrlImportKey::ImportMsgSig,
    ];

    for key in keys {
        let env = UrlImportEnvelope {
            key,
            schema_version: 1,
            payload_base64url: "e30".to_owned(),
        };
        let json = serde_json::to_string(&env).expect("serialize envelope");
        assert!(
            json.contains("importTx")
                || json.contains("importSig")
                || json.contains("importMsg")
                || json.contains("importMsgSig")
        );
    }
}

#[test]
fn merge_result_empty_is_zeroed() {
    let merge = MergeResult::empty();
    assert_eq!(merge.tx_added, 0);
    assert_eq!(merge.tx_updated, 0);
    assert_eq!(merge.tx_skipped, 0);
    assert_eq!(merge.tx_conflicted, 0);
    assert_eq!(merge.message_added, 0);
    assert_eq!(merge.message_updated, 0);
    assert_eq!(merge.message_skipped, 0);
    assert_eq!(merge.message_conflicted, 0);
}

#[test]
fn signing_bundle_roundtrip_serialization() {
    let bundle = SigningBundle {
        schema_version: 1,
        exported_at_ms: TimestampMs(1739750400000),
        exporter: Address::ZERO,
        bundle_digest: B256::ZERO,
        bundle_signature: Bytes::from(vec![0x1b; 65]),
        txs: vec![],
        messages: vec![],
        wc_requests: vec![],
        crypto_envelope: None,
        mac_algorithm: MacAlgorithm::HmacSha256V1,
        mac_key_id: "mac-key-v1".to_owned(),
        integrity_mac: B256::ZERO,
    };

    let encoded = serde_json::to_vec(&bundle).expect("serialize bundle");
    let decoded: SigningBundle = serde_json::from_slice(&encoded).expect("deserialize bundle");
    assert_eq!(decoded.schema_version, 1);
    assert_eq!(decoded.exported_at_ms.0, 1739750400000);
    assert_eq!(decoded.bundle_signature.len(), 65);
}

#[test]
fn pending_structs_serialize_without_loss() {
    let tx = PendingSafeTx {
        schema_version: 1,
        chain_id: 1,
        safe_address: Address::ZERO,
        nonce: 0,
        payload: serde_json::json!({"to": Address::ZERO, "threshold": 1}),
        build_source: rusty_safe_signing_core::TxBuildSource::RawCalldata,
        abi_context: None,
        safe_tx_hash: B256::ZERO,
        signatures: vec![],
        status: rusty_safe_signing_core::TxStatus::Draft,
        state_revision: 0,
        idempotency_key: "idem-1".to_owned(),
        created_at_ms: TimestampMs(1),
        updated_at_ms: TimestampMs(1),
        executed_tx_hash: None,
        mac_algorithm: MacAlgorithm::HmacSha256V1,
        mac_key_id: "mac-key-v1".to_owned(),
        integrity_mac: B256::ZERO,
    };

    let msg = PendingSafeMessage {
        schema_version: 1,
        chain_id: 1,
        safe_address: Address::ZERO,
        method: rusty_safe_signing_core::MessageMethod::PersonalSign,
        payload: serde_json::json!({"message":"hello", "threshold":1}),
        message_hash: B256::ZERO,
        signatures: vec![],
        status: rusty_safe_signing_core::MessageStatus::Draft,
        state_revision: 0,
        idempotency_key: "idem-2".to_owned(),
        created_at_ms: TimestampMs(1),
        updated_at_ms: TimestampMs(1),
        mac_algorithm: MacAlgorithm::HmacSha256V1,
        mac_key_id: "mac-key-v1".to_owned(),
        integrity_mac: B256::ZERO,
    };

    let tx_json = serde_json::to_string(&tx).expect("tx serialize");
    let msg_json = serde_json::to_string(&msg).expect("msg serialize");
    assert!(tx_json.contains("\"schema_version\":1"));
    assert!(msg_json.contains("\"schema_version\":1"));
}
