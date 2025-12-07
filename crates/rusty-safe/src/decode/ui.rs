//! Calldata decode UI rendering

use eframe::egui;

use super::types::*;

/// Render the full decode section
pub fn render_decode_section(
    ui: &mut egui::Ui,
    decode: &mut DecodedTransaction,
) {
    ui.add_space(10.0);

    match &mut decode.kind {
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
            render_multisend_section(ui, multi);
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
                let value_text = egui::RichText::new(&ap.value).monospace().small();
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
                let value_text = egui::RichText::new(&lp.value).monospace().small();
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
    multi: &mut MultiSendDecode,
) {
    // Header with summary and expand/collapse buttons
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!(
            "📦 MultiSend ({} transactions)",
            multi.transactions.len()
        )).strong());

        // Show verification state or summary badges
        match &multi.verification_state {
            VerificationState::Pending => {
                ui.label(egui::RichText::new("⏳ Waiting...").weak());
            }
            VerificationState::InProgress { total } => {
                ui.spinner();
                ui.label(format!("Verifying {} transactions...", total));
            }
            VerificationState::Complete => {
                render_summary_badges(ui, &multi.summary);
            }
        }
        
        ui.add_space(20.0);
        
        // Expand All button
        if ui.small_button("⬇ Expand All").clicked() {
            for tx in &mut multi.transactions {
                tx.is_expanded = true;
            }
        }
        
        // Collapse All button
        if ui.small_button("⬆ Collapse All").clicked() {
            for tx in &mut multi.transactions {
                tx.is_expanded = false;
            }
        }
    });

    ui.add_space(8.0);

    // Collapsible transactions
    for tx in &mut multi.transactions {
        render_multisend_tx(ui, tx);
    }
}

/// Verification status for coloring
enum VerifyStatus {
    Match,
    Mismatch,
    Pending,
}

/// Build a compact header with color based on verification status
fn build_tx_header(tx: &MultiSendTx) -> egui::RichText {
    let (status_emoji, status) = match &tx.decode {
        Some(d) if d.comparison.is_match() => ("✓", VerifyStatus::Match),
        Some(d) if d.comparison.is_mismatch() => ("✗", VerifyStatus::Mismatch),
        Some(_) => ("◇", VerifyStatus::Pending),
        None => ("□", VerifyStatus::Pending),
    };

    // Try to get method name and params - prefer api_decode (always available), 
    // fall back to decode.api if available
    let api_data = tx.api_decode.as_ref()
        .or_else(|| tx.decode.as_ref().and_then(|d| d.api.as_ref()));
    
    let method_part = api_data
        .map(|api| {
            // Build compact params: method(val1, val2, ...)
            let params_str = api.params
                .iter()
                .take(3) // Limit to first 3 params for compactness
                .map(|p| truncate_param(&p.value, 12))
                .collect::<Vec<_>>()
                .join(", ");
            
            if api.params.len() > 3 {
                format!("{}({}, ...)", api.method, params_str)
            } else if params_str.is_empty() {
                api.method.clone()
            } else {
                format!("{}({})", api.method, params_str)
            }
        })
        .unwrap_or_else(|| truncate_address(&tx.to));

    let value_part = if tx.value == "0" {
        "0 ETH".to_string()
    } else {
        format_wei(&tx.value)
    };

    let header_text = format!("#{} {} ({}) {}", tx.index + 1, method_part, value_part, status_emoji);
    
    // Color based on verification status
    let color = match status {
        VerifyStatus::Match => egui::Color32::from_rgb(100, 200, 100),    // Green
        VerifyStatus::Mismatch => egui::Color32::from_rgb(220, 80, 80),   // Red
        VerifyStatus::Pending => egui::Color32::GRAY,                      // Gray for pending
    };
    
    egui::RichText::new(header_text).color(color)
}

/// Truncate a parameter value for display in header
fn truncate_param(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        return value.to_string();
    }
    // For addresses/hashes, show prefix...suffix
    if value.starts_with("0x") && value.len() > 10 {
        format!("{}...{}", &value[..6], &value[value.len()-4..])
    } else {
        format!("{}...", &value[..max_len])
    }
}

/// Truncate address for display
fn truncate_address(addr: &str) -> String {
    if addr.len() > 12 {
        format!("{}...{}", &addr[..6], &addr[addr.len()-4..])
    } else {
        addr.to_string()
    }
}

/// Format wei value nicely
fn format_wei(wei: &str) -> String {
    // Try to parse and format with units
    if let Ok(val) = wei.parse::<u128>() {
        if val == 0 {
            return "0 ETH".to_string();
        }
        let eth = val as f64 / 1e18;
        if eth >= 0.001 {
            return format!("{:.4} ETH", eth);
        }
    }
    format!("{} wei", wei)
}

/// Render a single MultiSend transaction (collapsible)
fn render_multisend_tx(
    ui: &mut egui::Ui,
    tx: &mut MultiSendTx,
) {
    let header = build_tx_header(tx);

    // Use .open() for external state control (collapse all / expand all)
    let response = egui::CollapsingHeader::new(header)
        .id_salt(format!("multisend_tx_{}", tx.index))
        .open(Some(tx.is_expanded))
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

            // Decode comparison (results already available from bulk verification)
            if let Some(decode) = &tx.decode {
                render_single_comparison(ui, decode);
            } else if tx.data == "0x" || tx.data.is_empty() {
                ui.label(egui::RichText::new("No calldata").weak());
            } else {
                ui.label(egui::RichText::new("Verification unavailable").weak());
            }
        });

    // Track expand state (purely visual now)
    if response.header_response.clicked() {
        tx.is_expanded = !tx.is_expanded;
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
                egui::RichText::new("⚠️ Decoded independently (API didn't provide decode to verify against)")
                    .color(egui::Color32::from_rgb(220, 180, 50)),
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
    // Use a scrollable area for long data, show full value
    if data.len() > 100 {
        egui::ScrollArea::horizontal()
            .max_width(400.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new(data).monospace().small());
            });
    } else {
        ui.label(egui::RichText::new(data).monospace().small());
    }
}


