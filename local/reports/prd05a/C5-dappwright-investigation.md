# C5 dappwright Investigation (MetaMask E2E)

Generated: 2026-02-21

## Scope

- Add `TenKeyLabs/dappwright` as a repo dependency (git submodule).
- Evaluate whether dappwright can improve or replace parts of current MetaMask E2E coverage.
- Focus on Chromium + MetaMask connect/sign/send-transaction flows.

## Submodule Status

- Added submodule path: `deps/dappwright`
- Submodule URL: `git@github.com:TenKeyLabs/dappwright.git`
- Checked-out ref: `a3c2c0d4d604261b4a1df90c9de479ae8c600651` (`v2.13.3-1-ga3c2c0d`)

## Upstream Signals

- Active maintenance observed (latest commit in submodule: 2026-02-17).
- MetaMask recommended version in dappwright source: `13.17.0`.
- dappwright explicitly documents that Chromium extension tests should run with `headless: false` + `xvfb-run` in CI.

## Local Runtime Validation

Environment used for probe:

- Node: `v20.19.6`
- Playwright browser install: Chromium build `1194`
- Runner: `xvfb-run --auto-servernum`

Commands executed:

1. `xvfb-run --auto-servernum yarn playwright test test/3-dapp.spec.ts --project MetaMask -g "should be able to connect" --reporter=list`
   - Result: PASS
2. `xvfb-run --auto-servernum yarn playwright test test/3-dapp.spec.ts --project MetaMask -g "should be able to sign messages" --reporter=list`
   - Result: FAIL (`Target page, context or browser has been closed`)
3. `xvfb-run --auto-servernum yarn playwright test test/3-dapp.spec.ts --project MetaMask -g "should be able to confirm without altering gas settings" --reporter=list`
   - Result: FAIL (`Target page, context or browser has been closed`)

Artifacts:

- dappwright traces/errors in `deps/dappwright/test-results/`

## Technical Fit vs Current E2E

Current repo uses Synpress + Playwright with custom MetaMask bootstrap in:

- `e2e/tests/metamask/metamask-bootstrap.mjs`
- `e2e/tests/metamask/metamask-patched-fixtures.mjs`
- `e2e/tests/metamask/metamask-eip1193.spec.mjs`

dappwright provides a cleaner fixture/bootstrap API:

- `bootstrap(...)` in `deps/dappwright/src/bootstrap.ts`
- wallet actions (`approve`, `sign`, `confirmTransaction`) in `deps/dappwright/src/wallets/metamask/actions/transaction.ts`

But dappwright popup handling currently waits for `context.waitForEvent("page")` and then interacts with popup (`performPopupAction`), which still appears sensitive to popup lifecycle races in this environment.

## Recommendation

Use dappwright selectively for phase-gated MetaMask tests:

1. Adopt dappwright first for deterministic extension bootstrap + connect/switch-network smoke coverage.
2. Keep existing Synpress/custom path (or direct manual popup handling) for sign/typedData/sendTx until dappwright sign/tx stability is proven in our environment.
3. Standardize CI runner for extension tests to headed mode with virtual display (`xvfb-run`), regardless of framework.

## Risks / Notes

- dappwright has an open issue about locale-dependent onboarding (`#540`), so enforce English locale in CI profile.
- Our environment has a Node/Bun shim conflict; force explicit Node binary for reliability in E2E scripts.
- Full migration from Synpress to dappwright is not yet justified by current sign/tx probe results.
