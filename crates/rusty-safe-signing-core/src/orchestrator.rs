use alloy::primitives::B256;

use crate::ports::{PortError, ProviderPort, QueuePort, SafeServicePort, WalletConnectPort};

#[derive(Debug, Clone)]
pub enum SigningCommand {
    ConnectProvider,
    ProposeTx { safe_tx_hash: B256 },
    ConfirmTx { safe_tx_hash: B256, signature: Vec<u8> },
    ExecuteTx { safe_tx_hash: B256 },
    RespondWalletConnect { request_id: String },
}

pub struct Orchestrator<P, S, W, Q>
where
    P: ProviderPort,
    S: SafeServicePort,
    W: WalletConnectPort,
    Q: QueuePort,
{
    pub provider: P,
    pub safe_service: S,
    pub walletconnect: W,
    pub queue: Q,
}

impl<P, S, W, Q> Orchestrator<P, S, W, Q>
where
    P: ProviderPort,
    S: SafeServicePort,
    W: WalletConnectPort,
    Q: QueuePort,
{
    pub fn new(provider: P, safe_service: S, walletconnect: W, queue: Q) -> Self {
        Self {
            provider,
            safe_service,
            walletconnect,
            queue,
        }
    }

    pub fn handle(&self, command: SigningCommand) -> Result<(), PortError> {
        match command {
            SigningCommand::ConnectProvider => {
                let _ = self.provider.request_accounts()?;
                Ok(())
            }
            SigningCommand::ProposeTx { .. }
            | SigningCommand::ConfirmTx { .. }
            | SigningCommand::ExecuteTx { .. }
            | SigningCommand::RespondWalletConnect { .. } => {
                Err(PortError::NotImplemented("orchestrator.handle"))
            }
        }
    }
}
