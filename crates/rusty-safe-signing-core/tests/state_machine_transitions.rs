use rusty_safe_signing_core::{
    message_transition, replay_final_hash, tx_transition, wc_transition, MessageAction,
    MessageStatus, TxAction, TxStatus, WcAction, WcStatus,
};

#[test]
fn tx_happy_path_transitions() {
    let (s1, _) = tx_transition(TxStatus::Draft, TxAction::Sign).expect("draft -> sign");
    assert_eq!(s1, TxStatus::Signing);
    let (s2, _) = tx_transition(s1, TxAction::Propose).expect("signing -> propose");
    assert_eq!(s2, TxStatus::Proposed);
    let (s3, _) = tx_transition(s2, TxAction::Confirm).expect("proposed -> confirm");
    assert_eq!(s3, TxStatus::Confirming);
    let (s4, _) = tx_transition(s3, TxAction::ThresholdMet).expect("confirming -> ready");
    assert_eq!(s4, TxStatus::ReadyToExecute);
    let (s5, _) = tx_transition(s4, TxAction::ExecuteStart).expect("ready -> executing");
    assert_eq!(s5, TxStatus::Executing);
    let (s6, _) = tx_transition(s5, TxAction::ExecuteSuccess).expect("executing -> executed");
    assert_eq!(s6, TxStatus::Executed);
}

#[test]
fn tx_illegal_transition_is_rejected() {
    let err = tx_transition(TxStatus::Draft, TxAction::ExecuteStart).expect_err("must fail");
    assert!(err.to_string().contains("illegal tx transition"));
}

#[test]
fn message_threshold_path_transitions() {
    let (s1, _) =
        message_transition(MessageStatus::Draft, MessageAction::Sign).expect("draft -> sign");
    assert_eq!(s1, MessageStatus::Signing);
    let (s2, _) =
        message_transition(s1, MessageAction::AwaitThreshold).expect("signing -> awaiting");
    assert_eq!(s2, MessageStatus::AwaitingThreshold);
    let (s3, _) =
        message_transition(s2, MessageAction::ThresholdMet).expect("awaiting -> threshold_met");
    assert_eq!(s3, MessageStatus::ThresholdMet);
    let (s4, _) = message_transition(s3, MessageAction::Respond).expect("threshold -> responded");
    assert_eq!(s4, MessageStatus::Responded);
}

#[test]
fn message_illegal_transition_is_rejected() {
    let err =
        message_transition(MessageStatus::Draft, MessageAction::Respond).expect_err("must fail");
    assert!(err.to_string().contains("illegal message transition"));
}

#[test]
fn wc_deferred_path_transitions() {
    let (s1, _) = wc_transition(WcStatus::Pending, WcAction::Route).expect("pending -> routed");
    assert_eq!(s1, WcStatus::Routed);
    let (s2, _) =
        wc_transition(s1, WcAction::RespondDeferred).expect("routed -> responding deferred");
    assert_eq!(s2, WcStatus::RespondingDeferred);
    let (s3, _) =
        wc_transition(s2, WcAction::RespondSuccess).expect("responding deferred -> responded");
    assert_eq!(s3, WcStatus::Responded);
}

#[test]
fn wc_illegal_transition_is_rejected() {
    let err = wc_transition(WcStatus::Pending, WcAction::RespondSuccess).expect_err("must fail");
    assert!(err.to_string().contains("illegal wc transition"));
}

#[test]
fn replay_hash_is_deterministic() {
    let entries = vec![
        "1:Draft->Signing".to_owned(),
        "2:Signing->Proposed".to_owned(),
        "3:Proposed->Confirming".to_owned(),
    ];
    let hash_a = replay_final_hash(&entries);
    let hash_b = replay_final_hash(&entries);
    assert_eq!(hash_a, hash_b);
}
