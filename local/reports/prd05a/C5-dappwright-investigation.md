# C5 dappwright Investigation (Driver Arbitration Report)

Generated: 2026-02-21T14:24:32Z
Run ID: run-20260221T142432Z
Schema: c5e2e-v1

## Driver Modes

- Supported modes: `synpress`, `dappwright`, `mixed`
- Release-gate driver policy (current): `synpress`
- Promotion criteria: dappwright promotion requires >=95% pass in 20-run CI soak and zero HARNESS_FAIL in 2 consecutive daily runs.
- Fallback policy: if dappwright fails bootstrap/connect/network probes, release-gate driver remains synpress.

## Comparative Reliability (Bootstrap / Connect / Network)

| Driver | Bootstrap | Connect | Network |
|---|---|---|---|
| synpress | PASS (NONE) | FAIL (HARNESS_FAIL) | FAIL (HARNESS_FAIL) |
| dappwright | PASS (NONE) | FAIL (HARNESS_FAIL) | FAIL (HARNESS_FAIL) |
| mixed | PASS (NONE) | FAIL (HARNESS_FAIL) | FAIL (HARNESS_FAIL) |

## Probe Reasons

- synpress/bootstrap: Runtime profile check passed.
- synpress/connect: probe timed out after 90s
- synpress/network: probe timed out after 90s
- dappwright/bootstrap: Runtime profile check passed.
- dappwright/connect: probe timed out after 90s
- dappwright/network: probe timed out after 90s
- mixed/bootstrap: Runtime profile check passed.
- mixed/connect: probe timed out after 90s
- mixed/network: probe timed out after 90s

## Artifacts

- Probe run directory: `local/reports/prd05a/driver-compare/run-20260221T142432Z`
- JSON report: `local/reports/prd05a/C5-dappwright-investigation.json`
- Reproducer command: `scripts/run_prd05a_driver_comparison.sh`
