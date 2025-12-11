//! Main application state and update loop

use alloy::primitives::ChainId;
use eframe::egui;
use safe_hash::SafeWarnings;
use safe_utils::{get_all_supported_chain_names, DomainHasher, MessageHasher, Of, SafeHasher, SafeWalletVersion};
use std::sync::{Arc, Mutex};

use crate::api::SafeTransaction;
use crate::decode::{self, SignatureLookup, TransactionKind, SingleDecode, ComparisonResult, get_selector};

/// Log to console (works in both WASM and native)
macro_rules! debug_log {
    ($($arg:tt)*) => {
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::console::log_1(&format!($($arg)*).into());
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            eprintln!("[app] {}", format!($($arg)*));
        }
    };
}
use crate::expected;
use crate::hasher::{get_warnings_for_tx, get_warnings_from_api_tx, compute_hashes, compute_hashes_from_api_tx, fetch_transaction};
use crate::state::{Eip712State, MsgVerifyState, TxVerifyState, OfflineState, SAFE_VERSIONS};
use crate::ui;

/// Result from async fetch operation
#[derive(Clone)]
pub enum FetchResult {
    Success(SafeTransaction),
    Error(String),
}

/// Result from async decode operation
#[derive(Clone)]
pub enum DecodeResult {
    Single {
        selector: String,
        local_decode: Result<decode::LocalDecode, String>,
    },
    MultiSendBulk {
        multi: decode::MultiSendDecode,
    },
}

/// Result from async Safe info fetch
#[derive(Clone)]
pub enum SafeInfoResult {
    Success(crate::hasher::SafeInfo),
    Error(String),
}

/// Result from async offline decode
#[derive(Clone)]
pub enum OfflineDecodeResult {
    Success(decode::OfflineDecodeResult),
    Error(String),
}

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
    /// Offline verification state
    offline_state: OfflineState,
    /// Cached chain names from safe_utils
    chain_names: Vec<String>,
    /// Async fetch result receiver
    fetch_result: Arc<Mutex<Option<FetchResult>>>,
    /// Signature lookup client (with cache)
    signature_lookup: SignatureLookup,
    /// Async decode result receiver
    decode_result: Arc<Mutex<Option<DecodeResult>>>,
    /// Async Safe info fetch result receiver
    safe_info_result: Arc<Mutex<Option<SafeInfoResult>>>,
    /// Async offline decode result receiver
    offline_decode_result: Arc<Mutex<Option<OfflineDecodeResult>>>,
    /// Fetched Safe info
    safe_info: Option<crate::hasher::SafeInfo>,
    /// Whether Safe info fetch is in progress
    safe_info_loading: bool,
}

/// Available tabs in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Transaction,
    Message,
    Eip712,
    Offline,
}

impl App {
    /// Create a new App instance
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Load cached Safe address from LocalStorage
        let cached_address = crate::state::load_cached_safe_address().unwrap_or_default();
        
        let mut tx_state = TxVerifyState::default();
        let mut msg_state = MsgVerifyState::default();
        let mut eip712_state = Eip712State::default();
        let mut offline_state = OfflineState::default();
        
        // Apply cached address to all states
        if !cached_address.is_empty() {
            tx_state.safe_address = cached_address.clone();
            msg_state.safe_address = cached_address.clone();
            eip712_state.safe_address = cached_address.clone();
            offline_state.safe_address = cached_address;
        }
        
        Self {
            active_tab: Tab::default(),
            tx_state,
            msg_state,
            eip712_state,
            offline_state,
            chain_names: get_all_supported_chain_names(),
            fetch_result: Arc::new(Mutex::new(None)),
            signature_lookup: SignatureLookup::new(),
            decode_result: Arc::new(Mutex::new(None)),
            safe_info_result: Arc::new(Mutex::new(None)),
            offline_decode_result: Arc::new(Mutex::new(None)),
            safe_info: None,
            safe_info_loading: false,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());

        // Check for async fetch results
        self.check_fetch_result(ctx);

        // Check for async decode results
        self.check_decode_result();
        
        // Check for async Safe info results
        self.check_safe_info_result();
        
        // Check for async offline decode results
        self.check_offline_decode_result();

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
                ui.selectable_value(&mut self.active_tab, Tab::Offline, "📴 Offline");
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
                    Tab::Offline => self.render_offline_tab(ui, ctx),
                }
                ui.add_space(20.0);
            });
        });
    }
}

impl App {
    fn render_transaction_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
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
            
            // If we have safe_info with a valid version, show it as read-only
            let version_from_api = self.safe_info.as_ref()
                .filter(|info| SAFE_VERSIONS.contains(&info.version.as_str()));
            
            if version_from_api.is_some() {
                // Show as disabled/read-only
                ui.add_enabled(false, egui::Button::new(&self.tx_state.safe_version).min_size(egui::vec2(100.0, 0.0)));
                ui.label(egui::RichText::new("(from API)").weak().small());
            } else {
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
            }
        });

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Safe Address:");
            let response = ui::address_input(ui, &mut self.tx_state.safe_address);
            
            // Cache address and fetch Safe info when focus leaves the field
            let should_fetch = response.lost_focus() && !response.ctx.input(|i| i.key_pressed(egui::Key::Escape));
            
            if should_fetch {
                crate::state::save_safe_address(&self.tx_state.safe_address);
                // Trigger Safe info fetch if address looks valid
                if self.tx_state.safe_address.starts_with("0x") && self.tx_state.safe_address.len() == 42 {
                    self.trigger_safe_info_fetch();
                }
            }
            
            if self.safe_info_loading {
                ui.spinner();
            }
        });

        // Show Safe info if available
        if let Some(ref info) = self.safe_info {
            ui.add_space(5.0);
            
            // Threshold info
            ui.horizontal(|ui| {
                ui.add_space(100.0);
                ui.label(egui::RichText::new(format!(
                    "Threshold: {}/{}",
                    info.threshold,
                    info.owners.len()
                )).size(14.0));
            });
            
            // List signers
            for (i, owner) in info.owners.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.add_space(120.0);
                    ui.label(egui::RichText::new(format!("{:?}", owner)).monospace().small());
                });
            }
            
            // Modules info (if any)
            if !info.modules.is_empty() {
                ui.add_space(3.0);
                ui.horizontal(|ui| {
                    ui.add_space(100.0);
                    ui.label(egui::RichText::new(format!(
                        "Modules: {}",
                        info.modules.len()
                    )).size(14.0));
                });
                
                for (i, module) in info.modules.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.add_space(120.0);
                        ui.label(egui::RichText::new(format!("{:?}", module)).monospace().small());
                    });
                }
            }
        }

        ui.add_space(5.0);

        ui.horizontal(|ui| {
            ui.label("Nonce:");
            
            // Decrement button
            if ui.small_button("◀").on_hover_text("Previous nonce").clicked() {
                if let Ok(n) = self.tx_state.nonce.parse::<u64>() {
                    if n > 0 {
                        self.tx_state.nonce = (n - 1).to_string();
                    }
                }
            }
            
            ui::number_input(ui, &mut self.tx_state.nonce, "e.g., 42");
            
            // Increment button
            if ui.small_button("▶").on_hover_text("Next nonce").clicked() {
                if let Ok(n) = self.tx_state.nonce.parse::<u64>() {
                    self.tx_state.nonce = (n + 1).to_string();
                }
            }
            
            // Show latest nonce info
            if let Some(ref info) = self.safe_info {
                if ui.small_button(format!("⟳ Latest: {}", info.nonce))
                    .on_hover_text("Click to use latest nonce (next available)")
                    .clicked() 
                {
                    // Set to latest - 1 since we want the most recent queued tx
                    if info.nonce > 0 {
                        self.tx_state.nonce = (info.nonce - 1).to_string();
                    } else {
                        self.tx_state.nonce = "0".to_string();
                    }
                }
            }
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

        // Expected values section (only shown when NOT in offline mode)
        if !self.tx_state.offline_mode {
            ui.add_space(10.0);
            expected::render_section(ui, &mut self.tx_state.expected);
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
                    self.fetch_and_verify(ctx);
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
                    let data = &tx.data;
                    if data.is_empty() || data == "0x" {
                        ui.label(egui::RichText::new("0x (empty)").monospace());
                    } else if data.len() > 66 {
                        // Long data - show preview with toggle
                        ui.vertical(|ui| {
                            if self.tx_state.show_full_data {
                                // Show full data with word wrap
                                let wrapped = data.chars()
                                    .collect::<Vec<_>>()
                                    .chunks(64)
                                    .map(|c| c.iter().collect::<String>())
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                ui.label(egui::RichText::new(&wrapped).monospace().small());
                                if ui.small_button("▲ Show less").clicked() {
                                    self.tx_state.show_full_data = false;
                                }
                            } else {
                                // Show preview
                                ui.label(egui::RichText::new(format!("{}...", &data[..66])).monospace());
                                if ui.small_button("▼ Show more").clicked() {
                                    self.tx_state.show_full_data = true;
                                }
                            }
                        });
                    } else {
                        ui.label(egui::RichText::new(data).monospace());
                    }
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
                });

            // Calldata decode section
            if let Some(decode_state) = &mut self.tx_state.decode {
                ui.add_space(15.0);
                ui::section_header(ui, "Calldata Verification");
                decode::render_decode_section(ui, decode_state);
            }
        }

        // Expected values validation result (before other warnings)
        expected::render_result(ui, &self.tx_state.expected);

        if self.tx_state.warnings.has_warnings() {
            ui.add_space(15.0);
            ui::section_header(ui, "⚠️ Warnings");

            let w = &self.tx_state.warnings;
            if w.delegatecall {
                ui::warning_message(ui, "⚠️ DELEGATECALL - can modify Safe state!", egui::Color32::from_rgb(220, 50, 50));
            }
            if w.non_zero_gas_token {
                ui::warning_message(ui, "Non-zero gas token", egui::Color32::from_rgb(220, 180, 50));
            }
            if w.non_zero_refund_receiver {
                ui::warning_message(ui, "Non-zero refund receiver", egui::Color32::from_rgb(220, 180, 50));
            }
            if w.dangerous_methods {
                ui::warning_message(ui, "⚠️ Dangerous method (owner/threshold change)", egui::Color32::from_rgb(220, 120, 50));
            }
            for mismatch in &w.argument_mismatches {
                ui::warning_message(
                    ui,
                    &format!("Mismatch in {}: API={}, computed={}", mismatch.field, mismatch.api_value, mismatch.user_value),
                    egui::Color32::from_rgb(220, 50, 50),
                );
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
            let response = ui::address_input(ui, &mut self.msg_state.safe_address);
            if response.lost_focus() || response.changed() {
                crate::state::save_safe_address(&self.msg_state.safe_address);
            }
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
            let response = ui::address_input(ui, &mut self.eip712_state.safe_address);
            if response.lost_focus() || response.changed() {
                crate::state::save_safe_address(&self.eip712_state.safe_address);
            }
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
        self.tx_state.warnings = SafeWarnings::new();

        // Check warnings using safe_hash::check_suspicious_content
        self.tx_state.warnings = get_warnings_for_tx(
            &self.tx_state.to,
            &self.tx_state.value,
            &self.tx_state.data,
            self.tx_state.operation,
            &self.tx_state.safe_tx_gas,
            &self.tx_state.base_gas,
            &self.tx_state.gas_price,
            &self.tx_state.gas_token,
            &self.tx_state.refund_receiver,
        );

        // Compute hashes using safe_hash::tx_signing_hashes
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
                self.tx_state.error = Some(format!("{:#}", e));
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

    fn fetch_and_verify(&mut self, ctx: &egui::Context) {
        self.tx_state.is_loading = true;
        self.tx_state.error = None;
        self.tx_state.warnings = SafeWarnings::new();
        self.tx_state.hashes = None;
        self.tx_state.fetched_tx = None;

        let chain_name = self.tx_state.chain_name.clone();
        let safe_address = self.tx_state.safe_address.clone();
        let nonce: u64 = match self.tx_state.nonce.trim().parse() {
            Ok(n) => n,
            Err(_) => {
                self.tx_state.error = Some("Invalid nonce".to_string());
                self.tx_state.is_loading = false;
                return;
            }
        };

        let result = Arc::clone(&self.fetch_result);
        let ctx = ctx.clone();

        // Spawn async task
        #[cfg(target_arch = "wasm32")]
        {
            wasm_bindgen_futures::spawn_local(async move {
                let fetch_result = fetch_transaction(&chain_name, &safe_address, nonce).await;
                let mut result_guard = result.lock().unwrap();
                *result_guard = Some(match fetch_result {
                    Ok(tx) => FetchResult::Success(tx),
                    Err(e) => FetchResult::Error(format!("{:#}", e)),
                });
                ctx.request_repaint();
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let fetch_result = rt.block_on(fetch_transaction(&chain_name, &safe_address, nonce));
                let mut result_guard = result.lock().unwrap();
                *result_guard = Some(match fetch_result {
                    Ok(tx) => FetchResult::Success(tx),
                    Err(e) => FetchResult::Error(format!("{:#}", e)),
                });
                ctx.request_repaint();
            });
        }
    }

    fn check_fetch_result(&mut self, ctx: &egui::Context) {
        let result = {
            let mut guard = self.fetch_result.lock().unwrap();
            guard.take()
        };

        if let Some(result) = result {
            self.tx_state.is_loading = false;

            match result {
                FetchResult::Success(tx) => {
                    // Compute hashes from the fetched transaction using validate_safe_tx_hash
                    match compute_hashes_from_api_tx(
                        &self.tx_state.chain_name,
                        &self.tx_state.safe_address,
                        &self.tx_state.safe_version,
                        &tx,
                    ) {
                        Ok((hashes, mismatch)) => {
                            // Add hash mismatch to warnings if present
                            if let Some(m) = mismatch {
                                self.tx_state.warnings.argument_mismatches.push(m);
                            }
                            self.tx_state.hashes = Some(hashes);
                        }
                        Err(e) => {
                            self.tx_state.error = Some(format!("Hash computation failed: {:#}", e));
                        }
                    }

                    // Get warnings using check_suspicious_content (via get_warnings_from_api_tx)
                    let chain_id = ChainId::of(&self.tx_state.chain_name).ok();
                    self.tx_state.warnings.union(get_warnings_from_api_tx(&tx, chain_id));

                    // Validate against expected values if any were provided
                    if self.tx_state.expected.has_values() {
                        self.tx_state.expected.result =
                            Some(expected::validate_against_api(&tx, &self.tx_state.expected));
                    }

                    // Initialize calldata decode
                    debug_log!("Parsing calldata: {} bytes", tx.data.len());
                    let decode_state = decode::parse_initial(&tx.data, tx.data_decoded.as_ref());
                    debug_log!("Decode kind: {:?}, selector: {}", 
                        match &decode_state.kind {
                            TransactionKind::Empty => "Empty",
                            TransactionKind::Single(_) => "Single",
                            TransactionKind::MultiSend(_) => "MultiSend",
                            TransactionKind::Unknown => "Unknown",
                        },
                        decode_state.selector
                    );
                    // Determine what verification to trigger
                    let verification_action = match &decode_state.kind {
                        TransactionKind::Single(_) if !decode_state.selector.is_empty() => {
                            Some(("single", decode_state.selector.clone(), tx.data.clone(), 0))
                        }
                        TransactionKind::MultiSend(multi) => {
                            Some(("multi", String::new(), String::new(), multi.transactions.len()))
                        }
                        _ => None,
                    };

                    self.tx_state.decode = Some(decode_state);

                    // Trigger verification based on transaction type
                    if let Some((kind, selector, data, tx_count)) = verification_action {
                        match kind {
                            "single" => {
                                debug_log!("Triggering 4byte lookup for selector: {}", selector);
                                self.trigger_decode_lookup(&selector, &data);
                            }
                            "multi" => {
                                debug_log!("Triggering bulk verification for {} transactions", tx_count);
                                // Update verification state
                                if let Some(ref mut decode) = self.tx_state.decode {
                                    if let TransactionKind::MultiSend(ref mut multi) = decode.kind {
                                        multi.verification_state = decode::VerificationState::InProgress { 
                                            total: tx_count 
                                        };
                                    }
                                }
                                self.trigger_multisend_bulk_verify(ctx);
                            }
                            _ => {}
                        }
                    }

                    self.tx_state.fetched_tx = Some(tx);
                }
                FetchResult::Error(e) => {
                    self.tx_state.error = Some(e);
                }
            }
        }
    }

    fn check_decode_result(&mut self) {
        let result = {
            let mut guard = self.decode_result.lock().unwrap();
            guard.take()
        };

        if let Some(result) = result {
            debug_log!("Received decode result");
            match result {
                DecodeResult::Single { selector: _, local_decode } => {
                    debug_log!("Processing single decode result: {:?}", 
                        local_decode.as_ref().map(|d| &d.method).ok());
                    if let Some(ref mut decode) = self.tx_state.decode {
                        if let TransactionKind::Single(ref mut single) = decode.kind {
                            match local_decode {
                                Ok(local) => {
                                    debug_log!("Local decode success: {}", local.method);
                                    single.local = Some(local);
                                    single.comparison = decode::compare_decodes(
                                        single.api.as_ref(),
                                        single.local.as_ref(),
                                    );
                                    debug_log!("Comparison result: {:?}", single.comparison);
                                }
                                Err(e) => {
                                    single.comparison = ComparisonResult::Failed(e);
                                }
                            }

                            // Update overall status
                            decode.status = match &single.comparison {
                                ComparisonResult::Match => decode::OverallStatus::AllMatch,
                                ComparisonResult::MethodMismatch { .. }
                                | ComparisonResult::ParamMismatch(_) => decode::OverallStatus::HasMismatches,
                                _ => decode::OverallStatus::PartiallyVerified,
                            };
                        }
                    }
                }
                DecodeResult::MultiSendBulk { multi: verified_multi } => {
                    debug_log!("Received bulk MultiSend verification result");
                    if let Some(ref mut decode) = self.tx_state.decode {
                        if let TransactionKind::MultiSend(ref mut multi) = decode.kind {
                            // Replace with the verified MultiSend
                            *multi = verified_multi;
                            
                            // Update overall status based on summary
                            decode.status = if multi.summary.mismatched > 0 {
                                decode::OverallStatus::HasMismatches
                            } else if multi.summary.verified == multi.summary.total {
                                decode::OverallStatus::AllMatch
                            } else if multi.summary.verified > 0 {
                                decode::OverallStatus::PartiallyVerified
                            } else {
                                decode::OverallStatus::Pending
                            };
                        }
                    }
                }
            }
        }
    }

    fn trigger_decode_lookup(&self, selector: &str, data: &str) {
        let lookup = self.signature_lookup.clone();
        let selector = selector.to_string();
        let data = data.to_string();
        let result = Arc::clone(&self.decode_result);

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen_futures::spawn_local;
            spawn_local(async move {
                let local_decode = Self::do_decode_lookup(&lookup, &selector, &data).await;
                let mut guard = result.lock().unwrap();
                *guard = Some(DecodeResult::Single { selector, local_decode });
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let local_decode = rt.block_on(Self::do_decode_lookup(&lookup, &selector, &data));
                let mut guard = result.lock().unwrap();
                *guard = Some(DecodeResult::Single { selector, local_decode });
            });
        }
    }

    fn trigger_multisend_bulk_verify(&mut self, ctx: &egui::Context) {
        // Get the MultiSend data
        let multi = if let Some(ref decode) = self.tx_state.decode {
            if let TransactionKind::MultiSend(ref m) = decode.kind {
                Some(m.clone())
            } else {
                None
            }
        } else {
            None
        };

        let Some(mut multi) = multi else {
            return;
        };

        let lookup = self.signature_lookup.clone();
        let result = Arc::clone(&self.decode_result);
        let ctx = ctx.clone();

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen_futures::spawn_local;
            spawn_local(async move {
                decode::verify_multisend_batch(&mut multi, &lookup).await;
                let mut guard = result.lock().unwrap();
                *guard = Some(DecodeResult::MultiSendBulk { multi });
                ctx.request_repaint();
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(decode::verify_multisend_batch(&mut multi, &lookup));
                let mut guard = result.lock().unwrap();
                *guard = Some(DecodeResult::MultiSendBulk { multi });
                ctx.request_repaint();
            });
        }
    }

    async fn do_decode_lookup(
        lookup: &SignatureLookup,
        selector: &str,
        data: &str,
    ) -> Result<decode::LocalDecode, String> {
        // Lookup signatures for selector (convert eyre error to String)
        let signatures = lookup.lookup(selector).await
            .map_err(|e| format!("{:#}", e))?;

        if signatures.is_empty() {
            return Err("No signatures found for selector".into());
        }

        // Try each signature until one decodes successfully
        for sig in &signatures {
            match decode::decode_with_signature(data, sig) {
                Ok(decoded) => return Ok(decoded),
                Err(_) => continue,
            }
        }

        Err(format!(
            "None of {} signatures decoded successfully",
            signatures.len()
        ))
    }
    
    fn check_safe_info_result(&mut self) {
        let result = {
            let mut guard = self.safe_info_result.lock().unwrap();
            guard.take()
        };

        if let Some(result) = result {
            self.safe_info_loading = false;
            match result {
                SafeInfoResult::Success(info) => {
                    debug_log!("Fetched Safe info: version={}, nonce={}, threshold={}/{}", 
                        info.version, info.nonce, info.threshold, info.owners.len());
                    
                    // Auto-fill version if it matches a supported version
                    let version_str = info.version.as_str();
                    if crate::state::SAFE_VERSIONS.contains(&version_str) {
                        self.tx_state.safe_version = version_str.to_string();
                    }
                    
                    self.safe_info = Some(info);
                }
                SafeInfoResult::Error(e) => {
                    debug_log!("Failed to fetch Safe info: {}", e);
                    // Don't clear safe_info on error, keep previous value
                }
            }
        }
    }
    
    fn trigger_safe_info_fetch(&mut self) {
        if self.safe_info_loading {
            return;
        }
        
        self.safe_info_loading = true;
        let chain_name = self.tx_state.chain_name.clone();
        let safe_address = self.tx_state.safe_address.clone();
        let result = Arc::clone(&self.safe_info_result);

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen_futures::spawn_local;
            spawn_local(async move {
                let fetch_result = crate::hasher::fetch_safe_info(&chain_name, &safe_address).await;
                let mut guard = result.lock().unwrap();
                *guard = Some(match fetch_result {
                    Ok(info) => SafeInfoResult::Success(info),
                    Err(e) => SafeInfoResult::Error(format!("{:#}", e)),
                });
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let fetch_result = rt.block_on(crate::hasher::fetch_safe_info(&chain_name, &safe_address));
                let mut guard = result.lock().unwrap();
                *guard = Some(match fetch_result {
                    Ok(info) => SafeInfoResult::Success(info),
                    Err(e) => SafeInfoResult::Error(format!("{:#}", e)),
                });
            });
        }
    }
    
    // =========================================================================
    // OFFLINE TAB
    // =========================================================================
    
    fn render_offline_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::styled_heading(ui, "Offline Verification");
        ui.label("Manually input transaction data for offline verification (uses 4byte signature lookup).");
        ui.add_space(15.0);
        
        // Chain and Version row
        ui.horizontal(|ui| {
            ui.label("Chain:");
            egui::ComboBox::from_id_salt("offline_chain_select")
                .selected_text(&self.offline_state.chain_name)
                .width(120.0)
                .show_ui(ui, |ui| {
                    for chain in &self.chain_names {
                        ui.selectable_value(
                            &mut self.offline_state.chain_name,
                            chain.clone(),
                            chain,
                        );
                    }
                });

            ui.add_space(20.0);
            ui.label("Safe Version:");
            egui::ComboBox::from_id_salt("offline_version_select")
                .selected_text(&self.offline_state.safe_version)
                .width(100.0)
                .show_ui(ui, |ui| {
                    for version in SAFE_VERSIONS {
                        ui.selectable_value(
                            &mut self.offline_state.safe_version,
                            version.to_string(),
                            *version,
                        );
                    }
                });
        });
        ui.add_space(10.0);
        
        // Safe Address
        ui.horizontal(|ui| {
            ui.label("Safe Address:");
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.offline_state.safe_address)
                    .hint_text("0x...")
                    .desired_width(400.0)
                    .font(egui::TextStyle::Monospace),
            );
            if resp.lost_focus() || resp.changed() {
                // Cache the address
                crate::state::save_safe_address(&self.offline_state.safe_address);
            }
        });
        ui.add_space(10.0);
        
        // Transaction inputs
        ui::section_header(ui, "Transaction Details");
        ui.add_space(5.0);
        
        // To address
        ui.horizontal(|ui| {
            ui.label("To:");
            ui.add(
                egui::TextEdit::singleline(&mut self.offline_state.to)
                    .hint_text("0x...")
                    .desired_width(400.0)
                    .font(egui::TextStyle::Monospace),
            );
        });
        
        // Value
        ui.horizontal(|ui| {
            ui.label("Value (wei):");
            ui.add(
                egui::TextEdit::singleline(&mut self.offline_state.value)
                    .hint_text("0")
                    .desired_width(200.0)
                    .font(egui::TextStyle::Monospace),
            );
        });
        
        // Data
        ui.horizontal(|ui| {
            ui.label("Data:");
            ui.add(
                egui::TextEdit::multiline(&mut self.offline_state.data)
                    .hint_text("0x... (calldata)")
                    .desired_width(500.0)
                    .desired_rows(3)
                    .font(egui::TextStyle::Monospace),
            );
        });
        
        // Operation
        ui.horizontal(|ui| {
            ui.label("Operation:");
            egui::ComboBox::from_id_salt("offline_operation_select")
                .selected_text(if self.offline_state.operation == 0 { "Call (0)" } else { "DelegateCall (1)" })
                .width(150.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.offline_state.operation, 0, "Call (0)");
                    ui.selectable_value(&mut self.offline_state.operation, 1, "DelegateCall (1)");
                });
        });
        
        // Nonce
        ui.horizontal(|ui| {
            ui.label("Nonce:");
            ui.add(
                egui::TextEdit::singleline(&mut self.offline_state.nonce)
                    .hint_text("0")
                    .desired_width(100.0)
                    .font(egui::TextStyle::Monospace),
            );
        });
        
        ui.add_space(10.0);
        
        // Advanced: Gas Parameters (collapsed by default)
        egui::CollapsingHeader::new("⚙️ Advanced: Gas Parameters")
            .default_open(false)
            .show(ui, |ui| {
                ui.add_space(5.0);
                
                ui.horizontal(|ui| {
                    ui.label("SafeTxGas:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.offline_state.safe_tx_gas)
                            .hint_text("0")
                            .desired_width(100.0)
                            .font(egui::TextStyle::Monospace),
                    );
                    
                    ui.add_space(20.0);
                    ui.label("BaseGas:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.offline_state.base_gas)
                            .hint_text("0")
                            .desired_width(100.0)
                            .font(egui::TextStyle::Monospace),
                    );
                    
                    ui.add_space(20.0);
                    ui.label("GasPrice:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.offline_state.gas_price)
                            .hint_text("0")
                            .desired_width(100.0)
                            .font(egui::TextStyle::Monospace),
                    );
                });
                
                ui.horizontal(|ui| {
                    ui.label("GasToken:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.offline_state.gas_token)
                            .hint_text("0x0000...0000")
                            .desired_width(400.0)
                            .font(egui::TextStyle::Monospace),
                    );
                });
                
                ui.horizontal(|ui| {
                    ui.label("RefundReceiver:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.offline_state.refund_receiver)
                            .hint_text("0x0000...0000")
                            .desired_width(400.0)
                            .font(egui::TextStyle::Monospace),
                    );
                });
                
                ui.add_space(5.0);
                ui.label(egui::RichText::new("Most transactions use default values (all zeros)").weak().small());
            });
        
        ui.add_space(15.0);
        
        // Compute button
        let can_compute = !self.offline_state.safe_address.is_empty()
            && !self.offline_state.to.is_empty()
            && !self.offline_state.is_loading;
        
        ui.horizontal(|ui| {
            if ui.add_enabled(can_compute, egui::Button::new("🔐 Compute Hash & Decode")).clicked() {
                self.trigger_offline_compute(ctx.clone());
            }
            
            if self.offline_state.is_loading {
                ui.spinner();
                ui.label("Computing...");
            }
        });
        
        // Error display
        if let Some(ref error) = self.offline_state.error {
            ui.add_space(10.0);
            ui.label(egui::RichText::new(format!("❌ {}", error)).color(egui::Color32::from_rgb(220, 80, 80)));
        }
        
        // Results
        if self.offline_state.hashes.is_some() || self.offline_state.decode_result.is_some() {
            ui.add_space(20.0);
            ui.separator();
            ui.add_space(10.0);
            
            // Hashes
            if let Some(ref hashes) = self.offline_state.hashes {
                ui::section_header(ui, "Transaction Hashes");
                ui.add_space(5.0);
                
                egui::Grid::new("offline_hashes")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("SafeTxHash:");
                        ui::copyable_hash(ui, &hashes.safe_tx_hash);
                        ui.end_row();
                        
                        ui.label("Domain Hash:");
                        ui::copyable_hash(ui, &hashes.domain_hash);
                        ui.end_row();
                        
                        ui.label("Message Hash:");
                        ui::copyable_hash(ui, &hashes.message_hash);
                        ui.end_row();
                    });
            }
            
            // Decode result
            if let Some(ref mut decode_result) = self.offline_state.decode_result {
                ui.add_space(15.0);
                decode::render_offline_decode_section(ui, decode_result);
            }
        }
    }
    
    fn check_offline_decode_result(&mut self) {
        let result = {
            let mut guard = self.offline_decode_result.lock().unwrap();
            guard.take()
        };
        
        if let Some(result) = result {
            self.offline_state.is_loading = false;
            match result {
                OfflineDecodeResult::Success(decode) => {
                    self.offline_state.decode_result = Some(decode);
                }
                OfflineDecodeResult::Error(e) => {
                    self.offline_state.error = Some(e);
                }
            }
        }
    }
    
    fn trigger_offline_compute(&mut self, ctx: egui::Context) {
        self.offline_state.is_loading = true;
        self.offline_state.error = None;
        self.offline_state.hashes = None;
        self.offline_state.decode_result = None;
        
        // Compute hashes synchronously (fast, doesn't need async)
        match crate::hasher::compute_hashes(
            &self.offline_state.chain_name,
            &self.offline_state.safe_address,
            &self.offline_state.safe_version,
            &self.offline_state.to,
            &self.offline_state.value,
            &self.offline_state.data,
            self.offline_state.operation,
            &self.offline_state.safe_tx_gas,
            &self.offline_state.base_gas,
            &self.offline_state.gas_price,
            &self.offline_state.gas_token,
            &self.offline_state.refund_receiver,
            &self.offline_state.nonce,
        ) {
            Ok(hashes) => {
                self.offline_state.hashes = Some(hashes);
            }
            Err(e) => {
                self.offline_state.is_loading = false;
                self.offline_state.error = Some(format!("{:#}", e));
                return;
            }
        }
        
        // Decode async (uses 4byte API)
        let data = self.offline_state.data.clone();
        let lookup = self.signature_lookup.clone();
        let result = Arc::clone(&self.offline_decode_result);
        
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen_futures::spawn_local;
            spawn_local(async move {
                let decode = decode::decode_offline(&data, &lookup).await;
                let mut guard = result.lock().unwrap();
                *guard = Some(OfflineDecodeResult::Success(decode));
                ctx.request_repaint();
            });
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let decode = rt.block_on(decode::decode_offline(&data, &lookup));
                let mut guard = result.lock().unwrap();
                *guard = Some(OfflineDecodeResult::Success(decode));
                ctx.request_repaint();
            });
        }
    }
}
