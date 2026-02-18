use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use alloy::primitives::{keccak256, Bytes, B256};
use serde_json::{json, Value};

use rusty_safe_signing_core::{PendingSafeTx, PortError, SafeServicePort};

#[derive(Debug, Clone, Default)]
pub struct SafeServiceAdapter {
    inner: Arc<Mutex<SafeServiceState>>,
}

#[derive(Debug, Default)]
struct SafeServiceState {
    txs: HashMap<B256, RemoteTxState>,
}

#[derive(Debug, Clone)]
struct RemoteTxState {
    chain_id: u64,
    safe_address: alloy::primitives::Address,
    proposed: bool,
    confirmations: Vec<Bytes>,
    executed_tx_hash: Option<B256>,
}

impl SafeServicePort for SafeServiceAdapter {
    fn propose_tx(&self, tx: &PendingSafeTx) -> Result<(), PortError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("safe service lock poisoned: {e}")))?;
        g.txs
            .entry(tx.safe_tx_hash)
            .and_modify(|state| state.proposed = true)
            .or_insert_with(|| RemoteTxState {
                chain_id: tx.chain_id,
                safe_address: tx.safe_address,
                proposed: true,
                confirmations: Vec::new(),
                executed_tx_hash: None,
            });
        Ok(())
    }

    fn confirm_tx(&self, safe_tx_hash: B256, signature: &Bytes) -> Result<(), PortError> {
        if signature.len() < 65 {
            return Err(PortError::Validation("INVALID_SIGNATURE_FORMAT".to_owned()));
        }
        let mut g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("safe service lock poisoned: {e}")))?;
        let state = g
            .txs
            .get_mut(&safe_tx_hash)
            .ok_or_else(|| PortError::NotFound(format!("remote tx not found: {safe_tx_hash}")))?;
        if !state.proposed {
            return Err(PortError::Conflict(
                "cannot confirm tx before propose".to_owned(),
            ));
        }
        if !state.confirmations.iter().any(|sig| sig == signature) {
            state.confirmations.push(signature.clone());
        }
        Ok(())
    }

    fn execute_tx(&self, tx: &PendingSafeTx) -> Result<B256, PortError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("safe service lock poisoned: {e}")))?;
        let state = g
            .txs
            .entry(tx.safe_tx_hash)
            .or_insert_with(|| RemoteTxState {
                chain_id: tx.chain_id,
                safe_address: tx.safe_address,
                proposed: true,
                confirmations: Vec::new(),
                executed_tx_hash: None,
            });
        if state.executed_tx_hash.is_none() {
            // Deterministic execution hash used for integration parity tests.
            let seed = format!("{}:{}:{}", tx.chain_id, tx.safe_address, tx.safe_tx_hash);
            state.executed_tx_hash = Some(keccak256(seed.as_bytes()));
        }
        Ok(state.executed_tx_hash.expect("set above"))
    }

    fn fetch_status(&self, safe_tx_hash: B256) -> Result<Value, PortError> {
        let g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("safe service lock poisoned: {e}")))?;
        let state = g
            .txs
            .get(&safe_tx_hash)
            .ok_or_else(|| PortError::NotFound(format!("remote tx not found: {safe_tx_hash}")))?;
        Ok(json!({
            "safeTxHash": safe_tx_hash,
            "chainId": state.chain_id,
            "safeAddress": state.safe_address,
            "proposed": state.proposed,
            "confirmations": state.confirmations.len(),
            "executedTxHash": state.executed_tx_hash,
        }))
    }
}
