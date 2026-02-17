use egui::Ui;

use crate::signing_ui::state::SigningUiState;

pub fn render_import_export(ui: &mut Ui, _state: &mut SigningUiState) {
    ui.heading("Import / Export / Share");
    ui.label("Phase A5 scaffold: deterministic merge and bundle UX goes here.");
}
