//! UI helper components

use eframe::egui;

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

/// Create a styled multiline text edit
pub fn multiline_input(ui: &mut egui::Ui, value: &mut String, hint: &str, rows: usize) -> egui::Response {
    ui.add(
        egui::TextEdit::multiline(value)
            .hint_text(hint)
            .desired_width(f32::INFINITY)
            .desired_rows(rows)
            .font(egui::TextStyle::Monospace),
    )
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
        ui.label(egui::RichText::new(hash).monospace().small());
        if ui.small_button("📋").on_hover_text("Copy to clipboard").clicked() {
            copy_to_clipboard(hash);
        }
    });
}

