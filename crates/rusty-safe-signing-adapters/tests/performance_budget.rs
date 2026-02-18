mod common;

use std::time::Instant;

use alloy::primitives::{Address, B256};
use rusty_safe_signing_core::{
    MacAlgorithm, PendingSafeMessage, PendingWalletConnectRequest, QueuePort, SigningCommand,
    TimestampMs, WcMethod, WcSessionStatus, WcStatus,
};

use common::{acquire_lock, new_orchestrator, safe_address};

#[test]
fn command_and_rehydration_p95_within_budget() {
    let orch = new_orchestrator();
    acquire_lock(&orch);

    let command_budget_ms = std::env::var("RUSTY_SAFE_COMMAND_BUDGET_MS")
        .ok()
        .and_then(|v| v.parse::<u128>().ok())
        .unwrap_or(150);
    let rehydration_budget_ms = std::env::var("RUSTY_SAFE_REHYDRATION_BUDGET_MS")
        .ok()
        .and_then(|v| v.parse::<u128>().ok())
        .unwrap_or(1_500);

    let mut command_latencies = Vec::new();
    for i in 0..120u64 {
        let start = Instant::now();
        orch.handle(SigningCommand::CreateSafeTx {
            chain_id: 1,
            safe_address: safe_address(),
            nonce: i,
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
        command_latencies.push(start.elapsed().as_millis());
    }

    for i in 0..120u64 {
        let message_hash: B256 = format!("0x{:064x}", i + 10_000)
            .parse()
            .expect("message hash");
        orch.queue
            .save_message(&PendingSafeMessage {
                schema_version: 1,
                chain_id: 1,
                safe_address: Address::ZERO,
                method: rusty_safe_signing_core::MessageMethod::PersonalSign,
                payload: serde_json::json!({"message":"hi", "threshold": 1}),
                message_hash,
                signatures: vec![],
                status: rusty_safe_signing_core::MessageStatus::Draft,
                state_revision: 0,
                idempotency_key: format!("idem-msg-{i}"),
                created_at_ms: TimestampMs(i),
                updated_at_ms: TimestampMs(i),
                mac_algorithm: MacAlgorithm::HmacSha256V1,
                mac_key_id: "mac-key-v1".to_owned(),
                integrity_mac: B256::ZERO,
            })
            .expect("save message");
        orch.queue
            .save_wc_request(&PendingWalletConnectRequest {
                request_id: format!("req-{i}"),
                topic: "topic-a".to_owned(),
                session_status: WcSessionStatus::Approved,
                chain_id: 1,
                method: WcMethod::EthSendTransaction,
                status: WcStatus::Pending,
                linked_safe_tx_hash: None,
                linked_message_hash: Some(message_hash),
                created_at_ms: TimestampMs(i),
                updated_at_ms: TimestampMs(i),
                expires_at_ms: None,
                state_revision: 0,
                correlation_id: format!("corr-{i}"),
            })
            .expect("save wc request");
    }

    let mut rehydration_latencies = Vec::new();
    for _ in 0..50 {
        let start = Instant::now();
        let _ = orch.queue.list_txs().expect("list txs");
        let _ = orch.queue.list_messages().expect("list messages");
        let _ = orch.queue.list_wc_requests().expect("list wc requests");
        rehydration_latencies.push(start.elapsed().as_millis());
    }

    let command_p95 = p95_ms(&mut command_latencies);
    let rehydration_p95 = p95_ms(&mut rehydration_latencies);

    println!(
        "PERF command_p95_ms={} rehydration_p95_ms={} budget_command_ms={} budget_rehydration_ms={}",
        command_p95, rehydration_p95, command_budget_ms, rehydration_budget_ms
    );

    assert!(
        command_p95 <= command_budget_ms,
        "command p95 {}ms exceeds budget {}ms",
        command_p95,
        command_budget_ms
    );
    assert!(
        rehydration_p95 <= rehydration_budget_ms,
        "rehydration p95 {}ms exceeds budget {}ms",
        rehydration_p95,
        rehydration_budget_ms
    );
}

fn p95_ms(values: &mut [u128]) -> u128 {
    values.sort_unstable();
    let idx = ((values.len() as f64) * 0.95).ceil() as usize;
    values[idx.saturating_sub(1)]
}
