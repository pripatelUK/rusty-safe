mod common;

use alloy::primitives::Address;
use rusty_safe_signing_core::{QueuePort, SigningCommand, TxStatus};

use common::{
    acquire_lock, new_orchestrator, owner_address, owner_signer, owner_tx_signature, safe_address,
    sign_tx_hash,
};

#[test]
fn manual_tx_signature_adds_signature_and_moves_to_signing() {
    let orch = new_orchestrator();
    acquire_lock(&orch);

    let create = orch
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
        .expect("hash parse");

    orch.handle(SigningCommand::AddTxSignature {
        safe_tx_hash: tx_hash,
        signer: owner_address(),
        signature: owner_tx_signature(tx_hash),
    })
    .expect("add signature");

    let tx = orch
        .queue
        .load_tx(tx_hash)
        .expect("load tx")
        .expect("tx present");
    assert_eq!(tx.status, TxStatus::Signing);
    assert_eq!(tx.signatures.len(), 1);
}

#[test]
fn duplicate_manual_signature_is_idempotent() {
    let orch = new_orchestrator();
    acquire_lock(&orch);

    let create = orch
        .handle(SigningCommand::CreateSafeTx {
            chain_id: 1,
            safe_address: safe_address(),
            nonce: 8,
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
        .expect("hash parse");

    let sig = owner_tx_signature(tx_hash);
    orch.handle(SigningCommand::AddTxSignature {
        safe_tx_hash: tx_hash,
        signer: owner_address(),
        signature: sig.clone(),
    })
    .expect("first signature add");

    orch.handle(SigningCommand::AddTxSignature {
        safe_tx_hash: tx_hash,
        signer: owner_address(),
        signature: sig,
    })
    .expect("duplicate signature should be accepted idempotently");

    let tx = orch
        .queue
        .load_tx(tx_hash)
        .expect("load tx")
        .expect("tx present");
    assert_eq!(tx.signatures.len(), 1);
}

#[test]
fn manual_signature_rejects_invalid_length() {
    let orch = new_orchestrator();
    acquire_lock(&orch);

    let create = orch
        .handle(SigningCommand::CreateSafeTx {
            chain_id: 1,
            safe_address: safe_address(),
            nonce: 9,
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
        .expect("hash parse");

    let err = orch
        .handle(SigningCommand::AddTxSignature {
            safe_tx_hash: tx_hash,
            signer: owner_address(),
            signature: alloy::primitives::Bytes::from(vec![0x11; 64]),
        })
        .expect_err("invalid signature length should fail");
    assert!(err.to_string().contains("INVALID_SIGNATURE_FORMAT"));
}

#[test]
fn manual_signature_rejects_recovery_mismatch() {
    let orch = new_orchestrator();
    acquire_lock(&orch);

    let create = orch
        .handle(SigningCommand::CreateSafeTx {
            chain_id: 1,
            safe_address: safe_address(),
            nonce: 10,
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
        .expect("hash parse");

    let wrong_signer: Address = "0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC"
        .parse()
        .expect("wrong signer");
    let sig = sign_tx_hash(tx_hash, &owner_signer());
    let err = orch
        .handle(SigningCommand::AddTxSignature {
            safe_tx_hash: tx_hash,
            signer: wrong_signer,
            signature: sig,
        })
        .expect_err("recovery mismatch should fail");
    assert!(err.to_string().contains("SIGNER_RECOVERY_MISMATCH"));
}
