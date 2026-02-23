# C6 Performance Report

Generated: 2026-02-23T18:35:38Z

## Result

- Command p95: 0ms (budget 150ms)
- Rehydration p95: 0ms (budget 1500ms)

## Evidence

- Command: `cargo test -p rusty-safe-signing-adapters --test performance_budget -- --nocapture`
- Raw marker: `PERF command_p95_ms=0 rehydration_p95_ms=0 budget_command_ms=150 budget_rehydration_ms=1500`
