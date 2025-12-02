//! Main application state and update loop

use eframe::egui;

/// The main application state
pub struct App {
    /// Current active tab
    active_tab: Tab,
}

/// Available tabs in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Transaction,
    Message,
    Eip712,
}

impl App {
    /// Create a new App instance
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            active_tab: Tab::default(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top panel with tabs
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, Tab::Transaction, "Transaction");
                ui.selectable_value(&mut self.active_tab, Tab::Message, "Message");
                ui.selectable_value(&mut self.active_tab, Tab::Eip712, "EIP-712");
            });
        });

        // Central panel with content
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.active_tab {
                Tab::Transaction => self.render_transaction_tab(ui),
                Tab::Message => self.render_message_tab(ui),
                Tab::Eip712 => self.render_eip712_tab(ui),
            }
        });
    }
}

impl App {
    fn render_transaction_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Transaction Verification");
        ui.separator();
        ui.label("Verify Safe transaction hashes before signing.");
        ui.add_space(20.0);
        ui.label("TODO: Transaction verification form");
    }

    fn render_message_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Message Verification");
        ui.separator();
        ui.label("Verify Safe message signing hashes.");
        ui.add_space(20.0);
        ui.label("TODO: Message verification form");
    }

    fn render_eip712_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("EIP-712 Typed Data");
        ui.separator();
        ui.label("Hash and verify EIP-712 typed data structures.");
        ui.add_space(20.0);
        ui.label("TODO: EIP-712 form");
    }
}

