mod common;

use alloy::primitives::{Address, PrimitiveSignature, B256};
use common::{owner_address, owner_signer, sign_tx_hash};
use rusty_safe_signing_adapters::QueueAdapter;
use rusty_safe_signing_core::{
    MacAlgorithm, PendingSafeMessage, PendingSafeTx, QueuePort, TimestampMs, TxBuildSource,
    TxStatus, UrlImportEnvelope, UrlImportKey,
};

fn to_base64url(input: &str) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(input)
}

#[test]
fn import_tx_and_signature_url_keys_are_supported() {
    let queue = QueueAdapter::default();

    let tx_hash: B256 = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        .parse()
        .expect("tx hash");

    let tx_payload = serde_json::json!({
        "schema_version": 1,
        "chain_id": 1,
        "safe_address": "0x000000000000000000000000000000000000BEEF",
        "nonce": 1,
        "payload": {
          "to": "0x000000000000000000000000000000000000CAFE",
          "value": "0",
          "data": "0x",
          "threshold": 1
        },
        "build_source": "RawCalldata",
        "abi_context": null,
        "safe_tx_hash": tx_hash,
        "signatures": [],
        "status": "Draft",
        "state_revision": 0,
        "idempotency_key": "idem-1",
        "created_at_ms": 1,
        "updated_at_ms": 1,
        "executed_tx_hash": null,
        "mac_algorithm": "HmacSha256V1",
        "mac_key_id": "mac-key-v1",
        "integrity_mac": "0x0000000000000000000000000000000000000000000000000000000000000000"
    });
    let tx_payload = serde_json::to_string(&tx_payload).expect("serialize tx payload");

    let merge = queue
        .import_url_payload(&UrlImportEnvelope {
            key: UrlImportKey::ImportTx,
            schema_version: 1,
            payload_base64url: to_base64url(&tx_payload),
        })
        .expect("importTx should succeed");
    assert_eq!(merge.tx_added, 1);

    let sig_payload = serde_json::json!({
        "txHash": tx_hash,
        "signature": {
            "signer": owner_address(),
            "data": sign_tx_hash(tx_hash, &owner_signer())
        }
    });
    let sig_payload = serde_json::to_string(&sig_payload).expect("serialize sig payload");

    let merge = queue
        .import_url_payload(&UrlImportEnvelope {
            key: UrlImportKey::ImportSig,
            schema_version: 1,
            payload_base64url: to_base64url(&sig_payload),
        })
        .expect("importSig should succeed");
    assert_eq!(merge.tx_updated, 1);
}

#[test]
fn import_sig_rejects_signer_mismatch() {
    let queue = QueueAdapter::default();

    let tx_hash: B256 = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        .parse()
        .expect("tx hash");
    let tx_payload = serde_json::json!({
        "schema_version": 1,
        "chain_id": 1,
        "safe_address": "0x000000000000000000000000000000000000BEEF",
        "nonce": 1,
        "payload": {
          "to": "0x000000000000000000000000000000000000CAFE",
          "value": "0",
          "data": "0x",
          "threshold": 1
        },
        "build_source": "RawCalldata",
        "abi_context": null,
        "safe_tx_hash": tx_hash,
        "signatures": [],
        "status": "Draft",
        "state_revision": 0,
        "idempotency_key": "idem-1",
        "created_at_ms": 1,
        "updated_at_ms": 1,
        "executed_tx_hash": null,
        "mac_algorithm": "HmacSha256V1",
        "mac_key_id": "mac-key-v1",
        "integrity_mac": "0x0000000000000000000000000000000000000000000000000000000000000000"
    });
    let tx_payload = serde_json::to_string(&tx_payload).expect("serialize tx payload");

    queue
        .import_url_payload(&UrlImportEnvelope {
            key: UrlImportKey::ImportTx,
            schema_version: 1,
            payload_base64url: to_base64url(&tx_payload),
        })
        .expect("importTx should succeed");

    let sig_payload = serde_json::json!({
        "txHash": tx_hash,
        "signature": {
            "signer": "0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC",
            "data": sign_tx_hash(tx_hash, &owner_signer())
        }
    });
    let sig_payload = serde_json::to_string(&sig_payload).expect("serialize sig payload");

    let err = queue
        .import_url_payload(&UrlImportEnvelope {
            key: UrlImportKey::ImportSig,
            schema_version: 1,
            payload_base64url: to_base64url(&sig_payload),
        })
        .expect_err("signer mismatch should fail");
    assert!(err.to_string().contains("importSig signer mismatch"));
}

#[test]
fn import_msg_and_msg_sig_url_keys_are_supported() {
    let queue = QueueAdapter::default();

    let msg_hash: B256 = "0xcccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
        .parse()
        .expect("msg hash");

    let msg = PendingSafeMessage {
        schema_version: 1,
        chain_id: 1,
        safe_address: Address::ZERO,
        method: rusty_safe_signing_core::MessageMethod::PersonalSign,
        payload: serde_json::json!({"message":"hi", "threshold": 1}),
        message_hash: msg_hash,
        signatures: vec![],
        status: rusty_safe_signing_core::MessageStatus::Draft,
        state_revision: 0,
        idempotency_key: "idem-msg".to_owned(),
        created_at_ms: TimestampMs(1),
        updated_at_ms: TimestampMs(1),
        mac_algorithm: MacAlgorithm::HmacSha256V1,
        mac_key_id: "mac-key-v1".to_owned(),
        integrity_mac: B256::ZERO,
    };

    let payload = serde_json::to_string(&msg).expect("serialize message payload");
    let merge = queue
        .import_url_payload(&UrlImportEnvelope {
            key: UrlImportKey::ImportMsg,
            schema_version: 1,
            payload_base64url: to_base64url(&payload),
        })
        .expect("importMsg should succeed");
    assert_eq!(merge.message_added, 1);

    let msg_sig_payload = serde_json::json!({
        "messageHash": msg_hash,
        "signature": {
            "signer": "0x1000000000000000000000000000000000000002",
            "data": format!("0x{}", "22".repeat(65))
        }
    });
    let msg_sig_payload = serde_json::to_string(&msg_sig_payload).expect("serialize msg sig");

    let merge = queue
        .import_url_payload(&UrlImportEnvelope {
            key: UrlImportKey::ImportMsgSig,
            schema_version: 1,
            payload_base64url: to_base64url(&msg_sig_payload),
        })
        .expect("importMsgSig should succeed");
    assert_eq!(merge.message_updated, 1);
}

#[test]
fn invalid_url_schema_version_is_rejected() {
    let queue = QueueAdapter::default();
    let err = queue
        .import_url_payload(&UrlImportEnvelope {
            key: UrlImportKey::ImportTx,
            schema_version: 9,
            payload_base64url: "e30".to_owned(),
        })
        .expect_err("schema mismatch should fail");

    assert!(err.to_string().contains("URL_IMPORT_SCHEMA_INVALID"));
}

#[test]
fn export_bundle_contains_mac_and_digest() {
    let queue = QueueAdapter::default();
    let tx = PendingSafeTx {
        schema_version: 1,
        chain_id: 1,
        safe_address: Address::ZERO,
        nonce: 1,
        payload: serde_json::json!({"to": Address::ZERO, "threshold": 1}),
        build_source: TxBuildSource::RawCalldata,
        abi_context: None,
        safe_tx_hash: B256::ZERO,
        signatures: vec![],
        status: TxStatus::Draft,
        state_revision: 0,
        idempotency_key: "idem-tx".to_owned(),
        created_at_ms: TimestampMs(1),
        updated_at_ms: TimestampMs(1),
        executed_tx_hash: None,
        mac_algorithm: MacAlgorithm::HmacSha256V1,
        mac_key_id: "mac-key-v1".to_owned(),
        integrity_mac: B256::ZERO,
    };
    queue.save_tx(&tx).expect("save tx");

    let bundle = queue
        .export_bundle(&[
            "tx:0x0000000000000000000000000000000000000000000000000000000000000000".to_owned(),
        ])
        .expect("export bundle");
    assert_eq!(bundle.txs.len(), 1);
    assert_ne!(bundle.bundle_digest, B256::ZERO);
    assert_ne!(bundle.integrity_mac, B256::ZERO);
    let signature =
        PrimitiveSignature::from_raw(bundle.bundle_signature.as_ref()).expect("signature decode");
    let recovered = signature
        .recover_address_from_prehash(&bundle.bundle_digest)
        .expect("recover exporter");
    assert_eq!(recovered, bundle.exporter);
}
