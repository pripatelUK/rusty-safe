//! Bridge between egui shell and signing workspace crates.
//! This is intentionally thin for Phase A0 and should remain UI-facing only.

use alloy::primitives::B256;

use rusty_safe_signing_adapters::{
    Eip1193Adapter, QueueAdapter, SafeServiceAdapter, WalletConnectAdapter,
};
use rusty_safe_signing_core::{Orchestrator, PortError, SigningCommand};

pub struct SigningBridge {
    orchestrator: Orchestrator<Eip1193Adapter, SafeServiceAdapter, WalletConnectAdapter, QueueAdapter>,
}

impl Default for SigningBridge {
    fn default() -> Self {
        Self {
            orchestrator: Orchestrator::new(
                Eip1193Adapter,
                SafeServiceAdapter,
                WalletConnectAdapter,
                QueueAdapter,
            ),
        }
    }
}

impl SigningBridge {
    pub fn connect_provider(&self) -> Result<(), PortError> {
        self.orchestrator.handle(SigningCommand::ConnectProvider)
    }

    pub fn propose_tx(&self, safe_tx_hash: B256) -> Result<(), PortError> {
        self.orchestrator
            .handle(SigningCommand::ProposeTx { safe_tx_hash })
    }
}
