#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

required_ids=(
  "PARITY-TX-01"
  "PARITY-TX-02"
  "PARITY-MSG-01"
  "PARITY-WC-01"
  "PARITY-ABI-01"
  "PARITY-COLLAB-01"
  "PARITY-HW-01"
)

trace_file="local/reports/prd05a/parity-traceability.md"
prd_file="prds/05A-PRD-PARITY-WAVE.md"

if [[ ! -f "$prd_file" ]]; then
  echo "missing PRD file: $prd_file"
  exit 1
fi

if [[ ! -f "$trace_file" ]]; then
  echo "missing traceability report: $trace_file"
  exit 1
fi

if rg -n "PENDING_COMMIT" "$trace_file" >/dev/null 2>&1; then
  echo "traceability report still contains PENDING_COMMIT placeholder"
  exit 1
fi

for id in "${required_ids[@]}"; do
  if ! rg -n "$id" "$prd_file" >/dev/null 2>&1; then
    echo "missing parity id in PRD: $id"
    exit 1
  fi
  if ! rg -n "$id" "$trace_file" >/dev/null 2>&1; then
    echo "missing parity id in traceability report: $id"
    exit 1
  fi
done

echo "[traceability] PRD and parity traceability report contain all mandatory PARITY IDs"
