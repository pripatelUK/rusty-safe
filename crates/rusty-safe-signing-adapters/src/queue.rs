use alloy::primitives::B256;

use rusty_safe_signing_core::{PendingSafeMessage, PendingSafeTx, PortError, QueuePort};

#[derive(Debug, Clone, Default)]
pub struct QueueAdapter;

impl QueuePort for QueueAdapter {
    fn save_tx(&self, _tx: &PendingSafeTx) -> Result<(), PortError> {
        Err(PortError::NotImplemented("queue.save_tx"))
    }

    fn save_message(&self, _message: &PendingSafeMessage) -> Result<(), PortError> {
        Err(PortError::NotImplemented("queue.save_message"))
    }

    fn load_tx(&self, _safe_tx_hash: B256) -> Result<Option<PendingSafeTx>, PortError> {
        Err(PortError::NotImplemented("queue.load_tx"))
    }

    fn load_message(&self, _message_hash: B256) -> Result<Option<PendingSafeMessage>, PortError> {
        Err(PortError::NotImplemented("queue.load_message"))
    }
}
