mod common;

use rusty_safe_signing_core::{QueuePort, SigningCommand, TxStatus};

use common::{acquire_lock, new_orchestrator, owner_address, safe_address, signature_bytes};

#[test]
fn tx_lifecycle_create_sign_propose_confirm_execute() {
    let orch = new_orchestrator();
    acquire_lock(&orch);

    let create = orch
        .handle(SigningCommand::CreateSafeTx {
            chain_id: 1,
            safe_address: safe_address(),
            nonce: 42,
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
                "threshold": 2,
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

    orch.handle(SigningCommand::AddTxSignature {
        safe_tx_hash: tx_hash,
        signer: owner_address(),
        signature: signature_bytes(0x31),
    })
    .expect("add tx signature");

    orch.handle(SigningCommand::ProposeTx {
        safe_tx_hash: tx_hash,
    })
    .expect("propose tx");

    orch.handle(SigningCommand::ConfirmTx {
        safe_tx_hash: tx_hash,
        signature: signature_bytes(0x32),
    })
    .expect("confirm tx");

    let before_exec = orch
        .queue
        .load_tx(tx_hash)
        .expect("load tx")
        .expect("tx present");
    assert_eq!(before_exec.status, TxStatus::ReadyToExecute);

    orch.handle(SigningCommand::ExecuteTx {
        safe_tx_hash: tx_hash,
    })
    .expect("execute tx");

    let after_exec = orch
        .queue
        .load_tx(tx_hash)
        .expect("load tx")
        .expect("tx present");
    assert_eq!(after_exec.status, TxStatus::Executed);
    assert!(after_exec.executed_tx_hash.is_some());

    let logs = orch
        .queue
        .load_transition_log(&format!("tx:{tx_hash}"))
        .expect("load tx logs");
    assert!(!logs.is_empty());
    for (idx, record) in logs.iter().enumerate() {
        assert_eq!(record.event_seq as usize, idx + 1);
    }
}

#[test]
fn mutating_commands_require_writer_lock() {
    let orch = new_orchestrator();
    let err = orch
        .handle(SigningCommand::CreateSafeTx {
            chain_id: 1,
            safe_address: safe_address(),
            nonce: 1,
            payload: serde_json::json!({
                "to": "0x000000000000000000000000000000000000CAFE",
                "value": "0",
                "data": "0x",
                "operation": 0,
                "threshold": 1,
                "safeVersion": "1.3.0"
            }),
        })
        .expect_err("command should require writer lock");
    assert!(err.to_string().contains("WRITER_LOCK_CONFLICT"));
}
