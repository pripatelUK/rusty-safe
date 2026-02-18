use std::sync::{Arc, Mutex};
use std::thread;

use alloy::primitives::{Address, Bytes, B256};
use serde_json::json;
use tiny_http::{Method, Response, Server, StatusCode};

use rusty_safe_signing_adapters::{
    Eip1193Adapter, RuntimeProfile, SafeServiceAdapter, SigningAdapterConfig, WalletConnectAdapter,
};
use rusty_safe_signing_core::{
    PendingSafeTx, PortError, ProviderEventKind, ProviderPort, SafeServicePort, WalletConnectPort,
    WcSessionAction, WcSessionStatus,
};

#[test]
fn eip1193_event_recovery_is_deterministic() {
    let adapter = Eip1193Adapter::default();
    let account_a: Address = "0x1000000000000000000000000000000000000001"
        .parse()
        .expect("account a");
    let account_b: Address = "0x2000000000000000000000000000000000000002"
        .parse()
        .expect("account b");

    adapter
        .debug_inject_accounts_changed(vec![account_a, account_b])
        .expect("inject accounts");
    adapter
        .debug_inject_chain_changed(8453)
        .expect("inject chain");

    let accounts = adapter.request_accounts().expect("request accounts");
    assert_eq!(accounts, vec![account_a, account_b]);
    assert_eq!(adapter.chain_id().expect("chain"), 8453);

    let events = adapter.drain_events().expect("drain events");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].sequence + 1, events[1].sequence);
    assert_eq!(events[0].kind, ProviderEventKind::AccountsChanged);
    assert_eq!(events[1].kind, ProviderEventKind::ChainChanged);

    let no_events = adapter.drain_events().expect("drain empty events");
    assert!(no_events.is_empty());
}

#[test]
fn safe_service_http_runtime_handles_propose_confirm_execute() {
    let state = Arc::new(Mutex::new(Vec::<String>::new()));
    let (base_url, _join) = spawn_mock_server(Arc::clone(&state));

    let cfg = SigningAdapterConfig {
        safe_service_http_enabled: true,
        safe_service_base_url: base_url,
        safe_service_timeout_ms: 5_000,
        safe_service_retry_count: 1,
        ..SigningAdapterConfig::default()
    };

    let adapter = SafeServiceAdapter::with_config(cfg);
    let tx = sample_tx();

    adapter.propose_tx(&tx).expect("propose");
    adapter
        .confirm_tx(tx.safe_tx_hash, &Bytes::from(vec![0x11; 65]))
        .expect("confirm");
    let exec_hash = adapter.execute_tx(&tx).expect("execute");
    assert_eq!(
        exec_hash,
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            .parse::<B256>()
            .expect("hash")
    );

    let status = adapter.fetch_status(tx.safe_tx_hash).expect("status");
    assert!(status
        .get("transactionHash")
        .and_then(|v| v.as_str())
        .is_some());

    let calls = state.lock().expect("state lock");
    assert!(calls.iter().any(|p| p.contains("/multisig-transactions/")));
    assert!(calls.iter().any(|p| p.contains("/confirmations/")));
}

#[test]
fn walletconnect_http_runtime_pair_and_session_action() {
    let state = Arc::new(Mutex::new(Vec::<String>::new()));
    let (base_url, _join) = spawn_mock_server(Arc::clone(&state));

    let cfg = SigningAdapterConfig {
        walletconnect_bridge_url: Some(base_url),
        safe_service_retry_count: 1,
        safe_service_timeout_ms: 5_000,
        ..SigningAdapterConfig::default()
    };

    let adapter = WalletConnectAdapter::with_config(cfg);

    adapter
        .pair("wc:fixture@2?relay-protocol=irn&symKey=abc")
        .expect("pair");
    adapter
        .session_action("wc-topic-fixture", WcSessionAction::Approve)
        .expect("approve");
    adapter.sync().expect("sync");

    let sessions = adapter.list_sessions().expect("list sessions");
    assert!(!sessions.is_empty());
    assert_eq!(sessions[0].status, WcSessionStatus::Proposed);

    let calls = state.lock().expect("state lock");
    assert!(calls.iter().any(|p| p.contains("/pair")));
    assert!(calls
        .iter()
        .any(|p| p.contains("/session/wc-topic-fixture/action")));
    assert!(calls.iter().any(|p| p.contains("/sync")));
}

#[test]
fn production_profile_requires_eip1193_runtime() {
    let cfg = SigningAdapterConfig {
        runtime_profile: RuntimeProfile::Production,
        eip1193_proxy_url: None,
        ..SigningAdapterConfig::default()
    };
    let adapter = Eip1193Adapter::with_config(cfg);
    let err = adapter
        .request_accounts()
        .expect_err("runtime should be required");
    assert!(matches!(err, PortError::Policy(_)));
}

#[test]
fn production_profile_requires_safe_service_runtime() {
    let cfg = SigningAdapterConfig {
        runtime_profile: RuntimeProfile::Production,
        safe_service_http_enabled: false,
        ..SigningAdapterConfig::default()
    };
    let adapter = SafeServiceAdapter::with_config(cfg);
    let err = adapter
        .fetch_status(
            "0x0101010101010101010101010101010101010101010101010101010101010101"
                .parse()
                .expect("hash"),
        )
        .expect_err("runtime should be required");
    assert!(matches!(err, PortError::Policy(_)));
}

#[test]
fn production_profile_requires_walletconnect_runtime() {
    let cfg = SigningAdapterConfig {
        runtime_profile: RuntimeProfile::Production,
        walletconnect_bridge_url: None,
        ..SigningAdapterConfig::default()
    };
    let adapter = WalletConnectAdapter::with_config(cfg);
    let err = adapter
        .pair("wc:fixture@2?relay-protocol=irn&symKey=abc")
        .expect_err("runtime should be required");
    assert!(matches!(err, PortError::Policy(_)));
}

fn sample_tx() -> PendingSafeTx {
    PendingSafeTx {
        schema_version: 1,
        chain_id: 1,
        safe_address: "0x000000000000000000000000000000000000BEEF"
            .parse()
            .expect("safe"),
        nonce: 1,
        payload: json!({
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
        build_source: rusty_safe_signing_core::TxBuildSource::RawCalldata,
        abi_context: None,
        safe_tx_hash: "0x0101010101010101010101010101010101010101010101010101010101010101"
            .parse()
            .expect("safe tx hash"),
        signatures: vec![],
        status: rusty_safe_signing_core::TxStatus::Draft,
        state_revision: 0,
        idempotency_key: "idem-http".to_owned(),
        created_at_ms: rusty_safe_signing_core::TimestampMs(1),
        updated_at_ms: rusty_safe_signing_core::TimestampMs(1),
        executed_tx_hash: None,
        mac_algorithm: rusty_safe_signing_core::MacAlgorithm::HmacSha256V1,
        mac_key_id: "mac-key-v1".to_owned(),
        integrity_mac: B256::ZERO,
    }
}

fn spawn_mock_server(
    calls: Arc<Mutex<Vec<String>>>,
) -> (String, thread::JoinHandle<Result<(), PortError>>) {
    let server = Server::http("127.0.0.1:0").expect("start server");
    let addr = format!("http://{}", server.server_addr());

    let join = thread::spawn(move || {
        for _ in 0..16 {
            let req = match server.recv() {
                Ok(r) => r,
                Err(_) => break,
            };
            let method = req.method().clone();
            let path = req.url().to_owned();
            let path_lower = path.to_ascii_lowercase();
            if let Ok(mut g) = calls.lock() {
                g.push(path.clone());
            }

            let (code, payload) = match (method, path_lower.as_str()) {
                (Method::Post, p)
                    if p.ends_with(
                        "/api/v1/safes/0x000000000000000000000000000000000000beef/multisig-transactions/",
                    ) =>
                {
                    (201, json!({"ok": true}))
                }
                (Method::Post, p) if p.contains("/confirmations/") => (201, json!({"ok": true})),
                (Method::Get, p) if p.contains("/api/v1/multisig-transactions/") => (
                    200,
                    json!({"transactionHash":"0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}),
                ),
                (Method::Post, "/pair") => (200, json!({"ok": true})),
                (Method::Post, p) if p.ends_with("/action") => (200, json!({"ok": true})),
                (Method::Get, "/sessions") => (
                    200,
                    json!([{
                        "topic": "wc-topic-fixture",
                        "status": "Proposed",
                        "dapp_name": "Fixture",
                        "dapp_url": null,
                        "dapp_icons": [],
                        "capability_snapshot": null,
                        "updated_at_ms": 0
                    }]),
                ),
                (Method::Get, "/requests/pending") => (200, json!([])),
                (Method::Post, "/sync") => (200, json!({"ok": true})),
                (Method::Post, p) if p.ends_with("/response") => (200, json!({"ok": true})),
                (Method::Post, p) if p.ends_with("/error") => (200, json!({"ok": true})),
                _ => (404, json!({"error":"not found"})),
            };

            let response =
                Response::from_string(payload.to_string()).with_status_code(StatusCode(code));
            let _ = req.respond(response);
        }
        Ok(())
    });

    (addr, join)
}
