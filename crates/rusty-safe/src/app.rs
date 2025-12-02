//! Main application state and update loop

use alloy::primitives::ChainId;
use eframe::egui;
use safe_utils::{get_all_supported_chain_names, DomainHasher, MessageHasher, Of, SafeHasher, SafeWalletVersion};

use crate::hasher::{check_warnings, compute_hashes};
use crate::state::{Eip712State, MsgVerifyState, TxVerifyState, SAFE_VERSIONS};
use crate::ui;

/// The main application state
pub struct App {
    /// Current active tab
    active_tab: Tab,
    /// Transaction verification state
    tx_state: TxVerifyState,
    /// Message verification state
    msg_state: MsgVerifyState,
    /// EIP-712 state
    eip712_state: Eip712State,
    /// Cached chain names from safe_utils
    chain_names: Vec<String>,
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
            tx_state: TxVerifyState::default(),
            msg_state: MsgVerifyState::default(),
            eip712_state: Eip712State::default(),
            chain_names: get_all_supported_chain_names(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new("🔐 Rusty-Safe")
                        .size(22.0)
                        .color(egui::Color32::from_rgb(0, 212, 170)),
                );
                ui.add_space(30.0);
                ui.separator();
                ui.add_space(10.0);
                ui.selectable_value(&mut self.active_tab, Tab::Transaction, "📝 Transaction");
                ui.selectable_value(&mut self.active_tab, Tab::Message, "💬 Message");
                ui.selectable_value(&mut self.active_tab, Tab::Eip712, "🔢 EIP-712");
            });
            ui.add_space(4.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(10.0);
                match self.active_tab {
                    Tab::Transaction => self.render_transaction_tab(ui, ctx),
                    Tab::Message => self.render_message_tab(ui),
                    Tab::Eip712 => self.render_eip712_tab(ui),
                }
                ui.add_space(20.0);
            });
        });
    }
}

impl App {
    fn render_transaction_tab(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui::styled_heading(ui, "Transaction Verification");
        ui.label("Verify Safe transaction hashes before signing.");
        ui.add_space(15.0);

        // Chain and Version row - using safe_utils chain names
        ui.horizontal(|ui| {
            ui.label("Chain:");
            egui::ComboBox::from_id_salt("chain_select")
                .selected_text(&self.tx_state.chain_name)
                .width(180.0)
                .show_ui(ui, |ui| {
                    for chain_name in &self.chain_names {
                        ui.selectable_value(
                            &mut self.tx_state.chain_name,
                            chain_name.clone(),
                            chain_name,
                        );
                    }
                });

            ui.add_space(20.0);
            ui.label("Safe Version:");
            egui::ComboBox::from_id_salt("version_select")
                .selected_text(&self.tx_state.safe_version)
                .width(100.0)
                .show_ui(ui, |ui| {
                    for version in SAFE_VERSIONS {
                        ui.selectable_value(
                            &mut self.tx_state.safe_version,
                            version.to_string(),
                            *version,
                        );
                    }
                });
        });

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Safe Address:");
            ui::address_input(ui, &mut self.tx_state.safe_address);
        });

        ui.add_space(5.0);

        ui.horizontal(|ui| {
            ui.label("Nonce:");
            ui::number_input(ui, &mut self.tx_state.nonce, "e.g., 42");
        });

        ui.add_space(10.0);

        ui.checkbox(
            &mut self.tx_state.offline_mode,
            "Offline Mode (manually provide all parameters)",
        );

        if self.tx_state.offline_mode {
            ui.add_space(10.0);
            ui::section_header(ui, "Transaction Parameters");

            egui::Grid::new("offline_inputs")
                .num_columns(2)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ui.label("To:");
                    ui::address_input(ui, &mut self.tx_state.to);
                    ui.end_row();

                    ui.label("Value (wei):");
                    ui::number_input(ui, &mut self.tx_state.value, "0");
                    ui.end_row();

                    ui.label("Data (hex):");
                    ui::multiline_input(ui, &mut self.tx_state.data, "0x...", 3);
                    ui.end_row();

                    ui.label("Operation:");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.tx_state.operation, 0, "Call (0)");
                        ui.selectable_value(&mut self.tx_state.operation, 1, "DelegateCall (1)");
                    });
                    ui.end_row();

                    ui.label("Safe Tx Gas:");
                    ui::number_input(ui, &mut self.tx_state.safe_tx_gas, "0");
                    ui.end_row();

                    ui.label("Base Gas:");
                    ui::number_input(ui, &mut self.tx_state.base_gas, "0");
                    ui.end_row();

                    ui.label("Gas Price:");
                    ui::number_input(ui, &mut self.tx_state.gas_price, "0");
                    ui.end_row();

                    ui.label("Gas Token:");
                    ui::address_input(ui, &mut self.tx_state.gas_token);
                    ui.end_row();

                    ui.label("Refund Receiver:");
                    ui::address_input(ui, &mut self.tx_state.refund_receiver);
                    ui.end_row();
                });
        }

        ui.add_space(15.0);

        ui.horizontal(|ui| {
            let can_compute = !self.tx_state.safe_address.is_empty()
                && !self.tx_state.nonce.is_empty()
                && !self.tx_state.is_loading;

            if self.tx_state.offline_mode {
                if ui.button("🔐 Compute Hashes").clicked() && can_compute {
                    self.compute_offline_hashes();
                }
            } else {
                if ui.button("🔍 Fetch & Verify").clicked() && can_compute {
                    self.fetch_and_verify();
                }
            }

            if ui.button("🗑 Clear").clicked() {
                self.tx_state.clear_results();
            }
        });

        if self.tx_state.is_loading {
            ui.add_space(10.0);
            ui::loading_spinner(ui);
        }

        if let Some(error) = &self.tx_state.error {
            ui.add_space(10.0);
            ui::error_message(ui, error);
        }

        if let Some(tx) = &self.tx_state.fetched_tx {
            ui.add_space(15.0);
            ui::section_header(ui, "Transaction Details");

            egui::Grid::new("tx_details")
                .num_columns(2)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    ui.label("To:");
                    ui.label(egui::RichText::new(format!("{}", tx.to)).monospace());
                    ui.end_row();

                    ui.label("Value:");
                    ui.label(format!("{} wei", tx.value));
                    ui.end_row();

                    ui.label("Data:");
                    let data = tx.data.as_deref().unwrap_or("");
                    let data_display = if data.len() > 66 {
                        format!("{}...", &data[..66])
                    } else if data.is_empty() {
                        "0x (empty)".to_string()
                    } else {
                        data.to_string()
                    };
                    ui.label(egui::RichText::new(data_display).monospace());
                    ui.end_row();

                    ui.label("Operation:");
                    ui.label(if tx.operation == 0 {
                        "Call (0)"
                    } else {
                        "DelegateCall (1)"
                    });
                    ui.end_row();

                    ui.label("Confirmations:");
                    ui.label(format!("{} / {}", tx.confirmations.len(), tx.confirmations_required));
                    ui.end_row();

                    // Show decoded data if available
                    if let Some(decoded) = &tx.data_decoded {
                        ui.label("Method:");
                        ui.label(egui::RichText::new(&decoded.method).strong());
                        ui.end_row();
                    }
                });
        }

        if !self.tx_state.warnings.is_empty() {
            ui.add_space(15.0);
            ui::section_header(ui, "⚠️ Warnings");

            for warning in &self.tx_state.warnings {
                ui::warning_message(ui, &warning.message(), warning.severity().color());
            }
        }

        if let Some(hashes) = &self.tx_state.hashes {
            ui.add_space(15.0);
            ui::section_header(ui, "Hash Results");

            egui::Grid::new("hash_results")
                .num_columns(3)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Domain Hash:").strong());
                    ui.label(egui::RichText::new(&hashes.domain_hash).monospace().size(12.0));
                    if ui.small_button("📋").on_hover_text("Copy").clicked() {
                        ui::copy_to_clipboard(&hashes.domain_hash);
                    }
                    ui.end_row();

                    ui.label(egui::RichText::new("Message Hash:").strong());
                    ui.label(egui::RichText::new(&hashes.message_hash).monospace().size(12.0));
                    if ui.small_button("📋").on_hover_text("Copy").clicked() {
                        ui::copy_to_clipboard(&hashes.message_hash);
                    }
                    ui.end_row();

                    ui.label(egui::RichText::new("Safe Tx Hash:").strong());
                    ui.label(egui::RichText::new(&hashes.safe_tx_hash).monospace().size(12.0));
                    if ui.small_button("📋").on_hover_text("Copy").clicked() {
                        ui::copy_to_clipboard(&hashes.safe_tx_hash);
                    }
                    ui.end_row();
                });

            ui.add_space(10.0);
            if let Some(matches) = hashes.matches_api {
                if matches {
                    ui::success_message(ui, "Computed hash matches API data");
                } else {
                    ui::error_message(ui, "Computed hash does NOT match API data!");
                }
            }
        }
    }

    fn render_message_tab(&mut self, ui: &mut egui::Ui) {
        ui::styled_heading(ui, "Message Verification");
        ui.label("Verify Safe message signing hashes.");
        ui.add_space(20.0);

        ui.horizontal(|ui| {
            ui.label("Chain:");
            egui::ComboBox::from_id_salt("msg_chain_select")
                .selected_text(&self.msg_state.chain_name)
                .width(180.0)
                .show_ui(ui, |ui| {
                    for chain_name in &self.chain_names {
                        ui.selectable_value(
                            &mut self.msg_state.chain_name,
                            chain_name.clone(),
                            chain_name,
                        );
                    }
                });

            ui.add_space(20.0);
            ui.label("Safe Version:");
            egui::ComboBox::from_id_salt("msg_version_select")
                .selected_text(&self.msg_state.safe_version)
                .width(100.0)
                .show_ui(ui, |ui| {
                    for version in SAFE_VERSIONS {
                        ui.selectable_value(
                            &mut self.msg_state.safe_version,
                            version.to_string(),
                            *version,
                        );
                    }
                });
        });

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Safe Address:");
            ui::address_input(ui, &mut self.msg_state.safe_address);
        });

        ui.add_space(10.0);

        ui.checkbox(&mut self.msg_state.is_hex, "Message is hex bytes");

        ui.add_space(5.0);

        ui.label("Message:");
        ui::multiline_input(
            ui,
            &mut self.msg_state.message,
            if self.msg_state.is_hex {
                "0x..."
            } else {
                "Enter message text..."
            },
            5,
        );

        ui.add_space(15.0);

        if ui.button("🔐 Compute Hash").clicked() {
            self.compute_message_hash();
        }

        if let Some(hashes) = &self.msg_state.hashes {
            ui.add_space(15.0);
            ui::section_header(ui, "Hash Results");

            egui::Grid::new("msg_hash_results")
                .num_columns(3)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Raw Hash:").strong());
                    ui.label(egui::RichText::new(&hashes.raw_hash).monospace().size(12.0));
                    if ui.small_button("📋").clicked() {
                        ui::copy_to_clipboard(&hashes.raw_hash);
                    }
                    ui.end_row();

                    ui.label(egui::RichText::new("Message Hash:").strong());
                    ui.label(egui::RichText::new(&hashes.message_hash).monospace().size(12.0));
                    if ui.small_button("📋").clicked() {
                        ui::copy_to_clipboard(&hashes.message_hash);
                    }
                    ui.end_row();

                    ui.label(egui::RichText::new("Safe Msg Hash:").strong());
                    ui.label(egui::RichText::new(&hashes.safe_msg_hash).monospace().size(12.0));
                    if ui.small_button("📋").clicked() {
                        ui::copy_to_clipboard(&hashes.safe_msg_hash);
                    }
                    ui.end_row();
                });
        }

        if let Some(error) = &self.msg_state.error {
            ui.add_space(10.0);
            ui::error_message(ui, error);
        }
    }

    fn render_eip712_tab(&mut self, ui: &mut egui::Ui) {
        ui::styled_heading(ui, "EIP-712 Typed Data");
        ui.label("Hash and verify EIP-712 typed data structures.");
        ui.add_space(20.0);

        ui.horizontal(|ui| {
            ui.label("Chain:");
            egui::ComboBox::from_id_salt("eip712_chain_select")
                .selected_text(&self.eip712_state.chain_name)
                .width(180.0)
                .show_ui(ui, |ui| {
                    for chain_name in &self.chain_names {
                        ui.selectable_value(
                            &mut self.eip712_state.chain_name,
                            chain_name.clone(),
                            chain_name,
                        );
                    }
                });

            ui.add_space(20.0);
            ui.label("Safe Version:");
            egui::ComboBox::from_id_salt("eip712_version_select")
                .selected_text(&self.eip712_state.safe_version)
                .width(100.0)
                .show_ui(ui, |ui| {
                    for version in SAFE_VERSIONS {
                        ui.selectable_value(
                            &mut self.eip712_state.safe_version,
                            version.to_string(),
                            *version,
                        );
                    }
                });
        });

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Safe Address:");
            ui::address_input(ui, &mut self.eip712_state.safe_address);
        });

        ui.add_space(10.0);

        ui.label("EIP-712 JSON:");
        ui::multiline_input(
            ui,
            &mut self.eip712_state.json_input,
            r#"{"types": {...}, "domain": {...}, "message": {...}}"#,
            10,
        );

        ui.add_space(15.0);

        if ui.button("🔐 Compute Hash").clicked() {
            // TODO: Implement using safe_utils::Eip712Hasher
            self.eip712_state.error = Some("EIP-712 hashing coming soon".to_string());
        }

        if let Some(error) = &self.eip712_state.error {
            ui.add_space(10.0);
            ui::error_message(ui, error);
        }
    }

    fn compute_offline_hashes(&mut self) {
        self.tx_state.error = None;
        self.tx_state.warnings.clear();

        // Check warnings
        self.tx_state.warnings = check_warnings(
            &self.tx_state.to,
            &self.tx_state.value,
            &self.tx_state.data,
            self.tx_state.operation,
            &self.tx_state.gas_token,
            &self.tx_state.refund_receiver,
        );

        // Compute hashes using safe_utils
        match compute_hashes(
            &self.tx_state.chain_name,
            &self.tx_state.safe_address,
            &self.tx_state.safe_version,
            &self.tx_state.to,
            &self.tx_state.value,
            &self.tx_state.data,
            self.tx_state.operation,
            &self.tx_state.safe_tx_gas,
            &self.tx_state.base_gas,
            &self.tx_state.gas_price,
            &self.tx_state.gas_token,
            &self.tx_state.refund_receiver,
            &self.tx_state.nonce,
        ) {
            Ok(hashes) => {
                self.tx_state.hashes = Some(hashes);
            }
            Err(e) => {
                self.tx_state.error = Some(e);
            }
        }
    }

    fn compute_message_hash(&mut self) {
        self.msg_state.error = None;
        self.msg_state.hashes = None;

        // Use safe_utils::Of to get chain ID from name
        let chain_id = match ChainId::of(&self.msg_state.chain_name) {
            Ok(id) => id,
            Err(e) => {
                self.msg_state.error = Some(format!("Invalid chain: {}", e));
                return;
            }
        };

        let safe_version = match SafeWalletVersion::parse(&self.msg_state.safe_version) {
            Ok(v) => v,
            Err(e) => {
                self.msg_state.error = Some(format!("Invalid version: {}", e));
                return;
            }
        };

        let safe_addr: alloy::primitives::Address = match self.msg_state.safe_address.parse() {
            Ok(a) => a,
            Err(e) => {
                self.msg_state.error = Some(format!("Invalid address: {}", e));
                return;
            }
        };

        // Use safe_utils::MessageHasher
        let msg_hasher = MessageHasher::new(self.msg_state.message.clone());
        let raw_hash = msg_hasher.raw_hash();
        let message_hash = msg_hasher.hash();

        // Use safe_utils::DomainHasher
        let domain_hasher = DomainHasher::new(safe_version, chain_id, safe_addr);
        let domain_hash = domain_hasher.hash();

        // Use safe_utils::SafeHasher
        let safe_hasher = SafeHasher::new(domain_hash, message_hash);
        let safe_msg_hash = safe_hasher.hash();

        self.msg_state.hashes = Some(crate::state::MsgHashes {
            raw_hash: format!("{:?}", raw_hash),
            message_hash: format!("{:?}", message_hash),
            safe_msg_hash: format!("{:?}", safe_msg_hash),
        });
    }

    fn fetch_and_verify(&mut self) {
        // For now, show a message - async fetch will be wired up next
        self.tx_state.error = Some(
            "Async fetch not yet wired. Use Offline Mode and manually enter tx details.".to_string(),
        );
    }
}
