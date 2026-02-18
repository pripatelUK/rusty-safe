use std::str::FromStr;

use alloy::dyn_abi::{DynSolType, DynSolValue, JsonAbiExt};
use alloy::json_abi::{Function, JsonAbi};
use alloy::primitives::{keccak256, Address, Bytes, FixedBytes, I256, U256};
use serde_json::Value;

use rusty_safe_signing_core::{AbiPort, PortError};

#[derive(Debug, Clone, Default)]
pub struct AbiAdapter;

impl AbiPort for AbiAdapter {
    fn encode_calldata(
        &self,
        abi_json: &str,
        method_signature: &str,
        args: &[String],
    ) -> Result<(Bytes, [u8; 4]), PortError> {
        let abi: JsonAbi = serde_json::from_str(abi_json)
            .map_err(|e| PortError::Validation(format!("invalid abi json: {e}")))?;
        let function = select_function(&abi, method_signature)?;
        if function.inputs.len() != args.len() {
            return Err(PortError::Validation(format!(
                "argument count mismatch: expected {}, got {}",
                function.inputs.len(),
                args.len()
            )));
        }

        let mut dyn_args = Vec::with_capacity(args.len());
        for (input, arg) in function.inputs.iter().zip(args.iter()) {
            let ty: DynSolType = input.ty.parse().map_err(|e| {
                PortError::Validation(format!("unsupported type '{}': {e}", input.ty))
            })?;
            let parsed = serde_json::from_str::<Value>(arg).unwrap_or(Value::String(arg.clone()));
            let value = parse_dyn_value(&parsed, &ty).map_err(|e| {
                PortError::Validation(format!("arg '{}' parse failed: {e}", input.name))
            })?;
            dyn_args.push(value);
        }

        let encoded = function
            .abi_encode_input(&dyn_args)
            .map_err(|e| PortError::Validation(format!("abi encoding failed: {e}")))?;
        let selector = self.selector_from_method_signature(method_signature)?;
        if encoded.len() < 4 || encoded[0..4] != selector {
            return Err(PortError::Validation("ABI_SELECTOR_MISMATCH".to_owned()));
        }
        Ok((Bytes::from(encoded), selector))
    }

    fn selector_from_method_signature(&self, method_signature: &str) -> Result<[u8; 4], PortError> {
        let hash = keccak256(method_signature.as_bytes());
        let mut selector = [0u8; 4];
        selector.copy_from_slice(&hash.as_slice()[0..4]);
        Ok(selector)
    }
}

fn select_function<'a>(
    abi: &'a JsonAbi,
    method_signature: &str,
) -> Result<&'a Function, PortError> {
    let (method_name, full_sig_opt) = if method_signature.contains('(') {
        (
            method_signature
                .split_once('(')
                .map(|(name, _)| name)
                .unwrap_or(method_signature),
            Some(method_signature),
        )
    } else {
        (method_signature, None)
    };

    let candidates = abi
        .function(method_name)
        .ok_or_else(|| PortError::Validation(format!("method not found: {method_name}")))?;

    if let Some(full_sig) = full_sig_opt {
        if let Some(function) = candidates
            .iter()
            .find(|f| function_signature(f) == full_sig)
        {
            return Ok(function);
        }
        return Err(PortError::Validation(format!(
            "method signature not found: {full_sig}"
        )));
    }

    candidates
        .first()
        .ok_or_else(|| PortError::Validation(format!("method has no overloads: {method_name}")))
}

fn function_signature(function: &Function) -> String {
    let mut out = String::new();
    out.push_str(&function.name);
    out.push('(');
    for (idx, input) in function.inputs.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push_str(&input.ty);
    }
    out.push(')');
    out
}

fn parse_dyn_value(value: &Value, ty: &DynSolType) -> Result<DynSolValue, String> {
    match ty {
        DynSolType::Bool => value
            .as_bool()
            .map(DynSolValue::Bool)
            .ok_or_else(|| "expected bool".to_owned()),
        DynSolType::Uint(bits) => match value {
            Value::String(s) => U256::from_str(s)
                .or_else(|_| U256::from_str_radix(s.trim_start_matches("0x"), 16))
                .map(|x| DynSolValue::Uint(x, *bits))
                .map_err(|e| format!("invalid uint: {e}")),
            Value::Number(n) => U256::from_str(&n.to_string())
                .map(|x| DynSolValue::Uint(x, *bits))
                .map_err(|e| format!("invalid uint: {e}")),
            _ => Err("expected uint string/number".to_owned()),
        },
        DynSolType::Int(bits) => match value {
            Value::String(s) => I256::from_str(s)
                .map(|x| DynSolValue::Int(x, *bits))
                .map_err(|e| format!("invalid int: {e}")),
            Value::Number(n) => I256::from_str(&n.to_string())
                .map(|x| DynSolValue::Int(x, *bits))
                .map_err(|e| format!("invalid int: {e}")),
            _ => Err("expected int string/number".to_owned()),
        },
        DynSolType::Address => value
            .as_str()
            .ok_or_else(|| "expected address string".to_owned())
            .and_then(|s| {
                Address::from_str(s)
                    .map(DynSolValue::Address)
                    .map_err(|e| format!("invalid address: {e}"))
            }),
        DynSolType::FixedBytes(size) => value
            .as_str()
            .ok_or_else(|| "expected fixed bytes string".to_owned())
            .and_then(|s| {
                FixedBytes::from_str(s)
                    .map(|x| DynSolValue::FixedBytes(x, *size))
                    .map_err(|e| format!("invalid fixed bytes: {e}"))
            }),
        DynSolType::Bytes => value
            .as_str()
            .ok_or_else(|| "expected bytes string".to_owned())
            .and_then(|s| {
                Bytes::from_str(s)
                    .map(|x| DynSolValue::Bytes(x.into()))
                    .map_err(|e| format!("invalid bytes: {e}"))
            }),
        DynSolType::String => value
            .as_str()
            .map(|s| DynSolValue::String(s.to_owned()))
            .ok_or_else(|| "expected string".to_owned()),
        DynSolType::Array(inner) => {
            let arr = value
                .as_array()
                .ok_or_else(|| "expected array for dynamic array".to_owned())?;
            let mut out = Vec::with_capacity(arr.len());
            for val in arr {
                out.push(parse_dyn_value(val, inner)?);
            }
            Ok(DynSolValue::Array(out))
        }
        DynSolType::FixedArray(inner, size) => {
            let arr = value
                .as_array()
                .ok_or_else(|| "expected array for fixed array".to_owned())?;
            if arr.len() != *size {
                return Err(format!(
                    "fixed array length mismatch: expected {}, got {}",
                    size,
                    arr.len()
                ));
            }
            let mut out = Vec::with_capacity(arr.len());
            for val in arr {
                out.push(parse_dyn_value(val, inner)?);
            }
            Ok(DynSolValue::FixedArray(out))
        }
        DynSolType::Tuple(inner) => {
            let arr = value
                .as_array()
                .ok_or_else(|| "expected tuple array".to_owned())?;
            if arr.len() != inner.len() {
                return Err(format!(
                    "tuple length mismatch: expected {}, got {}",
                    inner.len(),
                    arr.len()
                ));
            }
            let mut out = Vec::with_capacity(arr.len());
            for (val, inner_ty) in arr.iter().zip(inner.iter()) {
                out.push(parse_dyn_value(val, inner_ty)?);
            }
            Ok(DynSolValue::Tuple(out))
        }
        _ => Err("type not supported in parity wave".to_owned()),
    }
}
