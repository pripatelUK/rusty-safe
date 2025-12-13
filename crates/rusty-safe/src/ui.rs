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

/// Render an address as a clickable hyperlink that opens in block explorer
pub fn address_link(ui: &mut egui::Ui, chain_name: &str, address: &str) -> egui::Response {
    let explorer_url = get_explorer_address_url(chain_name, address);
    let response = ui.link(egui::RichText::new(address).monospace())
        .on_hover_text("Open in block explorer");
    if response.clicked() {
        open_url_new_tab(&explorer_url);
    }
    response
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
        if ui.small_button("📋").on_hover_text("Copy to clipboard").clicked() {
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
pub fn multiline_input(ui: &mut egui::Ui, value: &mut String, hint: &str, rows: usize) -> egui::Response {
    // Calculate height based on row count (approximate line height)
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
    let height = row_height * rows as f32 + ui.spacing().item_spacing.y * 5.0;
    
    let mut response = None;
    egui::ScrollArea::vertical()
        .max_height(height)
        .show(ui, |ui| {
            response = Some(ui.add(
                egui::TextEdit::multiline(value)
                    .hint_text(hint)
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace),
            ));
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
        if ui.small_button("📋").on_hover_text("Copy to clipboard").clicked() {
            copy_to_clipboard(hash);
        }
    });
}

