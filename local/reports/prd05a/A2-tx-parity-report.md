# A2 Tx Parity Report

Generated: 2026-02-18T01:16:45Z (UTC)  
Base commit: `c2f20e3`  
Gate anchor commit: `0a084be`

## PARITY IDs

- `PARITY-TX-01`
- `PARITY-TX-02`
- `PARITY-ABI-01`

## Checklist

1. Create tx draft (raw calldata): `PASS`
2. Create tx draft (ABI-assisted): `PASS`
3. Manual tx signature ingestion: `PASS`
4. Duplicate signature idempotency: `PASS`
5. Propose/confirm/execute lifecycle: `PASS`
6. Threshold progression to `ReadyToExecute`: `PASS`
7. Deterministic transition sequencing (`event_seq` monotonic): `PASS`
8. Writer-lock gating on mutating commands: `PASS`

## Evidence (Tests)

- `crates/rusty-safe-signing-adapters/tests/tx_e2e.rs`
- `crates/rusty-safe-signing-adapters/tests/tx_manual_signature.rs`
- `crates/rusty-safe-signing-adapters/tests/abi_builder.rs`
- `crates/rusty-safe-signing-adapters/tests/queue_lock.rs`
- `crates/rusty-safe-signing-core/tests/state_machine_transitions.rs`

## Replay Determinism Evidence

- `crates/rusty-safe-signing-core/tests/state_machine_transitions.rs::replay_hash_is_deterministic`

## Command Run Reference

- `cargo test --workspace` (includes all tx parity tests) -> `PASS`
