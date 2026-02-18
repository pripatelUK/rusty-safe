mod common;

use rusty_safe_signing_core::{
    PendingWalletConnectRequest, QueuePort, SigningCommand, TimestampMs, WcMethod, WcSessionStatus,
    WcStatus,
};

use common::{acquire_lock, new_orchestrator, safe_address};

#[test]
fn walletconnect_deferred_response_happy_path() {
    let orch = new_orchestrator();
    acquire_lock(&orch);

    let create = orch
        .handle(SigningCommand::CreateSafeTx {
            chain_id: 1,
            safe_address: safe_address(),
            nonce: 11,
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
    let tx_hash = create
        .transition
        .expect("transition")
        .flow_id
        .trim_start_matches("tx:")
        .parse()
        .expect("parse tx hash");

    orch.queue
        .save_wc_request(&PendingWalletConnectRequest {
            request_id: "wc-deferred-1".to_owned(),
            topic: "topic-1".to_owned(),
            session_status: WcSessionStatus::Approved,
            chain_id: 1,
            method: WcMethod::EthSendTransaction,
            status: WcStatus::Routed,
            linked_safe_tx_hash: Some(tx_hash),
            linked_message_hash: None,
            created_at_ms: TimestampMs(1),
            updated_at_ms: TimestampMs(1),
            expires_at_ms: Some(TimestampMs(1_739_760_000_000)),
            state_revision: 0,
            correlation_id: "corr-deferred".to_owned(),
        })
        .expect("save wc request");
    orch.walletconnect
        .insert_request(PendingWalletConnectRequest {
            request_id: "wc-deferred-1".to_owned(),
            topic: "topic-1".to_owned(),
            session_status: WcSessionStatus::Approved,
            chain_id: 1,
            method: WcMethod::EthSendTransaction,
            status: WcStatus::Routed,
            linked_safe_tx_hash: Some(tx_hash),
            linked_message_hash: None,
            created_at_ms: TimestampMs(1),
            updated_at_ms: TimestampMs(1),
            expires_at_ms: Some(TimestampMs(1_739_760_000_000)),
            state_revision: 0,
            correlation_id: "corr-deferred".to_owned(),
        })
        .expect("seed wc runtime request");

    orch.handle(SigningCommand::RespondWalletConnect {
        request_id: "wc-deferred-1".to_owned(),
        result: serde_json::json!({"executedTxHash":"0x1234"}),
        deferred: true,
    })
    .expect("respond deferred");

    let req = orch
        .queue
        .load_wc_request("wc-deferred-1")
        .expect("load request")
        .expect("request exists");
    assert_eq!(req.status, WcStatus::Responded);

    let logs = orch
        .queue
        .load_transition_log("wc:wc-deferred-1")
        .expect("load wc logs");
    assert!(!logs.is_empty());
}

#[test]
fn walletconnect_deferred_response_requires_approved_session() {
    let orch = new_orchestrator();
    acquire_lock(&orch);

    orch.queue
        .save_wc_request(&PendingWalletConnectRequest {
            request_id: "wc-deferred-2".to_owned(),
            topic: "topic-2".to_owned(),
            session_status: WcSessionStatus::Proposed,
            chain_id: 1,
            method: WcMethod::EthSendTransaction,
            status: WcStatus::Routed,
            linked_safe_tx_hash: Some(alloy::primitives::B256::ZERO),
            linked_message_hash: None,
            created_at_ms: TimestampMs(1),
            updated_at_ms: TimestampMs(1),
            expires_at_ms: Some(TimestampMs(1_739_760_000_000)),
            state_revision: 0,
            correlation_id: "corr-deferred-2".to_owned(),
        })
        .expect("save wc request");

    let err = orch
        .handle(SigningCommand::RespondWalletConnect {
            request_id: "wc-deferred-2".to_owned(),
            result: serde_json::json!({"ok":true}),
            deferred: true,
        })
        .expect_err("respond must fail for unapproved session");

    assert!(err.to_string().contains("WC_SESSION_NOT_APPROVED"));
}
