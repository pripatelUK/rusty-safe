use alloy::primitives::{Address, B256};
use serde_json::Value;

use rusty_safe_signing_core::{PortError, ProviderPort, SignatureMethod};

#[derive(Debug, Clone, Default)]
pub struct Eip1193Adapter;

impl ProviderPort for Eip1193Adapter {
    fn request_accounts(&self) -> Result<Vec<Address>, PortError> {
        Err(PortError::NotImplemented("eip1193.request_accounts"))
    }

    fn chain_id(&self) -> Result<u64, PortError> {
        Err(PortError::NotImplemented("eip1193.chain_id"))
    }

    fn wallet_get_capabilities(&self) -> Result<Option<Value>, PortError> {
        Err(PortError::NotImplemented("eip1193.wallet_get_capabilities"))
    }

    fn sign_payload(
        &self,
        _method: SignatureMethod,
        _payload: &[u8],
        _expected_signer: Address,
    ) -> Result<Vec<u8>, PortError> {
        Err(PortError::NotImplemented("eip1193.sign_payload"))
    }

    fn send_transaction(&self, _tx_payload: &Value) -> Result<B256, PortError> {
        Err(PortError::NotImplemented("eip1193.send_transaction"))
    }
}
