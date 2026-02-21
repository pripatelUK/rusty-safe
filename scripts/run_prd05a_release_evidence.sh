#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

timestamp="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

scripts/check_signing_boundaries.sh
scripts/check_prd05a_traceability.sh
cargo fmt --all -- --check
cargo clippy -p rusty-safe-signing-core -p rusty-safe-signing-adapters --all-targets -- -D warnings
cargo test --workspace
cargo check -p rusty-safe-signing-adapters --target wasm32-unknown-unknown
cargo check -p rusty-safe --target wasm32-unknown-unknown
scripts/run_prd05a_safe_service_live.sh
scripts/run_prd05a_performance.sh
scripts/run_prd05a_differential.sh
scripts/run_prd05a_compat_matrix.sh
scripts/run_prd05a_hardware_smoke.sh

mkdir -p local/reports/prd05a
cat > local/reports/prd05a/C10-release-evidence-summary.md <<EOF
# C10 Release Evidence Summary

Generated: ${timestamp}

## Executed Gates

1. Boundary checks: PASS
2. Traceability checks: PASS
3. Format check: PASS
4. Signing clippy strict: PASS
5. Workspace tests: PASS
6. Performance report: local/reports/prd05a/C6-performance-report.md
7. Differential parity report: local/reports/prd05a/C9-differential-parity-report.md
8. Safe service live validation report: local/reports/prd05a/C2-safe-service-live-report.md
9. MetaMask preflight + Playwright E2E report: local/reports/prd05a/C5-metamask-e2e-report.md (details in local/reports/prd05a/C5-metamask-e2e.log)
10. Compatibility matrix report: local/reports/prd05a/C5-compatibility-matrix-report.md
11. Hardware passthrough smoke report: local/reports/prd05a/C5-hardware-passthrough-smoke.md

## Milestone Discipline

- Continuation milestones tracked in prds/05A-CONTINUATION-MILESTONES.md
- Release checklist tracked in prds/05A-RELEASE-GATE-CHECKLIST.md
EOF

echo "wrote local/reports/prd05a/C10-release-evidence-summary.md"
