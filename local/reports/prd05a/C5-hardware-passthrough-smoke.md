# C5 Hardware Passthrough Smoke

Generated: 2026-02-18T19:06:50Z

| Device | Status | Notes |
|---|---|---|
| Ledger (wallet passthrough) | BLOCKED | missing PRD05A_LEDGER_SMOKE_LOG |
| Trezor (wallet passthrough) | BLOCKED | missing PRD05A_TREZOR_SMOKE_LOG |

## Repro

- Provide smoke logs through `PRD05A_LEDGER_SMOKE_LOG` and `PRD05A_TREZOR_SMOKE_LOG`.
- Each log must include a literal `PASS` marker to satisfy release evidence gate.
