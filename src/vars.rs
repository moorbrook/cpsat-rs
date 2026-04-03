//! Variable handle types.
//!
//! These are `Copy` index newtypes, not borrows. They can be freely
//! stored in `Vec`, `HashMap`, passed to functions, etc. Using a handle
//! from one model in another model is a logic error (debug assertions
//! may catch this in the future).

/// Handle to an integer variable in a `CpModel`.
/// Inner value is the index into `CpModelProto.variables`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntVar(pub(crate) i32);

/// Handle to a Boolean variable in a `CpModel`.
/// A `BoolVar` is an `IntVar` with domain `[0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoolVar(pub(crate) i32);

impl BoolVar {
    /// Get the negation of this Boolean variable.
    /// In CP-SAT, negation is encoded as the bitwise NOT of the variable index.
    #[must_use]
    pub fn negated(self) -> Self {
        Self(!self.0)
    }

    /// Convert to the underlying integer variable.
    #[must_use]
    pub fn as_int_var(self) -> IntVar {
        IntVar(self.0)
    }

    /// Get the raw literal index (may be negative for negated literals).
    #[must_use]
    pub fn index(self) -> i32 {
        self.0
    }
}

impl std::ops::Not for BoolVar {
    type Output = Self;
    fn not(self) -> Self {
        self.negated()
    }
}

impl From<BoolVar> for IntVar {
    fn from(b: BoolVar) -> Self {
        IntVar(b.0)
    }
}

/// Handle to an interval variable in a `CpModel`.
/// Inner value is the index into `CpModelProto.constraints`
/// (the constraint that defines the interval).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntervalVar(pub(crate) i32);
