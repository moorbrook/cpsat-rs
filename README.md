# cpsat-rs

Rust bindings for [Google OR-Tools](https://developers.google.com/optimization/) CP-SAT solver.

Provides a Rust builder API for constructing CP-SAT models, with solving delegated to OR-Tools via FFI.

## How it works

Model construction happens in Rust using prost-generated protobuf types. The model is serialized, passed to OR-Tools' C API (`SolveCpModelWithParameters`), and the response is deserialized back. There are two `unsafe` blocks in `src/ffi.rs` for this boundary.

```
Rust builder API → CpModelProto (prost) → serialize → C FFI → OR-Tools → deserialize → response
```

This is not a pure-Rust solver. It requires the OR-Tools C++ library installed on the system.

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

OR-Tools C++ library must be installed:

```sh
# macOS
brew install or-tools

# Linux: see https://developers.google.com/optimization/install

# If not in a standard location:
export ORTOOLS_PREFIX=/path/to/or-tools
```

Also requires `protoc` for proto compilation during build.

## What's included

- `IntVar`, `BoolVar`, `IntervalVar` handle types
- `LinearExpr` with arithmetic operators
- Constraints: linear, all_different, no_overlap, cumulative, circuit, table, automaton, element, boolean
- Scheduling: interval variables, no_overlap, no_overlap_2d, cumulative
- Minimize/maximize objectives
- Solution hints
- Solver parameters (time limit, workers, etc.)

## Examples

```sh
cargo run --example nqueens      # 8-Queens puzzle
cargo run --example jobshop      # FT06 job shop scheduling (optimal=55)
```

## Limitations

- Requires OR-Tools system library (not pure Rust)
- FFI boundary: two `unsafe` blocks for serialize/deserialize + free
- No solve cancellation (blocking FFI call)
- Variable handles are plain indices with no model provenance check
- Proto files vendored from OR-Tools v9.15

## License

Apache-2.0
