use egui::Ui;

use crate::signing_ui::state::SigningUiState;

pub fn render_queue(ui: &mut Ui, state: &mut SigningUiState) {
    ui.heading("Signing Queue");
    ui.label("Phase A1 scaffold: queue flow rendering will be wired to signing_bridge.");
    if ui.button("Open Selected Flow").clicked() {
        state.selected_flow_id = Some("tx:placeholder".to_owned());
    }
}
