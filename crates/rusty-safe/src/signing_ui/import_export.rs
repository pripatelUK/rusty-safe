use egui::Ui;
use rusty_safe_signing_core::{SigningBundle, UrlImportEnvelope, UrlImportKey};

use crate::signing_bridge::SigningBridge;
use crate::signing_ui::state::SigningUiState;

pub fn render_import_export(ui: &mut Ui, state: &mut SigningUiState, bridge: &SigningBridge) {
    ui.heading("Import / Export / Share");

    if let Some(err) = state.last_error.as_ref() {
        ui.colored_label(egui::Color32::RED, err);
    }
    if let Some(info) = state.last_info.as_ref() {
        ui.colored_label(egui::Color32::LIGHT_GREEN, info);
    }

    ui.separator();
    ui.heading("Import Bundle JSON");
    ui.text_edit_multiline(&mut state.import_export_state.import_bundle_json);
    if ui.button("Import Bundle").clicked() {
        let bundle: SigningBundle =
            match serde_json::from_str(&state.import_export_state.import_bundle_json) {
                Ok(v) => v,
                Err(e) => {
                    state.set_error(format!("bundle parse failed: {e}"));
                    return;
                }
            };
        match bridge.import_bundle(bundle) {
            Ok(result) => {
                if let Some(merge) = result.merge {
                    state.set_info(format!(
                        "merge result: tx +{} ~{} ={} !{} | msg +{} ~{} ={} !{}",
                        merge.tx_added,
                        merge.tx_updated,
                        merge.tx_skipped,
                        merge.tx_conflicted,
                        merge.message_added,
                        merge.message_updated,
                        merge.message_skipped,
                        merge.message_conflicted
                    ));
                } else {
                    state.set_info("bundle import complete");
                }
            }
            Err(e) => state.set_error(e.to_string()),
        }
    }

    ui.separator();
    ui.heading("Import URL Payload");
    ui.horizontal(|ui| {
        ui.label("Key:");
        ui.text_edit_singleline(&mut state.import_export_state.url_key);
    });
    ui.label("Base64URL payload:");
    ui.text_edit_multiline(&mut state.import_export_state.url_payload);
    if ui.button("Import URL Payload").clicked() {
        let key = match parse_url_key(&state.import_export_state.url_key) {
            Ok(k) => k,
            Err(e) => {
                state.set_error(e);
                return;
            }
        };
        let envelope = UrlImportEnvelope {
            key,
            schema_version: 1,
            payload_base64url: state.import_export_state.url_payload.trim().to_owned(),
        };
        match bridge.import_url_payload(envelope) {
            Ok(result) => {
                if let Some(merge) = result.merge {
                    state.set_info(format!(
                        "url merge: tx +{} ~{} ={} !{} | msg +{} ~{} ={} !{}",
                        merge.tx_added,
                        merge.tx_updated,
                        merge.tx_skipped,
                        merge.tx_conflicted,
                        merge.message_added,
                        merge.message_updated,
                        merge.message_skipped,
                        merge.message_conflicted
                    ));
                } else {
                    state.set_info("url import complete");
                }
            }
            Err(e) => state.set_error(e.to_string()),
        }
    }

    ui.separator();
    ui.heading("Export Bundle");
    ui.label("Flow IDs CSV (example: tx:0x...,msg:0x...,wc:req-id)");
    ui.text_edit_singleline(&mut state.import_export_state.export_flow_ids_csv);
    if ui.button("Export Selected Flows").clicked() {
        let flow_ids = state
            .import_export_state
            .export_flow_ids_csv
            .split(',')
            .map(|x| x.trim())
            .filter(|x| !x.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if flow_ids.is_empty() {
            state.set_error("no flow ids provided");
            return;
        }
        match bridge.export_bundle(flow_ids) {
            Ok(result) => {
                if let Some(bundle) = result.exported_bundle {
                    match serde_json::to_string_pretty(&bundle) {
                        Ok(s) => {
                            state.import_export_state.export_result_json = s;
                            state.set_info("bundle export complete");
                        }
                        Err(e) => state.set_error(format!("bundle serialization failed: {e}")),
                    }
                } else {
                    state.set_error("export command returned no bundle");
                }
            }
            Err(e) => state.set_error(e.to_string()),
        }
    }

    ui.label("Export result:");
    ui.text_edit_multiline(&mut state.import_export_state.export_result_json);
}

fn parse_url_key(raw: &str) -> Result<UrlImportKey, String> {
    match raw.trim() {
        "importTx" => Ok(UrlImportKey::ImportTx),
        "importSig" => Ok(UrlImportKey::ImportSig),
        "importMsg" => Ok(UrlImportKey::ImportMsg),
        "importMsgSig" => Ok(UrlImportKey::ImportMsgSig),
        _ => Err("invalid url key (expected importTx/importSig/importMsg/importMsgSig)".to_owned()),
    }
}
