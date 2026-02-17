use egui::Ui;

use crate::signing_ui::state::SigningUiState;

pub fn render_wc_requests(ui: &mut Ui, _state: &mut SigningUiState) {
    ui.heading("WalletConnect Requests");
    ui.label("Phase A4 scaffold: quick/deferred response flows go here.");
}
