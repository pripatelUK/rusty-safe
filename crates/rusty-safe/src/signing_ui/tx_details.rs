use alloy::primitives::{Address, Bytes, B256};
use egui::Ui;

use crate::signing_bridge::SigningBridge;
use crate::signing_ui::state::SigningUiState;

pub fn render_tx_details(ui: &mut Ui, state: &mut SigningUiState, bridge: &SigningBridge) {
    ui.heading("Transaction Details");

    let Some(flow_id) = state.selected_flow_id.clone() else {
        ui.label("No selected tx flow. Select a tx in Queue first.");
        return;
    };
    let Some(hash_str) = flow_id.strip_prefix("tx:") else {
        ui.label("Selected flow is not a tx.");
        return;
    };
    let hash: B256 = match hash_str.parse() {
        Ok(v) => v,
        Err(e) => {
            state.set_error(format!("invalid tx flow id: {e}"));
            return;
        }
    };

    let tx = match bridge.load_tx(hash) {
        Ok(Some(tx)) => tx,
        Ok(None) => {
            ui.label("Tx not found in queue.");
            return;
        }
        Err(e) => {
            state.set_error(e.to_string());
            return;
        }
    };

    ui.monospace(format!("Safe Tx Hash: {}", tx.safe_tx_hash));
    ui.label(format!("Status: {:?}", tx.status));
    ui.label(format!("Chain: {}", tx.chain_id));
    ui.label(format!("Safe: {}", tx.safe_address));
    ui.label(format!("Nonce: {}", tx.nonce));
    ui.label(format!("Build: {:?}", tx.build_source));

    if let Some(executed) = tx.executed_tx_hash {
        ui.colored_label(
            egui::Color32::LIGHT_GREEN,
            format!("Executed Tx Hash: {executed}"),
        );
    }

    ui.separator();
    ui.label("Tx payload:");
    let mut payload_str =
        serde_json::to_string_pretty(&tx.payload).unwrap_or_else(|_| "{}".to_owned());
    ui.add(
        egui::TextEdit::multiline(&mut payload_str)
            .desired_rows(6)
            .desired_width(f32::INFINITY)
            .interactive(false),
    );

    if let Some(ctx) = tx.abi_context.as_ref() {
        ui.separator();
        ui.label("ABI composition:");
        ui.monospace(format!("Method: {}", ctx.method_signature));
        ui.monospace(format!(
            "Selector: 0x{}",
            alloy::primitives::hex::encode(ctx.method_selector)
        ));
        ui.monospace(format!("Override: {}", ctx.raw_calldata_override));
    }

    ui.separator();
    ui.heading("Signatures");
    if tx.signatures.is_empty() {
        ui.label("No signatures yet.");
    } else {
        egui::Grid::new("tx_signature_grid")
            .num_columns(5)
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Signer");
                ui.strong("Source");
                ui.strong("Method");
                ui.strong("Recovered");
                ui.strong("Added At");
                ui.end_row();
                for sig in &tx.signatures {
                    ui.monospace(sig.signer.to_string());
                    ui.label(format!("{:?}", sig.source));
                    ui.label(format!("{:?}", sig.method));
                    ui.label(
                        sig.recovered_signer
                            .map(|x| x.to_string())
                            .unwrap_or_else(|| "-".to_owned()),
                    );
                    ui.label(sig.added_at_ms.0.to_string());
                    ui.end_row();
                }
            });
    }

    ui.separator();
    ui.heading("Manual Signature");
    ui.horizontal(|ui| {
        ui.label("Signer:");
        ui.text_edit_singleline(&mut state.tx_form.manual_signer);
    });
    ui.label("Signature bytes (0x...):");
    ui.text_edit_multiline(&mut state.tx_form.manual_signature);
    if ui.button("Add Signature").clicked() {
        let signer: Address = match state.tx_form.manual_signer.trim().parse() {
            Ok(v) => v,
            Err(e) => {
                state.set_error(format!("invalid signer address: {e}"));
                return;
            }
        };
        let signature: Bytes = match state.tx_form.manual_signature.trim().parse() {
            Ok(v) => v,
            Err(e) => {
                state.set_error(format!("invalid signature: {e}"));
                return;
            }
        };
        match bridge.add_tx_signature(tx.safe_tx_hash, signer, signature) {
            Ok(_) => state.set_info("tx signature added"),
            Err(e) => state.set_error(e.to_string()),
        }
    }

    ui.separator();
    ui.heading("Propose / Confirm / Execute");
    if ui.button("Propose Tx").clicked() {
        match bridge.propose_tx(tx.safe_tx_hash) {
            Ok(_) => state.set_info("tx proposed"),
            Err(e) => state.set_error(e.to_string()),
        }
    }

    ui.horizontal(|ui| {
        ui.label("Confirm signature:");
        ui.text_edit_singleline(&mut state.tx_form.confirm_signature);
    });
    if ui.button("Confirm Tx").clicked() {
        let signature: Bytes = match state.tx_form.confirm_signature.trim().parse() {
            Ok(v) => v,
            Err(e) => {
                state.set_error(format!("invalid confirm signature: {e}"));
                return;
            }
        };
        match bridge.confirm_tx(tx.safe_tx_hash, signature) {
            Ok(_) => state.set_info("tx confirmed"),
            Err(e) => state.set_error(e.to_string()),
        }
    }

    if ui.button("Execute Tx").clicked() {
        match bridge.execute_tx(tx.safe_tx_hash) {
            Ok(_) => state.set_info("tx executed"),
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
                    "#{} {} -> {} (side_effect={}, outcome={})",
                    rec.event_seq,
                    rec.state_before,
                    rec.state_after,
                    rec.side_effect_dispatched,
                    rec.side_effect_outcome.unwrap_or_else(|| "-".to_owned())
                ));
            }
        }
        Err(e) => state.set_error(e.to_string()),
    }
}
