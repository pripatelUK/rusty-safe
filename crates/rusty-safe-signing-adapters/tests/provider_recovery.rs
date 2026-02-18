mod common;

use alloy::primitives::Address;
use rusty_safe_signing_core::{QueuePort, SigningCommand};

use common::{acquire_lock, new_orchestrator, safe_address};

#[test]
fn recover_provider_events_marks_active_flows_deterministically() {
    let orch = new_orchestrator();
    acquire_lock(&orch);

    let tx_result = orch
        .handle(SigningCommand::CreateSafeTx {
            chain_id: 1,
            safe_address: safe_address(),
            nonce: 7,
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
    let tx_flow = tx_result.transition.expect("tx transition").flow_id;

    let message_result = orch
        .handle(SigningCommand::CreateMessage {
            chain_id: 1,
            safe_address: safe_address(),
            method: rusty_safe_signing_core::MessageMethod::PersonalSign,
            payload: serde_json::json!({
                "message": "localsafe parity message",
                "threshold": 1,
                "safeVersion": "1.3.0"
            }),
        })
        .expect("create message");
    let msg_flow = message_result.transition.expect("msg transition").flow_id;

    let account_a: Address = "0x1000000000000000000000000000000000000001"
        .parse()
        .expect("account a");
    let account_b: Address = "0x2000000000000000000000000000000000000002"
        .parse()
        .expect("account b");
    orch.provider
        .debug_inject_accounts_changed(vec![account_a, account_b])
        .expect("inject accounts");
    orch.provider
        .debug_inject_chain_changed(8453)
        .expect("inject chain");

    let result = orch
        .handle(SigningCommand::RecoverProviderEvents {
            expected_chain_id: 1,
        })
        .expect("recover events");
    let summary = result.provider_recovery.expect("summary");
    assert_eq!(summary.drained_events, 2);
    assert!(summary.accounts_changed);
    assert!(summary.chain_changed);
    assert_eq!(summary.latest_chain_id, Some(8453));
    assert!(summary.expected_chain_mismatch);
    assert_eq!(summary.latest_account_count, 2);
    assert_eq!(summary.tx_flows_marked, 1);
    assert_eq!(summary.message_flows_marked, 1);

    let tx_log = orch.queue.load_transition_log(&tx_flow).expect("tx log");
    assert!(tx_log.iter().any(|x| x
        .side_effect_outcome
        .as_deref()
        .unwrap_or_default()
        .contains("provider_recovery")));
    let msg_log = orch.queue.load_transition_log(&msg_flow).expect("msg log");
    assert!(msg_log.iter().any(|x| x
        .side_effect_outcome
        .as_deref()
        .unwrap_or_default()
        .contains("provider_recovery")));
}
