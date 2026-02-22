#!/usr/bin/env bash

# Common helpers for PRD05A wallet E2E scripts.

set -euo pipefail

PRD05A_SCHEMA_VERSION="c5e2e-v1"

prd05a_now_utc() {
  date -u +"%Y-%m-%dT%H:%M:%SZ"
}

prd05a_run_id() {
  date -u +"run-%Y%m%dT%H%M%SZ"
}

prd05a_is_linux() {
  [[ "$(uname -s)" == "Linux" ]]
}

prd05a_should_use_xvfb() {
  if ! prd05a_is_linux; then
    return 1
  fi

  # Deterministic default for extension E2E: force xvfb unless explicitly disabled.
  local force_xvfb="${PRD05A_FORCE_XVFB:-1}"
  if [[ "$force_xvfb" == "1" || "$force_xvfb" == "true" ]]; then
    return 0
  fi

  [[ -z "${DISPLAY:-}" ]]
}

prd05a_with_display() {
  if prd05a_should_use_xvfb; then
    if ! command -v xvfb-run >/dev/null 2>&1; then
      return 127
    fi
    xvfb-run --auto-servernum --server-args="-screen 0 1920x1080x24" "$@"
    return $?
  fi
  "$@"
}

prd05a_resolve_node() {
  local node_bin="${PRD05A_NODE_BIN:-}"
  if [[ -z "$node_bin" ]]; then
    if [[ -x "$HOME/.nvm/versions/node/v20.19.6/bin/node" ]]; then
      node_bin="$HOME/.nvm/versions/node/v20.19.6/bin/node"
    elif command -v node >/dev/null 2>&1; then
      node_bin="$(command -v node)"
    fi
  fi
  if [[ -z "$node_bin" || ! -x "$node_bin" ]]; then
    return 1
  fi
  echo "$node_bin"
}

prd05a_node_major() {
  local node_bin="$1"
  "$node_bin" -p 'process.versions.node.split(".")[0]'
}

prd05a_node_version() {
  local node_bin="$1"
  "$node_bin" -v
}

prd05a_chromium_version() {
  local chromium_bin="${PRD05A_CHROMIUM_BIN:-chromium}"
  if ! command -v "$chromium_bin" >/dev/null 2>&1; then
    chromium_bin="${PRD05A_CHROMIUM_BIN_FALLBACK:-google-chrome}"
  fi

  local version="$("$chromium_bin" --version 2>/dev/null || true)"
  if [[ -z "$version" ]]; then
    version="NOT_AVAILABLE"
  fi
  echo "${chromium_bin}|${version}"
}

prd05a_json_escape() {
  local value="${1:-}"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  value="${value//$'\n'/\\n}"
  value="${value//$'\r'/\\r}"
  value="${value//$'\t'/\\t}"
  echo "$value"
}

prd05a_write_json() {
  local path="$1"
  local schema_version="$2"
  local generated="$3"
  local run_id="$4"
  local status="$5"
  local taxonomy="$6"
  local driver_mode="$7"
  local release_gate_driver="$8"
  local node_version="$9"
  local chromium_bin="${10}"
  local chromium_version="${11}"
  local locale="${12}"
  local artifacts_json="${13}"
  local triage_label="${14}"
  local reason="${15}"
  local gate_tier="${16:-${PRD05A_GATE_MODE:-blocking}}"
  local gate_effect="${17:-}"
  if [[ -z "$gate_effect" ]]; then
    case "$gate_tier" in
      canary) gate_effect="CANARY" ;;
      manual) gate_effect="MANUAL" ;;
      *) gate_effect="BLOCKING" ;;
    esac
  fi

  cat >"$path" <<EOF
{
  "schema_version": "$(prd05a_json_escape "$schema_version")",
  "generated": "$(prd05a_json_escape "$generated")",
  "run_id": "$(prd05a_json_escape "$run_id")",
  "status": "$(prd05a_json_escape "$status")",
  "taxonomy": "$(prd05a_json_escape "$taxonomy")",
  "driver_mode": "$(prd05a_json_escape "$driver_mode")",
  "release_gate_driver": "$(prd05a_json_escape "$release_gate_driver")",
  "gate_tier": "$(prd05a_json_escape "$gate_tier")",
  "gate_effect": "$(prd05a_json_escape "$gate_effect")",
  "triage_label": "$(prd05a_json_escape "$triage_label")",
  "node_version": "$(prd05a_json_escape "$node_version")",
  "chromium_bin": "$(prd05a_json_escape "$chromium_bin")",
  "chromium_version": "$(prd05a_json_escape "$chromium_version")",
  "locale": "$(prd05a_json_escape "$locale")",
  "reason": "$(prd05a_json_escape "$reason")",
  "artifacts": ${artifacts_json}
}
EOF
}
