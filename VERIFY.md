# Build Verification

This page explains how you can verify that RustySafe is exactly what it claims to be.

## Why Does This Matter?

When you use a web application, you're trusting that the code running in your browser matches what the developers published. With RustySafe, you don't have to trust us — you can verify it yourself.

This is similar to how Etherscan lets you verify smart contract code matches what's deployed on Ethereum.

---

## Quick Check

View the current build information directly:

- **[View Build Info](https://rustysafe.com/BUILD_INFO.txt)** — Shows the WASM fingerprint, commit, and build time
- **[View Source Code](https://github.com/pripatelUK/rusty-safe)** — The complete open-source code
- **[GitHub Attestation](https://github.com/pripatelUK/rusty-safe/attestations)** — Cryptographic proof from GitHub

---

## What the Build Info Means

| Field | What It Means |
|-------|---------------|
| **WASM File** | The filename of the application (includes a unique hash). |
| **WASM SHA256** | A unique fingerprint of the application. If even one character of code changes, this fingerprint changes completely. |
| **Commit** | The exact version of the source code used to build this application. You can click through to see every line of code. |
| **Build Time** | When this version was built. |

---

## How to Verify (Non-Technical)

1. **Open the Build Info** — Visit [rustysafe.com/BUILD_INFO.txt](https://rustysafe.com/BUILD_INFO.txt)
2. **Note the Commit** — Click the commit link to see the exact source code on GitHub
3. **Check the Attestation** — GitHub provides cryptographic proof that this build came from that exact commit

If the attestation is valid, you can be confident that the code you're running matches the public source code.

---

## Why You Can Trust This

- **Open Source** — All code is publicly available on GitHub
- **Reproducible Builds** — Anyone can rebuild the app and get the same fingerprint
- **GitHub Attestation** — Cryptographic proof that GitHub built this from the specified commit
- **No Backdoors** — You can inspect every line of code yourself

---

---

# Technical Verification Guide

*This section is for developers who want to perform full verification.*

## Verify the WASM Hash

1. Get the WASM filename from [BUILD_INFO.txt](https://rustysafe.com/BUILD_INFO.txt)
2. Download the deployed WASM and compute its SHA256 hash:

```bash
# Use the WASM File from BUILD_INFO.txt
curl -o app.wasm https://rustysafe.com/<WASM_FILE>
sha256sum app.wasm
```

Compare the output with the hash published at [rustysafe.com/BUILD_INFO.txt](https://rustysafe.com/BUILD_INFO.txt).

---

## Reproducible Build

Clone the repository and build using the exact same Docker environment:

```bash
# Clone and checkout the specific commit
git clone https://github.com/pripatelUK/rusty-safe
cd rusty-safe
git checkout <commit-hash-from-BUILD_INFO.txt>

# Build using Docker (pins exact Rust/trunk versions)
docker build -f Dockerfile.build -t builder .
docker create --name temp builder
docker cp temp:/build/crates/rusty-safe/dist .
docker rm temp

# Compare hashes
sha256sum dist/*.wasm
```

The hash should match the deployed version.

---

## Verify GitHub Attestation

Using the GitHub CLI:

```bash
# Use the WASM File from BUILD_INFO.txt
curl -o app.wasm https://rustysafe.com/<WASM_FILE>
gh attestation verify app.wasm -R pripatelUK/rusty-safe
```

This cryptographically proves the WASM was built by GitHub Actions from the specified commit.

---

## Build Environment

The Docker build environment pins:
- **Rust**: 1.83.0
- **Target**: wasm32-unknown-unknown
- **Build Tool**: trunk (latest locked version)

See [Dockerfile.build](https://github.com/pripatelUK/rusty-safe/blob/main/Dockerfile.build) for the complete specification.
