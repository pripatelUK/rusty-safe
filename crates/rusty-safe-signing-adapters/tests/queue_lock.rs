use alloy::primitives::{Address, B256};
use rusty_safe_signing_adapters::QueueAdapter;
use rusty_safe_signing_core::{
    AppWriterLock, MacAlgorithm, PendingSafeTx, QueuePort, TimestampMs, TxBuildSource, TxStatus,
};

#[test]
fn writer_lock_conflict_is_enforced() {
    let queue = QueueAdapter::default();

    queue
        .acquire_writer_lock(AppWriterLock {
            holder_tab_id: "tab-a".to_owned(),
            tab_nonce: B256::ZERO,
            lock_epoch: 1,
            acquired_at_ms: TimestampMs(1),
            expires_at_ms: TimestampMs(10),
        })
        .expect("first lock");

    let err = queue
        .acquire_writer_lock(AppWriterLock {
            holder_tab_id: "tab-b".to_owned(),
            tab_nonce: B256::ZERO,
            lock_epoch: 2,
            acquired_at_ms: TimestampMs(2),
            expires_at_ms: TimestampMs(12),
        })
        .expect_err("second lock should conflict");

    assert!(err.to_string().contains("WRITER_LOCK_CONFLICT"));
}

#[test]
fn state_revision_regression_is_rejected() {
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
        state_revision: 2,
        idempotency_key: "idem-2".to_owned(),
        created_at_ms: TimestampMs(1),
        updated_at_ms: TimestampMs(1),
        executed_tx_hash: None,
        mac_algorithm: MacAlgorithm::HmacSha256V1,
        mac_key_id: "mac-key-v1".to_owned(),
        integrity_mac: B256::ZERO,
    };

    queue.save_tx(&tx).expect("save baseline tx");

    let mut older = tx.clone();
    older.state_revision = 1;
    let err = queue
        .save_tx(&older)
        .expect_err("older revision should be rejected");
    assert!(err.to_string().contains("state revision regression"));
}
