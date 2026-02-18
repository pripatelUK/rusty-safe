use alloy::primitives::{keccak256, B256};

use crate::domain::{MessageStatus, TxStatus, WcStatus};
use crate::ports::PortError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigningState {
    Idle,
    Draft,
    Signing,
    AwaitingThreshold,
    ReadyToExecute,
    Executing,
    Responding,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateTransition {
    pub from: String,
    pub to: String,
    pub reason: &'static str,
}

pub fn tx_transition(
    current: TxStatus,
    action: TxAction,
) -> Result<(TxStatus, StateTransition), PortError> {
    use TxAction as A;
    use TxStatus as S;

    let (next, reason) = match (current, action) {
        (S::Draft, A::Sign) => (S::Signing, "first_signature"),
        (S::Signing, A::Sign) => (S::Signing, "additional_signature"),
        (S::Proposed, A::Sign) => (S::Proposed, "additional_signature"),
        (S::Confirming, A::Sign) => (S::Confirming, "additional_signature"),
        (S::ReadyToExecute, A::Sign) => (S::ReadyToExecute, "additional_signature"),
        (S::Signing, A::Propose) => (S::Proposed, "proposed"),
        (S::Proposed, A::Propose) => (S::Proposed, "already_proposed"),
        (S::Proposed, A::Confirm) => (S::Confirming, "confirmed"),
        (S::Confirming, A::Confirm) => (S::Confirming, "additional_confirm"),
        (S::Signing, A::ThresholdMet)
        | (S::Proposed, A::ThresholdMet)
        | (S::Confirming, A::ThresholdMet) => (S::ReadyToExecute, "threshold_met"),
        (S::ReadyToExecute, A::ExecuteStart) => (S::Executing, "execution_started"),
        (S::Executing, A::ExecuteSuccess) => (S::Executed, "execution_succeeded"),
        (S::Draft, A::Cancel) | (S::Signing, A::Cancel) | (S::Proposed, A::Cancel) => {
            (S::Cancelled, "cancelled")
        }
        (_, A::Fail) => (S::Failed, "failed"),
        (state, act) => {
            return Err(PortError::Validation(format!(
                "illegal tx transition: {state:?} + {act:?}"
            )));
        }
    };
    Ok((
        next,
        StateTransition {
            from: format!("{current:?}"),
            to: format!("{next:?}"),
            reason,
        },
    ))
}

pub fn message_transition(
    current: MessageStatus,
    action: MessageAction,
) -> Result<(MessageStatus, StateTransition), PortError> {
    use MessageAction as A;
    use MessageStatus as S;

    let (next, reason) = match (current, action) {
        (S::Draft, A::Sign) => (S::Signing, "first_signature"),
        (S::Signing, A::Sign) => (S::Signing, "additional_signature"),
        (S::AwaitingThreshold, A::Sign) => (S::AwaitingThreshold, "additional_signature"),
        (S::Signing, A::AwaitThreshold) => (S::AwaitingThreshold, "awaiting_threshold"),
        (S::Signing, A::ThresholdMet) | (S::AwaitingThreshold, A::ThresholdMet) => {
            (S::ThresholdMet, "threshold_met")
        }
        (S::ThresholdMet, A::Respond) => (S::Responded, "responded"),
        (S::Draft, A::Cancel) | (S::Signing, A::Cancel) | (S::AwaitingThreshold, A::Cancel) => {
            (S::Cancelled, "cancelled")
        }
        (_, A::Fail) => (S::Failed, "failed"),
        (state, act) => {
            return Err(PortError::Validation(format!(
                "illegal message transition: {state:?} + {act:?}"
            )));
        }
    };
    Ok((
        next,
        StateTransition {
            from: format!("{current:?}"),
            to: format!("{next:?}"),
            reason,
        },
    ))
}

pub fn wc_transition(
    current: WcStatus,
    action: WcAction,
) -> Result<(WcStatus, StateTransition), PortError> {
    use WcAction as A;
    use WcStatus as S;

    let (next, reason) = match (current, action) {
        (S::Pending, A::Route) => (S::Routed, "routed"),
        (S::Routed, A::AwaitThreshold) => (S::AwaitingThreshold, "awaiting_threshold"),
        (S::Routed, A::RespondImmediate) => (S::RespondingImmediate, "responding_immediate"),
        (S::Routed, A::RespondDeferred) | (S::AwaitingThreshold, A::RespondDeferred) => {
            (S::RespondingDeferred, "responding_deferred")
        }
        (S::RespondingImmediate, A::RespondSuccess)
        | (S::RespondingDeferred, A::RespondSuccess) => (S::Responded, "responded"),
        (S::Pending, A::Expire) | (S::Routed, A::Expire) | (S::AwaitingThreshold, A::Expire) => {
            (S::Expired, "expired")
        }
        (_, A::Fail) => (S::Failed, "failed"),
        (state, act) => {
            return Err(PortError::Validation(format!(
                "illegal wc transition: {state:?} + {act:?}"
            )));
        }
    };
    Ok((
        next,
        StateTransition {
            from: format!("{current:?}"),
            to: format!("{next:?}"),
            reason,
        },
    ))
}

pub fn replay_final_hash(entries: &[String]) -> B256 {
    let mut combined = String::new();
    for e in entries {
        combined.push_str(e);
        combined.push('\n');
    }
    keccak256(combined.as_bytes())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxAction {
    Sign,
    Propose,
    Confirm,
    ThresholdMet,
    ExecuteStart,
    ExecuteSuccess,
    Fail,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageAction {
    Sign,
    AwaitThreshold,
    ThresholdMet,
    Respond,
    Fail,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WcAction {
    Route,
    AwaitThreshold,
    RespondImmediate,
    RespondDeferred,
    RespondSuccess,
    Expire,
    Fail,
}
