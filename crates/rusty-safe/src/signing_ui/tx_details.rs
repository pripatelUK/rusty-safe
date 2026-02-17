use egui::Ui;

use crate::signing_ui::state::SigningUiState;

pub fn render_tx_details(ui: &mut Ui, _state: &mut SigningUiState) {
    ui.heading("Transaction Details");
    ui.label("Phase A2 scaffold: sign/propose/confirm/execute actions go here.");
}
