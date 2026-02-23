mod common;

use alloy::primitives::Address;
use rusty_safe_signing_core::{QueuePort, SigningCommand, TxBuildSource};

use common::{acquire_lock, new_orchestrator, safe_address};

#[test]
fn create_safe_tx_from_abi_persists_abi_context() {
    let orch = new_orchestrator();
    acquire_lock(&orch);

    let to: Address = "0x000000000000000000000000000000000000CAFE"
        .parse()
        .expect("to address");
    let abi_json = r#"[
      {
        "type": "function",
        "name": "transfer",
        "stateMutability": "nonpayable",
        "inputs": [
          {"name": "to", "type": "address"},
          {"name": "amount", "type": "uint256"}
        ],
        "outputs": []
      }
    ]"#;

    let result = orch
        .handle(SigningCommand::CreateSafeTxFromAbi {
            chain_id: 1,
            safe_address: safe_address(),
            nonce: 1,
            to,
            abi_json: abi_json.to_owned(),
            method_signature: "transfer(address,uint256)".to_owned(),
            args: vec![
                "\"0x000000000000000000000000000000000000dEaD\"".to_owned(),
                "\"100\"".to_owned(),
            ],
            value: "0".to_owned(),
        })
        .expect("create abi tx");

    let flow_id = result
        .transition
        .as_ref()
        .expect("transition")
        .flow_id
        .clone();
    let hash = flow_id
        .trim_start_matches("tx:")
        .parse()
        .expect("tx hash parse");
    let tx = orch
        .queue
        .load_tx(hash)
        .expect("load tx")
        .expect("tx present");

    assert_eq!(tx.build_source, TxBuildSource::AbiMethodForm);
    let ctx = tx.abi_context.expect("abi context");
    assert_eq!(ctx.method_signature, "transfer(address,uint256)");
    assert!(!ctx.raw_calldata_override);
    assert!(tx.payload.get("data").is_some());
}

#[test]
fn create_safe_tx_from_abi_rejects_bad_argument_count() {
    let orch = new_orchestrator();
    acquire_lock(&orch);

    let to: Address = "0x000000000000000000000000000000000000CAFE"
        .parse()
        .expect("to address");
    let abi_json = r#"[
      {
        "type": "function",
        "name": "transfer",
        "stateMutability": "nonpayable",
        "inputs": [
          {"name": "to", "type": "address"},
          {"name": "amount", "type": "uint256"}
        ],
        "outputs": []
      }
    ]"#;

    let err = orch
        .handle(SigningCommand::CreateSafeTxFromAbi {
            chain_id: 1,
            safe_address: safe_address(),
            nonce: 1,
            to,
            abi_json: abi_json.to_owned(),
            method_signature: "transfer(address,uint256)".to_owned(),
            args: vec!["\"0x000000000000000000000000000000000000dEaD\"".to_owned()],
            value: "0".to_owned(),
        })
        .expect_err("argument mismatch should fail");

    assert!(err.to_string().contains("argument count mismatch"));
}

#[test]
fn create_safe_tx_from_abi_has_deterministic_encoding() {
    let orch = new_orchestrator();
    acquire_lock(&orch);

    let to: Address = "0x000000000000000000000000000000000000CAFE"
        .parse()
        .expect("to address");
    let abi_json = r#"[
      {
        "type": "function",
        "name": "transfer",
        "stateMutability": "nonpayable",
        "inputs": [
          {"name": "to", "type": "address"},
          {"name": "amount", "type": "uint256"}
        ],
        "outputs": []
      }
    ]"#;

    let build_once = |nonce| {
        orch.handle(SigningCommand::CreateSafeTxFromAbi {
            chain_id: 1,
            safe_address: safe_address(),
            nonce,
            to,
            abi_json: abi_json.to_owned(),
            method_signature: "transfer(address,uint256)".to_owned(),
            args: vec![
                "\"0x000000000000000000000000000000000000dEaD\"".to_owned(),
                "\"100\"".to_owned(),
            ],
            value: "0".to_owned(),
        })
        .expect("create abi tx")
    };

    let first = build_once(10).transition.expect("transition").flow_id;
    let second = build_once(11).transition.expect("transition").flow_id;

    let first_hash = first.trim_start_matches("tx:").parse().expect("first hash");
    let second_hash = second
        .trim_start_matches("tx:")
        .parse()
        .expect("second hash");
    let first_tx = orch
        .queue
        .load_tx(first_hash)
        .expect("load first")
        .expect("first tx");
    let second_tx = orch
        .queue
        .load_tx(second_hash)
        .expect("load second")
        .expect("second tx");

    assert_eq!(first_tx.payload.get("data"), second_tx.payload.get("data"));
    assert_eq!(
        first_tx
            .abi_context
            .as_ref()
            .expect("first ctx")
            .method_selector,
        second_tx
            .abi_context
            .as_ref()
            .expect("second ctx")
            .method_selector
    );
}

#[test]
fn create_safe_tx_from_abi_rejects_unknown_method_signature() {
    let orch = new_orchestrator();
    acquire_lock(&orch);

    let to: Address = "0x000000000000000000000000000000000000CAFE"
        .parse()
        .expect("to address");
    let abi_json = r#"[
      {
        "type": "function",
        "name": "transfer",
        "stateMutability": "nonpayable",
        "inputs": [
          {"name": "to", "type": "address"},
          {"name": "amount", "type": "uint256"}
        ],
        "outputs": []
      }
    ]"#;

    let err = orch
        .handle(SigningCommand::CreateSafeTxFromAbi {
            chain_id: 1,
            safe_address: safe_address(),
            nonce: 1,
            to,
            abi_json: abi_json.to_owned(),
            method_signature: "approve(address,uint256)".to_owned(),
            args: vec![
                "\"0x000000000000000000000000000000000000dEaD\"".to_owned(),
                "\"100\"".to_owned(),
            ],
            value: "0".to_owned(),
        })
        .expect_err("unknown method signature should fail");

    assert!(err.to_string().contains("method not found"));
}
