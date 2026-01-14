//! Calldata decode UI rendering

use super::types::*;
use crate::ui::{self, validate_address, AddressValidation};
use eframe::egui;
use safe_utils::Of;

/// Check if a value looks like a tuple/array (starts with [ and ends with ])
fn is_tuple_or_array(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() > 2
}

/// Parse tuple/array elements, handling nested structures
fn parse_tuple_elements(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    // Remove outer brackets
    let inner = &trimmed[1..trimmed.len() - 1];

    if inner.is_empty() {
        return vec![];
    }

    let mut elements = Vec::new();
    let mut current = String::new();
    let mut bracket_depth = 0;

    for ch in inner.chars() {
        match ch {
            '[' | '(' => {
                bracket_depth += 1;
                current.push(ch);
            }
            ']' | ')' => {
                bracket_depth -= 1;
                current.push(ch);
            }
            ',' if bracket_depth == 0 => {
                elements.push(current.trim().to_string());
                current = String::new();
            }
            _ => {
                current.push(ch);
            }
        }
    }

    // Don't forget the last element
    if !current.trim().is_empty() {
        elements.push(current.trim().to_string());
    }

    elements
}

/// Render a single value element with smart type detection
fn render_single_value(
    ui_ctx: &mut egui::Ui,
    value: &str,
    safe_ctx: &crate::state::SafeContext,
    color: Option<egui::Color32>,
    id_salt: &str,
) {
    let validation = validate_address(value);

    if validation != AddressValidation::Invalid {
        ui_ctx.horizontal(|ui| {
            let chain_name = &safe_ctx.chain_name;
            let explorer_url = ui::get_explorer_address_url(chain_name, value);

            let text_color = if let Some(c) = color {
                c
            } else if validation == AddressValidation::ChecksumMismatch {
                egui::Color32::from_rgb(220, 180, 50) // Yellow for checksum warning
            } else {
                ui.visuals().hyperlink_color
            };

            // Look up name in address book
            let chain_id = alloy::primitives::ChainId::of(chain_name).unwrap_or(1);
            let name = safe_ctx.address_book.get_name(value, chain_id);
            let label_text = if let Some(n) = name {
                format!("{} ({})", value, n)
            } else {
                value.to_string()
            };

            let response = ui
                .link(
                    egui::RichText::new(label_text)
                        .monospace()
                        .color(text_color),
                )
                .on_hover_text(if validation == AddressValidation::ChecksumMismatch {
                    "⚠️ Checksum mismatch! Click to open in block explorer"
                } else {
                    "Open in block explorer"
                });

            if response.clicked() {
                ui::open_url_new_tab(&explorer_url);
            }

            if validation == AddressValidation::ChecksumMismatch {
                ui.label(egui::RichText::new("⚠️").color(egui::Color32::from_rgb(220, 180, 50)))
                    .on_hover_text("Address has an invalid EIP-55 checksum");
            }
        });
    } else if ui::is_large_uint(value) {
        ui::render_uint_with_popup(ui_ctx, value, id_salt);
    } else {
        let text = egui::RichText::new(value).monospace();
        let text = if let Some(c) = color {
            text.color(c)
        } else {
            text
        };
        ui_ctx.label(text);
    }
}

/// Render tuple/array elements with individual type handling
fn render_tuple_value(
    ui_ctx: &mut egui::Ui,
    value: &str,
    safe_ctx: &crate::state::SafeContext,
    color: Option<egui::Color32>,
    id_salt: &str,
) {
    let elements = parse_tuple_elements(value);

    ui_ctx.vertical(|ui| {
        for (i, elem) in elements.iter().enumerate() {
            // Use horizontal_wrapped to allow long values to wrap
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(format!("[{}]:", i)).weak().small());
                let elem_id = format!("{}_{}", id_salt, i);
                // Recursively handle nested tuples
                render_param_value(ui, elem, safe_ctx, color, &elem_id);
            });
        }
    });
}

/// Render a parameter value - handles addresses, large uints, tuples/arrays
fn render_param_value(
    ui_ctx: &mut egui::Ui,
    value: &str,
    safe_ctx: &crate::state::SafeContext,
    color: Option<egui::Color32>,
    id_salt: &str,
) {
    if is_tuple_or_array(value) {
        // Tuple/array - render each element separately
        render_tuple_value(ui_ctx, value, safe_ctx, color, id_salt);
    } else {
        // Single value
        render_single_value(ui_ctx, value, safe_ctx, color, id_salt);
    }
}

/// Render the full decode section
pub fn render_decode_section(
    ui: &mut egui::Ui,
    decode: &mut DecodedTransaction,
    safe_ctx: &crate::state::SafeContext,
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
            render_single_section(ui, single, &decode.selector, safe_ctx);
        }
        TransactionKind::MultiSend(multi) => {
            render_multisend_section(ui, multi, safe_ctx);
        }
        TransactionKind::Unknown => {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("📦 Calldata").strong());
                ui.label(
                    egui::RichText::new(format!("Unknown selector: {}", decode.selector)).weak(),
                );
            });
            ui.add_space(5.0);
            render_raw_data(ui, &decode.raw_data);
        }
    }
}

/// Render single function call decode
fn render_single_section(
    ui: &mut egui::Ui,
    decode: &SingleDecode,
    selector: &str,
    safe_ctx: &crate::state::SafeContext,
) {
    // Wrap in a card for visual grouping
    egui::Frame::none()
        .fill(ui.visuals().faint_bg_color)
        .rounding(6.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            // Header with prominent status
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("📦 Calldata Decoding").strong());
                ui.label(
                    egui::RichText::new(format!("[{}]", selector))
                        .monospace()
                        .small(),
                );
                render_status_badge(ui, &decode.comparison);
            });

            ui.add_space(10.0);

            render_single_comparison_with_chain(ui, decode, safe_ctx);
        });
}

/// Render side-by-side comparison for a single decode (no chain awareness - for backwards compat)
pub fn render_single_comparison(ui: &mut egui::Ui, decode: &SingleDecode) {
    let safe_ctx = crate::state::SafeContext::default();
    render_single_comparison_with_chain(ui, decode, &safe_ctx);
}

/// Render side-by-side comparison for a single decode with chain-aware address links
fn render_single_comparison_with_chain(
    ui: &mut egui::Ui,
    decode: &SingleDecode,
    safe_ctx: &crate::state::SafeContext,
) {
    let id_prefix = format!("decode_{:p}", decode);

    egui::Grid::new(format!("decode_compare_{:p}", decode))
        .num_columns(2)
        .spacing([30.0, 4.0])
        .min_col_width(200.0)
        .show(ui, |ui| {
            // Headers
            ui.label(egui::RichText::new("Safe API").strong().underline());
            ui.label(
                egui::RichText::new("Independent (4byte)")
                    .strong()
                    .underline(),
            );
            ui.end_row();

            // Method row
            render_method_row(ui, decode);

            // Separator
            ui.separator();
            ui.separator();
            ui.end_row();

            // Parameter rows
            render_params_rows(ui, decode, safe_ctx, &id_prefix);
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

    // Local method - show [unverified] in red if not from verified contract
    if let Some(local) = &decode.local {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(&local.method).monospace().strong());
            if !local.verified {
                ui.label(
                    egui::RichText::new("[unverified]")
                        .color(egui::Color32::from_rgb(220, 80, 80))
                        .small(),
                );
            }
        });
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
fn render_params_rows(
    ui: &mut egui::Ui,
    decode: &SingleDecode,
    safe_ctx: &crate::state::SafeContext,
    id_prefix: &str,
) {
    static EMPTY_API: Vec<ApiParam> = Vec::new();
    static EMPTY_LOCAL: Vec<LocalParam> = Vec::new();

    let api_params = decode
        .api
        .as_ref()
        .map(|a| a.params.as_slice())
        .unwrap_or(&EMPTY_API);
    let local_params = decode
        .local
        .as_ref()
        .map(|l| l.params.as_slice())
        .unwrap_or(&EMPTY_LOCAL);

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
                ui.label(egui::RichText::new(label).small());
                let color = if has_mismatch {
                    Some(egui::Color32::from_rgb(220, 80, 80))
                } else {
                    None
                };
                let id_salt = format!("{}_api_{}", id_prefix, i);
                render_param_value(ui, &ap.value, safe_ctx, color, &id_salt);
            });
        } else {
            ui.label(egui::RichText::new("—").weak());
        }

        // Local param
        if let Some(lp) = local_param {
            let label = format!("param{} ({}):", i, lp.typ);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(label).small());
                let color = if has_mismatch {
                    Some(egui::Color32::from_rgb(100, 200, 100))
                } else {
                    None
                };
                let id_salt = format!("{}_local_{}", id_prefix, i);
                render_param_value(ui, &lp.value, safe_ctx, color, &id_salt);
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
    safe_ctx: &crate::state::SafeContext,
) {
    // Header with summary and expand/collapse buttons
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "📦 MultiSend ({} transactions)",
                multi.transactions.len()
            ))
            .strong(),
        );

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
        render_multisend_tx(ui, tx, safe_ctx);
    }
}

/// Verification status for coloring
enum VerifyStatus {
    Match,        // Green  - independently verified
    Mismatch,     // Red    - verification failed
    Unverifiable, // Yellow - couldn't verify (OnlyApi, OnlyLocal, Failed)
    Pending,      // Gray   - still loading
}

/// Build a compact header with color based on verification status
fn build_tx_header(tx: &MultiSendTx) -> egui::RichText {
    let (status_emoji, status) = match &tx.decode {
        Some(d) if d.comparison.is_match() => ("✓", VerifyStatus::Match),
        Some(d) if d.comparison.is_mismatch() => ("✗", VerifyStatus::Mismatch),
        Some(d) => {
            // Distinguish unverifiable from still-loading
            match &d.comparison {
                ComparisonResult::OnlyApi
                | ComparisonResult::OnlyLocal
                | ComparisonResult::Failed(_) => ("⚠", VerifyStatus::Unverifiable),
                _ => ("◇", VerifyStatus::Pending),
            }
        }
        None => ("□", VerifyStatus::Pending),
    };

    // Try to get method name and params - prefer api_decode, then decode.api, then decode.local
    let api_data = tx
        .api_decode
        .as_ref()
        .or_else(|| tx.decode.as_ref().and_then(|d| d.api.as_ref()));

    let method_part = if let Some(api) = api_data {
        // Build compact params from API decode: method(val1, val2, ...)
        let params_str = api
            .params
            .iter()
            .take(3)
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
    } else if let Some(local) = tx.decode.as_ref().and_then(|d| d.local.as_ref()) {
        // Fall back to local 4byte decode
        let params_str = local
            .params
            .iter()
            .take(3)
            .map(|p| truncate_param(&p.value, 12))
            .collect::<Vec<_>>()
            .join(", ");

        let unverified_tag = if !local.verified { " [unverified]" } else { "" };

        if local.params.len() > 3 {
            format!("{}({}, ...){}", local.method, params_str, unverified_tag)
        } else if params_str.is_empty() {
            format!("{}{}", local.method, unverified_tag)
        } else {
            format!("{}({}){}", local.method, params_str, unverified_tag)
        }
    } else {
        truncate_address(&tx.to)
    };

    let value_part = if tx.value == "0" {
        "0 ETH".to_string()
    } else {
        format_wei(&tx.value)
    };

    let header_text = format!(
        "#{} {} ({}) {}",
        tx.index + 1,
        method_part,
        value_part,
        status_emoji
    );

    // Color based on verification status
    let color = match status {
        VerifyStatus::Match => egui::Color32::from_rgb(100, 200, 100), // Green
        VerifyStatus::Mismatch => egui::Color32::from_rgb(220, 80, 80), // Red
        VerifyStatus::Unverifiable => egui::Color32::from_rgb(220, 180, 50), // Yellow
        VerifyStatus::Pending => egui::Color32::GRAY,                  // Gray
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
        format!("{}...{}", &value[..6], &value[value.len() - 4..])
    } else {
        format!("{}...", &value[..max_len])
    }
}

/// Truncate address for display
fn truncate_address(addr: &str) -> String {
    if addr.len() > 12 {
        format!("{}...{}", &addr[..6], &addr[addr.len() - 4..])
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
    safe_ctx: &crate::state::SafeContext,
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
                    let chain_id =
                        alloy::primitives::ChainId::of(&safe_ctx.chain_name).unwrap_or(1);
                    let name = safe_ctx.address_book.get_name(&tx.to, chain_id);
                    ui::address_link(ui, &safe_ctx.chain_name, &tx.to, name);
                    ui.end_row();

                    ui.label("Value:");
                    ui.label(format!("{} wei", tx.value));
                    ui.end_row();

                    ui.label("Operation:");
                    ui.label(if tx.operation == 0 {
                        "Call"
                    } else {
                        "DelegateCall"
                    });
                    ui.end_row();
                });

            ui.add_space(8.0);

            // Decode comparison (results already available from bulk verification)
            if let Some(decode) = &tx.decode {
                render_single_comparison_with_chain(ui, decode, safe_ctx);
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
            ui::success_banner(ui, "Decodings match - independently verified");
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
                    .weak(),
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
                egui::RichText::new(
                    "⚠️ Decoded independently (API didn't provide decode to verify against)",
                )
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
                ui.label(egui::RichText::new(data).monospace());
            });
    } else {
        ui.label(egui::RichText::new(data).monospace());
    }
}

// =============================================================================
// OFFLINE MODE UI RENDERING
// =============================================================================

/// Render offline decode section (4byte lookup only, no API comparison)
pub fn render_offline_decode_section(
    ui: &mut egui::Ui,
    result: &mut OfflineDecodeResult,
    safe_ctx: &crate::state::SafeContext,
) {
    ui.add_space(10.0);

    match result {
        OfflineDecodeResult::Empty => {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("📦 Calldata").strong());
                ui.label(egui::RichText::new("(empty - native ETH transfer)").weak());
                ui.label(egui::RichText::new("✅").color(egui::Color32::from_rgb(100, 200, 100)));
            });
        }
        OfflineDecodeResult::Single { local, status } => {
            render_offline_single_section(ui, local, status, safe_ctx);
        }
        OfflineDecodeResult::MultiSend(txs) => {
            render_offline_multisend_section(ui, txs, safe_ctx);
        }
        OfflineDecodeResult::RawHex(data) => {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("📦 Calldata").strong());
                ui.label(egui::RichText::new("(could not parse)").weak());
            });
            ui.add_space(5.0);
            render_raw_data(ui, data);
        }
    }
}

/// Render single function call for offline mode
fn render_offline_single_section(
    ui: &mut egui::Ui,
    local: &LocalDecode,
    status: &OfflineDecodeStatus,
    safe_ctx: &crate::state::SafeContext,
) {
    // Header with status
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("📦 Calldata Decoding").strong());
        render_offline_status_badge(ui, status);
    });

    ui.add_space(8.0);

    // Show decode result
    match status {
        OfflineDecodeStatus::Decoded => {
            render_offline_decode(ui, local, safe_ctx, "offline_single");
        }
        OfflineDecodeStatus::Unknown(selector) => {
            ui.label(
                egui::RichText::new(format!("❌ Unknown function {}", selector))
                    .color(egui::Color32::from_rgb(220, 80, 80)),
            );
        }
        OfflineDecodeStatus::Failed(err) => {
            ui.label(
                egui::RichText::new(format!("❌ Decode failed: {}", err))
                    .color(egui::Color32::from_rgb(220, 80, 80)),
            );
        }
    }
}

/// Render offline local decode (method + params)
fn render_offline_decode(
    ui: &mut egui::Ui,
    local: &LocalDecode,
    safe_ctx: &crate::state::SafeContext,
    id_prefix: &str,
) {
    // Method name with verification status
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(&local.method).monospace().strong());
        if !local.verified {
            ui.label(
                egui::RichText::new("[unverified]")
                    .color(egui::Color32::from_rgb(220, 80, 80))
                    .small(),
            );
        }
    });

    ui.add_space(4.0);

    // Parameters
    if local.params.is_empty() {
        ui.label(egui::RichText::new("(no parameters)").weak());
    } else {
        egui::Grid::new(format!("offline_params_{}", id_prefix))
            .num_columns(2)
            .spacing([10.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                for (i, param) in local.params.iter().enumerate() {
                    ui.label(egui::RichText::new(format!("param{} ({}):", i, param.typ)).small());
                    let id_salt = format!("{}_{}", id_prefix, i);
                    render_param_value(ui, &param.value, safe_ctx, None, &id_salt);
                    ui.end_row();
                }
            });
    }
}

/// Render offline status badge
fn render_offline_status_badge(ui: &mut egui::Ui, status: &OfflineDecodeStatus) {
    match status {
        OfflineDecodeStatus::Decoded => {
            ui.label(egui::RichText::new("✅").color(egui::Color32::from_rgb(100, 200, 100)));
        }
        OfflineDecodeStatus::Unknown(_) | OfflineDecodeStatus::Failed(_) => {
            ui.label(egui::RichText::new("❌").color(egui::Color32::from_rgb(220, 80, 80)));
        }
    }
}

/// Render MultiSend section for offline mode
fn render_offline_multisend_section(
    ui: &mut egui::Ui,
    txs: &mut [OfflineMultiSendTx],
    safe_ctx: &crate::state::SafeContext,
) {
    // Header with count and expand/collapse buttons
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("📦 MultiSend ({} transactions)", txs.len())).strong(),
        );

        // Summary badges
        let decoded = txs.iter().filter(|t| t.status.is_decoded()).count();
        let errors = txs.iter().filter(|t| t.status.is_error()).count();

        if decoded > 0 {
            ui.label(
                egui::RichText::new(format!("✅ {}", decoded))
                    .color(egui::Color32::from_rgb(100, 200, 100)),
            );
        }
        if errors > 0 {
            ui.label(
                egui::RichText::new(format!("❌ {}", errors))
                    .color(egui::Color32::from_rgb(220, 80, 80)),
            );
        }

        ui.add_space(20.0);

        // Expand All button
        if ui.small_button("⬇ Expand All").clicked() {
            for tx in txs.iter_mut() {
                tx.is_expanded = true;
            }
        }

        // Collapse All button
        if ui.small_button("⬆ Collapse All").clicked() {
            for tx in txs.iter_mut() {
                tx.is_expanded = false;
            }
        }
    });

    ui.add_space(8.0);

    // Render each transaction
    for tx in txs.iter_mut() {
        render_offline_multisend_tx(ui, tx, safe_ctx);
    }
}

/// Build header for offline MultiSend transaction
fn build_offline_tx_header(tx: &OfflineMultiSendTx) -> egui::RichText {
    let (status_emoji, color) = match &tx.status {
        OfflineDecodeStatus::Decoded => ("✓", egui::Color32::from_rgb(100, 200, 100)),
        OfflineDecodeStatus::Unknown(_) | OfflineDecodeStatus::Failed(_) => {
            ("✗", egui::Color32::from_rgb(220, 80, 80))
        }
    };

    // Method part
    let method_part = tx
        .local_decode
        .as_ref()
        .map(|d| {
            let params_str = d
                .params
                .iter()
                .take(3)
                .map(|p| truncate_param(&p.value, 12))
                .collect::<Vec<_>>()
                .join(", ");

            let unverified_tag = if !d.verified { " [unverified]" } else { "" };

            if d.params.len() > 3 {
                format!("{}({}, ...){}", d.method, params_str, unverified_tag)
            } else if params_str.is_empty() {
                format!("{}{}", d.method, unverified_tag)
            } else {
                format!("{}({}){}", d.method, params_str, unverified_tag)
            }
        })
        .unwrap_or_else(|| {
            if tx.data.len() >= 10 {
                format!("Unknown {}", &tx.data[..10])
            } else if tx.data == "0x" || tx.data.is_empty() {
                "ETH transfer".to_string()
            } else {
                truncate_address(&tx.to)
            }
        });

    let value_part = if tx.value == "0" {
        "0 ETH".to_string()
    } else {
        format_wei(&tx.value)
    };

    let header_text = format!(
        "#{} {} ({}) {}",
        tx.index + 1,
        method_part,
        value_part,
        status_emoji
    );

    egui::RichText::new(header_text).color(color)
}

/// Render a single offline MultiSend transaction (collapsible)
fn render_offline_multisend_tx(
    ui: &mut egui::Ui,
    tx: &mut OfflineMultiSendTx,
    safe_ctx: &crate::state::SafeContext,
) {
    let header = build_offline_tx_header(tx);

    let response = egui::CollapsingHeader::new(header)
        .id_salt(format!("offline_multisend_tx_{}", tx.index))
        .open(Some(tx.is_expanded))
        .show(ui, |ui| {
            ui.add_space(4.0);

            // Transaction details
            egui::Grid::new(format!("offline_multisend_details_{}", tx.index))
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("To:");
                    let chain_id =
                        alloy::primitives::ChainId::of(&safe_ctx.chain_name).unwrap_or(1);
                    let name = safe_ctx.address_book.get_name(&tx.to, chain_id);
                    ui::address_link(ui, &safe_ctx.chain_name, &tx.to, name);
                    ui.end_row();

                    ui.label("Value:");
                    ui.label(format!("{} wei", tx.value));
                    ui.end_row();

                    ui.label("Operation:");
                    ui.label(if tx.operation == 0 {
                        "Call"
                    } else {
                        "DelegateCall"
                    });
                    ui.end_row();
                });

            ui.add_space(8.0);

            // Decode result
            match &tx.status {
                OfflineDecodeStatus::Decoded => {
                    if let Some(local) = &tx.local_decode {
                        let id_prefix = format!("offline_multi_{}", tx.index);
                        render_offline_decode(ui, local, safe_ctx, &id_prefix);
                    } else if tx.data == "0x" || tx.data.is_empty() {
                        ui.label(egui::RichText::new("No calldata (ETH transfer)").weak());
                    }
                }
                OfflineDecodeStatus::Unknown(selector) => {
                    ui.label(
                        egui::RichText::new(format!("❌ Unknown function {}", selector))
                            .color(egui::Color32::from_rgb(220, 80, 80)),
                    );
                    if !tx.data.is_empty() && tx.data != "0x" {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Raw calldata:").weak());
                        render_raw_data(ui, &tx.data);
                    }
                }
                OfflineDecodeStatus::Failed(err) => {
                    ui.label(
                        egui::RichText::new(format!("❌ Decode failed: {}", err))
                            .color(egui::Color32::from_rgb(220, 80, 80)),
                    );
                }
            }
        });

    if response.header_response.clicked() {
        tx.is_expanded = !tx.is_expanded;
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_address() {
        // Valid checksummed address
        let valid = "0x52906951E511101BA707440006734B19E59F6C87";
        assert_eq!(validate_address(valid), AddressValidation::Valid);

        // Valid lowercase address
        let lowercase = "0x52906951e511101ba707440006734b19e59f6c87";
        assert_eq!(validate_address(lowercase), AddressValidation::Valid);

        // Valid uppercase address
        let uppercase = format!(
            "0x{}",
            "52906951E511101BA707440006734B19E59F6C87".to_uppercase()
        );
        assert_eq!(validate_address(&uppercase), AddressValidation::Valid);

        // Checksum mismatch
        let mismatch = "0x52906951e511101ba707440006734b19e59f6c87";
        let mut mismatch_chars: Vec<char> = mismatch.chars().collect();
        mismatch_chars[2] = 'A'; // Change one char to uppercase incorrectly
        let mismatch_str: String = mismatch_chars.into_iter().collect();
        assert_eq!(
            validate_address(&mismatch_str),
            AddressValidation::ChecksumMismatch
        );

        // Invalid address (too short)
        assert_eq!(validate_address("0x123"), AddressValidation::Invalid);

        // Invalid address (non-hex)
        assert_eq!(
            validate_address("0xGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG"),
            AddressValidation::Invalid
        );
    }
}
