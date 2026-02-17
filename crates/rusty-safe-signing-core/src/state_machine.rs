use crate::domain::PendingSafeTx;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigningState {
    Idle,
    Draft,
    Signing,
    ReadyToExecute,
    Executing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateTransition {
    pub from: SigningState,
    pub to: SigningState,
    pub reason: &'static str,
}

pub fn derive_initial_state(tx: &PendingSafeTx) -> SigningState {
    if tx.state_revision == 0 {
        SigningState::Draft
    } else {
        SigningState::Signing
    }
}
