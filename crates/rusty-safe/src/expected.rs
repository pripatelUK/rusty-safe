//! Expected values validation
//!
//! Allows users to verify that the API-returned transaction matches
//! their expectations (to, value, data, operation).

use alloy::primitives::{Address, U256};
use eframe::egui;
use safe_hash::{Mismatch, SafeTransaction};

//─────────────────────────────────────────────────────────────────────────────
// STATE
//─────────────────────────────────────────────────────────────────────────────

/// State for expected values validation
#[derive(Debug, Clone, Default)]
pub struct ExpectedState {
    /// Expected "to" address
    pub to: String,
    /// Expected value in wei
    pub value: String,
    /// Expected calldata
    pub data: String,
    /// Expected operation: None = any, Some(0) = Call, Some(1) = DelegateCall
    pub operation: Option<u8>,
    /// Validation result after fetch
    pub result: Option<ValidationResult>,
}

/// Result of validating expected values against API response
#[derive(Debug, Clone)]
pub enum ValidationResult {
    /// All provided expected values match the API response
    Match,
    /// One or more expected values don't match
    Mismatches(Vec<Mismatch>),
    /// One or more expected values couldn't be parsed (validation incomplete)
    ParseErrors(Vec<String>),
}

impl ExpectedState {
    /// Check if any expected values have been provided
    pub fn has_values(&self) -> bool {
        !self.to.is_empty()
            || !self.value.is_empty()
            || !self.data.is_empty()
            || self.operation.is_some()
    }

    /// Clear validation result (keeps input values)
    pub fn clear_result(&mut self) {
        self.result = None;
    }

    /// Clear all state
    pub fn clear_all(&mut self) {
        self.to.clear();
        self.value.clear();
        self.data.clear();
        self.operation = None;
        self.result = None;
    }
}

//─────────────────────────────────────────────────────────────────────────────
// UI
//─────────────────────────────────────────────────────────────────────────────

/// Render the expected values collapsible section
pub fn render_section(ui: &mut egui::Ui, state: &mut ExpectedState) {
    egui::CollapsingHeader::new("🔍 Verify Expected Values")
        .default_open(state.has_values())
        .show(ui, |ui| {
            ui.add_space(5.0);
            ui.label(
                egui::RichText::new("Enter expected values to verify against API response:").small(),
            );
            ui.add_space(8.0);

            egui::Grid::new("expected_inputs")
                .num_columns(2)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    // Expected To
                    ui.label("Expected To:");
                    ui.add(
                        egui::TextEdit::singleline(&mut state.to)
                            .hint_text("0x...")
                            .desired_width(380.0)
                            .font(egui::TextStyle::Monospace),
                    );
                    ui.end_row();

                    // Expected Value
                    ui.label("Expected Value:");
                    ui.add(
                        egui::TextEdit::singleline(&mut state.value)
                            .hint_text("0 (wei)")
                            .desired_width(200.0),
                    );
                    ui.end_row();

                    // Expected Data
                    ui.label("Expected Data:");
                    ui.add(
                        egui::TextEdit::multiline(&mut state.data)
                            .hint_text("0x...")
                            .desired_width(380.0)
                            .desired_rows(2)
                            .font(egui::TextStyle::Monospace),
                    );
                    ui.end_row();

                    // Expected Operation
                    ui.label("Operation:");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut state.operation, None, "Any");
                        ui.selectable_value(&mut state.operation, Some(0), "Call");
                        ui.selectable_value(&mut state.operation, Some(1), "DelegateCall");
                    });
                    ui.end_row();
                });
        });
}

/// Render validation result (match/mismatches/parse errors)
pub fn render_result(ui: &mut egui::Ui, state: &ExpectedState) {
    if let Some(result) = &state.result {
        ui.add_space(10.0);

        match result {
            ValidationResult::Match => {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("✅ Expected values match API response")
                            .color(egui::Color32::from_rgb(100, 200, 100))
                            .strong(),
                    );
                });
            }
            ValidationResult::Mismatches(mismatches) => {
                for m in mismatches {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "❌ Mismatch in '{}': Expected {}, API has {}",
                                m.field, m.user_value, m.api_value
                            ))
                            .color(egui::Color32::from_rgb(220, 80, 80)),
                        );
                    });
                    ui.add_space(2.0);
                }
            }
            ValidationResult::ParseErrors(errors) => {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("⚠️ Validation incomplete - invalid input:")
                            .color(egui::Color32::from_rgb(220, 180, 50))
                            .strong(),
                    );
                });
                for err in errors {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("  • {}", err))
                                .color(egui::Color32::from_rgb(220, 180, 50)),
                        );
                    });
                    ui.add_space(2.0);
                }
            }
        }
    }
}

//─────────────────────────────────────────────────────────────────────────────
// VALIDATION LOGIC
//─────────────────────────────────────────────────────────────────────────────

/// Validate API transaction against expected values
pub fn validate_against_api(api_tx: &SafeTransaction, state: &ExpectedState) -> ValidationResult {
    if !state.has_values() {
        return ValidationResult::Match;
    }

    let mut mismatches = Vec::new();
    let mut parse_errors = Vec::new();

    // Check 'to' address
    if !state.to.is_empty() {
        match state.to.trim().parse::<Address>() {
            Ok(expected_to) => {
                if expected_to != api_tx.to {
                    mismatches.push(Mismatch {
                        field: "to".to_string(),
                        api_value: api_tx.to.to_string(),
                        user_value: expected_to.to_string(),
                    });
                }
            }
            Err(_) => {
                parse_errors.push(format!("Invalid 'to' address: '{}'", state.to.trim()));
            }
        }
    }

    // Check value
    if !state.value.is_empty() {
        match parse_u256(&state.value) {
            Ok(expected_value) => match U256::from_str_radix(&api_tx.value, 10) {
                Ok(api_value) => {
                    if expected_value != api_value {
                        mismatches.push(Mismatch {
                            field: "value".to_string(),
                            api_value: api_value.to_string(),
                            user_value: expected_value.to_string(),
                        });
                    }
                }
                Err(_) => {
                    parse_errors.push(format!("API returned invalid value: '{}'", api_tx.value));
                }
            },
            Err(_) => {
                parse_errors.push(format!("Invalid expected value: '{}'", state.value.trim()));
            }
        }
    }

    // Check data
    if !state.data.is_empty() && state.data.trim() != "0x" {
        let expected_data = normalize_hex(&state.data);
        let api_data = normalize_hex(&api_tx.data);
        if expected_data != api_data {
            mismatches.push(Mismatch {
                field: "data".to_string(),
                api_value: api_tx.data.clone(),
                user_value: state.data.clone(),
            });
        }
    }

    // Check operation
    if let Some(expected_op) = state.operation {
        if expected_op != api_tx.operation {
            mismatches.push(Mismatch {
                field: "operation".to_string(),
                api_value: op_to_string(api_tx.operation),
                user_value: op_to_string(expected_op),
            });
        }
    }

    // Parse errors take precedence - validation is incomplete
    if !parse_errors.is_empty() {
        ValidationResult::ParseErrors(parse_errors)
    } else if mismatches.is_empty() {
        ValidationResult::Match
    } else {
        ValidationResult::Mismatches(mismatches)
    }
}

//─────────────────────────────────────────────────────────────────────────────
// HELPERS
//─────────────────────────────────────────────────────────────────────────────

fn parse_u256(value: &str) -> Result<U256, ()> {
    let value = value.trim();
    if value.is_empty() || value == "0" {
        return Ok(U256::ZERO);
    }
    if value.starts_with("0x") || value.starts_with("0X") {
        U256::from_str_radix(&value[2..], 16).map_err(|_| ())
    } else {
        value.parse().map_err(|_| ())
    }
}

fn normalize_hex(s: &str) -> String {
    let s = s.trim().to_lowercase();
    if s.starts_with("0x") {
        s
    } else {
        format!("0x{}", s)
    }
}

fn op_to_string(op: u8) -> String {
    match op {
        0 => "Call".to_string(),
        1 => "DelegateCall".to_string(),
        _ => format!("Unknown({})", op),
    }
}
