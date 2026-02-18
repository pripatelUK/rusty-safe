# C9 Differential Parity Report

Generated: 2026-02-18T18:06:25Z

## Result

- Differential harness: PASS
- Critical diffs: 0

## Evidence

- Command: `cargo test -p rusty-safe-signing-adapters --test parity_differential -- --nocapture`
- Raw marker: `DIFF parity_tx=Executed parity_msg=ThresholdMet parity_wc=Proposed->Approved->Disconnected parity_abi=a9059cbb parity_collab=importTx,importSig,importMsg,importMsgSig`
- Fixtures root: `fixtures/signing/`
