use alloy::primitives::Address;
use rusty_safe_signing_adapters::HashingAdapter;
use rusty_safe_signing_core::HashingPort;

#[test]
fn safe_tx_hash_is_deterministic() {
    let adapter = HashingAdapter;
    let safe: Address = "0x000000000000000000000000000000000000BEEF"
        .parse()
        .expect("safe address");

    let payload = serde_json::json!({
        "to": "0x000000000000000000000000000000000000CAFE",
        "value": "0",
        "data": "0x",
        "operation": 0,
        "safeTxGas": "0",
        "baseGas": "0",
        "gasPrice": "0",
        "gasToken": "0x0000000000000000000000000000000000000000",
        "refundReceiver": "0x0000000000000000000000000000000000000000",
        "threshold": 1,
        "safeVersion": "1.3.0"
    });

    let a = adapter
        .safe_tx_hash(1, safe, 10, &payload)
        .expect("first hash");
    let b = adapter
        .safe_tx_hash(1, safe, 10, &payload)
        .expect("second hash");
    assert_eq!(a, b);
}

#[test]
fn message_hash_is_deterministic() {
    let adapter = HashingAdapter;
    let safe: Address = "0x000000000000000000000000000000000000BEEF"
        .parse()
        .expect("safe address");

    let payload = serde_json::json!({
        "message": "hello",
        "threshold": 1,
        "safeVersion": "1.3.0"
    });

    let a = adapter
        .message_hash(
            1,
            safe,
            rusty_safe_signing_core::MessageMethod::PersonalSign,
            &payload,
        )
        .expect("first hash");
    let b = adapter
        .message_hash(
            1,
            safe,
            rusty_safe_signing_core::MessageMethod::PersonalSign,
            &payload,
        )
        .expect("second hash");
    assert_eq!(a, b);
}
