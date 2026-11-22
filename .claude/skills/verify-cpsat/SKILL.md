---
name: verify-cpsat
description: Verify cpsat-rs end to end across CP-SAT solver constraints, JSP benchmarks, examples, property/safety invariants, strict Clippy/rustdoc checks, and crate packaging.
---

# verify-cpsat

Project-local verification skill and deterministic runner for `cpsat-rs`.
See [features/README.md](features/README.md) for the complete feature map and per-feature expectations.

## Launch

Compile all library, test, and example targets cleanly offline/locked before running verification suites:

```bash
./.claude/skills/verify-cpsat/scripts/verify-cpsat.sh launch
```

This runs `cargo test --no-run --all-targets --locked` from the repository root and confirms the protobuf compiler and `libortools` link cleanly.

## Doctor

Check that the Rust toolchain, `cargo-nextest`, `protoc`, `jq`, OR-Tools C++ headers/libraries, and `/tmp` scratch directories are ready before driving suites:

```bash
./.claude/skills/verify-cpsat/scripts/verify-cpsat.sh doctor
```

Doctor checks:
1. `cargo`, `cargo-nextest`, `protoc`, and `jq` are available on `PATH`.
2. An OR-Tools installation containing `include/ortools` exists via `ORTOOLS_PREFIX`, `ORTOOL_PREFIX`, Homebrew (`/opt/homebrew/opt/or-tools`, `/usr/local/opt/or-tools`), or standard system paths (`/usr`, `/usr/local`, `/opt/or-tools`, `/opt/ortools`).
3. `/tmp` is writable for isolated work dirs (`/tmp/cpsat-verify-work/`) and proof receipts (`/tmp/cpsat-verify-proofs/`).

## Drive

Run a single feature suite or the full matrix and record durable proof artifacts:

```bash
./.claude/skills/verify-cpsat/scripts/verify-cpsat.sh drive all proof-run-1
```

Available feature selectors:
- `solver-and-constraints` — validates `smoke`, `solution_validation`, and crate doc-tests.
- `scheduling-and-benchmarks` — validates `scheduling`, `jsp_benchmarks`, and the `nqueens` and `jobshop` examples.
- `properties-and-safety` — validates `proptest_roundtrip`, debug unit tests, and `--release` overflow/FFI unit tests.
- `lint-doc-and-package` — validates `cargo fmt --check`, strict `clippy`, `RUSTDOCFLAGS="-D warnings" cargo doc`, and `cargo package --list` exclusions.
- `all` — executes all four suites in order under one `run-id`.

## Evidence

Every command executed by `drive` stores immutable proof artifacts under `/tmp/cpsat-verify-proofs/<run-id>/`:
- `<step>/command.txt` — exact command line executed inside the repository root.
- `<step>/stdout.txt` — captured stdout stream.
- `<step>/stderr.txt` — captured stderr stream.
- `<step>/exit_code.txt` — observed process exit code.
- `summary.json` — machine-readable pass summary across all executed steps and features.

Proof directories are never deleted by `cleanup` so receipts remain inspectable after disposable work directories are removed.

## Cleanup

Remove the temporary work directory while preserving all proof receipts:

```bash
./.claude/skills/verify-cpsat/scripts/verify-cpsat.sh cleanup proof-run-1
```

This removes `/tmp/cpsat-verify-work/<run-id>/` and verifies `/tmp/cpsat-verify-proofs/<run-id>/summary.json` remains intact.

## Helpers

- Runner script: [`scripts/verify-cpsat.sh`](scripts/verify-cpsat.sh)
- Feature index: [`features/README.md`](features/README.md)
- Optional override: set `REPO_ROOT` to target another checkout path, and always set `PAGER=cat` when running `cargo nextest list` interactively or in scripts.
