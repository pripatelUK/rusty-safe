use alloy::primitives::B256;
use egui::Ui;
use rusty_safe_signing_core::{
    PendingWalletConnectRequest, ProviderCapabilitySnapshot, TimestampMs, WcMethod,
    WcSessionAction, WcSessionContext, WcSessionStatus, WcStatus,
};

use crate::signing_bridge::SigningBridge;
use crate::signing_ui::state::SigningUiState;

pub fn render_wc_requests(ui: &mut Ui, state: &mut SigningUiState, bridge: &SigningBridge) {
    ui.heading("WalletConnect Requests");

    ui.horizontal(|ui| {
        ui.label("Pair URI:");
        ui.text_edit_singleline(&mut state.wc_state.pair_uri);
        if ui.button("Pair Session").clicked() {
            if state.wc_state.pair_uri.trim().is_empty() {
                state.set_error("pair URI is required");
            } else {
                match bridge.wc_pair(state.wc_state.pair_uri.trim().to_owned()) {
                    Ok(_) => state.set_info("pair request accepted"),
                    Err(e) => state.set_error(e.to_string()),
                }
            }
        }
    });

    if ui.button("Seed Demo Session + Request").clicked() {
        match seed_demo(state, bridge) {
            Ok(()) => state.set_info("seeded demo WalletConnect request"),
            Err(e) => state.set_error(e),
        }
    }

    ui.separator();
    ui.heading("Sessions");
    match bridge.list_wc_sessions() {
        Ok(sessions) => {
            if sessions.is_empty() {
                ui.label("No WalletConnect sessions.");
            }
            for session in sessions {
                ui.group(|ui| {
                    ui.monospace(format!("Topic: {}", session.topic));
                    ui.label(format!("Status: {:?}", session.status));
                    if let Some(name) = session.dapp_name.as_ref() {
                        ui.label(format!("dApp: {name}"));
                    }
                    if let Some(snapshot) = session.capability_snapshot.as_ref() {
                        ui.label(format!(
                            "wallet_getCapabilities: {}",
                            snapshot.wallet_get_capabilities_supported
                        ));
                    }
                    ui.horizontal(|ui| {
                        if ui.small_button("Approve").clicked() {
                            match bridge
                                .wc_session_action(session.topic.clone(), WcSessionAction::Approve)
                            {
                                Ok(_) => state.set_info("session approved"),
                                Err(e) => state.set_error(e.to_string()),
                            }
                        }
                        if ui.small_button("Reject").clicked() {
                            match bridge
                                .wc_session_action(session.topic.clone(), WcSessionAction::Reject)
                            {
                                Ok(_) => state.set_info("session rejected"),
                                Err(e) => state.set_error(e.to_string()),
                            }
                        }
                        if ui.small_button("Disconnect").clicked() {
                            match bridge.wc_session_action(
                                session.topic.clone(),
                                WcSessionAction::Disconnect,
                            ) {
                                Ok(_) => state.set_info("session disconnected"),
                                Err(e) => state.set_error(e.to_string()),
                            }
                        }
                    });
                });
            }
        }
        Err(e) => state.set_error(e.to_string()),
    }

    ui.separator();
    ui.heading("Pending Requests");
    match bridge.list_wc_requests() {
        Ok(requests) => {
            if requests.is_empty() {
                ui.label("No WalletConnect requests.");
            }
            for req in requests {
                ui.group(|ui| {
                    ui.monospace(format!("Request ID: {}", req.request_id));
                    ui.label(format!("Method: {:?}", req.method));
                    ui.label(format!("Status: {:?}", req.status));
                    ui.label(format!("Session Status: {:?}", req.session_status));
                    ui.label(format!("Topic: {}", req.topic));
                    if let Some(exp) = req.expires_at_ms {
                        ui.label(format!("Expires: {}", exp.0));
                    }
                    if let Some(hash) = req.linked_safe_tx_hash {
                        ui.label(format!("Linked Tx: {hash}"));
                    }
                    if let Some(hash) = req.linked_message_hash {
                        ui.label(format!("Linked Message: {hash}"));
                    }
                    if ui.small_button("Select").clicked() {
                        state.wc_state.request_id = req.request_id.clone();
                    }
                });
            }
        }
        Err(e) => state.set_error(e.to_string()),
    }

    ui.separator();
    ui.heading("Respond Request");
    ui.horizontal(|ui| {
        ui.label("Request ID:");
        ui.text_edit_singleline(&mut state.wc_state.request_id);
    });
    ui.checkbox(&mut state.wc_state.deferred, "Deferred response mode");
    ui.label("Response JSON");
    ui.text_edit_multiline(&mut state.wc_state.response_json);

    if ui.button("Respond WalletConnect").clicked() {
        let parsed: serde_json::Value = match serde_json::from_str(&state.wc_state.response_json) {
            Ok(v) => v,
            Err(e) => {
                state.set_error(format!("invalid response JSON: {e}"));
                return;
            }
        };
        match bridge.respond_walletconnect(
            state.wc_state.request_id.clone(),
            parsed,
            state.wc_state.deferred,
        ) {
            Ok(_) => state.set_info("WalletConnect request responded"),
            Err(e) => state.set_error(e.to_string()),
        }
    }
}

fn seed_demo(state: &SigningUiState, bridge: &SigningBridge) -> Result<(), String> {
    let topic = if state.wc_state.active_topic.trim().is_empty() {
        "wc-topic-demo".to_owned()
    } else {
        state.wc_state.active_topic.clone()
    };
    let now = 1_739_750_400_000u64;

    bridge
        .debug_seed_wc_session(WcSessionContext {
            topic: topic.clone(),
            status: WcSessionStatus::Proposed,
            dapp_name: Some("Demo dApp".to_owned()),
            dapp_url: Some("https://example.org".to_owned()),
            dapp_icons: vec![],
            capability_snapshot: Some(ProviderCapabilitySnapshot {
                wallet_get_capabilities_supported: true,
                capabilities_json: Some(serde_json::json!({"wallet_getCapabilities": true})),
                collected_at_ms: TimestampMs(now),
            }),
            updated_at_ms: TimestampMs(now),
        })
        .map_err(|e| e.to_string())?;

    let req_id = if state.wc_state.request_id.trim().is_empty() {
        "wc-req-demo".to_owned()
    } else {
        state.wc_state.request_id.clone()
    };
    let chain_id = state
        .wc_state
        .seed_chain_id
        .trim()
        .parse::<u64>()
        .map_err(|e| format!("invalid seed chain id: {e}"))?;

    let request = PendingWalletConnectRequest {
        request_id: req_id,
        topic,
        session_status: WcSessionStatus::Proposed,
        chain_id,
        method: WcMethod::EthSendTransaction,
        status: WcStatus::Pending,
        linked_safe_tx_hash: Some(B256::ZERO),
        linked_message_hash: None,
        created_at_ms: TimestampMs(now),
        updated_at_ms: TimestampMs(now),
        expires_at_ms: Some(TimestampMs(now + 3_600_000)),
        state_revision: 0,
        correlation_id: "corr-demo".to_owned(),
    };

    bridge
        .debug_save_wc_request(request)
        .map_err(|e| e.to_string())
}
