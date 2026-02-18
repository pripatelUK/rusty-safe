use alloy::primitives::{Address, B256};
use egui::Ui;

use crate::signing_bridge::SigningBridge;
use crate::signing_ui::state::{SigningQueueRow, SigningSurface, SigningUiState};

pub fn render_queue(ui: &mut Ui, state: &mut SigningUiState, bridge: &SigningBridge) {
    ui.heading("Signing Queue");
    ui.label("Pending tx/message/WalletConnect flows.");

    if let Some(err) = state.last_error.as_ref() {
        ui.colored_label(egui::Color32::RED, err);
    }
    if let Some(info) = state.last_info.as_ref() {
        ui.colored_label(egui::Color32::LIGHT_GREEN, info);
    }

    ui.horizontal(|ui| {
        ui.label("Safe:");
        ui.text_edit_singleline(&mut state.active_safe_address);
        ui.label("Chain:");
        let mut chain_str = state.active_chain_id.to_string();
        if ui.text_edit_singleline(&mut chain_str).changed() {
            if let Ok(parsed) = chain_str.parse::<u64>() {
                state.active_chain_id = parsed;
            }
        }
        if ui.button("Refresh").clicked() {
            match refresh_queue_rows(state, bridge) {
                Ok(()) => state.set_info("queue refreshed"),
                Err(e) => state.set_error(e),
            }
        }
    });

    if state.queue_rows.is_empty() {
        let _ = refresh_queue_rows(state, bridge);
    }

    ui.add_space(8.0);
    egui::CollapsingHeader::new("Create Tx (Raw)")
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Nonce");
                ui.text_edit_singleline(&mut state.tx_form.nonce);
                ui.label("To");
                ui.text_edit_singleline(&mut state.tx_form.to);
            });
            ui.horizontal(|ui| {
                ui.label("Value");
                ui.text_edit_singleline(&mut state.tx_form.value);
                ui.label("Threshold");
                ui.text_edit_singleline(&mut state.tx_form.threshold);
                ui.label("Safe Version");
                ui.text_edit_singleline(&mut state.tx_form.safe_version);
            });
            ui.label("Calldata");
            ui.text_edit_multiline(&mut state.tx_form.data);

            if ui.button("Create Raw Tx Draft").clicked() {
                match create_raw_tx(state, bridge) {
                    Ok(hash) => {
                        state.selected_flow_id = Some(format!("tx:{hash}"));
                        state.active_surface = SigningSurface::TxDetails;
                        let _ = refresh_queue_rows(state, bridge);
                        state.set_info(format!("created tx {hash}"));
                    }
                    Err(e) => state.set_error(e),
                }
            }
        });

    egui::CollapsingHeader::new("Create Tx (ABI)")
        .default_open(false)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Nonce");
                ui.text_edit_singleline(&mut state.tx_form.nonce);
                ui.label("To");
                ui.text_edit_singleline(&mut state.tx_form.to);
            });
            ui.label("ABI JSON");
            ui.text_edit_multiline(&mut state.tx_form.abi_json);
            ui.horizontal(|ui| {
                ui.label("Method");
                ui.text_edit_singleline(&mut state.tx_form.abi_method);
            });
            ui.label("Args (comma-separated JSON values)");
            ui.text_edit_singleline(&mut state.tx_form.abi_args);

            if ui.button("Create ABI Tx Draft").clicked() {
                match create_abi_tx(state, bridge) {
                    Ok(hash) => {
                        state.selected_flow_id = Some(format!("tx:{hash}"));
                        state.active_surface = SigningSurface::TxDetails;
                        let _ = refresh_queue_rows(state, bridge);
                        state.set_info(format!("created abi tx {hash}"));
                    }
                    Err(e) => state.set_error(e),
                }
            }
        });

    ui.separator();
    if state.queue_rows.is_empty() {
        ui.label("No queued flows yet.");
    } else {
        egui::Grid::new("signing_queue_grid")
            .num_columns(7)
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Type");
                ui.strong("Flow ID");
                ui.strong("State");
                ui.strong("Sigs");
                ui.strong("Origin");
                ui.strong("Build");
                ui.strong("Updated");
                ui.end_row();
                for row in &state.queue_rows {
                    if ui
                        .selectable_label(
                            state.selected_flow_id.as_deref() == Some(row.flow_id.as_str()),
                            &row.flow_kind,
                        )
                        .clicked()
                    {
                        state.selected_flow_id = Some(row.flow_id.clone());
                    }
                    ui.monospace(&row.flow_id);
                    ui.label(&row.state);
                    ui.label(&row.signature_progress);
                    ui.label(&row.origin);
                    ui.label(row.build_source.clone().unwrap_or_else(|| "-".to_owned()));
                    ui.label(row.updated_ms.to_string());
                    ui.end_row();
                }
            });
    }

    if ui.button("Open Selected Flow").clicked() {
        if let Some(flow_id) = state.selected_flow_id.as_ref() {
            if flow_id.starts_with("tx:") {
                state.active_surface = SigningSurface::TxDetails;
            } else if flow_id.starts_with("msg:") {
                state.active_surface = SigningSurface::MessageDetails;
            } else if flow_id.starts_with("wc:") {
                state.active_surface = SigningSurface::WalletConnect;
                state.wc_state.request_id = flow_id.trim_start_matches("wc:").to_owned();
            }
            state.set_info(format!("opened flow {flow_id}"));
        } else {
            state.set_error("no flow selected");
        }
    }
}

fn refresh_queue_rows(state: &mut SigningUiState, bridge: &SigningBridge) -> Result<(), String> {
    let mut rows = Vec::new();
    for tx in bridge.list_txs().map_err(|e| e.to_string())? {
        let threshold = tx
            .payload
            .get("threshold")
            .and_then(|v| v.as_u64())
            .unwrap_or(1);
        rows.push(SigningQueueRow {
            flow_id: format!("tx:{}", tx.safe_tx_hash),
            flow_kind: "tx".to_owned(),
            state: format!("{:?}", tx.status),
            signature_progress: format!("{}/{}", tx.signatures.len(), threshold),
            origin: "local".to_owned(),
            build_source: Some(format!("{:?}", tx.build_source)),
            updated_ms: tx.updated_at_ms.0,
        });
    }
    for msg in bridge.list_messages().map_err(|e| e.to_string())? {
        let threshold = msg
            .payload
            .get("threshold")
            .and_then(|v| v.as_u64())
            .unwrap_or(1);
        rows.push(SigningQueueRow {
            flow_id: format!("msg:{}", msg.message_hash),
            flow_kind: "message".to_owned(),
            state: format!("{:?}", msg.status),
            signature_progress: format!("{}/{}", msg.signatures.len(), threshold),
            origin: "local".to_owned(),
            build_source: None,
            updated_ms: msg.updated_at_ms.0,
        });
    }
    for req in bridge.list_wc_requests().map_err(|e| e.to_string())? {
        rows.push(SigningQueueRow {
            flow_id: format!("wc:{}", req.request_id),
            flow_kind: format!("wc:{:?}", req.method),
            state: format!("{:?}", req.status),
            signature_progress: "-".to_owned(),
            origin: "walletconnect".to_owned(),
            build_source: None,
            updated_ms: req.updated_at_ms.0,
        });
    }
    rows.sort_by(|a, b| b.updated_ms.cmp(&a.updated_ms));
    state.queue_rows = rows;
    Ok(())
}

fn create_raw_tx(state: &SigningUiState, bridge: &SigningBridge) -> Result<B256, String> {
    let safe: Address = state
        .active_safe_address
        .trim()
        .parse()
        .map_err(|e| format!("invalid safe address: {e}"))?;
    let to: Address = state
        .tx_form
        .to
        .trim()
        .parse()
        .map_err(|e| format!("invalid to address: {e}"))?;
    let nonce = state
        .tx_form
        .nonce
        .trim()
        .parse::<u64>()
        .map_err(|e| format!("invalid nonce: {e}"))?;
    let threshold = state
        .tx_form
        .threshold
        .trim()
        .parse::<u64>()
        .map_err(|e| format!("invalid threshold: {e}"))?;

    let payload = serde_json::json!({
        "to": to,
        "value": state.tx_form.value,
        "data": state.tx_form.data,
        "operation": 0u8,
        "safeTxGas": "0",
        "baseGas": "0",
        "gasPrice": "0",
        "gasToken": Address::ZERO,
        "refundReceiver": Address::ZERO,
        "threshold": threshold.max(1),
        "safeVersion": state.tx_form.safe_version,
    });

    let result = bridge
        .create_safe_tx(state.active_chain_id, safe, nonce, payload)
        .map_err(|e| e.to_string())?;
    let flow_id = result
        .transition
        .as_ref()
        .map(|t| t.flow_id.clone())
        .ok_or_else(|| "missing transition log record".to_owned())?;
    let hash: B256 = flow_id
        .strip_prefix("tx:")
        .ok_or_else(|| "transition flow id missing tx prefix".to_owned())?
        .parse()
        .map_err(|e| format!("failed to parse tx hash: {e}"))?;
    Ok(hash)
}

fn create_abi_tx(state: &SigningUiState, bridge: &SigningBridge) -> Result<B256, String> {
    let safe: Address = state
        .active_safe_address
        .trim()
        .parse()
        .map_err(|e| format!("invalid safe address: {e}"))?;
    let to: Address = state
        .tx_form
        .to
        .trim()
        .parse()
        .map_err(|e| format!("invalid to address: {e}"))?;
    let nonce = state
        .tx_form
        .nonce
        .trim()
        .parse::<u64>()
        .map_err(|e| format!("invalid nonce: {e}"))?;
    let args = state
        .tx_form
        .abi_args
        .split(',')
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .map(|x| x.to_owned())
        .collect::<Vec<_>>();

    let result = bridge
        .create_safe_tx_from_abi(
            state.active_chain_id,
            safe,
            nonce,
            to,
            state.tx_form.abi_json.clone(),
            state.tx_form.abi_method.clone(),
            args,
            state.tx_form.value.clone(),
        )
        .map_err(|e| e.to_string())?;
    let flow_id = result
        .transition
        .as_ref()
        .map(|t| t.flow_id.clone())
        .ok_or_else(|| "missing transition log record".to_owned())?;
    let hash: B256 = flow_id
        .strip_prefix("tx:")
        .ok_or_else(|| "transition flow id missing tx prefix".to_owned())?
        .parse()
        .map_err(|e| format!("failed to parse tx hash: {e}"))?;
    Ok(hash)
}
