#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

timestamp="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
base_url="${PRD05A_SAFE_SERVICE_LIVE_BASE_URL:-https://safe-transaction-sepolia.safe.global}"
timeout_s="${PRD05A_SAFE_SERVICE_LIVE_TIMEOUT_S:-20}"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

curl_json() {
  local method="$1"
  local url="$2"
  local body_file="${3:-}"
  local out_file="$4"
  if [[ -n "$body_file" ]]; then
    curl -sSL -m "$timeout_s" -o "$out_file" -w '%{http_code}' \
      -H 'Content-Type: application/json' -X "$method" --data @"$body_file" "$url"
  else
    curl -sSL -m "$timeout_s" -o "$out_file" -w '%{http_code}' -X "$method" "$url"
  fi
}

index_file="$tmp_dir/indexing.json"
index_status="$(curl_json GET "${base_url}/api/v1/about/indexing/" "" "$index_file")"
index_synced="$(jq -r '.synced // "unknown"' "$index_file" 2>/dev/null || echo "unknown")"

missing_hash="0x0000000000000000000000000000000000000000000000000000000000000000"
missing_file="$tmp_dir/missing_tx.json"
missing_status="$(curl_json GET "${base_url}/api/v1/multisig-transactions/${missing_hash}/" "" "$missing_file")"

proposal_req="$tmp_dir/proposal.json"
cat >"$proposal_req" <<'EOF'
{"to":"0x0000000000000000000000000000000000000000","value":"0","data":"0x","operation":0,"safeTxGas":"0","baseGas":"0","gasPrice":"0","gasToken":"0x0000000000000000000000000000000000000000","refundReceiver":"0x0000000000000000000000000000000000000000","nonce":"0","contractTransactionHash":"0x1111111111111111111111111111111111111111111111111111111111111111","sender":"0x0000000000000000000000000000000000000000","signature":"0x","origin":"rusty-safe-live-check"}
EOF
proposal_file="$tmp_dir/proposal_resp.json"
proposal_status="$(curl_json POST "${base_url}/api/v1/safes/0x0000000000000000000000000000000000000000/multisig-transactions/" "$proposal_req" "$proposal_file")"

confirm_req="$tmp_dir/confirm.json"
cat >"$confirm_req" <<'EOF'
{"signature":"0x1111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111"}
EOF
confirm_file="$tmp_dir/confirm_resp.json"
confirm_status="$(curl_json POST "${base_url}/api/v1/multisig-transactions/0x1111111111111111111111111111111111111111111111111111111111111111/confirmations/" "$confirm_req" "$confirm_file")"

overall="PASS"
if [[ "$index_status" != "200" ]]; then
  overall="FAIL"
fi
if [[ "$missing_status" != "404" ]]; then
  overall="FAIL"
fi
if [[ "$proposal_status" != "400" && "$proposal_status" != "422" && "$proposal_status" != "429" ]]; then
  overall="FAIL"
fi
if [[ "$confirm_status" != "404" && "$confirm_status" != "400" && "$confirm_status" != "429" ]]; then
  overall="FAIL"
fi

mkdir -p local/reports/prd05a
cat > local/reports/prd05a/C2-safe-service-live-report.md <<EOF
# C2 Safe Service Live Validation Report

Generated: ${timestamp}

## Target

- Base URL: \`${base_url}\`
- Timeout: \`${timeout_s}s\`
- Overall: **${overall}**

## Probes

| Probe | Expected | Actual | Result |
|---|---|---|---|
| Indexing endpoint | 200 | ${index_status} | $( [[ "$index_status" == "200" ]] && echo "PASS" || echo "FAIL" ) |
| Unknown tx fetch | 404 | ${missing_status} | $( [[ "$missing_status" == "404" ]] && echo "PASS" || echo "FAIL" ) |
| Invalid propose payload | 400/422/429 | ${proposal_status} | $( [[ "$proposal_status" == "400" || "$proposal_status" == "422" || "$proposal_status" == "429" ]] && echo "PASS" || echo "FAIL" ) |
| Unknown tx confirmation | 404/400/429 | ${confirm_status} | $( [[ "$confirm_status" == "404" || "$confirm_status" == "400" || "$confirm_status" == "429" ]] && echo "PASS" || echo "FAIL" ) |

## Observations

- Service synced: \`${index_synced}\`
- Unknown tx response: \`$(jq -c '.' "$missing_file" 2>/dev/null | head -c 160)\`
- Invalid propose response: \`$(jq -c '.' "$proposal_file" 2>/dev/null | head -c 160)\`
- Confirmation response: \`$(jq -c '.' "$confirm_file" 2>/dev/null | head -c 160)\`

## Repro

\`\`\`bash
scripts/run_prd05a_safe_service_live.sh
\`\`\`
EOF

echo "wrote local/reports/prd05a/C2-safe-service-live-report.md"
