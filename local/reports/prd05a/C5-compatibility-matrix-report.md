# C5 Compatibility Matrix Report

Generated: 2026-02-18T18:06:25Z

## Chromium Runtime

- Binary: `google-chrome`
- Version: `NOT_AVAILABLE`

## Matrix

| Wallet | Browser | Status | Notes |
|---|---|---|---|
| MetaMask | Chromium | BLOCKED | missing PRD05A_METAMASK_PROFILE_DIR |
| Rabby | Chromium | BLOCKED | missing PRD05A_RABBY_PROFILE_DIR |

## Repro

- Set `PRD05A_METAMASK_PROFILE_DIR` and `PRD05A_RABBY_PROFILE_DIR` to browser profile paths before rerun.
- Command: `scripts/run_prd05a_compat_matrix.sh`
