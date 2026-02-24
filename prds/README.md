# Signing PRD Set

This folder contains the signing context and requirements set for `rusty-safe`.

## Documents

- `prds/00-MARKDOWN-CONTEXT-AUDIT.md`
  - Full markdown audit across this repo (tracked + local ignored docs) and relevant `localsafe.eth` markdown.
- `prds/LOCALSAFE_SIGNING_FLOW_AND_CAPABILITIES.md`
  - Capability analysis of `deps/localsafe.eth` with concrete signing flow breakdown.
- `prds/01-PRD-TRANSACTION-SIGNING-MVP.md`
  - Product requirements for Safe transaction signing/execution in `rusty-safe`.
- `prds/02-PRD-SAFE-MESSAGE-SIGNING.md`
  - Product requirements for Safe message signing, collection, and response output.
- `prds/03-PRD-WALLETCONNECT-SIGNING.md`
  - Product requirements for WalletConnect request handling and signing responses.
- `prds/04-SIGNING-INTEGRATION-VIABILITY-REPORT.md`
  - Comparative feasibility report for Foundry, Alloy (Ledger/Trezor), `safers-cli`, and direct Ledger SDK paths.
- `prds/05A-PRD-PARITY-WAVE.md`
  - Canonical execution PRD for parity wave delivery (tx/message/WalletConnect/signature collaboration parity).
- `prds/05A-E2E-WALLET-RUNTIME-PLAN.md`
  - Deterministic wallet-mock-only blocking E2E/release-gate plan for parity closure.
- `prds/05A-E2E-REAL-WALLET-HARDWARE-TRACK.md`
  - Companion non-blocking plan for MetaMask/Rabby canaries and Ledger/Trezor passthrough validation.
- `prds/05B-PRD-HARDENING-WAVE.md`
  - Canonical execution PRD for hardening wave delivery (multitab, reconcile, policy, rollout hardening), including the migrated legacy PRD 05 full snapshot appendix.

## Ordering

1. Read the markdown audit.
2. Read localsafe capability analysis.
3. Read signing integration viability report.
4. Execute PRD 05A parity wave.
5. Execute the 05A real-wallet/hardware companion track (non-blocking).
6. Execute PRD 05B hardening wave.
7. Use PRD 01/02/03 as historical decomposition references where needed.
