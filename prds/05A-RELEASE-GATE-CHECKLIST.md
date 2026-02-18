# PRD 05A Release Gate Checklist

Status: Draft  
Owner: Rusty Safe

## Required Evidence

### 1. Security

- [ ] Security review completed for C2/C3/C4 runtime integrations.
- [ ] No open critical/high findings.
- [ ] Signature-context and replay protections verified.

### 2. Compatibility

- [ ] Chromium + MetaMask matrix pass.
- [ ] Chromium + Rabby matrix pass.
- [ ] Ledger passthrough smoke pass.
- [ ] Trezor passthrough smoke pass.

### 3. Functional Parity

- [x] `PARITY-TX-01` complete.
- [x] `PARITY-TX-02` complete.
- [x] `PARITY-MSG-01` complete.
- [x] `PARITY-WC-01` complete.
- [x] `PARITY-ABI-01` complete.
- [x] `PARITY-COLLAB-01` complete.
- [ ] `PARITY-HW-01` runtime proof complete.

### 4. Performance

- [x] Command latency p95 <= 150ms.
- [x] Rehydration latency p95 <= 1500ms.
- [ ] No regressions beyond agreed tolerance.

### 5. CI Gates

- [x] `scripts/check_signing_boundaries.sh` passes.
- [x] `scripts/check_prd05a_traceability.sh` passes.
- [x] `cargo fmt --all -- --check` passes.
- [x] Strict clippy for signing crates passes.
- [x] `cargo test --workspace` passes.

### 6. Milestone/Tag Discipline

- [ ] All continuation milestones have `-gate-green` commits.
- [ ] Required tags (`prd05a-<milestone>-gate`) created.
- [ ] Branch closure report completed.

## Sign-off

- Engineering Lead: __________________ Date: __________
- Security Reviewer: _________________ Date: __________
- Product Owner: _____________________ Date: __________
