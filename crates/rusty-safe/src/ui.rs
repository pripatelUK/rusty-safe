//! UI helper components

use eframe::egui;

/// Get block explorer URL for an address on a given chain
/// Supports all chains from safe-utils: arbitrum, aurora, avalanche, base, blast, bsc,
/// celo, ethereum, gnosis, linea, mantle, monad, optimism, polygon, scroll, sepolia,
/// worldchain, xlayer, zksync, base-sepolia, gnosis-chiado, polygon-zkevm
pub fn get_explorer_address_url(chain_name: &str, address: &str) -> String {
    let base = match chain_name.to_lowercase().as_str() {
        // Mainnets
        "ethereum" | "mainnet" => "https://etherscan.io",
        "arbitrum" => "https://arbiscan.io",
        "aurora" => "https://explorer.aurora.dev",
        "avalanche" => "https://snowtrace.io",
        "base" => "https://basescan.org",
        "blast" => "https://blastscan.io",
        "bsc" | "binance" => "https://bscscan.com",
        "celo" => "https://celoscan.io",
        "gnosis" => "https://gnosisscan.io",
        "linea" => "https://lineascan.build",
        "mantle" => "https://mantlescan.xyz",
        "monad" => "https://monadvision.com",
        "optimism" => "https://optimistic.etherscan.io",
        "polygon" => "https://polygonscan.com",
        "scroll" => "https://scrollscan.com",
        "worldchain" => "https://worldscan.org",
        "xlayer" => "https://www.okx.com/web3/explorer/xlayer",
        "zksync" => "https://explorer.zksync.io",
        "polygon-zkevm" => "https://zkevm.polygonscan.com",
        // Testnets
        "sepolia" => "https://sepolia.etherscan.io",
        "base-sepolia" => "https://sepolia.basescan.org",
        "gnosis-chiado" => "https://gnosis-chiado.blockscout.com",
        // Fallback
        _ => "https://etherscan.io",
    };
    format!("{}/address/{}", base, address)
}

/// Open URL in a new browser tab
#[cfg(target_arch = "wasm32")]
pub fn open_url_new_tab(url: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.open_with_url_and_target(url, "_blank");
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn open_url_new_tab(url: &str) {
    let _ = open::that(url);
}

pub use crate::state::{validate_address, AddressValidation};

/// Render an address as a clickable hyperlink that opens in block explorer
pub fn address_link(
    ui: &mut egui::Ui,
    chain_name: &str,
    address: &str,
    name: Option<String>,
) -> egui::Response {
    let validation = validate_address(address);
    let explorer_url = get_explorer_address_url(chain_name, address);

    let text_color = if validation == AddressValidation::ChecksumMismatch {
        egui::Color32::from_rgb(220, 180, 50) // Yellow for checksum warning
    } else {
        ui.visuals().hyperlink_color
    };

    ui.horizontal(|ui| {
        let label_text = if let Some(n) = name {
            format!("{} ({})", address, n)
        } else {
            address.to_string()
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
            open_url_new_tab(&explorer_url);
        }

        if validation == AddressValidation::ChecksumMismatch {
            ui.label(egui::RichText::new("⚠️").color(egui::Color32::from_rgb(220, 180, 50)))
                .on_hover_text("Address has an invalid EIP-55 checksum");
        }

        response
    })
    .inner
}

/// Styled heading with accent color
pub fn styled_heading(ui: &mut egui::Ui, text: &str) {
    ui.heading(egui::RichText::new(text).color(egui::Color32::from_rgb(0, 212, 170)));
}

/// Section header with separator
pub fn section_header(ui: &mut egui::Ui, text: &str) {
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(text).strong().size(14.0));
    });
    ui.separator();
}

/// Labeled field with copy button
pub fn labeled_field_with_copy(ui: &mut egui::Ui, label: &str, value: &str) -> bool {
    let mut copied = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("{}:", label)).strong());
        ui.label(egui::RichText::new(value).monospace());
        if ui
            .small_button("📋")
            .on_hover_text("Copy to clipboard")
            .clicked()
        {
            copied = true;
        }
    });
    copied
}

/// Copy to clipboard (platform-specific)
#[cfg(not(target_arch = "wasm32"))]
pub fn copy_to_clipboard(text: &str) {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(text);
    }
}

#[cfg(target_arch = "wasm32")]
pub fn copy_to_clipboard(text: &str) {
    if let Some(window) = web_sys::window() {
        let navigator = window.navigator();
        let clipboard = navigator.clipboard();
        let _ = clipboard.write_text(text);
    }
}

/// Create a styled text edit for address input
pub fn address_input(ui: &mut egui::Ui, value: &mut String) -> egui::Response {
    ui.add(
        egui::TextEdit::singleline(value)
            .hint_text("0x...")
            .desired_width(400.0)
            .font(egui::TextStyle::Monospace),
    )
}

/// Create a styled text edit for number input
pub fn number_input(ui: &mut egui::Ui, value: &mut String, hint: &str) -> egui::Response {
    ui.add(
        egui::TextEdit::singleline(value)
            .hint_text(hint)
            .desired_width(150.0)
            .font(egui::TextStyle::Monospace),
    )
}

/// Create a styled multiline text edit with fixed height and internal scrolling
pub fn multiline_input(
    ui: &mut egui::Ui,
    value: &mut String,
    hint: &str,
    rows: usize,
) -> egui::Response {
    // Calculate height based on row count (approximate line height)
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
    let height = row_height * rows as f32 + ui.spacing().item_spacing.y * 5.0;

    let mut response = None;
    egui::ScrollArea::vertical()
        .max_height(height)
        .show(ui, |ui| {
            response = Some(
                ui.add(
                    egui::TextEdit::multiline(value)
                        .hint_text(hint)
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace),
                ),
            );
        });
    response.unwrap()
}

/// Loading spinner
pub fn loading_spinner(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spinner();
        ui.label("Loading...");
    });
}

/// Error message display
pub fn error_message(ui: &mut egui::Ui, message: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("❌").size(16.0));
        ui.label(egui::RichText::new(message).color(egui::Color32::from_rgb(220, 80, 80)));
    });
}

/// Success message display
pub fn success_message(ui: &mut egui::Ui, message: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("✅").size(16.0));
        ui.label(egui::RichText::new(message).color(egui::Color32::from_rgb(80, 200, 120)));
    });
}

/// Warning message display
pub fn warning_message(ui: &mut egui::Ui, message: &str, color: egui::Color32) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("⚠️").size(14.0));
        ui.label(egui::RichText::new(message).color(color));
    });
}

/// Display a hash value with copy button
pub fn copyable_hash(ui: &mut egui::Ui, hash: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(hash).monospace());
        if ui
            .small_button("📋")
            .on_hover_text("Copy to clipboard")
            .clicked()
        {
            copy_to_clipboard(hash);
        }
    });
}

// =============================================================================
// STYLED BUTTONS
// =============================================================================

/// Primary action button - teal/accent colored, prominent
pub fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let accent = egui::Color32::from_rgb(0, 180, 150);
    let btn = egui::Button::new(egui::RichText::new(text).size(14.0).color(egui::Color32::WHITE))
        .min_size(egui::vec2(130.0, 34.0))
        .fill(accent);
    ui.add(btn)
}

/// Primary button with enabled state
pub fn primary_button_enabled(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
    let accent = egui::Color32::from_rgb(0, 180, 150);
    let btn = egui::Button::new(egui::RichText::new(text).size(14.0).color(egui::Color32::WHITE))
        .min_size(egui::vec2(130.0, 34.0))
        .fill(accent);
    ui.add_enabled(enabled, btn)
}

/// Secondary action button - subdued, outline style
pub fn secondary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let btn = egui::Button::new(egui::RichText::new(text).size(14.0))
        .min_size(egui::vec2(90.0, 34.0));
    ui.add(btn)
}

// =============================================================================
// VISUAL GROUPING
// =============================================================================

/// Render content in a subtle card/frame
pub fn card(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(ui.visuals().faint_bg_color)
        .rounding(6.0)
        .inner_margin(12.0)
        .show(ui, add_contents);
}

/// Render content in a highlighted card (slightly brighter)
pub fn card_highlighted(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    let bg = ui.visuals().faint_bg_color.linear_multiply(1.3);
    egui::Frame::none()
        .fill(bg)
        .rounding(6.0)
        .inner_margin(12.0)
        .show(ui, add_contents);
}

// =============================================================================
// LEDGER BINARY FORMAT
// =============================================================================

/// Convert a hex hash to binary literal format for Ledger display
/// Matches the shell script behavior: printable ASCII (32-126) shown as-is,
/// non-printable bytes escaped as \xNN
/// e.g., "0xad06b099..." -> "\xad\x06\xb0\x99..."
pub fn hash_to_binary_literal(hash: &str) -> String {
    let hex = hash.strip_prefix("0x").unwrap_or(hash);
    let mut result = String::new();

    // Convert hex pairs to bytes
    let bytes: Vec<u8> = hex
        .as_bytes()
        .chunks(2)
        .filter_map(|chunk| {
            if chunk.len() == 2 {
                let s = std::str::from_utf8(chunk).ok()?;
                u8::from_str_radix(s, 16).ok()
            } else {
                None
            }
        })
        .collect();

    // Format each byte: printable ASCII as-is, others as \xNN
    for byte in bytes {
        if (32..=126).contains(&byte) {
            // Printable ASCII - show as character
            result.push(byte as char);
        } else {
            // Non-printable - escape as \xNN
            result.push_str(&format!("\\x{:02x}", byte));
        }
    }

    result
}

// =============================================================================
// UINT DECIMAL POPUP
// =============================================================================

/// State for uint decimal popup
#[derive(Default, Clone)]
struct UintPopupState {
    decimals: u8,
}

/// Check if a value looks like a large uint (numeric, no decimals, > 1e6)
pub fn is_large_uint(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Check if it's purely numeric
    if !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    // Must be larger than 1,000,000 to be interesting
    trimmed.len() > 6
}

/// Format a uint value with the given number of decimals
pub fn format_uint_with_decimals(value: &str, decimals: u8) -> String {
    if decimals == 0 {
        return add_thousand_separators(value);
    }

    let trimmed = value.trim();
    let len = trimmed.len();
    let dec = decimals as usize;

    if len <= dec {
        // Value is smaller than the decimal places
        let zeros = "0".repeat(dec - len);
        format!("0.{}{}", zeros, trimmed.trim_start_matches('0'))
    } else {
        // Split into integer and decimal parts
        let int_part = &trimmed[..len - dec];
        let dec_part = &trimmed[len - dec..];

        // Trim trailing zeros from decimal part
        let dec_trimmed = dec_part.trim_end_matches('0');

        let int_formatted = add_thousand_separators(int_part);

        if dec_trimmed.is_empty() {
            format!("{}.0", int_formatted)
        } else {
            format!("{}.{}", int_formatted, dec_trimmed)
        }
    }
}

/// Add thousand separators to a numeric string
fn add_thousand_separators(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::new();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }

    result
}

/// Render a clickable uint value with decimal popup
pub fn render_uint_with_popup(ui: &mut egui::Ui, value: &str, id_salt: &str) {
    let popup_id = ui.make_persistent_id(format!("uint_popup_{}", id_salt));

    // Get/create state for this popup
    let mut state = ui.memory(|m| {
        m.data
            .get_temp::<UintPopupState>(popup_id)
            .unwrap_or(UintPopupState { decimals: 18 })
    });

    // Clickable value label
    let response = ui.add(egui::Button::new(egui::RichText::new(value).monospace()).frame(false));

    if response.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup_id));
    }

    // Show popup below the value
    egui::popup_below_widget(
        ui,
        popup_id,
        &response,
        egui::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_min_width(200.0);

            // Formatted value with copy button
            let formatted = format_uint_with_decimals(value, state.decimals);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&formatted).monospace().strong());
                if ui.small_button("📋").on_hover_text("Copy").clicked() {
                    copy_to_clipboard(&formatted);
                }
            });

            ui.separator();

            // Decimals input field
            ui.horizontal(|ui| {
                ui.label("Decimals:");
                let mut dec_str = state.decimals.to_string();
                let response = ui.add(
                    egui::TextEdit::singleline(&mut dec_str)
                        .desired_width(40.0)
                        .font(egui::TextStyle::Monospace),
                );
                if response.changed() {
                    if let Ok(d) = dec_str.parse::<u8>() {
                        if d <= 77 {
                            state.decimals = d;
                        }
                    }
                }
            });

            // Preset buttons
            ui.horizontal(|ui| {
                for (label, dec) in [("18", 18), ("9", 9), ("8", 8), ("6", 6), ("0", 0)] {
                    let selected = state.decimals == dec;
                    if ui.selectable_label(selected, label).clicked() {
                        state.decimals = dec;
                    }
                }
            });

            // Hint text
            ui.label(egui::RichText::new("18=wei  9=gwei  6=USDC").weak().small());
        },
    );

    // Store state
    ui.memory_mut(|m| m.data.insert_temp(popup_id, state));
}
