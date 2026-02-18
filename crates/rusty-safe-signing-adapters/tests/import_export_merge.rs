mod common;

use alloy::primitives::{Address, Bytes, B256};
use rusty_safe_signing_adapters::crypto::{canonical_json_bytes, derive_crypto, hmac_sha256_b256};
use rusty_safe_signing_core::{
    MacAlgorithm, PendingSafeTx, QueuePort, SigningBundle, TimestampMs, TxBuildSource, TxStatus,
};

use common::{acquire_lock, new_orchestrator, safe_address};

#[test]
fn import_bundle_merges_by_state_revision() {
    let orch = new_orchestrator();

    let tx_hash: B256 = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        .parse()
        .expect("hash");
    orch.queue
        .save_tx(&PendingSafeTx {
            schema_version: 1,
            chain_id: 1,
            safe_address: safe_address(),
            nonce: 1,
            payload: serde_json::json!({"to": Address::ZERO, "threshold": 1}),
            build_source: TxBuildSource::RawCalldata,
            abi_context: None,
            safe_tx_hash: tx_hash,
            signatures: vec![],
            status: TxStatus::Draft,
            state_revision: 1,
            idempotency_key: "idem-old".to_owned(),
            created_at_ms: TimestampMs(1),
            updated_at_ms: TimestampMs(1),
            executed_tx_hash: None,
            mac_algorithm: MacAlgorithm::HmacSha256V1,
            mac_key_id: "mac-key-v1".to_owned(),
            integrity_mac: B256::ZERO,
        })
        .expect("seed tx");

    let imported = PendingSafeTx {
        schema_version: 1,
        chain_id: 1,
        safe_address: safe_address(),
        nonce: 1,
        payload: serde_json::json!({"to": Address::ZERO, "threshold": 1}),
        build_source: TxBuildSource::RawCalldata,
        abi_context: None,
        safe_tx_hash: tx_hash,
        signatures: vec![],
        status: TxStatus::Signing,
        state_revision: 2,
        idempotency_key: "idem-new".to_owned(),
        created_at_ms: TimestampMs(2),
        updated_at_ms: TimestampMs(2),
        executed_tx_hash: None,
        mac_algorithm: MacAlgorithm::HmacSha256V1,
        mac_key_id: "mac-key-v1".to_owned(),
        integrity_mac: B256::ZERO,
    };

    let integrity_mac = {
        let payload = serde_json::json!({
            "txs": [imported.clone()],
            "messages": [],
            "wc_requests": [],
        });
        let canonical = canonical_json_bytes(&payload).expect("canonical payload");
        let derived =
            derive_crypto(b"rusty-safe-dev-passphrase", [0u8; 16]).expect("derive fallback key");
        hmac_sha256_b256(&derived.mac_key, &canonical).expect("hmac")
    };

    let merge = orch
        .queue
        .import_bundle(&SigningBundle {
            schema_version: 1,
            exported_at_ms: TimestampMs(10),
            exporter: safe_address(),
            bundle_digest: B256::ZERO,
            bundle_signature: Bytes::from(vec![0x1b; 65]),
            txs: vec![imported],
            messages: vec![],
            wc_requests: vec![],
            crypto_envelope: None,
            mac_algorithm: MacAlgorithm::HmacSha256V1,
            mac_key_id: "mac-key-v1".to_owned(),
            integrity_mac,
        })
        .expect("import bundle");

    assert_eq!(merge.tx_updated, 1);
    let tx = orch
        .queue
        .load_tx(tx_hash)
        .expect("load tx")
        .expect("tx exists");
    assert_eq!(tx.state_revision, 2);
    assert_eq!(tx.status, TxStatus::Signing);
}

#[test]
fn export_bundle_contains_selected_flows() {
    let orch = new_orchestrator();
    acquire_lock(&orch);

    let create = orch
        .handle(rusty_safe_signing_core::SigningCommand::CreateSafeTx {
            chain_id: 1,
            safe_address: safe_address(),
            nonce: 33,
            payload: serde_json::json!({
                "to": "0x000000000000000000000000000000000000CAFE",
                "value": "0",
                "data": "0x",
                "operation": 0,
                "safeTxGas": "0",
                "baseGas": "0",
                "gasPrice": "0",
                "gasToken": "0x0000000000000000000000000000000000000000",
                "refundReceiver": "0x0000000000000000000000000000000000000000",
                "threshold": 1,
                "safeVersion": "1.3.0"
            }),
        })
        .expect("create tx");
    let flow_id = create.transition.expect("transition").flow_id;
    let bundle = orch
        .queue
        .export_bundle(&[flow_id])
        .expect("export selected flow");
    assert_eq!(bundle.schema_version, 1);
    assert_eq!(bundle.txs.len(), 1);
    assert!(bundle.bundle_signature.len() >= 65);
    assert_ne!(bundle.bundle_digest, B256::ZERO);
}
