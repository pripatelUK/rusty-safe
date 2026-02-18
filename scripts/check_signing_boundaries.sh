#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[boundary] checking forbidden deps in rusty-safe-signing-core"
forbidden=(egui eframe reqwest web-sys tokio)
core_tree="$(cargo tree -p rusty-safe-signing-core --depth 1 --prefix none 2>/dev/null || true)"
for dep in "${forbidden[@]}"; do
  if echo "$core_tree" | grep -E "^${dep}( |$)" >/dev/null 2>&1; then
    echo "forbidden dependency detected in core: $dep"
    exit 1
  fi
done

echo "[boundary] checking shell import boundaries"
if rg -n "rusty_safe_signing_adapters::" crates/rusty-safe/src/app.rs >/dev/null 2>&1; then
  echo "app.rs imports adapters directly; use signing_bridge only"
  exit 1
fi

if rg -n "crate::signing_ui|egui|eframe" crates/rusty-safe-signing-core/src >/dev/null 2>&1; then
  echo "core crate references UI shell modules"
  exit 1
fi

echo "[boundary] checks passed"
