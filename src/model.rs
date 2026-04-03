//! CpModel builder — the main entry point for constructing optimization models.

use crate::expressions::{BoundedLinearExpr, LinearExpr};
use crate::proto::{self, constraint_proto, CpModelProto, CpObjectiveProto, ConstraintProto,
    IntegerVariableProto, LinearConstraintProto, IntervalConstraintProto,
    NoOverlapConstraintProto, NoOverlap2DConstraintProto, CumulativeConstraintProto,
    AllDifferentConstraintProto, CircuitConstraintProto, TableConstraintProto,
    AutomatonConstraintProto, ElementConstraintProto, BoolArgumentProto,
    LinearExpressionProto, PartialVariableAssignment, LinearArgumentProto};
use crate::vars::{BoolVar, IntVar, IntervalVar};
use prost::Message;

/// Domain specification for integer variables.
pub trait IntoDomain {
    /// Convert to the flat domain representation used by CP-SAT proto.
    /// Format: pairs of [lb, ub] intervals, flattened into a single Vec.
    fn into_domain(self) -> Vec<i64>;
}

impl IntoDomain for (i64, i64) {
    fn into_domain(self) -> Vec<i64> {
        vec![self.0, self.1]
    }
}

impl IntoDomain for std::ops::RangeInclusive<i64> {
    fn into_domain(self) -> Vec<i64> {
        vec![*self.start(), *self.end()]
    }
}

impl IntoDomain for std::ops::Range<i64> {
    fn into_domain(self) -> Vec<i64> {
        vec![self.start, self.end - 1]
    }
}

impl IntoDomain for i64 {
    fn into_domain(self) -> Vec<i64> {
        vec![self, self]
    }
}

impl IntoDomain for Vec<(i64, i64)> {
    fn into_domain(self) -> Vec<i64> {
        self.into_iter().flat_map(|(a, b)| [a, b]).collect()
    }
}

impl IntoDomain for std::ops::RangeInclusive<i32> {
    fn into_domain(self) -> Vec<i64> {
        vec![*self.start() as i64, *self.end() as i64]
    }
}

/// A CP-SAT model builder.
///
/// Construct variables, add constraints, set an objective, then solve.
///
/// # Example
/// ```no_run
/// use cpsat_rs::prelude::*;
///
/// let mut model = CpModel::new();
/// let x = model.new_int_var(0..=10, "x");
/// let y = model.new_int_var(0..=10, "y");
/// model.add((x + y).le(15));
/// model.minimize(x + y);
/// let response = CpSolver::solve(&model).unwrap();
/// ```
pub struct CpModel {
    pub(crate) proto: CpModelProto,
}

impl CpModel {
    /// Create a new empty model.
    pub fn new() -> Self {
        Self {
            proto: CpModelProto::default(),
        }
    }

    // ───── Variable creation ─────

    /// Add an integer variable with the given domain.
    pub fn new_int_var(&mut self, domain: impl IntoDomain, name: impl AsRef<str>) -> IntVar {
        let idx = self.proto.variables.len() as i32;
        self.proto.variables.push(IntegerVariableProto {
            name: name.as_ref().to_string(),
            domain: domain.into_domain(),
        });
        IntVar(idx)
    }

    /// Add a Boolean variable (domain [0, 1]).
    pub fn new_bool_var(&mut self, name: impl AsRef<str>) -> BoolVar {
        let idx = self.proto.variables.len() as i32;
        self.proto.variables.push(IntegerVariableProto {
            name: name.as_ref().to_string(),
            domain: vec![0, 1],
        });
        BoolVar(idx)
    }

    /// Add a constant variable.
    pub fn new_constant(&mut self, value: i64) -> IntVar {
        let idx = self.proto.variables.len() as i32;
        self.proto.variables.push(IntegerVariableProto {
            name: String::new(),
            domain: vec![value, value],
        });
        IntVar(idx)
    }

    /// Add an interval variable defined by start, size, and end expressions.
    /// Implicitly enforces start + size == end.
    pub fn new_interval_var(
        &mut self,
        start: impl Into<LinearExpr>,
        size: impl Into<LinearExpr>,
        end: impl Into<LinearExpr>,
        name: impl AsRef<str>,
    ) -> IntervalVar {
        let idx = self.proto.constraints.len() as i32;
        self.proto.constraints.push(ConstraintProto {
            name: name.as_ref().to_string(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::Interval(
                IntervalConstraintProto {
                    start: Some(start.into().to_proto()),
                    size: Some(size.into().to_proto()),
                    end: Some(end.into().to_proto()),
                },
            )),
        });
        IntervalVar(idx)
    }

    /// Add an optional interval variable, active only when `is_present` is true.
    pub fn new_optional_interval_var(
        &mut self,
        start: impl Into<LinearExpr>,
        size: impl Into<LinearExpr>,
        end: impl Into<LinearExpr>,
        is_present: BoolVar,
        name: impl AsRef<str>,
    ) -> IntervalVar {
        let idx = self.proto.constraints.len() as i32;
        self.proto.constraints.push(ConstraintProto {
            name: name.as_ref().to_string(),
            enforcement_literal: vec![is_present.index()],
            constraint: Some(constraint_proto::Constraint::Interval(
                IntervalConstraintProto {
                    start: Some(start.into().to_proto()),
                    size: Some(size.into().to_proto()),
                    end: Some(end.into().to_proto()),
                },
            )),
        });
        IntervalVar(idx)
    }

    // ───── Constraints ─────

    /// Add a bounded linear constraint: lb <= expr <= ub.
    pub fn add(&mut self, bounded: BoundedLinearExpr) -> &mut Self {
        let expr = bounded.expr;
        // Move constant to the bound side: sum(c*v) + k in [lb, ub]
        // becomes sum(c*v) in [lb - k, ub - k]
        let adj_lb = if bounded.lb == i64::MIN { i64::MIN } else { bounded.lb - expr.constant };
        let adj_ub = if bounded.ub == i64::MAX { i64::MAX } else { bounded.ub - expr.constant };

        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::Linear(
                LinearConstraintProto {
                    vars: expr.terms.iter().map(|(v, _)| v.0).collect(),
                    coeffs: expr.terms.iter().map(|(_, c)| *c).collect(),
                    domain: vec![adj_lb, adj_ub],
                },
            )),
        });
        self
    }

    /// All variables must take different values.
    pub fn add_all_different(&mut self, vars: &[IntVar]) -> &mut Self {
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::AllDiff(
                AllDifferentConstraintProto {
                    exprs: vars.iter().map(|v| LinearExpressionProto {
                        vars: vec![v.0],
                        coeffs: vec![1],
                        offset: 0,
                    }).collect(),
                },
            )),
        });
        self
    }

    /// Intervals must not overlap in time (disjunctive constraint).
    pub fn add_no_overlap(&mut self, intervals: &[IntervalVar]) -> &mut Self {
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::NoOverlap(
                NoOverlapConstraintProto {
                    intervals: intervals.iter().map(|iv| iv.0).collect(),
                },
            )),
        });
        self
    }

    /// 2D no-overlap: x_intervals[i] and y_intervals[i] define rectangles
    /// that must not overlap.
    pub fn add_no_overlap_2d(
        &mut self,
        x_intervals: &[IntervalVar],
        y_intervals: &[IntervalVar],
    ) -> &mut Self {
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::NoOverlap2d(
                NoOverlap2DConstraintProto {
                    x_intervals: x_intervals.iter().map(|iv| iv.0).collect(),
                    y_intervals: y_intervals.iter().map(|iv| iv.0).collect(),
                },
            )),
        });
        self
    }

    /// Cumulative constraint: at any point in time, the sum of demands
    /// of active intervals must not exceed capacity.
    pub fn add_cumulative(
        &mut self,
        intervals: &[IntervalVar],
        demands: &[LinearExpr],
        capacity: impl Into<LinearExpr>,
    ) -> &mut Self {
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::Cumulative(
                CumulativeConstraintProto {
                    intervals: intervals.iter().map(|iv| iv.0).collect(),
                    demands: demands.iter().map(|d| d.to_proto()).collect(),
                    capacity: Some(capacity.into().to_proto()),
                },
            )),
        });
        self
    }

    /// Circuit constraint: find a Hamiltonian circuit.
    /// Each arc is (tail_node, head_node, literal).
    pub fn add_circuit(&mut self, arcs: &[(i32, i32, BoolVar)]) -> &mut Self {
        let (tails, heads, literals): (Vec<_>, Vec<_>, Vec<_>) = arcs
            .iter()
            .map(|&(t, h, l)| (t, h, l.index()))
            .fold((vec![], vec![], vec![]), |(mut t, mut h, mut l), (ti, hi, li)| {
                t.push(ti); h.push(hi); l.push(li);
                (t, h, l)
            });
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::Circuit(
                CircuitConstraintProto { tails, heads, literals },
            )),
        });
        self
    }

    /// Table constraint: vars must take one of the allowed tuples.
    pub fn add_table(
        &mut self,
        vars: &[IntVar],
        tuples: &[Vec<i64>],
        negated: bool,
    ) -> &mut Self {
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::Table(
                TableConstraintProto {
                    vars: vars.iter().map(|v| v.0).collect(),
                    values: tuples.iter().flatten().copied().collect(),
                    negated,
                    exprs: vec![],
                },
            )),
        });
        self
    }

    /// Automaton constraint.
    pub fn add_automaton(
        &mut self,
        vars: &[IntVar],
        starting_state: i64,
        final_states: &[i64],
        transitions: &[(i64, i64, i64)],
    ) -> &mut Self {
        let (tails, heads, labels): (Vec<_>, Vec<_>, Vec<_>) = transitions
            .iter()
            .map(|&(t, l, h)| (t, h, l))
            .fold((vec![], vec![], vec![]), |(mut t, mut h, mut l), (ti, hi, li)| {
                t.push(ti); h.push(hi); l.push(li);
                (t, h, l)
            });
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::Automaton(
                AutomatonConstraintProto {
                    vars: vars.iter().map(|v| v.0).collect(),
                    starting_state,
                    final_states: final_states.to_vec(),
                    transition_tail: tails,
                    transition_head: heads,
                    transition_label: labels,
                    exprs: vec![],
                },
            )),
        });
        self
    }

    /// Element constraint: target == array[index].
    pub fn add_element(
        &mut self,
        index: IntVar,
        array: &[IntVar],
        target: IntVar,
    ) -> &mut Self {
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::Element(
                ElementConstraintProto {
                    index: index.0,
                    target: target.0,
                    vars: array.iter().map(|v| v.0).collect(),
                    linear_index: None,
                    linear_target: None,
                    exprs: vec![],
                },
            )),
        });
        self
    }

    /// At least one literal must be true.
    pub fn add_bool_or(&mut self, literals: &[BoolVar]) -> &mut Self {
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::BoolOr(
                BoolArgumentProto {
                    literals: literals.iter().map(|l| l.index()).collect(),
                },
            )),
        });
        self
    }

    /// All literals must be true.
    pub fn add_bool_and(&mut self, literals: &[BoolVar]) -> &mut Self {
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::BoolAnd(
                BoolArgumentProto {
                    literals: literals.iter().map(|l| l.index()).collect(),
                },
            )),
        });
        self
    }

    /// Exactly one literal must be true.
    pub fn add_exactly_one(&mut self, literals: &[BoolVar]) -> &mut Self {
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::ExactlyOne(
                BoolArgumentProto {
                    literals: literals.iter().map(|l| l.index()).collect(),
                },
            )),
        });
        self
    }

    /// At most one literal can be true.
    pub fn add_at_most_one(&mut self, literals: &[BoolVar]) -> &mut Self {
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::AtMostOne(
                BoolArgumentProto {
                    literals: literals.iter().map(|l| l.index()).collect(),
                },
            )),
        });
        self
    }

    /// Implication: if a is true then b must be true.
    pub fn add_implication(&mut self, a: BoolVar, b: BoolVar) -> &mut Self {
        // Encoded as: a => b  ≡  ¬a ∨ b
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![a.index()],
            constraint: Some(constraint_proto::Constraint::BoolAnd(
                BoolArgumentProto {
                    literals: vec![b.index()],
                },
            )),
        });
        self
    }

    /// Max constraint: target == max(vars).
    pub fn add_max_equality(&mut self, target: IntVar, vars: &[IntVar]) -> &mut Self {
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::LinMax(
                LinearArgumentProto {
                    target: Some(LinearExpressionProto {
                        vars: vec![target.0],
                        coeffs: vec![1],
                        offset: 0,
                    }),
                    exprs: vars.iter().map(|v| LinearExpressionProto {
                        vars: vec![v.0],
                        coeffs: vec![1],
                        offset: 0,
                    }).collect(),
                },
            )),
        });
        self
    }

    /// Min constraint: target == min(vars).
    pub fn add_min_equality(&mut self, target: IntVar, vars: &[IntVar]) -> &mut Self {
        // min(vars) = -max(-vars)
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::LinMax(
                LinearArgumentProto {
                    target: Some(LinearExpressionProto {
                        vars: vec![target.0],
                        coeffs: vec![-1],
                        offset: 0,
                    }),
                    exprs: vars.iter().map(|v| LinearExpressionProto {
                        vars: vec![v.0],
                        coeffs: vec![-1],
                        offset: 0,
                    }).collect(),
                },
            )),
        });
        self
    }

    // ───── Objective ─────

    /// Minimize the given expression.
    pub fn minimize(&mut self, expr: impl Into<LinearExpr>) {
        let e = expr.into();
        self.proto.objective = Some(CpObjectiveProto {
            vars: e.terms.iter().map(|(v, _)| v.0).collect(),
            coeffs: e.terms.iter().map(|(_, c)| *c).collect(),
            offset: e.constant as f64,
            scaling_factor: 0.0,
            domain: vec![],
            integer_after_offset: 0,
            integer_scaling_factor: 0,
            integer_before_offset: 0,
            scaling_was_exact: false,
        });
    }

    /// Maximize the given expression.
    pub fn maximize(&mut self, expr: impl Into<LinearExpr>) {
        let e = expr.into();
        // Maximize f(x) = minimize -f(x)
        self.proto.objective = Some(CpObjectiveProto {
            vars: e.terms.iter().map(|(v, _)| v.0).collect(),
            coeffs: e.terms.iter().map(|(_, c)| -c).collect(),
            offset: -(e.constant as f64),
            scaling_factor: -1.0,
            domain: vec![],
            integer_after_offset: 0,
            integer_scaling_factor: 0,
            integer_before_offset: 0,
            scaling_was_exact: false,
        });
    }

    // ───── Hints ─────

    /// Provide a solution hint for a variable.
    pub fn add_hint(&mut self, var: IntVar, value: i64) {
        let hint = self.proto.solution_hint.get_or_insert_with(|| {
            PartialVariableAssignment { vars: vec![], values: vec![] }
        });
        hint.vars.push(var.0);
        hint.values.push(value);
    }

    // ───── Serialization ─────

    /// Get a reference to the underlying proto.
    pub fn to_proto(&self) -> &CpModelProto {
        &self.proto
    }

    /// Get a mutable reference to the underlying proto (escape hatch).
    pub fn raw_proto_mut(&mut self) -> &mut CpModelProto {
        &mut self.proto
    }

    /// Serialize the model to protobuf bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.proto.encoded_len());
        self.proto.encode(&mut buf).expect("prost encode cannot fail on valid proto");
        buf
    }
}

impl Default for CpModel {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CpModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CpModel({} vars, {} constraints{})",
            self.proto.variables.len(),
            self.proto.constraints.len(),
            if self.proto.objective.is_some() { ", has objective" } else { "" },
        )
    }
}

// Allow `model.add(x <= 10)` syntax via operator overloading on IntVar
// These produce BoundedLinearExpr which model.add() accepts.

/// Trait for creating bounded expressions from comparisons.
pub trait Le<Rhs> {
    /// Create a `<= rhs` bounded expression.
    fn le(self, rhs: Rhs) -> BoundedLinearExpr;
}

/// Trait for creating bounded expressions from comparisons.
pub trait Ge<Rhs> {
    /// Create a `>= rhs` bounded expression.
    fn ge(self, rhs: Rhs) -> BoundedLinearExpr;
}

impl Le<i64> for IntVar {
    fn le(self, rhs: i64) -> BoundedLinearExpr {
        LinearExpr::from(self).le(rhs)
    }
}

impl Ge<i64> for IntVar {
    fn ge(self, rhs: i64) -> BoundedLinearExpr {
        LinearExpr::from(self).ge(rhs)
    }
}

impl Le<i64> for LinearExpr {
    fn le(self, rhs: i64) -> BoundedLinearExpr {
        self.le(rhs)
    }
}

impl Ge<i64> for LinearExpr {
    fn ge(self, rhs: i64) -> BoundedLinearExpr {
        self.ge(rhs)
    }
}
