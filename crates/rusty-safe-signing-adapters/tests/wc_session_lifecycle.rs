mod common;

use rusty_safe_signing_core::{
    PendingWalletConnectRequest, QueuePort, SigningCommand, TimestampMs, WalletConnectPort,
    WcMethod, WcSessionAction, WcSessionContext, WcSessionStatus, WcStatus,
};

use common::{acquire_lock, new_orchestrator};

#[test]
fn walletconnect_session_lifecycle_approve_reject_disconnect() {
    let orch = new_orchestrator();
    acquire_lock(&orch);

    let topic = "wc-topic-lifecycle".to_owned();
    orch.walletconnect
        .insert_session(WcSessionContext {
            topic: topic.clone(),
            status: WcSessionStatus::Proposed,
            dapp_name: Some("Lifecycle dApp".to_owned()),
            dapp_url: Some("https://example.org".to_owned()),
            dapp_icons: vec![],
            capability_snapshot: None,
            updated_at_ms: TimestampMs(1),
        })
        .expect("insert session");

    orch.handle(SigningCommand::WcSessionAction {
        topic: topic.clone(),
        action: WcSessionAction::Approve,
    })
    .expect("approve session");

    let sessions = orch.walletconnect.list_sessions().expect("list sessions");
    let approved = sessions
        .iter()
        .find(|s| s.topic == topic)
        .expect("session exists after approve");
    assert_eq!(approved.status, WcSessionStatus::Approved);

    orch.handle(SigningCommand::WcSessionAction {
        topic: topic.clone(),
        action: WcSessionAction::Disconnect,
    })
    .expect("disconnect session");

    let sessions = orch.walletconnect.list_sessions().expect("list sessions");
    let disconnected = sessions
        .iter()
        .find(|s| s.topic == topic)
        .expect("session exists after disconnect");
    assert_eq!(disconnected.status, WcSessionStatus::Disconnected);
}

#[test]
fn walletconnect_reject_flow_is_persisted() {
    let orch = new_orchestrator();
    acquire_lock(&orch);

    let topic = "wc-topic-reject".to_owned();
    orch.walletconnect
        .insert_session(WcSessionContext {
            topic: topic.clone(),
            status: WcSessionStatus::Proposed,
            dapp_name: None,
            dapp_url: None,
            dapp_icons: vec![],
            capability_snapshot: None,
            updated_at_ms: TimestampMs(1),
        })
        .expect("insert session");

    let request = PendingWalletConnectRequest {
        request_id: "wc-req-1".to_owned(),
        topic: topic.clone(),
        session_status: WcSessionStatus::Proposed,
        chain_id: 1,
        method: WcMethod::PersonalSign,
        status: WcStatus::Pending,
        linked_safe_tx_hash: None,
        linked_message_hash: None,
        created_at_ms: TimestampMs(1),
        updated_at_ms: TimestampMs(1),
        expires_at_ms: Some(TimestampMs(1000)),
        state_revision: 0,
        correlation_id: "corr-1".to_owned(),
    };
    orch.queue
        .save_wc_request(&request)
        .expect("save wc request in queue");

    orch.handle(SigningCommand::WcSessionAction {
        topic: topic.clone(),
        action: WcSessionAction::Reject,
    })
    .expect("reject session");

    let sessions = orch.walletconnect.list_sessions().expect("list sessions");
    let rejected = sessions
        .iter()
        .find(|s| s.topic == topic)
        .expect("session exists after reject");
    assert_eq!(rejected.status, WcSessionStatus::Rejected);
}
