# Feature: Properties and Safety

## Sub-features
- Property-based tests (`tests/proptest_roundtrip.rs`): protobuf roundtrip fidelity (`CpModel::to_bytes` / `from_bytes`), valid domain handling, `min`/`max` objective bounds, `BoolVar` double-negation & sign invariants, and interval minimization.
- Checked arithmetic & release-mode safety tests (`src/model.rs` `#[cfg(test)]`): `i64::MAX + 1` disjoint interval guard (`domain_i64_max_interval_panics`) and `i64::MIN` interval linking offset negation (`interval_linking_offset_i64_min_panics`) verified under `--release`.
- FFI boundary unit tests (`src/ffi.rs` `#[cfg(test)]`): rejecting negative C response buffer lengths (`SolveError::FfiError`), `i32` model size overflow (`SolveError::ModelTooLarge`), RAII `MallocGuard` cleanup, and compile-time `Send + Sync` public type assertions (`src/lib.rs`).

## How to get to it (user POV)
- Construct non-trivial disjoint domains cleanly via `CpModel::new_int_var(vec![(0, 5), (10, 20)], "x")` and rely on explicit panics rather than silent wrap/saturation on extreme `i64` bounds.
- Run `cargo nextest run --test proptest_roundtrip --lib --locked` and `cargo test --release --lib --locked`.

## Driving it with verify-cpsat.sh
```bash
./.claude/skills/verify-cpsat/scripts/verify-cpsat.sh drive properties-and-safety proof-safety
```
The runner executes all 8 proptest suites and 5 unit tests in debug mode via `cargo-nextest`, then explicitly runs the 5 library tests under `--release` so checked arithmetic (`checked_add` / `checked_neg`) is proven when integer overflow checks are disabled.

## Gotchas
- Debug-only test runs do not distinguish checked arithmetic from wrapping `+ 1`; always run `cargo test --release --lib` when verifying `IntoDomain` and `add_interval_linking`.
- `MallocGuard` assumes OR-Tools allocates `cres` with C `malloc` (`cp_solver_c.cc`); any custom OR-Tools build using a non-matching allocator on Windows would require matching CRT linkage.
