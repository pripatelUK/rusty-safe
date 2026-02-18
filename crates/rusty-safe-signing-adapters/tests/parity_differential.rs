mod common;

use std::fs;
use std::path::PathBuf;

use alloy::primitives::B256;
use serde::Deserialize;

use rusty_safe_signing_core::{QueuePort, SigningCommand, WalletConnectPort, WcSessionAction};

use common::{acquire_lock, new_orchestrator};

#[derive(Debug, Deserialize)]
struct TxFixture {
    chain_id: u64,
    safe_address: String,
    nonce: u64,
    payload: serde_json::Value,
    manual_signature: FixtureSignature,
    confirm_signature: String,
    expect: TxExpect,
}

#[derive(Debug, Deserialize)]
struct TxExpect {
    final_status: String,
    signature_count: usize,
}

#[derive(Debug, Deserialize)]
struct FixtureSignature {
    signer: String,
    signature: String,
}

#[derive(Debug, Deserialize)]
struct MessageFixture {
    chain_id: u64,
    safe_address: String,
    method: String,
    payload: serde_json::Value,
    manual_signature: FixtureSignature,
    expect: MessageExpect,
}

#[derive(Debug, Deserialize)]
struct MessageExpect {
    final_status: String,
    signature_count: usize,
}

#[derive(Debug, Deserialize)]
struct WcFixture {
    pair_uri: String,
    expect_status_flow: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct AbiFixture {
    chain_id: u64,
    safe_address: String,
    nonce: u64,
    to: String,
    abi_json: String,
    method_signature: String,
    args: Vec<String>,
    value: String,
    expect_selector: String,
}

#[derive(Debug, Deserialize)]
struct UrlFixture {
    keys: Vec<String>,
}

#[test]
fn differential_parity_snapshots_match_required_flows() {
    let fixtures_root = fixtures_root();
    let tx_fixture: TxFixture = load_fixture(fixtures_root.join("tx/tx_lifecycle_basic.json"));
    let message_fixture: MessageFixture =
        load_fixture(fixtures_root.join("message/message_lifecycle_basic.json"));
    let wc_fixture: WcFixture =
        load_fixture(fixtures_root.join("wc/wc_session_lifecycle_basic.json"));
    let abi_fixture: AbiFixture = load_fixture(fixtures_root.join("abi/abi_transfer_fixture.json"));
    let url_fixture: UrlFixture = load_fixture(fixtures_root.join("url/url_keys_fixture.json"));

    let orch = new_orchestrator();
    acquire_lock(&orch);

    // PARITY-TX-01 / PARITY-TX-02
    let tx_create = orch
        .handle(SigningCommand::CreateSafeTx {
            chain_id: tx_fixture.chain_id,
            safe_address: tx_fixture.safe_address.parse().expect("safe address"),
            nonce: tx_fixture.nonce,
            payload: tx_fixture.payload.clone(),
        })
        .expect("create tx");
    let tx_flow = tx_create.transition.expect("tx transition").flow_id;
    let tx_hash: B256 = tx_flow.trim_start_matches("tx:").parse().expect("tx hash");
    orch.handle(SigningCommand::AddTxSignature {
        safe_tx_hash: tx_hash,
        signer: tx_fixture
            .manual_signature
            .signer
            .parse()
            .expect("manual signer"),
        signature: tx_fixture
            .manual_signature
            .signature
            .parse()
            .expect("manual signature"),
    })
    .expect("manual signature add");
    orch.handle(SigningCommand::ProposeTx {
        safe_tx_hash: tx_hash,
    })
    .expect("propose tx");
    orch.handle(SigningCommand::ConfirmTx {
        safe_tx_hash: tx_hash,
        signature: tx_fixture
            .confirm_signature
            .parse()
            .expect("confirm signature"),
    })
    .expect("confirm tx");
    orch.handle(SigningCommand::ExecuteTx {
        safe_tx_hash: tx_hash,
    })
    .expect("execute tx");
    let tx = orch
        .queue
        .load_tx(tx_hash)
        .expect("load tx")
        .expect("tx exists");
    assert_eq!(format!("{:?}", tx.status), tx_fixture.expect.final_status);
    assert_eq!(tx.signatures.len(), tx_fixture.expect.signature_count);

    // PARITY-MSG-01
    let message_method = match message_fixture.method.as_str() {
        "personal_sign" => rusty_safe_signing_core::MessageMethod::PersonalSign,
        "eth_signTypedData_v4" => rusty_safe_signing_core::MessageMethod::EthSignTypedDataV4,
        "eth_signTypedData" => rusty_safe_signing_core::MessageMethod::EthSignTypedData,
        _ => rusty_safe_signing_core::MessageMethod::EthSign,
    };
    let message_create = orch
        .handle(SigningCommand::CreateMessage {
            chain_id: message_fixture.chain_id,
            safe_address: message_fixture.safe_address.parse().expect("safe address"),
            method: message_method,
            payload: message_fixture.payload.clone(),
        })
        .expect("create message");
    let message_flow = message_create.transition.expect("msg transition").flow_id;
    let message_hash: B256 = message_flow
        .trim_start_matches("msg:")
        .parse()
        .expect("message hash");
    orch.handle(SigningCommand::AddMessageSignature {
        message_hash,
        signer: message_fixture
            .manual_signature
            .signer
            .parse()
            .expect("manual signer"),
        signature: message_fixture
            .manual_signature
            .signature
            .parse()
            .expect("manual signature"),
    })
    .expect("add message signature");
    let message = orch
        .queue
        .load_message(message_hash)
        .expect("load message")
        .expect("message exists");
    assert_eq!(
        format!("{:?}", message.status),
        message_fixture.expect.final_status
    );
    assert_eq!(
        message.signatures.len(),
        message_fixture.expect.signature_count
    );

    // PARITY-WC-01
    orch.handle(SigningCommand::WcPair {
        uri: wc_fixture.pair_uri,
    })
    .expect("wc pair");
    let sessions = orch.walletconnect.list_sessions().expect("list sessions");
    let topic = sessions.first().expect("paired session").topic.clone();
    assert_eq!(
        format!("{:?}", sessions.first().expect("session").status),
        wc_fixture.expect_status_flow[0]
    );
    orch.handle(SigningCommand::WcSessionAction {
        topic: topic.clone(),
        action: WcSessionAction::Approve,
    })
    .expect("approve wc session");
    let sessions = orch.walletconnect.list_sessions().expect("list sessions");
    let approved = sessions
        .iter()
        .find(|s| s.topic == topic)
        .expect("session after approve");
    assert_eq!(
        format!("{:?}", approved.status),
        wc_fixture.expect_status_flow[1]
    );
    orch.handle(SigningCommand::WcSessionAction {
        topic: topic.clone(),
        action: WcSessionAction::Disconnect,
    })
    .expect("disconnect wc session");
    let sessions = orch.walletconnect.list_sessions().expect("list sessions");
    let disconnected = sessions
        .iter()
        .find(|s| s.topic == topic)
        .expect("session after disconnect");
    assert_eq!(
        format!("{:?}", disconnected.status),
        wc_fixture.expect_status_flow[2]
    );

    // PARITY-ABI-01
    let abi_result = orch
        .handle(SigningCommand::CreateSafeTxFromAbi {
            chain_id: abi_fixture.chain_id,
            safe_address: abi_fixture.safe_address.parse().expect("safe address"),
            nonce: abi_fixture.nonce,
            to: abi_fixture.to.parse().expect("to"),
            abi_json: abi_fixture.abi_json,
            method_signature: abi_fixture.method_signature,
            args: abi_fixture.args,
            value: abi_fixture.value,
        })
        .expect("create tx from abi");
    let abi_flow = abi_result.transition.expect("abi transition").flow_id;
    let abi_hash: B256 = abi_flow.trim_start_matches("tx:").parse().expect("tx hash");
    let abi_tx = orch
        .queue
        .load_tx(abi_hash)
        .expect("load abi tx")
        .expect("abi tx exists");
    let selector = abi_tx
        .abi_context
        .as_ref()
        .expect("abi context")
        .method_selector;
    assert_eq!(hex_selector(selector), abi_fixture.expect_selector);

    // PARITY-COLLAB-01 URL key compatibility.
    let expected_keys = vec!["importTx", "importSig", "importMsg", "importMsgSig"];
    assert_eq!(url_fixture.keys, expected_keys);

    println!(
        "DIFF parity_tx={} parity_msg={} parity_wc={} parity_abi={} parity_collab={}",
        tx_fixture.expect.final_status,
        message_fixture.expect.final_status,
        wc_fixture.expect_status_flow.join("->"),
        abi_fixture.expect_selector,
        url_fixture.keys.join(",")
    );
}

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/signing")
}

fn load_fixture<T: for<'de> Deserialize<'de>>(path: PathBuf) -> T {
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read fixture {:?}: {e}", path));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse fixture {:?}: {e}", path))
}

fn hex_selector(selector: [u8; 4]) -> String {
    alloy::hex::encode(selector)
}
