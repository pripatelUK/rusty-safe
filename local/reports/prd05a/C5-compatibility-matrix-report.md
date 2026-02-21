# C5 Compatibility Matrix Report

Generated: 2026-02-19T02:38:52Z

## Chromium Runtime

- Binary: `google-chrome`
- Version: `NOT_AVAILABLE`

## Matrix

| Wallet | Browser | Status | Notes |
|---|---|---|---|
| MetaMask | Chromium | FAIL | metamask cache preflight failed (post-unlock onboarding state); see C5-metamask-e2e.log |
| Rabby | Chromium | BLOCKED | missing PRD05A_RABBY_PROFILE_DIR |

## Repro

- MetaMask gate command: `scripts/run_prd05a_metamask_e2e.sh`
- MetaMask report: `local/reports/prd05a/C5-metamask-e2e-report.md`
- Rabby currently remains profile-based; set `PRD05A_RABBY_PROFILE_DIR` for manual matrix evidence.
- Command: `scripts/run_prd05a_compat_matrix.sh`
