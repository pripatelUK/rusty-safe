use alloy::primitives::{Address, Bytes, B256};
use egui::Ui;
use rusty_safe_signing_core::MessageMethod;

use crate::signing_bridge::SigningBridge;
use crate::signing_ui::state::SigningUiState;

pub fn render_message_details(ui: &mut Ui, state: &mut SigningUiState, bridge: &SigningBridge) {
    ui.heading("Message Details");

    egui::CollapsingHeader::new("Create Message Draft")
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Method:");
                ui.text_edit_singleline(&mut state.message_form.method);
                ui.label("Threshold:");
                ui.text_edit_singleline(&mut state.message_form.threshold);
                ui.label("Safe Version:");
                ui.text_edit_singleline(&mut state.message_form.safe_version);
            });
            ui.label("Payload JSON");
            ui.text_edit_multiline(&mut state.message_form.payload);

            if ui.button("Create Message").clicked() {
                match create_message(state, bridge) {
                    Ok(hash) => {
                        state.selected_flow_id = Some(format!("msg:{hash}"));
                        state.set_info(format!("created message {hash}"));
                    }
                    Err(e) => state.set_error(e),
                }
            }
        });

    let Some(flow_id) = state.selected_flow_id.clone() else {
        ui.label("No selected message flow.");
        return;
    };
    let Some(hash_str) = flow_id.strip_prefix("msg:") else {
        ui.label("Selected flow is not a message.");
        return;
    };
    let hash: B256 = match hash_str.parse() {
        Ok(v) => v,
        Err(e) => {
            state.set_error(format!("invalid message flow id: {e}"));
            return;
        }
    };

    let msg = match bridge.load_message(hash) {
        Ok(Some(msg)) => msg,
        Ok(None) => {
            ui.label("Message not found in queue.");
            return;
        }
        Err(e) => {
            state.set_error(e.to_string());
            return;
        }
    };

    ui.separator();
    ui.monospace(format!("Message Hash: {}", msg.message_hash));
    ui.label(format!("Status: {:?}", msg.status));
    ui.label(format!("Method: {:?}", msg.method));

    let mut payload_str =
        serde_json::to_string_pretty(&msg.payload).unwrap_or_else(|_| "{}".to_owned());
    ui.add(
        egui::TextEdit::multiline(&mut payload_str)
            .desired_rows(4)
            .desired_width(f32::INFINITY)
            .interactive(false),
    );

    ui.separator();
    ui.heading("Signatures");
    for sig in &msg.signatures {
        ui.monospace(format!(
            "{} | {:?} | {:?}",
            sig.signer, sig.source, sig.method
        ));
    }

    ui.heading("Manual Signature");
    ui.horizontal(|ui| {
        ui.label("Signer:");
        ui.text_edit_singleline(&mut state.message_form.manual_signer);
    });
    ui.label("Signature bytes:");
    ui.text_edit_multiline(&mut state.message_form.manual_signature);

    if ui.button("Add Message Signature").clicked() {
        let signer: Address = match state.message_form.manual_signer.trim().parse() {
            Ok(v) => v,
            Err(e) => {
                state.set_error(format!("invalid signer: {e}"));
                return;
            }
        };
        let signature: Bytes = match state.message_form.manual_signature.trim().parse() {
            Ok(v) => v,
            Err(e) => {
                state.set_error(format!("invalid signature: {e}"));
                return;
            }
        };
        match bridge.add_message_signature(msg.message_hash, signer, signature) {
            Ok(_) => state.set_info("message signature added"),
            Err(e) => state.set_error(e.to_string()),
        }
    }

    ui.separator();
    ui.heading("Transition Timeline");
    match bridge.load_transition_log(&flow_id) {
        Ok(records) => {
            if records.is_empty() {
                ui.label("No timeline entries yet.");
            }
            for rec in records {
                ui.monospace(format!(
                    "#{} {} -> {}",
                    rec.event_seq, rec.state_before, rec.state_after
                ));
            }
        }
        Err(e) => state.set_error(e.to_string()),
    }
}

fn parse_method(method: &str) -> Result<MessageMethod, String> {
    match method.trim() {
        "personal_sign" => Ok(MessageMethod::PersonalSign),
        "eth_sign" => Ok(MessageMethod::EthSign),
        "eth_signTypedData" => Ok(MessageMethod::EthSignTypedData),
        "eth_signTypedData_v4" => Ok(MessageMethod::EthSignTypedDataV4),
        _ => Err("unsupported message method".to_owned()),
    }
}

fn create_message(state: &SigningUiState, bridge: &SigningBridge) -> Result<B256, String> {
    let safe: Address = state
        .active_safe_address
        .trim()
        .parse()
        .map_err(|e| format!("invalid safe address: {e}"))?;
    let method = parse_method(&state.message_form.method)?;
    let threshold = state
        .message_form
        .threshold
        .trim()
        .parse::<u64>()
        .map_err(|e| format!("invalid threshold: {e}"))?;

    let mut payload: serde_json::Value = serde_json::from_str(&state.message_form.payload)
        .map_err(|e| format!("invalid payload JSON: {e}"))?;
    if payload.get("threshold").is_none() {
        payload
            .as_object_mut()
            .ok_or_else(|| "message payload must be a JSON object".to_owned())?
            .insert("threshold".to_owned(), serde_json::json!(threshold.max(1)));
    }
    if payload.get("safeVersion").is_none() {
        payload
            .as_object_mut()
            .ok_or_else(|| "message payload must be a JSON object".to_owned())?
            .insert(
                "safeVersion".to_owned(),
                serde_json::json!(state.message_form.safe_version),
            );
    }

    let result = bridge
        .create_message(state.active_chain_id, safe, method, payload)
        .map_err(|e| e.to_string())?;
    let flow_id = result
        .transition
        .as_ref()
        .map(|t| t.flow_id.clone())
        .ok_or_else(|| "missing transition log record".to_owned())?;
    flow_id
        .strip_prefix("msg:")
        .ok_or_else(|| "transition flow id missing msg prefix".to_owned())?
        .parse()
        .map_err(|e| format!("failed to parse message hash: {e}"))
}
