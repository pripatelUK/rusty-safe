use egui::Ui;

use crate::signing_ui::state::SigningUiState;

pub fn render_message_details(ui: &mut Ui, _state: &mut SigningUiState) {
    ui.heading("Message Details");
    ui.label("Phase A3 scaffold: message threshold progression goes here.");
}
