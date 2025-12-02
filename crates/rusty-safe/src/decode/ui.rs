//! Calldata decode UI rendering

use eframe::egui;

use super::types::*;

/// Render the full decode section
pub fn render_decode_section(
    ui: &mut egui::Ui,
    decode: &DecodedTransaction,
    on_expand: &mut impl FnMut(usize),
) {
    ui.add_space(10.0);

    match &decode.kind {
        TransactionKind::Empty => {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("📦 Calldata").strong());
                ui.label(egui::RichText::new("(empty)").weak());
            });
        }
        TransactionKind::Single(single) => {
            render_single_section(ui, single, &decode.selector);
        }
        TransactionKind::MultiSend(multi) => {
            render_multisend_section(ui, multi, on_expand);
        }
        TransactionKind::Unknown => {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("📦 Calldata").strong());
                ui.label(egui::RichText::new(format!("Unknown selector: {}", decode.selector)).weak());
            });
            ui.add_space(5.0);
            render_raw_data(ui, &decode.raw_data);
        }
    }
}

/// Render single function call decode
fn render_single_section(ui: &mut egui::Ui, decode: &SingleDecode, selector: &str) {
    // Header
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("📦 Calldata Decoding").strong());
        ui.label(egui::RichText::new(format!("[{}]", selector)).monospace().weak());
        render_status_badge(ui, &decode.comparison);
    });

    ui.add_space(8.0);

    render_single_comparison(ui, decode);
}

/// Render side-by-side comparison for a single decode
pub fn render_single_comparison(ui: &mut egui::Ui, decode: &SingleDecode) {
    egui::Grid::new(format!("decode_compare_{:p}", decode))
        .num_columns(2)
        .spacing([30.0, 4.0])
        .min_col_width(200.0)
        .show(ui, |ui| {
            // Headers
            ui.label(egui::RichText::new("Safe API").strong().underline());
            ui.label(egui::RichText::new("Independent (4byte)").strong().underline());
            ui.end_row();

            // Method row
            render_method_row(ui, decode);

            // Separator
            ui.separator();
            ui.separator();
            ui.end_row();

            // Parameter rows
            render_params_rows(ui, decode);
        });

    // Status message
    ui.add_space(8.0);
    render_comparison_message(ui, &decode.comparison);
}

/// Render method name row
fn render_method_row(ui: &mut egui::Ui, decode: &SingleDecode) {
    // API method
    if let Some(api) = &decode.api {
        ui.label(egui::RichText::new(&api.method).monospace().strong());
    } else {
        ui.label(egui::RichText::new("—").weak());
    }

    // Local method
    if let Some(local) = &decode.local {
        ui.label(egui::RichText::new(&local.method).monospace().strong());
    } else {
        match &decode.comparison {
            ComparisonResult::Pending => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(egui::RichText::new("Loading...").weak());
                });
            }
            ComparisonResult::Failed(e) => {
                ui.label(egui::RichText::new(format!("Failed: {}", e)).weak());
            }
            _ => {
                ui.label(egui::RichText::new("—").weak());
            }
        }
    }
    ui.end_row();
}

/// Render parameter comparison rows
fn render_params_rows(ui: &mut egui::Ui, decode: &SingleDecode) {
    static EMPTY_API: Vec<ApiParam> = Vec::new();
    static EMPTY_LOCAL: Vec<LocalParam> = Vec::new();

    let api_params = decode.api.as_ref().map(|a| a.params.as_slice()).unwrap_or(&EMPTY_API);
    let local_params = decode.local.as_ref().map(|l| l.params.as_slice()).unwrap_or(&EMPTY_LOCAL);

    let max_len = api_params.len().max(local_params.len());

    for i in 0..max_len {
        let api_param = api_params.get(i);
        let local_param = local_params.get(i);

        // Check if this param has a mismatch
        let has_mismatch = matches!(&decode.comparison, ComparisonResult::ParamMismatch(diffs) if diffs.iter().any(|d| d.index == i));

        // API param
        if let Some(ap) = api_param {
            let label = format!("{} ({}):", ap.name, ap.typ);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(label).weak().small());
                let value_text = egui::RichText::new(truncate_value(&ap.value, 40)).monospace();
                if has_mismatch {
                    ui.label(value_text.color(egui::Color32::from_rgb(220, 80, 80)));
                } else {
                    ui.label(value_text);
                }
            });
        } else {
            ui.label(egui::RichText::new("—").weak());
        }

        // Local param
        if let Some(lp) = local_param {
            let label = format!("param{} ({}):", i, lp.typ);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(label).weak().small());
                let value_text = egui::RichText::new(truncate_value(&lp.value, 40)).monospace();
                if has_mismatch {
                    ui.label(value_text.color(egui::Color32::from_rgb(100, 200, 100)));
                } else {
                    ui.label(value_text);
                }
            });
        } else {
            ui.label(egui::RichText::new("—").weak());
        }

        ui.end_row();
    }
}

/// Render MultiSend section
fn render_multisend_section(
    ui: &mut egui::Ui,
    multi: &MultiSendDecode,
    on_expand: &mut impl FnMut(usize),
) {
    // Header with summary
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!(
            "📦 MultiSend ({} transactions)",
            multi.transactions.len()
        )).strong());

        render_summary_badges(ui, &multi.summary);
    });

    ui.add_space(8.0);

    // Collapsible transactions
    for tx in &multi.transactions {
        render_multisend_tx(ui, tx, on_expand);
    }
}

/// Render a single MultiSend transaction (collapsible)
fn render_multisend_tx(
    ui: &mut egui::Ui,
    tx: &MultiSendTx,
    on_expand: &mut impl FnMut(usize),
) {
    let status_emoji = match &tx.decode {
        Some(d) if d.comparison.is_match() => "✅",
        Some(d) if d.comparison.is_mismatch() => "❌",
        Some(_) => "⚠️",
        None if tx.is_loading => "⏳",
        None => "📋",
    };

    let header = format!(
        "#{} {} → {} ({})",
        tx.index + 1,
        status_emoji,
        truncate_address(&tx.to),
        if tx.value == "0" {
            "0 ETH".to_string()
        } else {
            format!("{} wei", tx.value)
        }
    );

    let response = egui::CollapsingHeader::new(header)
        .id_salt(format!("multisend_tx_{}", tx.index))
        .default_open(tx.is_expanded)
        .show(ui, |ui| {
            ui.add_space(4.0);

            // Transaction details
            egui::Grid::new(format!("multisend_details_{}", tx.index))
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("To:");
                    ui.label(egui::RichText::new(&tx.to).monospace());
                    ui.end_row();

                    ui.label("Value:");
                    ui.label(format!("{} wei", tx.value));
                    ui.end_row();

                    ui.label("Operation:");
                    ui.label(if tx.operation == 0 { "Call" } else { "DelegateCall" });
                    ui.end_row();
                });

            ui.add_space(8.0);

            // Decode comparison
            if tx.is_loading {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Verifying calldata...");
                });
            } else if let Some(decode) = &tx.decode {
                render_single_comparison(ui, decode);
            } else if tx.data == "0x" || tx.data.is_empty() {
                ui.label(egui::RichText::new("No calldata").weak());
            } else {
                ui.label("Expand to verify calldata");
            }
        });

    // Trigger expand callback when first opened
    if response.header_response.clicked() && tx.decode.is_none() && !tx.is_loading && tx.data != "0x" {
        on_expand(tx.index);
    }
}

/// Render summary badges for MultiSend
fn render_summary_badges(ui: &mut egui::Ui, summary: &MultiSendSummary) {
    if summary.verified > 0 {
        ui.label(
            egui::RichText::new(format!("✅ {}", summary.verified))
                .color(egui::Color32::from_rgb(100, 200, 100)),
        );
    }
    if summary.mismatched > 0 {
        ui.label(
            egui::RichText::new(format!("❌ {}", summary.mismatched))
                .color(egui::Color32::from_rgb(220, 80, 80)),
        );
    }
    if summary.pending > 0 {
        ui.label(egui::RichText::new(format!("⏳ {}", summary.pending)).weak());
    }
}

/// Render status badge
fn render_status_badge(ui: &mut egui::Ui, result: &ComparisonResult) {
    match result {
        ComparisonResult::Match => {
            ui.label(egui::RichText::new("✅").color(egui::Color32::from_rgb(100, 200, 100)));
        }
        ComparisonResult::MethodMismatch { .. } | ComparisonResult::ParamMismatch(_) => {
            ui.label(egui::RichText::new("❌").color(egui::Color32::from_rgb(220, 80, 80)));
        }
        ComparisonResult::OnlyApi | ComparisonResult::OnlyLocal => {
            ui.label(egui::RichText::new("⚠️").color(egui::Color32::from_rgb(220, 180, 50)));
        }
        ComparisonResult::Pending => {
            ui.spinner();
        }
        ComparisonResult::Failed(_) => {
            ui.label(egui::RichText::new("⚠️").color(egui::Color32::from_rgb(220, 180, 50)));
        }
    }
}

/// Render comparison result message
fn render_comparison_message(ui: &mut egui::Ui, result: &ComparisonResult) {
    match result {
        ComparisonResult::Match => {
            ui.label(
                egui::RichText::new("✅ Decodings match - independently verified")
                    .color(egui::Color32::from_rgb(100, 200, 100)),
            );
        }
        ComparisonResult::MethodMismatch { api, local } => {
            ui.label(
                egui::RichText::new(format!(
                    "❌ Method mismatch! API: '{}', Independent: '{}'",
                    api, local
                ))
                .color(egui::Color32::from_rgb(220, 80, 80)),
            );
        }
        ComparisonResult::ParamMismatch(diffs) => {
            ui.label(
                egui::RichText::new(format!(
                    "❌ {} parameter(s) differ between API and independent decode!",
                    diffs.len()
                ))
                .color(egui::Color32::from_rgb(220, 80, 80)),
            );
            ui.label(
                egui::RichText::new("Trust the Independent column - this is what will execute")
                    .weak()
                    .small(),
            );
        }
        ComparisonResult::OnlyApi => {
            ui.label(
                egui::RichText::new("⚠️ Could not verify independently (4byte lookup failed)")
                    .color(egui::Color32::from_rgb(220, 180, 50)),
            );
        }
        ComparisonResult::OnlyLocal => {
            ui.label(
                egui::RichText::new("✅ Decoded independently (API didn't provide decode)")
                    .color(egui::Color32::from_rgb(100, 200, 100)),
            );
        }
        ComparisonResult::Pending => {
            // Already showing spinner
        }
        ComparisonResult::Failed(e) => {
            ui.label(
                egui::RichText::new(format!("⚠️ Decode failed: {}", e))
                    .color(egui::Color32::from_rgb(220, 180, 50)),
            );
        }
    }
}

/// Render raw calldata
fn render_raw_data(ui: &mut egui::Ui, data: &str) {
    let display = truncate_value(data, 66);
    ui.label(egui::RichText::new(display).monospace().small());
}

/// Truncate a value for display
fn truncate_value(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        value.to_string()
    } else {
        format!("{}...", &value[..max_len])
    }
}

/// Truncate address for display
fn truncate_address(addr: &str) -> String {
    if addr.len() >= 42 {
        format!("{}...{}", &addr[..6], &addr[addr.len() - 4..])
    } else {
        addr.to_string()
    }
}

