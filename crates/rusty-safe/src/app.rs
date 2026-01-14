//! Main application state and update loop

use alloy::hex;
use alloy::primitives::ChainId;
use eframe::egui;
use safe_hash::SafeWarnings;
use safe_utils::{
    get_all_supported_chain_names, DomainHasher, Eip712Hasher, MessageHasher, Of, SafeHasher,
    SafeWalletVersion,
};
use std::sync::{Arc, Mutex};

use crate::api::SafeTransaction;
use crate::decode::{self, ComparisonResult, SignatureLookup, TransactionKind};

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

/// Acquire mutex lock, recovering from poisoned state if necessary.
/// This prevents panics when a thread panicked while holding the lock.
macro_rules! lock_or_recover {
    ($mutex:expr) => {
        match $mutex.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                debug_log!("Warning: Mutex was poisoned, recovering");
                poisoned.into_inner()
            }
        }
    };
}
use crate::expected;
use crate::hasher::{
    compute_hashes_from_api_tx, fetch_transactions, get_warnings_for_tx, get_warnings_from_api_tx,
};
use crate::sidebar;
use crate::state::{
    get_chain_name, AddressValidation, Eip712State, MsgVerifyState, OfflineState, SafeContext,
    SidebarState, TxVerifyState, SAFE_VERSIONS,
};
use crate::ui;

/// Result from async fetch operation
#[derive(Clone)]
pub enum FetchResult {
    Success(Vec<SafeTransaction>),
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
    /// Shared Safe context (sidebar)
    safe_context: SafeContext,
    /// Sidebar UI state
    sidebar_state: SidebarState,
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
    /// Address book UI state
    address_book_open: bool,
    address_book_import_text: String,
    address_book_error: Option<String>,
    address_book_search: String,
    address_book_add_name: String,
    address_book_add_addr: String,
    address_book_add_chain: String,
}

/// Available tabs in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    VerifySafeApi,
    Message,
    Eip712,
    Offline,
}

impl App {
    /// Create a new App instance
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load custom font for logo
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "KumarOne".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/KumarOne-Regular.ttf")),
        );
        fonts.families.insert(
            egui::FontFamily::Name("KumarOne".into()),
            vec!["KumarOne".to_owned()],
        );
        cc.egui_ctx.set_fonts(fonts);

        Self {
            active_tab: Tab::default(),
            safe_context: SafeContext::load(cc.storage),
            sidebar_state: SidebarState::default(),
            tx_state: TxVerifyState::default(),
            msg_state: MsgVerifyState::default(),
            eip712_state: Eip712State::default(),
            offline_state: OfflineState::default(),
            chain_names: get_all_supported_chain_names(),
            fetch_result: Arc::new(Mutex::new(None)),
            signature_lookup: SignatureLookup::load(cc.storage),
            decode_result: Arc::new(Mutex::new(None)),
            safe_info_result: Arc::new(Mutex::new(None)),
            offline_decode_result: Arc::new(Mutex::new(None)),
            safe_info: None,
            safe_info_loading: false,
            address_book_open: false,
            address_book_import_text: String::new(),
            address_book_error: None,
            address_book_search: String::new(),
            address_book_add_name: String::new(),
            address_book_add_addr: String::new(),
            address_book_add_chain: "ethereum".to_string(),
        }
    }
}

impl eframe::App for App {
    /// Called by eframe to save state before shutdown
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.safe_context.save(storage);
        self.signature_lookup.save(storage);
    }

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

        // Header with tabs
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Rusty Safe")
                        .size(26.0)
                        .family(egui::FontFamily::Name("KumarOne".into()))
                        .color(egui::Color32::from_rgb(0, 212, 170)),
                );
                ui.add_space(30.0);
                ui.separator();
                ui.add_space(10.0);
                ui.selectable_value(
                    &mut self.active_tab,
                    Tab::VerifySafeApi,
                    "🔍 Verify Safe API",
                );
                ui.selectable_value(&mut self.active_tab, Tab::Message, "💬 Message");
                ui.selectable_value(&mut self.active_tab, Tab::Eip712, "🔢 EIP-712");
                ui.selectable_value(&mut self.active_tab, Tab::Offline, "📴 Offline");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("📖 Address Book").clicked() {
                        self.address_book_open = !self.address_book_open;
                    }
                });
            });
            ui.add_space(4.0);
        });

        // Address Book Window
        self.render_address_book_window(ctx);

        // Sidebar with Safe context
        let sidebar_action = sidebar::render(
            ctx,
            &mut self.sidebar_state,
            &mut self.safe_context,
            &self.safe_info,
            self.safe_info_loading,
            &self.chain_names,
        );

        // Handle sidebar actions
        match sidebar_action {
            sidebar::SidebarAction::FetchDetails => {
                self.trigger_safe_info_fetch();
            }
            sidebar::SidebarAction::ClearStorage => {
                self.safe_context.clear();
                self.signature_lookup = SignatureLookup::new();
            }
            sidebar::SidebarAction::None => {}
        }

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(10.0);
                match self.active_tab {
                    Tab::VerifySafeApi => self.render_verify_safe_api_tab(ui, ctx),
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
    fn render_verify_safe_api_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::styled_heading(ui, "Verify Safe API");
        ui.label("Verify Safe transaction hashes before signing.");

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // Select Transaction section
        ui.label(egui::RichText::new("Select Transaction").strong().size(13.0));
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.label("Nonce:");

            // Decrement button
            if ui
                .small_button("◀")
                .on_hover_text("Previous nonce")
                .clicked()
            {
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

            // Show latest nonce info and pending count
            if let Some(ref info) = self.safe_info {
                // Latest nonce button - shows the nonce that will be set (nonce - 1)
                let latest_nonce = if info.nonce > 0 { info.nonce - 1 } else { 0 };
                if ui
                    .small_button(format!("⟳ Latest: {}", latest_nonce))
                    .on_hover_text(format!("Set nonce to {}", latest_nonce))
                    .clicked()
                {
                    self.tx_state.nonce = latest_nonce.to_string();
                }

                // Show pending nonce count if available
                if let Some(pending_count) = info.pending_nonce_count {
                    if pending_count > 0 {
                        // Pending button - shows the nonce that will be set (count - 1)
                        let pending_nonce = pending_count - 1;
                        if ui
                            .small_button(format!("⏳ Pending: {}", pending_nonce))
                            .on_hover_text(format!("Set nonce to {}", pending_nonce))
                            .clicked()
                        {
                            self.tx_state.nonce = pending_nonce.to_string();
                        }
                    }
                }
            }
        });

        // Expected values section
        ui.add_space(10.0);
        expected::render_section(ui, &mut self.tx_state.expected);

        ui.add_space(15.0);

        ui.horizontal(|ui| {
            let can_compute = !self.safe_context.safe_address.is_empty()
                && !self.tx_state.nonce.is_empty()
                && !self.tx_state.is_loading;

            if ui::primary_button_enabled(ui, "🔍 Fetch & Verify", can_compute).clicked() {
                self.fetch_and_verify(ctx);
            }

            ui.add_space(8.0);

            if ui::secondary_button(ui, "🗑 Clear").clicked() {
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

        if self.tx_state.fetched_txs.len() > 1 {
            ui.add_space(10.0);
            ui::section_header(ui, "Select Transaction");
            ui::warning_banner(
                ui,
                "Multiple transactions found for this nonce. Safe API keeps all proposals with the same nonce (replacements/cancellations). Select one to verify.",
            );

            let mut selected_index = self.tx_state.selected_tx_index.unwrap_or(0);
            if selected_index >= self.tx_state.fetched_txs.len() {
                selected_index = 0;
            }

            let show_submission_date = true;
            let mut selection_changed = false;
            ui.add_space(6.0);
            egui::ScrollArea::vertical()
                .max_height(120.0)
                .show(ui, |ui| {
                    for (idx, tx) in self.tx_state.fetched_txs.iter().enumerate() {
                        let label = self.format_tx_label(idx, tx, false, show_submission_date);
                        let selected = selected_index == idx;
                        let indicator = if selected { "●" } else { "○" };

                        // Determine colors based on selection state
                        let (bg_color, text_color) = if selected {
                            (
                                egui::Color32::from_rgb(0, 150, 120),
                                egui::Color32::WHITE,
                            )
                        } else {
                            (
                                egui::Color32::from_rgb(45, 45, 45),
                                egui::Color32::from_rgb(200, 200, 200),
                            )
                        };

                        let full_label = format!("{} {}", indicator, label);
                        let response = egui::Frame::none()
                            .fill(bg_color)
                            .rounding(4.0)
                            .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.label(
                                    egui::RichText::new(&full_label)
                                        .size(13.0)
                                        .color(text_color),
                                );
                            })
                            .response
                            .interact(egui::Sense::click())
                            .on_hover_cursor(egui::CursorIcon::PointingHand);

                        if response.clicked() {
                            selected_index = idx;
                            selection_changed = true;
                        }
                        ui.add_space(4.0);
                    }
                });

            if selection_changed {
                self.tx_state.selected_tx_index = Some(selected_index);
                if let Some(tx) = self.tx_state.fetched_txs.get(selected_index).cloned() {
                    self.apply_fetched_tx(ctx, tx);
                }
            }
        }

        if let Some(tx) = &self.tx_state.fetched_tx {
            ui.add_space(15.0);
            ui::section_header(ui, "Transaction Details");

            let action_label = self.tx_action_label(tx);
            let status_label = Self::tx_status_label(tx);

            egui::Grid::new("tx_details")
                .num_columns(3)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Safe Tx Hash:");
                    ui.label(
                        egui::RichText::new(&tx.safe_tx_hash)
                            .monospace()
                            .size(11.0),
                    );
                    if ui.small_button("📋").on_hover_text("Copy").clicked() {
                        ui::copy_to_clipboard(&tx.safe_tx_hash);
                    }
                    ui.end_row();

                    ui.label("Status:");
                    ui.label(status_label);
                    ui.label(""); // Empty for alignment
                    ui.end_row();

                    ui.label("Action:");
                    ui.label(&action_label);
                    ui.label(""); // Empty for alignment
                    ui.end_row();

                    ui.label("Submitted:");
                    ui.label(Self::format_datetime(&tx.submission_date));
                    ui.label(""); // Empty for alignment
                    ui.end_row();

                    ui.label("To:");
                    let to_str = format!("{}", tx.to);
                    let chain_id =
                        alloy::primitives::ChainId::of(&self.safe_context.chain_name).unwrap_or(1);
                    let name = self.safe_context.address_book.get_name(&to_str, chain_id);
                    ui::address_link(ui, &self.safe_context.chain_name, &to_str, name);
                    if ui.small_button("📋").on_hover_text("Copy").clicked() {
                        ui::copy_to_clipboard(&to_str);
                    }
                    ui.end_row();

                    ui.label("Value:");
                    ui.label(ui::format_wei_value(&tx.value));
                    ui.label(""); // Empty for alignment
                    ui.end_row();

                    ui.label("Operation:");
                    ui.label(if tx.operation == 0 {
                        "Call (0)"
                    } else {
                        "DelegateCall (1)"
                    });
                    ui.label(""); // Empty for alignment
                    ui.end_row();

                    ui.label("Confirmations:");
                    ui.label(format!(
                        "{} / {}",
                        tx.confirmations.len(),
                        tx.confirmations_required
                    ));
                    ui.label(""); // Empty for alignment
                    ui.end_row();

                    if let Some(execution_date) = &tx.execution_date {
                        ui.label("Executed:");
                        ui.label(Self::format_datetime(execution_date));
                        ui.label(""); // Empty for alignment
                        ui.end_row();
                    }

                    if let Some(tx_hash) = &tx.transaction_hash {
                        ui.label("Transaction Hash:");
                        ui.label(egui::RichText::new(tx_hash).monospace().size(11.0));
                        if ui.small_button("📋").on_hover_text("Copy").clicked() {
                            ui::copy_to_clipboard(tx_hash);
                        }
                        ui.end_row();
                    }

                    if !tx.origin.is_empty() {
                        ui.label("Origin:");
                        ui.label(&tx.origin);
                        ui.label(""); // Empty for alignment
                        ui.end_row();
                    }
                });

            // Data field - full width outside grid
            let data = &tx.data;
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Data:").strong());

            if data.is_empty() || data == "0x" {
                ui.label(egui::RichText::new("0x (empty)").monospace());
            } else {
                // 3 lines = 64 chars * 3 = 192 chars
                let preview_len = 192.min(data.len());
                let needs_toggle = data.len() > preview_len;

                if self.tx_state.show_full_data || !needs_toggle {
                    // Show full data with word wrap
                    let wrapped = data
                        .chars()
                        .collect::<Vec<_>>()
                        .chunks(64)
                        .map(|c| c.iter().collect::<String>())
                        .collect::<Vec<_>>()
                        .join("\n");
                    ui.label(egui::RichText::new(&wrapped).monospace().size(11.0));

                    ui.horizontal(|ui| {
                        if ui
                            .small_button("📋 Copy")
                            .on_hover_text("Copy data")
                            .clicked()
                        {
                            ui::copy_to_clipboard(data);
                        }
                        if needs_toggle && ui.small_button("▲ Show less").clicked() {
                            self.tx_state.show_full_data = false;
                        }
                    });
                } else {
                    // Show 3-line preview
                    let preview = &data[..preview_len];
                    let wrapped = preview
                        .chars()
                        .collect::<Vec<_>>()
                        .chunks(64)
                        .map(|c| c.iter().collect::<String>())
                        .collect::<Vec<_>>()
                        .join("\n");
                    ui.label(
                        egui::RichText::new(format!("{}...", wrapped))
                            .monospace()
                            .size(11.0),
                    );

                    ui.horizontal(|ui| {
                        if ui
                            .small_button("📋 Copy")
                            .on_hover_text("Copy data")
                            .clicked()
                        {
                            ui::copy_to_clipboard(data);
                        }
                        if ui.small_button("▼ Show more").clicked() {
                            self.tx_state.show_full_data = true;
                        }
                    });
                }
            }

            // Calldata decode section
            if let Some(decode_state) = &mut self.tx_state.decode {
                ui.add_space(15.0);
                ui::section_header(ui, "Calldata Verification");
                decode::render_decode_section(ui, decode_state, &self.safe_context);
            }
        }

        // Expected values validation result (before other warnings)
        expected::render_result(ui, &self.tx_state.expected);

        let warnings_error = self.tx_state.warnings_error.as_deref();
        if self.tx_state.warnings.has_warnings() || warnings_error.is_some() {
            ui.add_space(15.0);
            ui::section_header(ui, "⚠️ Warnings");

            if let Some(error) = warnings_error {
                ui::error_message(ui, &format!("Warning computation failed: {}", error));
            }

            let w = &self.tx_state.warnings;
            if w.delegatecall {
                ui::error_banner(ui, "DELEGATECALL - can modify Safe state!");
            }
            if w.non_zero_gas_token {
                ui::warning_banner(ui, "Non-zero gas token");
            }
            if w.non_zero_refund_receiver {
                ui::warning_banner(ui, "Non-zero refund receiver");
            }
            if w.dangerous_methods {
                ui::warning_banner(ui, "Dangerous method (owner/threshold change)");
            }
            for mismatch in &w.argument_mismatches {
                ui::error_banner(
                    ui,
                    &format!(
                        "Mismatch in {}: API={}, computed={}",
                        mismatch.field, mismatch.api_value, mismatch.user_value
                    ),
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
                    ui.label(
                        egui::RichText::new(&hashes.domain_hash)
                            .monospace()
                            .size(12.0),
                    );
                    if ui.small_button("📋").on_hover_text("Copy").clicked() {
                        ui::copy_to_clipboard(&hashes.domain_hash);
                    }
                    ui.end_row();

                    ui.label(egui::RichText::new("Message Hash:").strong());
                    ui.label(
                        egui::RichText::new(&hashes.message_hash)
                            .monospace()
                            .size(12.0),
                    );
                    if ui.small_button("📋").on_hover_text("Copy").clicked() {
                        ui::copy_to_clipboard(&hashes.message_hash);
                    }
                    ui.end_row();

                    ui.label(egui::RichText::new("Safe Tx Hash:").strong());
                    ui.label(
                        egui::RichText::new(&hashes.safe_tx_hash)
                            .monospace()
                            .size(12.0),
                    );
                    if ui.small_button("📋").on_hover_text("Copy").clicked() {
                        ui::copy_to_clipboard(&hashes.safe_tx_hash);
                    }
                    ui.end_row();

                    // Ledger binary format
                    let binary_literal = ui::hash_to_binary_literal(&hashes.safe_tx_hash);
                    ui.label(egui::RichText::new("Ledger Binary:").strong());
                    ui.label(egui::RichText::new(&binary_literal).monospace().size(12.0));
                    if ui.small_button("📋").on_hover_text("Copy").clicked() {
                        ui::copy_to_clipboard(&binary_literal);
                    }
                    ui.end_row();
                });

            ui.add_space(10.0);
            if let Some(matches) = hashes.matches_api {
                if matches {
                    ui::success_banner(ui, "Computed hash matches API data");
                } else {
                    ui::error_banner(ui, "Computed hash does NOT match API data!");
                }
            }
        }
    }

    fn format_tx_label(
        &self,
        index: usize,
        tx: &SafeTransaction,
        compact: bool,
        show_submission_date: bool,
    ) -> String {
        let status = Self::tx_status_label(tx);
        let action = self.tx_action_label(tx);
        let hash = Self::shorten_middle(&tx.safe_tx_hash, 8, 6);
        if compact {
            let submitted = if show_submission_date {
                format!(" | {}", Self::format_datetime(&tx.submission_date))
            } else {
                String::new()
            };
            return format!(
                "Tx {}: {} | {}{} | {}",
                index + 1,
                status,
                action,
                submitted,
                hash
            );
        }

        let to = Self::shorten_middle(&format!("{}", tx.to), 6, 4);
        let submitted = Self::format_datetime(&tx.submission_date);
        format!(
            "Tx {}: {} | {} | to {} | submitted {} | {}",
            index + 1,
            status,
            action,
            to,
            submitted,
            hash
        )
    }

    fn tx_action_label(&self, tx: &SafeTransaction) -> String {
        if let Some(decoded) = &tx.data_decoded {
            if !decoded.method.is_empty() {
                return decoded.method.clone();
            }
        }

        let data_empty = Self::is_empty_data(&tx.data);
        let value_zero = Self::is_zero_value(&tx.value);
        let self_call = self.is_self_call(tx);

        if data_empty && value_zero && self_call && tx.operation == 0 {
            return "cancel (self-call)".to_string();
        }

        if data_empty && !value_zero {
            return "eth transfer".to_string();
        }

        if data_empty {
            return "empty calldata".to_string();
        }

        "unknown call".to_string()
    }

    fn is_self_call(&self, tx: &SafeTransaction) -> bool {
        let safe_address = self.safe_context.safe_address.trim();
        if safe_address.is_empty() {
            return false;
        }
        match safe_address.parse::<alloy::primitives::Address>() {
            Ok(addr) => addr == tx.to,
            Err(_) => false,
        }
    }

    fn is_empty_data(data: &str) -> bool {
        let trimmed = data.trim();
        trimmed.is_empty() || trimmed == "0x" || trimmed == "0X"
    }

    fn is_zero_value(value: &str) -> bool {
        let trimmed = value.trim();
        trimmed.is_empty() || trimmed == "0" || trimmed == "0x0" || trimmed == "0X0"
    }

    fn tx_status_label(tx: &SafeTransaction) -> &'static str {
        if tx.is_executed {
            match tx.is_successful {
                Some(true) => "executed (success)",
                Some(false) => "executed (failed)",
                None => "executed",
            }
        } else {
            "pending"
        }
    }

    fn shorten_middle(value: &str, head: usize, tail: usize) -> String {
        let trimmed = value.trim();
        if trimmed.len() <= head + tail + 3 {
            trimmed.to_string()
        } else {
            format!(
                "{}...{}",
                &trimmed[..head],
                &trimmed[trimmed.len() - tail..]
            )
        }
    }

    /// Format ISO datetime to readable format: "Jan 13, 2026 02:56"
    fn format_datetime(value: &str) -> String {
        let trimmed = value.trim();

        // Try to parse ISO format: 2026-01-13T02:56:59.850757Z
        if trimmed.len() >= 16 && trimmed.contains('T') {
            let months = [
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ];

            // Extract parts: YYYY-MM-DDTHH:MM
            if let (Ok(year), Ok(month), Ok(day), Ok(hour), Ok(min)) = (
                trimmed[0..4].parse::<u32>(),
                trimmed[5..7].parse::<usize>(),
                trimmed[8..10].parse::<u32>(),
                trimmed[11..13].parse::<u32>(),
                trimmed[14..16].parse::<u32>(),
            ) {
                if month >= 1 && month <= 12 {
                    return format!(
                        "{} {}, {} {:02}:{:02}",
                        months[month - 1],
                        day,
                        year,
                        hour,
                        min
                    );
                }
            }
        }

        // Fallback: truncate to first 19 chars
        if trimmed.len() > 19 {
            trimmed[..19].to_string()
        } else {
            trimmed.to_string()
        }
    }

    fn render_message_tab(&mut self, ui: &mut egui::Ui) {
        ui::styled_heading(ui, "Message Verification");
        ui.label("Verify Safe message signing hashes.");
        ui.add_space(15.0);

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

        if ui::primary_button(ui, "🔐 Compute Hash").clicked() {
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
                    ui.label(
                        egui::RichText::new(&hashes.message_hash)
                            .monospace()
                            .size(12.0),
                    );
                    if ui.small_button("📋").clicked() {
                        ui::copy_to_clipboard(&hashes.message_hash);
                    }
                    ui.end_row();

                    ui.label(egui::RichText::new("Safe Msg Hash:").strong());
                    ui.label(
                        egui::RichText::new(&hashes.safe_msg_hash)
                            .monospace()
                            .size(12.0),
                    );
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
        ui.add_space(15.0);

        ui.checkbox(
            &mut self.eip712_state.standalone,
            "Standalone mode (raw EIP-712 only, no Safe wrapping)",
        );
        ui.add_space(10.0);

        ui.label("EIP-712 JSON:");
        ui::multiline_input(
            ui,
            &mut self.eip712_state.json_input,
            r#"{"types": {...}, "domain": {...}, "primaryType": "...", "message": {...}}"#,
            12,
        );

        ui.add_space(15.0);

        if ui::primary_button(ui, "🔐 Compute Hash").clicked() {
            self.compute_eip712_hash();
        }

        if let Some(error) = &self.eip712_state.error {
            ui.add_space(10.0);
            ui::error_message(ui, error);
        }

        if let Some(hashes) = &self.eip712_state.hashes {
            ui.add_space(15.0);
            ui::section_header(ui, "EIP-712 Hash Results");

            egui::Grid::new("eip712_hash_results")
                .num_columns(3)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("EIP-712 Hash:").strong());
                    ui.label(
                        egui::RichText::new(&hashes.eip712_hash)
                            .monospace()
                            .size(12.0),
                    );
                    if ui.small_button("📋").clicked() {
                        ui::copy_to_clipboard(&hashes.eip712_hash);
                    }
                    ui.end_row();

                    ui.label(egui::RichText::new("Domain Hash:").strong());
                    ui.label(
                        egui::RichText::new(&hashes.eip712_domain_hash)
                            .monospace()
                            .size(12.0),
                    );
                    if ui.small_button("📋").clicked() {
                        ui::copy_to_clipboard(&hashes.eip712_domain_hash);
                    }
                    ui.end_row();

                    ui.label(egui::RichText::new("Message Hash:").strong());
                    ui.label(
                        egui::RichText::new(&hashes.eip712_message_hash)
                            .monospace()
                            .size(12.0),
                    );
                    if ui.small_button("📋").clicked() {
                        ui::copy_to_clipboard(&hashes.eip712_message_hash);
                    }
                    ui.end_row();
                });

            // Show Safe-wrapped hashes if not standalone
            if let (Some(safe_domain), Some(safe_msg), Some(safe_hash)) = (
                &hashes.safe_domain_hash,
                &hashes.safe_message_hash,
                &hashes.safe_hash,
            ) {
                ui.add_space(15.0);
                ui::section_header(ui, "Safe-Wrapped Hash Results");

                egui::Grid::new("safe_eip712_hash_results")
                    .num_columns(3)
                    .spacing([10.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Safe Domain Hash:").strong());
                        ui.label(egui::RichText::new(safe_domain).monospace().size(12.0));
                        if ui.small_button("📋").clicked() {
                            ui::copy_to_clipboard(safe_domain);
                        }
                        ui.end_row();

                        ui.label(egui::RichText::new("Safe Message Hash:").strong());
                        ui.label(egui::RichText::new(safe_msg).monospace().size(12.0));
                        if ui.small_button("📋").clicked() {
                            ui::copy_to_clipboard(safe_msg);
                        }
                        ui.end_row();

                        ui.label(
                            egui::RichText::new("Safe Hash:")
                                .strong()
                                .color(egui::Color32::from_rgb(0, 212, 170)),
                        );
                        ui.label(
                            egui::RichText::new(safe_hash)
                                .monospace()
                                .size(12.0)
                                .color(egui::Color32::from_rgb(0, 212, 170)),
                        );
                        if ui.small_button("📋").clicked() {
                            ui::copy_to_clipboard(safe_hash);
                        }
                        ui.end_row();
                    });
            }
        }
    }

    fn compute_eip712_hash(&mut self) {
        self.eip712_state.error = None;
        self.eip712_state.hashes = None;

        if self.eip712_state.json_input.trim().is_empty() {
            self.eip712_state.error = Some("Please enter EIP-712 JSON data".to_string());
            return;
        }

        // Parse and hash the EIP-712 typed data
        let hasher = Eip712Hasher::new(self.eip712_state.json_input.clone());
        let eip712_result = match hasher.hash() {
            Ok(r) => r,
            Err(e) => {
                self.eip712_state.error = Some(format!("Failed to parse EIP-712 data: {}", e));
                return;
            }
        };

        if self.eip712_state.standalone {
            // Standalone mode - just return the raw EIP-712 hashes
            self.eip712_state.hashes = Some(crate::state::Eip712Hashes {
                eip712_hash: eip712_result.eip_712_hash,
                eip712_domain_hash: eip712_result.domain_hash,
                eip712_message_hash: eip712_result.message_hash,
                safe_domain_hash: None,
                safe_message_hash: None,
                safe_hash: None,
            });
        } else {
            // Safe-wrapped mode - wrap the EIP-712 hash in a Safe message
            let chain_id = match ChainId::of(&self.safe_context.chain_name) {
                Ok(id) => id,
                Err(e) => {
                    self.eip712_state.error = Some(format!("Invalid chain: {}", e));
                    return;
                }
            };

            let safe_version = match SafeWalletVersion::parse(&self.safe_context.safe_version) {
                Ok(v) => v,
                Err(e) => {
                    self.eip712_state.error = Some(format!("Invalid version: {}", e));
                    return;
                }
            };

            let safe_addr: alloy::primitives::Address = match self.safe_context.safe_address.parse()
            {
                Ok(a) => a,
                Err(e) => {
                    self.eip712_state.error = Some(format!("Invalid Safe address: {}", e));
                    return;
                }
            };

            // Create message hash from the EIP-712 hash
            let eip712_hash_bytes =
                match hex::decode(eip712_result.eip_712_hash.trim_start_matches("0x")) {
                    Ok(b) => b,
                    Err(e) => {
                        self.eip712_state.error =
                            Some(format!("Failed to decode EIP-712 hash: {}", e));
                        return;
                    }
                };

            if eip712_hash_bytes.len() != 32 {
                self.eip712_state.error = Some("EIP-712 hash must be 32 bytes".to_string());
                return;
            }

            let mut hash_arr = [0u8; 32];
            hash_arr.copy_from_slice(&eip712_hash_bytes);
            let msg_hasher = MessageHasher::new_from_bytes(alloy::primitives::B256::from(hash_arr));
            let safe_message_hash = msg_hasher.hash();

            // Compute Safe domain hash
            let domain_hasher = DomainHasher::new(safe_version, chain_id, safe_addr);
            let safe_domain_hash = domain_hasher.hash();

            // Compute final Safe hash
            let safe_hasher = SafeHasher::new(safe_domain_hash, safe_message_hash);
            let safe_hash = safe_hasher.hash();

            self.eip712_state.hashes = Some(crate::state::Eip712Hashes {
                eip712_hash: eip712_result.eip_712_hash,
                eip712_domain_hash: eip712_result.domain_hash,
                eip712_message_hash: eip712_result.message_hash,
                safe_domain_hash: Some(format!("{:?}", safe_domain_hash)),
                safe_message_hash: Some(format!("{:?}", safe_message_hash)),
                safe_hash: Some(format!("{:?}", safe_hash)),
            });
        }
    }

    fn compute_message_hash(&mut self) {
        self.msg_state.error = None;
        self.msg_state.hashes = None;

        // Use safe_utils::Of to get chain ID from name
        let chain_id = match ChainId::of(&self.safe_context.chain_name) {
            Ok(id) => id,
            Err(e) => {
                self.msg_state.error = Some(format!("Invalid chain: {}", e));
                return;
            }
        };

        let safe_version = match SafeWalletVersion::parse(&self.safe_context.safe_version) {
            Ok(v) => v,
            Err(e) => {
                self.msg_state.error = Some(format!("Invalid version: {}", e));
                return;
            }
        };

        let safe_addr: alloy::primitives::Address = match self.safe_context.safe_address.parse() {
            Ok(a) => a,
            Err(e) => {
                self.msg_state.error = Some(format!("Invalid address: {}", e));
                return;
            }
        };

        // Use safe_utils::MessageHasher
        let (raw_hash, message_hash) = if self.msg_state.is_hex {
            // Parse hex bytes and hash directly
            let hex_str = self.msg_state.message.trim().trim_start_matches("0x");
            let bytes = match hex::decode(hex_str) {
                Ok(b) => b,
                Err(e) => {
                    self.msg_state.error = Some(format!("Invalid hex: {}", e));
                    return;
                }
            };
            let msg_hasher = MessageHasher::new_from_bytes(alloy::primitives::keccak256(&bytes));
            (msg_hasher.raw_hash(), msg_hasher.hash())
        } else {
            // Hash as UTF-8 string
            let msg_hasher = MessageHasher::new(self.msg_state.message.clone());
            (msg_hasher.raw_hash(), msg_hasher.hash())
        };

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
        self.tx_state.fetched_txs.clear();
        self.tx_state.selected_tx_index = None;
        self.tx_state.decode = None;
        self.tx_state.warnings_error = None;
        self.tx_state.show_full_data = false;

        let chain_name = self.safe_context.chain_name.clone();
        let safe_address = self.safe_context.safe_address.clone();
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
                let fetch_result = fetch_transactions(&chain_name, &safe_address, nonce).await;
                let mut result_guard = lock_or_recover!(result);
                *result_guard = Some(match fetch_result {
                    Ok(txs) => FetchResult::Success(txs),
                    Err(e) => FetchResult::Error(format!("{:#}", e)),
                });
                ctx.request_repaint();
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let fetch_result =
                    rt.block_on(fetch_transactions(&chain_name, &safe_address, nonce));
                let mut result_guard = lock_or_recover!(result);
                *result_guard = Some(match fetch_result {
                    Ok(txs) => FetchResult::Success(txs),
                    Err(e) => FetchResult::Error(format!("{:#}", e)),
                });
                ctx.request_repaint();
            });
        }
    }

    fn check_fetch_result(&mut self, ctx: &egui::Context) {
        let result = {
            let mut guard = lock_or_recover!(self.fetch_result);
            guard.take()
        };

        if let Some(result) = result {
            self.tx_state.is_loading = false;

            match result {
                FetchResult::Success(txs) => {
                    if txs.is_empty() {
                        self.tx_state.error =
                            Some("No transaction found for the specified nonce".to_string());
                        return;
                    }

                    let mut sorted = txs;
                    sorted.sort_by(|a, b| b.submission_date.cmp(&a.submission_date));
                    self.tx_state.fetched_txs = sorted;
                    let selected_index = self
                        .tx_state
                        .selected_tx_index
                        .unwrap_or(0)
                        .min(self.tx_state.fetched_txs.len() - 1);
                    self.tx_state.selected_tx_index = Some(selected_index);

                    if let Some(tx) = self.tx_state.fetched_txs.get(selected_index).cloned() {
                        self.apply_fetched_tx(ctx, tx);
                    }
                }
                FetchResult::Error(e) => {
                    self.tx_state.error = Some(e);
                }
            }
        }
    }

    fn apply_fetched_tx(&mut self, ctx: &egui::Context, tx: SafeTransaction) {
        self.tx_state.error = None;
        self.tx_state.hashes = None;
        self.tx_state.warnings = SafeWarnings::new();
        self.tx_state.warnings_error = None;
        self.tx_state.decode = None;
        self.tx_state.show_full_data = false;
        self.tx_state.expected.clear_result();

        // Compute hashes from the fetched transaction using validate_safe_tx_hash
        match compute_hashes_from_api_tx(
            &self.safe_context.chain_name,
            &self.safe_context.safe_address,
            &self.safe_context.safe_version,
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
        let chain_id = ChainId::of(&self.safe_context.chain_name).ok();
        match get_warnings_from_api_tx(&tx, chain_id) {
            Ok(warnings) => self.tx_state.warnings.union(warnings),
            Err(e) => {
                debug_log!("Warning computation failed: {:#}", e);
                self.tx_state.warnings_error = Some(format!("{:#}", e));
            }
        }

        // Validate against expected values if any were provided
        if self.tx_state.expected.has_values() {
            self.tx_state.expected.result =
                Some(expected::validate_against_api(&tx, &self.tx_state.expected));
        }

        // Initialize calldata decode
        debug_log!("Parsing calldata: {} bytes", tx.data.len());
        let decode_state = decode::parse_initial(&tx.data, tx.data_decoded.as_ref());
        debug_log!(
            "Decode kind: {:?}, selector: {}",
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
                    self.trigger_decode_lookup(ctx, &selector, &data);
                }
                "multi" => {
                    debug_log!("Triggering bulk verification for {} transactions", tx_count);
                    // Update verification state
                    if let Some(ref mut decode) = self.tx_state.decode {
                        if let TransactionKind::MultiSend(ref mut multi) = decode.kind {
                            multi.verification_state =
                                decode::VerificationState::InProgress { total: tx_count };
                        }
                    }
                    self.trigger_multisend_bulk_verify(ctx);
                }
                _ => {}
            }
        }

        self.tx_state.fetched_tx = Some(tx);
    }

    fn check_decode_result(&mut self) {
        let result = {
            let mut guard = lock_or_recover!(self.decode_result);
            guard.take()
        };

        if let Some(result) = result {
            debug_log!("Received decode result");
            match result {
                DecodeResult::Single {
                    selector: _,
                    local_decode,
                } => {
                    debug_log!(
                        "Processing single decode result: {:?}",
                        local_decode.as_ref().map(|d| &d.method).ok()
                    );
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
                                | ComparisonResult::ParamMismatch(_) => {
                                    decode::OverallStatus::HasMismatches
                                }
                                _ => decode::OverallStatus::PartiallyVerified,
                            };
                        }
                    }
                }
                DecodeResult::MultiSendBulk {
                    multi: verified_multi,
                } => {
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

    fn trigger_decode_lookup(&self, ctx: &egui::Context, selector: &str, data: &str) {
        let lookup = self.signature_lookup.clone();
        let selector = selector.to_string();
        let data = data.to_string();
        let result = Arc::clone(&self.decode_result);
        let ctx = ctx.clone();

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen_futures::spawn_local;
            spawn_local(async move {
                let local_decode = Self::do_decode_lookup(&lookup, &selector, &data).await;
                let mut guard = lock_or_recover!(result);
                *guard = Some(DecodeResult::Single {
                    selector,
                    local_decode,
                });
                ctx.request_repaint();
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let local_decode = rt.block_on(Self::do_decode_lookup(&lookup, &selector, &data));
                let mut guard = lock_or_recover!(result);
                *guard = Some(DecodeResult::Single {
                    selector,
                    local_decode,
                });
                ctx.request_repaint();
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
                let mut guard = lock_or_recover!(result);
                *guard = Some(DecodeResult::MultiSendBulk { multi });
                ctx.request_repaint();
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(decode::verify_multisend_batch(&mut multi, &lookup));
                let mut guard = lock_or_recover!(result);
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
        let signatures = lookup
            .lookup(selector)
            .await
            .map_err(|e| format!("{:#}", e))?;

        if signatures.is_empty() {
            return Err("No signatures found for selector".into());
        }

        // Try each signature until one decodes successfully
        // Signatures are sorted with verified first, so we prefer verified decodes
        for sig_info in &signatures {
            match decode::decode_with_signature(data, &sig_info.signature, sig_info.verified) {
                Ok(decoded) => return Ok(decoded),
                Err(_) => continue,
            }
        }

        Err(format!(
            "None of {} signatures decoded successfully",
            signatures.len()
        ))
    }

    /// Check for safe info result and schedule auto-fetch if successful
    fn check_safe_info_result(&mut self) {
        let result = {
            let mut guard = lock_or_recover!(self.safe_info_result);
            guard.take()
        };

        if let Some(result) = result {
            self.safe_info_loading = false;
            match result {
                SafeInfoResult::Success(info) => {
                    debug_log!(
                        "Fetched Safe info: version={}, nonce={}, threshold={}/{}",
                        info.version,
                        info.nonce,
                        info.threshold,
                        info.owners.len()
                    );

                    // Auto-fill version if it matches a supported version
                    let version_str = info.version.as_str();
                    if crate::state::SAFE_VERSIONS.contains(&version_str) {
                        self.safe_context.safe_version = version_str.to_string();
                    }

                    // Add to recent addresses
                    crate::state::add_recent_address(
                        &mut self.safe_context.recent_addresses,
                        &self.safe_context.safe_address,
                    );

                    // If we have a pre-fetched pending transaction, use it directly
                    // instead of making another API call
                    if let Some(pending_tx) = info.pending_transaction.clone() {
                        // Set nonce from the pending transaction
                        self.tx_state.nonce = pending_tx.nonce.to_string();

                        // Clear previous state
                        self.tx_state.is_loading = true;
                        self.tx_state.error = None;
                        self.tx_state.warnings = SafeWarnings::new();
                        self.tx_state.hashes = None;
                        self.tx_state.fetched_tx = None;
                        self.tx_state.fetched_txs.clear();
                        self.tx_state.selected_tx_index = None;
                        self.tx_state.decode = None;
                        self.tx_state.warnings_error = None;
                        self.tx_state.show_full_data = false;

                        // Populate fetch_result with the pre-fetched transaction
                        {
                            let mut guard = lock_or_recover!(self.fetch_result);
                            *guard = Some(FetchResult::Success(vec![pending_tx]));
                        }
                    } else {
                        // No pending transaction, set nonce to latest - 1 for manual fetch
                        let latest_nonce = if info.nonce > 0 {
                            info.nonce - 1
                        } else {
                            0
                        };
                        self.tx_state.nonce = latest_nonce.to_string();
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
        let chain_name = self.safe_context.chain_name.clone();
        let safe_address = self.safe_context.safe_address.clone();
        let result = Arc::clone(&self.safe_info_result);

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen_futures::spawn_local;
            spawn_local(async move {
                let fetch_result = crate::hasher::fetch_safe_info(&chain_name, &safe_address).await;
                let mut guard = lock_or_recover!(result);
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
                let fetch_result =
                    rt.block_on(crate::hasher::fetch_safe_info(&chain_name, &safe_address));
                let mut guard = lock_or_recover!(result);
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

        // Transaction inputs
        ui::section_header(ui, "Transaction Details");
        ui.add_space(5.0);

        egui::Grid::new("offline_tx_inputs")
            .num_columns(2)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label("To:");
                ui::address_input(ui, &mut self.offline_state.to);
                ui.end_row();

                ui.label("Value (wei):");
                ui::number_input(ui, &mut self.offline_state.value, "0");
                ui.end_row();

                ui.label("Data (hex):");
                ui::multiline_input(ui, &mut self.offline_state.data, "0x...", 10);
                ui.end_row();

                ui.label("Operation:");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.offline_state.operation, 0, "Call (0)");
                    ui.selectable_value(&mut self.offline_state.operation, 1, "DelegateCall (1)");
                });
                ui.end_row();

                ui.label("Nonce:");
                ui::number_input(ui, &mut self.offline_state.nonce, "0");
                ui.end_row();
            });

        ui.add_space(10.0);

        // Advanced: Gas Parameters (collapsed by default)
        egui::CollapsingHeader::new("⚙️ Advanced: Gas Parameters")
            .default_open(false)
            .show(ui, |ui| {
                ui.add_space(5.0);

                egui::Grid::new("offline_gas_inputs")
                    .num_columns(2)
                    .spacing([10.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("SafeTxGas:");
                        ui::number_input(ui, &mut self.offline_state.safe_tx_gas, "0");
                        ui.end_row();

                        ui.label("BaseGas:");
                        ui::number_input(ui, &mut self.offline_state.base_gas, "0");
                        ui.end_row();

                        ui.label("GasPrice:");
                        ui::number_input(ui, &mut self.offline_state.gas_price, "0");
                        ui.end_row();

                        ui.label("GasToken:");
                        ui::address_input(ui, &mut self.offline_state.gas_token);
                        ui.end_row();

                        ui.label("RefundReceiver:");
                        ui::address_input(ui, &mut self.offline_state.refund_receiver);
                        ui.end_row();
                    });

                ui.add_space(5.0);
                ui.label(
                    egui::RichText::new("Most transactions use default values (all zeros)").weak(),
                );
            });

        ui.add_space(15.0);

        // Compute button
        let can_compute = !self.safe_context.safe_address.is_empty()
            && !self.offline_state.to.is_empty()
            && !self.offline_state.is_loading;

        ui.horizontal(|ui| {
            if ui::primary_button_enabled(ui, "🔐 Compute Hash & Decode", can_compute).clicked() {
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
            ui.label(
                egui::RichText::new(format!("❌ {}", error))
                    .color(egui::Color32::from_rgb(220, 80, 80)),
            );
        }

        // Results
        if self.offline_state.hashes.is_some() || self.offline_state.decode_result.is_some() {
            ui.add_space(20.0);
            ui.separator();
            ui.add_space(10.0);

            // Warnings
            let warnings_error = self.offline_state.warnings_error.as_deref();
            if self.offline_state.warnings.has_warnings() || warnings_error.is_some() {
                ui::section_header(ui, "⚠️ Warnings");

                if let Some(error) = warnings_error {
                    ui::error_message(ui, &format!("Warning computation failed: {}", error));
                }

                let w = &self.offline_state.warnings;
                if w.delegatecall {
                    ui::error_banner(ui, "DELEGATECALL - can modify Safe state!");
                }
                if w.non_zero_gas_token {
                    ui::warning_banner(ui, "Non-zero gas token");
                }
                if w.non_zero_refund_receiver {
                    ui::warning_banner(ui, "Non-zero refund receiver");
                }

                ui.add_space(10.0);
            }

            // Calldata Decoding (before hashes, like Verify Safe API tab)
            if let Some(ref mut decode) = self.offline_state.decode_result {
                ui::section_header(ui, "Calldata Decoding");
                decode::render_offline_decode_section(ui, decode, &self.safe_context);
                ui.add_space(10.0);
            }

            // Hashes
            if let Some(ref hashes) = self.offline_state.hashes {
                ui::section_header(ui, "Hash Results");

                egui::Grid::new("offline_hash_results")
                    .num_columns(3)
                    .spacing([10.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Domain Hash:").strong());
                        ui.label(
                            egui::RichText::new(&hashes.domain_hash)
                                .monospace()
                                .size(12.0),
                        );
                        if ui.small_button("📋").on_hover_text("Copy").clicked() {
                            ui::copy_to_clipboard(&hashes.domain_hash);
                        }
                        ui.end_row();

                        ui.label(egui::RichText::new("Message Hash:").strong());
                        ui.label(
                            egui::RichText::new(&hashes.message_hash)
                                .monospace()
                                .size(12.0),
                        );
                        if ui.small_button("📋").on_hover_text("Copy").clicked() {
                            ui::copy_to_clipboard(&hashes.message_hash);
                        }
                        ui.end_row();

                        ui.label(egui::RichText::new("Safe Tx Hash:").strong());
                        ui.label(
                            egui::RichText::new(&hashes.safe_tx_hash)
                                .monospace()
                                .size(12.0),
                        );
                        if ui.small_button("📋").on_hover_text("Copy").clicked() {
                            ui::copy_to_clipboard(&hashes.safe_tx_hash);
                        }
                        ui.end_row();

                        // Ledger binary format
                        let binary_literal = ui::hash_to_binary_literal(&hashes.safe_tx_hash);
                        ui.label(egui::RichText::new("Ledger Binary:").strong());
                        ui.label(egui::RichText::new(&binary_literal).monospace().size(12.0));
                        if ui.small_button("📋").on_hover_text("Copy").clicked() {
                            ui::copy_to_clipboard(&binary_literal);
                        }
                        ui.end_row();
                    });
            }
        }
    }

    fn render_address_book_window(&mut self, ctx: &egui::Context) {
        let mut open = self.address_book_open;
        let is_empty = self.safe_context.address_book.entries.is_empty();

        egui::Window::new("📖 Address Book")
            .open(&mut open)
            .resizable(true)
            .default_width(580.0)
            .min_width(500.0)
            .show(ctx, |ui| {
                // Search Bar (only show if there are entries)
                if !is_empty {
                    ui.horizontal(|ui| {
                        ui.label("🔍");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.address_book_search)
                                .hint_text("Search by name or address...")
                                .desired_width(f32::INFINITY),
                        );
                    });
                    ui.add_space(8.0);
                }

                // Entries Table
                ui.label(egui::RichText::new("Entries").strong());
                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        let search_lower = self.address_book_search.to_lowercase();
                        let filtered_entries: Vec<_> = self
                            .safe_context
                            .address_book
                            .entries
                            .iter()
                            .enumerate()
                            .filter(|(_, e)| {
                                e.name.to_lowercase().contains(&search_lower)
                                    || e.address.to_lowercase().contains(&search_lower)
                            })
                            .collect();

                        let filtered_is_empty = filtered_entries.is_empty();
                        let mut to_remove = None;

                        let available_width = ui.available_width();
                        egui::Grid::new("address_book_entries_v2")
                            .num_columns(4)
                            .spacing([10.0, 12.0])
                            .striped(true)
                            .min_col_width(available_width * 0.2)
                            .show(ui, |ui| {
                                // Header
                                ui.label(egui::RichText::new("NAME").strong().small());
                                ui.label(egui::RichText::new("ADDRESS").strong().small());
                                ui.label(egui::RichText::new("CHAIN").strong().small());
                                ui.label(""); // Actions
                                ui.end_row();

                                for (original_idx, entry) in &filtered_entries {
                                    let validation =
                                        self.safe_context.address_book.validate_entry(entry);

                                    let name_color =
                                        if validation == AddressValidation::ChecksumMismatch {
                                            egui::Color32::from_rgb(220, 180, 50)
                                        } else if validation == AddressValidation::Invalid {
                                            egui::Color32::from_rgb(220, 80, 80)
                                        } else {
                                            ui.visuals().text_color()
                                        };

                                    // Name Column
                                    ui.horizontal(|ui| {
                                        if validation == AddressValidation::ChecksumMismatch {
                                            ui.label(
                                                egui::RichText::new("⚠️")
                                                    .color(egui::Color32::from_rgb(220, 180, 50)),
                                            )
                                            .on_hover_text(
                                                "Checksum mismatch - address was normalized",
                                            );
                                        } else if validation == AddressValidation::Invalid {
                                            ui.label(
                                                egui::RichText::new("❌")
                                                    .color(egui::Color32::from_rgb(220, 80, 80)),
                                            )
                                            .on_hover_text("Invalid address");
                                        }
                                        ui.label(
                                            egui::RichText::new(&entry.name)
                                                .strong()
                                                .color(name_color),
                                        );
                                    });

                                    // Address Column
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(&entry.address)
                                                .monospace()
                                                .color(name_color),
                                        );
                                        if ui
                                            .small_button("📋")
                                            .on_hover_text("Copy address")
                                            .clicked()
                                        {
                                            ui::copy_to_clipboard(&entry.address);
                                        }
                                    });

                                    // Chain Column
                                    let chain_name = get_chain_name(entry.chain_id);
                                    ui.label(egui::RichText::new(chain_name).weak());

                                    // Actions Column
                                    if ui.button("🗑").on_hover_text("Remove").clicked() {
                                        to_remove = Some(*original_idx);
                                    }
                                    ui.end_row();
                                }
                            });

                        // Drop filtered_entries borrow before mutable operation
                        drop(filtered_entries);

                        if let Some(idx) = to_remove {
                            self.safe_context.address_book.entries.remove(idx);
                        }

                        if self.safe_context.address_book.entries.is_empty() {
                            // Better empty state
                            ui.vertical_centered(|ui| {
                                ui.add_space(20.0);
                                ui.label(egui::RichText::new("📭").size(32.0));
                                ui.add_space(8.0);
                                ui.label(
                                    egui::RichText::new("No addresses saved yet")
                                        .size(14.0),
                                );
                                ui.label(
                                    egui::RichText::new("Add entries below or import from CSV")
                                        .small()
                                        .weak(),
                                );
                                ui.add_space(20.0);
                            });
                        } else if filtered_is_empty {
                            ui.label(egui::RichText::new("No matches found").weak().italics());
                        }
                    });

                ui.add_space(12.0);

                // Add Entry Section - Always visible, prominent when empty
                let add_header = egui::CollapsingHeader::new(
                    egui::RichText::new("➕ Add New Entry").strong()
                )
                .default_open(is_empty); // Auto-expand when address book is empty

                add_header.show(ui, |ui| {
                    ui::card(ui, |ui| {
                        egui::Grid::new("add_entry_grid")
                            .num_columns(2)
                            .spacing([10.0, 10.0])
                            .show(ui, |ui| {
                                ui.label("Name:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.address_book_add_name)
                                        .hint_text("e.g., My Wallet")
                                        .desired_width(280.0),
                                );
                                ui.end_row();

                                ui.label("Address:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.address_book_add_addr)
                                        .hint_text("0x...")
                                        .desired_width(280.0)
                                        .font(egui::TextStyle::Monospace),
                                );
                                ui.end_row();

                                ui.label("Chain:");
                                egui::ComboBox::from_id_salt("add_entry_chain")
                                    .width(280.0)
                                    .selected_text(&self.address_book_add_chain)
                                    .show_ui(ui, |ui| {
                                        for name in safe_utils::get_all_supported_chain_names() {
                                            ui.selectable_value(
                                                &mut self.address_book_add_chain,
                                                name.clone(),
                                                name,
                                            );
                                        }
                                    });
                                ui.end_row();
                            });

                        ui.add_space(8.0);

                        let can_add = !self.address_book_add_name.is_empty()
                            && !self.address_book_add_addr.is_empty();
                        if ui::primary_button_enabled(ui, "➕ Add Entry", can_add).clicked() {
                            if let Ok(chain_id) =
                                alloy::primitives::ChainId::of(&self.address_book_add_chain)
                            {
                                self.safe_context.address_book.add_or_update(
                                    crate::state::AddressBookEntry {
                                        address: self.address_book_add_addr.clone(),
                                        name: self.address_book_add_name.clone(),
                                        chain_id: u64::from(chain_id),
                                    },
                                );
                                self.address_book_add_addr.clear();
                                self.address_book_add_name.clear();
                                self.address_book_error = Some("✓ Entry added".to_string());
                            }
                        }
                    });
                });

                ui.add_space(4.0);

                egui::CollapsingHeader::new("📥 Import CSV").show(ui, |ui| {
                    ui::card(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Paste CSV data: address,name,chainId")
                                .small(),
                        );
                        ui.add_space(4.0);
                        ui::multiline_input(
                            ui,
                            &mut self.address_book_import_text,
                            "0xAddress...,Name,1",
                            4,
                        );
                        ui.add_space(6.0);

                        let can_import = !self.address_book_import_text.trim().is_empty();
                        if ui::primary_button_enabled(ui, "📥 Import", can_import).clicked() {
                            match self
                                .safe_context
                                .address_book
                                .import_csv(&self.address_book_import_text)
                            {
                                Ok((count, skipped)) => {
                                    if skipped > 0 {
                                        self.address_book_error = Some(format!(
                                            "✓ Imported {} entries, skipped {} invalid",
                                            count, skipped
                                        ));
                                    } else {
                                        self.address_book_error =
                                            Some(format!("✓ Successfully imported {} entries", count));
                                    }
                                    self.address_book_import_text.clear();
                                }
                                Err(e) => {
                                    self.address_book_error = Some(format!("❌ Import failed: {}", e));
                                }
                            }
                        }
                    });
                });

                ui.add_space(4.0);

                egui::CollapsingHeader::new("📤 Export").show(ui, |ui| {
                    ui::card(ui, |ui| {
                        let entry_count = self.safe_context.address_book.entries.len();
                        ui.label(
                            egui::RichText::new(format!("{} entries to export", entry_count))
                                .small(),
                        );
                        ui.add_space(6.0);

                        let can_export = entry_count > 0;
                        if ui::secondary_button(ui, "📋 Copy CSV to Clipboard").clicked() && can_export
                        {
                            let csv = self.safe_context.address_book.export_csv();
                            ui::copy_to_clipboard(&csv);
                            self.address_book_error = Some("✓ CSV copied to clipboard".to_string());
                        }
                    });
                });

                if let Some(ref msg) = self.address_book_error {
                    ui.add_space(5.0);
                    ui.label(
                        egui::RichText::new(msg)
                            .small()
                            .color(egui::Color32::from_rgb(100, 200, 100)),
                    );
                }
            });
        self.address_book_open = open;
    }

    fn check_offline_decode_result(&mut self) {
        let result = {
            let mut guard = lock_or_recover!(self.offline_decode_result);
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
            &self.safe_context.chain_name,
            &self.safe_context.safe_address,
            &self.safe_context.safe_version,
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
                // Compute warnings
                match get_warnings_for_tx(
                    &self.offline_state.to,
                    &self.offline_state.value,
                    &self.offline_state.data,
                    self.offline_state.operation,
                    &self.offline_state.safe_tx_gas,
                    &self.offline_state.base_gas,
                    &self.offline_state.gas_price,
                    &self.offline_state.gas_token,
                    &self.offline_state.refund_receiver,
                ) {
                    Ok(warnings) => {
                        self.offline_state.warnings = warnings;
                        self.offline_state.warnings_error = None;
                    }
                    Err(e) => {
                        debug_log!("Warning computation failed: {:#}", e);
                        self.offline_state.warnings_error = Some(format!("{:#}", e));
                    }
                }
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
                let mut guard = lock_or_recover!(result);
                *guard = Some(OfflineDecodeResult::Success(decode));
                ctx.request_repaint();
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let decode = rt.block_on(decode::decode_offline(&data, &lookup));
                let mut guard = lock_or_recover!(result);
                *guard = Some(OfflineDecodeResult::Success(decode));
                ctx.request_repaint();
            });
        }
    }
}
