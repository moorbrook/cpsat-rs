//! # `cpsat-rs` — Rust bindings for Google OR-Tools CP-SAT solver.
//!
//! Build constraint programming models in Rust and solve them with OR-Tools
//! CP-SAT (CDCL, LP relaxation, LNS, `FeasibilityJump`).
//!
//! ## Architecture
//!
//! Model construction happens in Rust using prost-generated protobuf types.
//! Solving delegates through `src/ffi.rs` (`extern "C"` call to
//! `SolveCpModelWithParameters`) with serialized protobuf request/response
//! buffers and an RAII `free` drop guard, with zero C++ object lifetime
//! management across FFI.
//!
//! ## Quick Start
//!
//! ```no_run
//! use cpsat_rs::prelude::*;
//!
//! let mut model = CpModel::new();
//! let x = model.new_int_var(0..=10, "x");
//! let y = model.new_int_var(0..=10, "y");
//! model.add((x + y).le(15));
//! model.minimize(x + y);
//!
//! let response = CpSolver::solve(&model).unwrap();
//! if response.is_optimal() {
//!     println!("x={}, y={}", response.value(x), response.value(y));
//! }
//! ```
//!
//! ## Requirements
//!
//! Requires Google OR-Tools C++ library installed:
//! ```sh
//! brew install or-tools  # macOS
//! ```

pub mod error;
pub mod expressions;
mod ffi;
pub mod model;
pub mod params;
pub mod solver;
pub mod vars;

#[allow(missing_docs, clippy::pedantic, clippy::all)]
pub(crate) mod proto {
    include!(concat!(env!("OUT_DIR"), "/operations_research.sat.rs"));
}

/// Prelude for convenient imports.
pub mod prelude {
    pub use crate::error::SolveError;
    pub use crate::expressions::{BoundedLinearExpr, LinearExpr};
    pub use crate::model::CpModel;
    pub use crate::proto::{CpSolverStatus, SatParameters};
    pub use crate::solver::{CpSolver, SolveResponse};
    pub use crate::vars::{BoolVar, IntVar, IntervalVar};
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    const fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn public_types_are_send_and_sync() {
        assert_send_sync::<CpModel>();
        assert_send_sync::<SolveResponse>();
        assert_send_sync::<SolveError>();
        assert_send_sync::<IntVar>();
        assert_send_sync::<BoolVar>();
        assert_send_sync::<IntervalVar>();
        assert_send_sync::<LinearExpr>();
        assert_send_sync::<BoundedLinearExpr>();
    }
}
