# Feature: Solver and Constraints

## Sub-features
- Integer (`IntVar`), Boolean (`BoolVar`), constant, and linear expression modeling (`LinearExpr`, `BoundedLinearExpr`, `CpModel::add`).
- Global and logical constraints verified end to end through OR-Tools FFI: `add_all_different`, `add_implication`, interval linking (`start + size == end`), optional interval presence enforcement, and variable-size intervals.
- Rustdoc compilation checks covering crate root and `CpModel` quick-start snippets.

## How to get to it (user POV)
- Add `cpsat-rs = "0.2.0"` to a Cargo project with OR-Tools v9.15 and `protoc` installed, build models via `cpsat_rs::prelude::*`, and call `CpSolver::solve(&model)`.
- Run `cargo nextest run --test smoke --test solution_validation --locked` and `cargo test --doc --locked`.

## Driving it with verify-cpsat.sh
```bash
./.claude/skills/verify-cpsat/scripts/verify-cpsat.sh drive solver-and-constraints proof-solver
```
The runner executes `smoke` (6 tests), `solution_validation` (6 tests verifying actual constraint satisfaction rather than objective coincidence), and `cargo test --doc` (2 doc-tests), saving raw command/stream receipts under `/tmp/cpsat-verify-proofs/proof-solver/`.

## Gotchas
- `CpModel` variable and constraint indices are `i32` handles scoped to a single model; mixing handles across models is a modeling bug.
- OR-Tools must be discoverable at build and runtime (`ORTOOLS_PREFIX` or standard system/Homebrew paths, and `DYLD_LIBRARY_PATH` / `LD_LIBRARY_PATH` when installed in a non-standard prefix).
