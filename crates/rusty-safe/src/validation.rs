//! Transaction validation utilities
//!
//! Replicates validation logic from safe-hash for use in GUI.

use safe_hash::{Mismatch, SafeTransaction};
use alloy::primitives::{Address, U256};

/// Expected transaction values for validation
#[derive(Debug, Clone, Default)]
pub struct ExpectedTxValues {
    pub to: Option<Address>,
    pub value: Option<U256>,
    pub data: Option<String>,
    pub operation: Option<u8>,
    pub safe_tx_gas: Option<U256>,
    pub base_gas: Option<U256>,
    pub gas_price: Option<U256>,
    pub gas_token: Option<Address>,
    pub refund_receiver: Option<Address>,
}

/// Validate transaction details against expected values
pub fn validate_tx_against_expected(
    api_tx: &SafeTransaction,
    expected: &ExpectedTxValues,
) -> Result<(), Vec<Mismatch>> {
    let mut errors = Vec::new();

    if let Some(to) = expected.to {
        if to != api_tx.to {
            errors.push(Mismatch {
                field: "to".to_string(),
                api_value: api_tx.to.to_string(),
                user_value: to.to_string(),
            });
        }
    }

    if let Some(value) = expected.value {
        if value != U256::ZERO {
            match U256::from_str_radix(&api_tx.value, 10) {
                Ok(api_value) => {
                    if value != api_value {
                        errors.push(Mismatch {
                            field: "value".to_string(),
                            api_value: api_value.to_string(),
                            user_value: value.to_string(),
                        });
                    }
                }
                Err(e) => {
                    errors.push(Mismatch {
                        field: "value".to_string(),
                        api_value: "".to_string(),
                        user_value: format!("Failed to parse API value: {}", e),
                    });
                }
            }
        }
    }

    if let Some(ref data) = expected.data {
        if data != "0x" && data != &api_tx.data {
            errors.push(Mismatch {
                field: "data".to_string(),
                api_value: api_tx.data.clone(),
                user_value: data.clone(),
            });
        }
    }

    if let Some(operation) = expected.operation {
        if operation != api_tx.operation {
            errors.push(Mismatch {
                field: "operation".to_string(),
                api_value: api_tx.operation.to_string(),
                user_value: operation.to_string(),
            });
        }
    }

    if let Some(gas_token) = expected.gas_token {
        if gas_token != Address::ZERO && gas_token != api_tx.gas_token {
            errors.push(Mismatch {
                field: "gas_token".to_string(),
                api_value: api_tx.gas_token.to_string(),
                user_value: gas_token.to_string(),
            });
        }
    }

    if let Some(refund_receiver) = expected.refund_receiver {
        if refund_receiver != Address::ZERO && refund_receiver != api_tx.refund_receiver {
            errors.push(Mismatch {
                field: "refund_receiver".to_string(),
                api_value: api_tx.refund_receiver.to_string(),
                user_value: refund_receiver.to_string(),
            });
        }
    }

    if let Some(safe_tx_gas) = expected.safe_tx_gas {
        if safe_tx_gas != U256::ZERO && safe_tx_gas != U256::from(api_tx.safe_tx_gas) {
            errors.push(Mismatch {
                field: "safe_tx_gas".to_string(),
                api_value: api_tx.safe_tx_gas.to_string(),
                user_value: safe_tx_gas.to_string(),
            });
        }
    }

    if let Some(base_gas) = expected.base_gas {
        if base_gas != U256::ZERO && base_gas != U256::from(api_tx.base_gas) {
            errors.push(Mismatch {
                field: "base_gas".to_string(),
                api_value: api_tx.base_gas.to_string(),
                user_value: base_gas.to_string(),
            });
        }
    }

    if let Some(gas_price) = expected.gas_price {
        if gas_price != U256::ZERO
            && gas_price != U256::from_str_radix(&api_tx.gas_price, 10).unwrap_or(U256::ZERO)
        {
            errors.push(Mismatch {
                field: "gas_price".to_string(),
                api_value: api_tx.gas_price.clone(),
                user_value: gas_price.to_string(),
            });
        }
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

