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
