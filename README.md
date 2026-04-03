# cpsat-rs

Safe, ergonomic Rust wrapper for [Google OR-Tools](https://developers.google.com/optimization/) CP-SAT solver.

Build constraint programming models in safe Rust, solve with the full power of OR-Tools CP-SAT (CDCL, LP relaxation, LNS, FeasibilityJump, parallel workers).

## Architecture

All model construction happens in **pure safe Rust**. The only FFI call is a single `extern "C"` function that passes serialized protobuf bytes to OR-Tools and receives response bytes back. Zero C++ object lifetime management. Two `unsafe` blocks in one file.

```
Safe Rust API (CpModel, IntVar, constraints)
  ↓
Generated prost types (CpModelProto)
  ↓
Single extern "C" call (SolveCpModelWithParameters)
  ↓
OR-Tools C API (libortools.dylib)
```

## Quick Start

```rust
use cpsat_rs::prelude::*;

let mut model = CpModel::new();
let x = model.new_int_var(0..=10, "x");
let y = model.new_int_var(0..=10, "y");
model.add((x + y).le(15));
model.minimize(x + y);

let response = CpSolver::solve(&model).unwrap();
if response.is_optimal() {
    println!("x={}, y={}", response.value(x), response.value(y));
}
```

## Requirements

Google OR-Tools C++ library must be installed:

```sh
# macOS
brew install or-tools

# Set ORTOOLS_PREFIX if not in a standard location
export ORTOOLS_PREFIX=$(brew --prefix or-tools)
```

## Features

- **Variables**: `IntVar`, `BoolVar` with arbitrary domains
- **Expressions**: `LinearExpr` with `+`, `-`, `*` operators
- **Constraints**: linear, all_different, no_overlap, cumulative, circuit, table, automaton, element, bool_or/and/exactly_one, implication
- **Scheduling**: `IntervalVar`, `add_no_overlap`, `add_no_overlap_2d`, `add_cumulative`
- **Objectives**: minimize/maximize linear expressions
- **Parameters**: time limits, worker count, logging, random seed
- **Solution hints**: warm-start from known solutions

## Examples

```sh
cargo run --example nqueens      # 8-Queens puzzle
cargo run --example jobshop      # FT06 job shop scheduling
```

## License

Apache-2.0 (same as OR-Tools)
