# C5 Compatibility Matrix Report

Generated: 2026-02-21T14:49:10Z
Run ID: run-20260221T144910Z
Schema: c5e2e-v1

## Chromium Runtime

- Binary: `google-chrome`
- Version: `NOT_AVAILABLE`

## Matrix

| Wallet | Browser | Status | Taxonomy | Notes |
|---|---|---|---|---|
| MetaMask | Chromium | PASS | NONE | Runtime profile check passed. |
| Rabby | Chromium | BLOCKED | ENV_BLOCKER | missing PRD05A_RABBY_PROFILE_DIR |

## Repro

- MetaMask gate command: `scripts/run_prd05a_metamask_e2e.sh`
- MetaMask reports:
  - `local/reports/prd05a/C5-metamask-e2e-report.md`
  - `local/reports/prd05a/C5-metamask-e2e.json`
- MetaMask runtime mode in matrix: `profile-only=1`, timeout=`240s`
- Rabby gate command: `scripts/run_prd05a_rabby_matrix.sh`
- Rabby reports:
  - `local/reports/prd05a/C5-rabby-runtime-report.md`
  - `local/reports/prd05a/C5-rabby-runtime-report.json`
- Command: `scripts/run_prd05a_compat_matrix.sh`

## Deferred Hardware Track (H1, Non-blocking for C5 Hot-wallet Release)

- Owner: Security lead
- Target: E5 gate date + 14 calendar days
- Status: deferred, non-blocking
