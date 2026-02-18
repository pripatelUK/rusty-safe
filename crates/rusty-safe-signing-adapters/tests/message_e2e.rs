mod common;

use rusty_safe_signing_core::{MessageStatus, QueuePort, SigningCommand};

use common::{acquire_lock, new_orchestrator, owner_address, safe_address, signature_bytes};

#[test]
fn message_lifecycle_reaches_threshold() {
    let orch = new_orchestrator();
    acquire_lock(&orch);

    let create = orch
        .handle(SigningCommand::CreateMessage {
            chain_id: 1,
            safe_address: safe_address(),
            method: rusty_safe_signing_core::MessageMethod::PersonalSign,
            payload: serde_json::json!({
                "message": "hello world",
                "threshold": 2,
                "safeVersion": "1.3.0"
            }),
        })
        .expect("create message");

    let msg_hash = create
        .transition
        .expect("transition")
        .flow_id
        .trim_start_matches("msg:")
        .parse()
        .expect("parse msg hash");

    orch.handle(SigningCommand::AddMessageSignature {
        message_hash: msg_hash,
        signer: owner_address(),
        signature: signature_bytes(0x41),
    })
    .expect("first message signature");

    let mid = orch
        .queue
        .load_message(msg_hash)
        .expect("load message")
        .expect("message present");
    assert_eq!(mid.status, MessageStatus::AwaitingThreshold);

    let second_signer: alloy::primitives::Address = "0x1000000000000000000000000000000000000002"
        .parse()
        .expect("address parse");
    orch.handle(SigningCommand::AddMessageSignature {
        message_hash: msg_hash,
        signer: second_signer,
        signature: signature_bytes(0x42),
    })
    .expect("second message signature");

    let done = orch
        .queue
        .load_message(msg_hash)
        .expect("load message")
        .expect("message present");
    assert_eq!(done.status, MessageStatus::ThresholdMet);
    assert_eq!(done.signatures.len(), 2);
}
