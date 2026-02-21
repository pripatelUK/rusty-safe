#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source "$ROOT_DIR/scripts/lib/prd05a_e2e_common.sh"

timestamp="$(prd05a_now_utc)"
run_id="$(prd05a_run_id)"
summary_md="local/reports/prd05a/C10-release-evidence-summary.md"
summary_json="local/reports/prd05a/C10-release-evidence-summary.json"

scripts/check_signing_boundaries.sh
scripts/check_prd05a_traceability.sh
scripts/check_prd05a_phase_discipline.sh
cargo fmt --all -- --check
cargo clippy -p rusty-safe-signing-core -p rusty-safe-signing-adapters --all-targets -- -D warnings
cargo test --workspace
cargo check -p rusty-safe-signing-adapters --target wasm32-unknown-unknown
cargo check -p rusty-safe --target wasm32-unknown-unknown
scripts/run_prd05a_safe_service_live.sh
scripts/run_prd05a_performance.sh
scripts/run_prd05a_differential.sh
scripts/run_prd05a_driver_comparison.sh
scripts/run_prd05a_compat_matrix.sh
PRD05A_SOAK_PROFILE_ONLY="${PRD05A_SOAK_PROFILE_ONLY:-1}" scripts/run_prd05a_metamask_soak.sh pr
scripts/run_prd05a_hardware_smoke.sh

mkdir -p local/reports/prd05a
cat >"$summary_md" <<EOF
# C10 Release Evidence Summary

Generated: ${timestamp}
Run ID: ${run_id}
Schema: ${PRD05A_SCHEMA_VERSION}

## Executed Gates

1. Boundary checks: PASS
2. Traceability checks: PASS
3. Phase discipline checks: PASS
4. Format check: PASS
5. Signing clippy strict: PASS
6. Workspace tests: PASS
7. Performance report: local/reports/prd05a/C6-performance-report.md
8. Differential parity report: local/reports/prd05a/C9-differential-parity-report.md
9. Safe service live validation report: local/reports/prd05a/C2-safe-service-live-report.md
10. Driver arbitration comparison report: local/reports/prd05a/C5-dappwright-investigation.md
11. MetaMask preflight + Playwright E2E report: local/reports/prd05a/C5-metamask-e2e-report.md
12. Compatibility matrix report: local/reports/prd05a/C5-compatibility-matrix-report.md
13. Soak report: local/reports/prd05a/C5-metamask-soak-report.md
14. Hardware passthrough smoke report (deferred/non-blocking for hot-wallet release): local/reports/prd05a/C5-hardware-passthrough-smoke.md

## Milestone Discipline

- Continuation milestones tracked in prds/05A-CONTINUATION-MILESTONES.md
- Release checklist tracked in prds/05A-RELEASE-GATE-CHECKLIST.md
EOF

cat >"$summary_json" <<EOF
{
  "schema_version": "${PRD05A_SCHEMA_VERSION}",
  "generated": "${timestamp}",
  "run_id": "${run_id}",
  "status": "PASS",
  "reports": {
    "performance": "local/reports/prd05a/C6-performance-report.md",
    "differential": "local/reports/prd05a/C9-differential-parity-report.md",
    "safe_service_live": "local/reports/prd05a/C2-safe-service-live-report.md",
    "phase_discipline": "local/reports/prd05a/C5-phase-discipline-report.md",
    "driver_comparison": "local/reports/prd05a/C5-dappwright-investigation.md",
    "driver_comparison_json": "local/reports/prd05a/C5-dappwright-investigation.json",
    "metamask_e2e": "local/reports/prd05a/C5-metamask-e2e-report.md",
    "metamask_e2e_json": "local/reports/prd05a/C5-metamask-e2e.json",
    "metamask_soak": "local/reports/prd05a/C5-metamask-soak-report.md",
    "metamask_soak_json": "local/reports/prd05a/C5-metamask-soak-report.json",
    "compat_matrix": "local/reports/prd05a/C5-compatibility-matrix-report.md",
    "compat_matrix_json": "local/reports/prd05a/C5-compatibility-matrix-report.json",
    "hardware_smoke": "local/reports/prd05a/C5-hardware-passthrough-smoke.md"
  },
  "artifacts": {
    "markdown_summary": "${summary_md}",
    "json_summary": "${summary_json}"
  }
}
EOF

echo "wrote ${summary_md}"
echo "wrote ${summary_json}"
