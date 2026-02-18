use alloy::primitives::{Address, B256};

use crate::domain::{UrlImportEnvelope, WcSessionAction};
use crate::ports::{PortError, ProviderPort, QueuePort, SafeServicePort, WalletConnectPort};

#[derive(Debug, Clone)]
pub enum SigningCommand {
    ConnectProvider,
    CreateSafeTxFromAbi {
        to: Address,
        abi_json: String,
        method_signature: String,
        args: Vec<String>,
    },
    AddTxSignature {
        safe_tx_hash: B256,
        signer: Address,
        signature: Vec<u8>,
    },
    ProposeTx { safe_tx_hash: B256 },
    ConfirmTx { safe_tx_hash: B256, signature: Vec<u8> },
    ExecuteTx { safe_tx_hash: B256 },
    WalletConnectSessionAction {
        topic: String,
        action: WcSessionAction,
    },
    RespondWalletConnect { request_id: String },
    ImportUrlPayload { envelope: UrlImportEnvelope },
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
                let _ = self.provider.wallet_get_capabilities();
                Ok(())
            }
            SigningCommand::CreateSafeTxFromAbi { .. }
            | SigningCommand::AddTxSignature { .. }
            | SigningCommand::ProposeTx { .. }
            | SigningCommand::ConfirmTx { .. }
            | SigningCommand::ExecuteTx { .. }
            | SigningCommand::WalletConnectSessionAction { .. }
            | SigningCommand::RespondWalletConnect { .. }
            | SigningCommand::ImportUrlPayload { .. } => {
                Err(PortError::NotImplemented("orchestrator.handle"))
            }
        }
    }
}
