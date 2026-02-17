# Localsafe Signing Flow And Capability Analysis

Submodule: `deps/localsafe.eth`  
Checked commit: `48a580f02b03bce3002d1bfe053f837d16932b53`

## Capability Summary

| Capability | Status In localsafe | Evidence |
| --- | --- | --- |
| Build Safe transaction | Implemented | `deps/localsafe.eth/app/hooks/useSafe.ts:379` |
| Compute Safe tx hash | Implemented | `deps/localsafe.eth/app/hooks/useSafe.ts:431` |
| Sign transaction | Implemented | `deps/localsafe.eth/app/hooks/useSafe.ts:438` |
| Execute transaction | Implemented | `deps/localsafe.eth/app/hooks/useSafe.ts:479` |
| Threshold-aware execute UX | Implemented | `deps/localsafe.eth/app/safe/[address]/tx/[txHash]/TxDetailsClient.tsx:104` |
| Manual add signature (tx) | Implemented | `deps/localsafe.eth/app/safe/[address]/tx/[txHash]/TxDetailsClient.tsx:399` |
| Export/import tx JSON | Implemented | `deps/localsafe.eth/app/provider/SafeTxProvider.tsx:159`, `deps/localsafe.eth/app/provider/SafeTxProvider.tsx:181` |
| Share tx/signature links | Implemented | `deps/localsafe.eth/app/safe/[address]/tx/[txHash]/TxDetailsClient.tsx:327`, `deps/localsafe.eth/app/safe/[address]/tx/[txHash]/TxDetailsClient.tsx:358` |
| Message signing (Safe message) | Implemented | `deps/localsafe.eth/app/safe/[address]/message/[messageHash]/MessageDetailsClient.tsx:128` |
| Manual add signature (message) | Implemented | `deps/localsafe.eth/app/safe/[address]/wc-sign/WalletConnectSignClient.tsx:324` |
| Dedicated sign-message page | Partially implemented | `deps/localsafe.eth/app/safe/[address]/sign-message/SignMessageClient.tsx:150` |
| WalletConnect session + request handling | Implemented | `deps/localsafe.eth/app/provider/WalletConnectProvider.tsx:110`, `deps/localsafe.eth/app/components/WalletConnectRequestHandler.tsx:23` |
| WalletConnect tx handling (`eth_sendTransaction`) | Implemented | `deps/localsafe.eth/app/safe/[address]/wc-tx/WalletConnectTxClient.tsx:97` |
| WalletConnect message methods | Implemented | `deps/localsafe.eth/app/safe/[address]/wc-sign/WalletConnectSignClient.tsx:159` |
| Wallet connectors (MetaMask/Rabby/Rainbow/WalletConnect/Ledger/OneKey) | Implemented | `deps/localsafe.eth/app/provider/WagmiConfigProvider.tsx:254`, `deps/localsafe.eth/app/provider/WagmiConfigProvider.tsx:260` |

## Signing Flows

## Flow 1: Native Transaction Signing

1. User creates tx from dashboard builder.
2. App builds `EthSafeTransaction` with Protocol Kit.
3. App computes `safeTxHash`.
4. Tx is saved in local tx store (localStorage-backed provider).
5. Owners add signatures.
6. When signatures reach threshold, execute button is enabled.
7. App executes tx and returns on-chain tx hash.

Primary code path:

- Build: `useSafe.ts:379`
- Hash: `useSafe.ts:431`
- Sign: `useSafe.ts:438`
- Execute: `useSafe.ts:479`
- Execute UI gating: `TxDetailsClient.tsx:766`

## Flow 2: Transaction Collaboration

1. Export tx + signatures JSON.
2. Share full tx link or signature-only link.
3. Receiver imports into local queue.
4. Imported signatures are merged by nonce/hash.

Primary code path:

- Tx store import/export: `SafeTxProvider.tsx:159`, `SafeTxProvider.tsx:181`
- Share link and signature link: `TxDetailsClient.tsx:327`, `TxDetailsClient.tsx:358`
- Dashboard URL import handlers: `SafeDashboardClient.tsx:70`

## Flow 3: WalletConnect Transaction Request

1. WalletConnect request arrives in provider.
2. Router redirects to `/wc-tx`.
3. User can:
   - Create and immediately respond with Safe tx hash.
   - Create and delay response until execution.
4. Delayed mode stores pending response context in session storage.
5. After execute in tx details page, app sends real on-chain hash to dApp.

Primary code path:

- Request route: `WalletConnectRequestHandler.tsx:38`
- Quick response: `WalletConnectTxClient.tsx:84`
- Delayed response state: `WalletConnectTxClient.tsx:148`
- Deferred response send after execute: `TxDetailsClient.tsx:267`

## Flow 4: WalletConnect Message Signing

1. WalletConnect signing request arrives (`personal_sign`, `eth_sign`, `eth_signTypedData(_v4)`).
2. Router sends user to `/wc-sign`.
3. App normalizes message per method.
4. App creates/signs Safe message via Protocol Kit.
5. If threshold met, app responds with `encodedSignatures()`.
6. If threshold not met, request stays active and user can add/import signatures.

Primary code path:

- Method handling: `WalletConnectSignClient.tsx:159`
- Sign message: `WalletConnectSignClient.tsx:227`
- Threshold response: `WalletConnectSignClient.tsx:251`
- Manual add signature and threshold re-check: `WalletConnectSignClient.tsx:324`

## Flow 5: Non-WalletConnect Message Signing

There are two message routes:

- Fully functional message details route:
  - `MessageDetailsClient.tsx:128` signs and handles threshold behavior.
- Partially implemented sign-message route:
  - `SignMessageClient.tsx:150` returns "Signing not yet implemented".

This means hash calculation is available broadly, but one message UI path is still hash-only.

## Storage And Trust Model

## Current Storage Model

- Transactions/signatures: localStorage (`SafeTxProvider.tsx`).
- Messages/signatures: localStorage (`SafeMessageProvider.tsx`).
- Active WalletConnect request handoff: sessionStorage (`wc-pending-request`, `wc-pending-response-*`, `wc-message-*`).

## Practical Consequences

- No canonical shared backend queue.
- Collaboration relies on exported JSON and URL payload sharing.
- Request state can expire or be lost with tab/session lifecycle.

## Security And Reliability Observations

1. Capability is broad and mostly complete for tx + message signing.
2. Local-first architecture is intentional but requires explicit backup/recovery UX.
3. Signature provenance/integrity checks are minimal.
4. WalletConnect handling is robust but session-expiry-sensitive.
5. Dedicated sign-message route needs completion for parity.

## Parity Targets For Rusty Safe

Minimum parity recommended:

1. Transaction build, sign, signature collection, execute.
2. Message sign and threshold collection.
3. Import/export/share signature workflows.
4. WalletConnect request routing and threshold-aware responses.

Hardening deltas recommended:

1. Add integrity metadata for persisted signing state.
2. Require explicit verification state before enabling sign.
3. Add deterministic tests for method-specific signing transformations.

