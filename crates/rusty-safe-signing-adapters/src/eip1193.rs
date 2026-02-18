use alloy::primitives::{keccak256, Address, Bytes, B256};
use serde_json::Value;

use rusty_safe_signing_core::{MessageMethod, PortError, ProviderPort};

#[derive(Debug, Clone)]
pub struct Eip1193Adapter {
    mock_account: Address,
    mock_chain_id: u64,
}

impl Default for Eip1193Adapter {
    fn default() -> Self {
        Self {
            mock_account: "0x1000000000000000000000000000000000000001"
                .parse()
                .expect("valid built-in mock account"),
            mock_chain_id: 1,
        }
    }
}

impl ProviderPort for Eip1193Adapter {
    fn request_accounts(&self) -> Result<Vec<Address>, PortError> {
        Ok(vec![self.mock_account])
    }

    fn chain_id(&self) -> Result<u64, PortError> {
        Ok(self.mock_chain_id)
    }

    fn wallet_get_capabilities(&self) -> Result<Option<Value>, PortError> {
        Ok(Some(serde_json::json!({
            "wallet_getCapabilities": true,
            "signMethods": [
                "personal_sign",
                "eth_signTypedData",
                "eth_signTypedData_v4",
                "eth_sendTransaction"
            ],
            "note": "mock adapter for parity-wave core/adapters integration"
        })))
    }

    fn sign_payload(
        &self,
        method: MessageMethod,
        payload: &[u8],
        expected_signer: Address,
    ) -> Result<Bytes, PortError> {
        // Deterministic mock signature payload used in tests. Real EIP-1193 calls are
        // delegated to browser runtime in follow-up transport hardening.
        let mut seed = Vec::new();
        seed.extend_from_slice(format!("{method:?}").as_bytes());
        seed.extend_from_slice(expected_signer.as_slice());
        seed.extend_from_slice(payload);
        let hash = keccak256(seed);
        let mut sig = Vec::with_capacity(65);
        sig.extend_from_slice(hash.as_slice());
        sig.extend_from_slice(hash.as_slice());
        sig.push(27);
        Ok(Bytes::from(sig))
    }

    fn send_transaction(&self, tx_payload: &Value) -> Result<B256, PortError> {
        let canonical = serde_json::to_vec(tx_payload)
            .map_err(|e| PortError::Validation(format!("tx payload serialization failed: {e}")))?;
        Ok(keccak256(canonical))
    }
}
