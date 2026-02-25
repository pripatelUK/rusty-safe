#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source "$ROOT_DIR/scripts/lib/prd05a_e2e_common.sh"

timestamp="$(prd05a_now_utc)"
run_id="$(prd05a_run_id)"
summary_md="local/reports/prd05a/C5-release-evidence-index.md"
summary_json="local/reports/prd05a/C5-release-evidence-index.json"

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
scripts/run_prd05a_wallet_mock_preflight.sh
scripts/run_prd05a_wallet_mock_gate.sh
PRD05A_SOAK_RUNS="${PRD05A_SOAK_RUNS:-20}" \
PRD05A_SOAK_MIN_PASSES="${PRD05A_SOAK_MIN_PASSES:-19}" \
scripts/run_prd05a_wallet_mock_soak.sh custom
scripts/run_prd05a_wallet_mock_runtime_slo.sh
scripts/run_prd05a_wallet_mock_determinism.sh
scripts/run_prd05a_wallet_mock_replay.sh
scripts/run_prd05a_metamask_canary.sh --profile-check || true

if [[ "${PRD05A_REQUIRE_MANUAL_SANITY:-0}" == "1" ]]; then
  scripts/run_prd05a_manual_metamask_checklist.sh verify
elif [[ ! -f local/reports/prd05a/C5-manual-metamask-sanity.md ]]; then
  scripts/run_prd05a_manual_metamask_checklist.sh template
fi

mkdir -p local/reports/prd05a
cat >"$summary_md" <<EOF
# C5 Release Evidence Index

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
10. Wallet-mock preflight report: local/reports/prd05a/C5-e0-determinism-report.md
11. Wallet-mock blocking gate report: local/reports/prd05a/C5-wallet-mock-gate-report.md
12. Wallet-mock soak report: local/reports/prd05a/C5-wallet-mock-soak-report.md
13. Wallet-mock runtime SLO report: local/reports/prd05a/C5-wallet-mock-runtime-slo-report.md
14. Wallet-mock determinism report: local/reports/prd05a/C5-wallet-mock-determinism-report.md
15. Wallet-mock replay report: local/reports/prd05a/C5-wallet-mock-replay-report.md
16. MetaMask canary report (non-blocking): local/reports/prd05a/C5-metamask-canary-report.md
17. Manual MetaMask sanity checklist (required at RC): local/reports/prd05a/C5-manual-metamask-sanity.md

## Milestone Discipline

- Continuation milestones tracked in prds/05A-CONTINUATION-MILESTONES.md
- Release checklist tracked in prds/05A-RELEASE-GATE-CHECKLIST.md
- Branch naming and E*-T* commit traceability enforced by CI checks
EOF

cat >"$summary_json" <<EOF
{
  "schema_version": "${PRD05A_SCHEMA_VERSION}",
  "generated": "${timestamp}",
  "run_id": "${run_id}",
  "status": "PASS",
  "gate_model": {
    "blocking": "wallet-mock",
    "canary": "metamask",
    "manual_release_sanity": "MANUAL-MM-001..004"
  },
  "reports": {
    "performance": "local/reports/prd05a/C6-performance-report.md",
    "differential": "local/reports/prd05a/C9-differential-parity-report.md",
    "safe_service_live": "local/reports/prd05a/C2-safe-service-live-report.md",
    "wallet_mock_preflight": "local/reports/prd05a/C5-e0-determinism-report.md",
    "wallet_mock_gate": "local/reports/prd05a/C5-wallet-mock-gate-report.md",
    "wallet_mock_gate_json": "local/reports/prd05a/C5-wallet-mock-gate.json",
    "wallet_mock_soak": "local/reports/prd05a/C5-wallet-mock-soak-report.md",
    "wallet_mock_soak_json": "local/reports/prd05a/C5-wallet-mock-soak-report.json",
    "wallet_mock_runtime_slo": "local/reports/prd05a/C5-wallet-mock-runtime-slo-report.md",
    "wallet_mock_runtime_slo_json": "local/reports/prd05a/C5-wallet-mock-runtime-slo-report.json",
    "wallet_mock_determinism": "local/reports/prd05a/C5-wallet-mock-determinism-report.md",
    "wallet_mock_determinism_json": "local/reports/prd05a/C5-wallet-mock-determinism-report.json",
    "wallet_mock_replay": "local/reports/prd05a/C5-wallet-mock-replay-report.md",
    "wallet_mock_replay_json": "local/reports/prd05a/C5-wallet-mock-replay-report.json",
    "metamask_canary": "local/reports/prd05a/C5-metamask-canary-report.md",
    "metamask_canary_json": "local/reports/prd05a/C5-metamask-canary-report.json",
    "manual_metamask_sanity": "local/reports/prd05a/C5-manual-metamask-sanity.md",
    "phase_discipline": "local/reports/prd05a/C5-phase-discipline-report.md",
    "release_checklist": "prds/05A-RELEASE-GATE-CHECKLIST.md"
  },
  "artifacts": {
    "markdown_summary": "${summary_md}",
    "json_summary": "${summary_json}"
  }
}
EOF

echo "wrote ${summary_md}"
echo "wrote ${summary_json}"
