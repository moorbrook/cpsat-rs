//! `CpModel` builder — the main entry point for constructing optimization models.

use crate::expressions::{BoundedLinearExpr, LinearExpr};
use crate::proto::{
    AllDifferentConstraintProto, AutomatonConstraintProto, BoolArgumentProto,
    CircuitConstraintProto, ConstraintProto, CpModelProto, CpObjectiveProto,
    CumulativeConstraintProto, ElementConstraintProto, IntegerVariableProto,
    IntervalConstraintProto, LinearArgumentProto, LinearConstraintProto, LinearExpressionProto,
    NoOverlap2DConstraintProto, NoOverlapConstraintProto, PartialVariableAssignment,
    TableConstraintProto, constraint_proto,
};
use crate::vars::{BoolVar, IntVar, IntervalVar};
use prost::Message;

/// Domain specification for integer variables.
pub trait IntoDomain {
    /// Convert to the flat domain representation used by CP-SAT proto.
    /// Format: pairs of `[lb, ub]` intervals, flattened into a single `Vec`.
    fn into_domain(self) -> Vec<i64>;
}

impl IntoDomain for (i64, i64) {
    fn into_domain(self) -> Vec<i64> {
        assert!(
            self.0 <= self.1,
            "Domain ({}, {}) has lb > ub",
            self.0,
            self.1
        );
        vec![self.0, self.1]
    }
}

impl IntoDomain for std::ops::RangeInclusive<i64> {
    fn into_domain(self) -> Vec<i64> {
        assert!(
            self.start() <= self.end(),
            "Domain {}..={} is empty",
            self.start(),
            self.end()
        );
        vec![*self.start(), *self.end()]
    }
}

impl IntoDomain for std::ops::Range<i64> {
    fn into_domain(self) -> Vec<i64> {
        assert!(
            self.start < self.end,
            "Domain {}..{} is empty",
            self.start,
            self.end
        );
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
        assert!(!self.is_empty(), "Domain interval list must not be empty");
        for &(a, b) in &self {
            assert!(a <= b, "Domain interval ({a}, {b}) has lb > ub");
        }
        // OR-Tools CpModelProto requires sorted, disjoint, non-adjacent intervals:
        // for all i < n-1: max_i + 1 < min_{i+1}.
        for w in self.windows(2) {
            assert!(
                w[0].1.checked_add(1).is_some_and(|next| next < w[1].0),
                "Domain intervals ({}, {}) and ({}, {}) are not sorted/disjoint/non-adjacent \
                 (require max + 1 < next_min)",
                w[0].0,
                w[0].1,
                w[1].0,
                w[1].1
            );
        }
        self.into_iter().flat_map(|(a, b)| [a, b]).collect()
    }
}

impl IntoDomain for std::ops::RangeInclusive<i32> {
    fn into_domain(self) -> Vec<i64> {
        assert!(
            self.start() <= self.end(),
            "Domain {}..={} is empty",
            self.start(),
            self.end()
        );
        vec![i64::from(*self.start()), i64::from(*self.end())]
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
#[derive(Clone)]
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

    fn next_var_index(&self) -> i32 {
        i32::try_from(self.proto.variables.len()).expect("CpModel variable count exceeds i32::MAX")
    }

    fn next_constraint_index(&self) -> i32 {
        i32::try_from(self.proto.constraints.len())
            .expect("CpModel constraint count exceeds i32::MAX")
    }

    /// Add an integer variable with the given domain.
    ///
    /// # Panics
    ///
    /// Panics if the domain is invalid (empty range, `lb > ub`, or
    /// non-sorted/non-disjoint intervals for `Vec<(i64, i64)>`), or if the
    /// variable count exceeds `i32::MAX`.
    pub fn new_int_var(&mut self, domain: impl IntoDomain, name: impl AsRef<str>) -> IntVar {
        let idx = self.next_var_index();
        self.proto.variables.push(IntegerVariableProto {
            name: name.as_ref().to_string(),
            domain: domain.into_domain(),
        });
        IntVar(idx)
    }

    /// Add a Boolean variable (domain `[0, 1]`).
    ///
    /// # Panics
    ///
    /// Panics if the variable count exceeds `i32::MAX`.
    pub fn new_bool_var(&mut self, name: impl AsRef<str>) -> BoolVar {
        let idx = self.next_var_index();
        self.proto.variables.push(IntegerVariableProto {
            name: name.as_ref().to_string(),
            domain: vec![0, 1],
        });
        BoolVar(idx)
    }

    /// Add a constant variable.
    ///
    /// # Panics
    ///
    /// Panics if the variable count exceeds `i32::MAX`.
    pub fn new_constant(&mut self, value: i64) -> IntVar {
        let idx = self.next_var_index();
        self.proto.variables.push(IntegerVariableProto {
            name: String::new(),
            domain: vec![value, value],
        });
        IntVar(idx)
    }

    /// Add an interval variable defined by start, size, and end expressions.
    ///
    /// Automatically adds the required `start + size == end` and `size >= 0`
    /// constraints (the proto does NOT enforce these implicitly).
    ///
    /// # Panics
    ///
    /// Panics if coefficient arithmetic overflows `i64` during constraint generation
    /// or if the constraint count exceeds `i32::MAX`.
    pub fn new_interval_var(
        &mut self,
        start: impl Into<LinearExpr>,
        size: impl Into<LinearExpr>,
        end: impl Into<LinearExpr>,
        name: impl AsRef<str>,
    ) -> IntervalVar {
        let start_expr = start.into();
        let size_expr = size.into();
        let end_expr = end.into();

        let idx = self.next_constraint_index();
        self.proto.constraints.push(ConstraintProto {
            name: name.as_ref().to_string(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::Interval(
                IntervalConstraintProto {
                    start: Some(start_expr.to_proto()),
                    size: Some(size_expr.to_proto()),
                    end: Some(end_expr.to_proto()),
                },
            )),
        });

        self.add_interval_linking(&start_expr, &size_expr, &end_expr, &[]);

        self.add_size_nonneg(&size_expr, &[]);

        IntervalVar(idx)
    }

    /// Add an optional interval variable, active only when `is_present` is true.
    ///
    /// Automatically adds the required `is_present => start + size == end` and
    /// `is_present => size >= 0` constraints.
    ///
    /// # Panics
    ///
    /// Panics if coefficient arithmetic overflows `i64` during constraint generation
    /// or if the constraint count exceeds `i32::MAX`.
    pub fn new_optional_interval_var(
        &mut self,
        start: impl Into<LinearExpr>,
        size: impl Into<LinearExpr>,
        end: impl Into<LinearExpr>,
        is_present: BoolVar,
        name: impl AsRef<str>,
    ) -> IntervalVar {
        let start_expr = start.into();
        let size_expr = size.into();
        let end_expr = end.into();
        let enforcement = vec![is_present.index()];

        let idx = self.next_constraint_index();
        self.proto.constraints.push(ConstraintProto {
            name: name.as_ref().to_string(),
            enforcement_literal: enforcement.clone(),
            constraint: Some(constraint_proto::Constraint::Interval(
                IntervalConstraintProto {
                    start: Some(start_expr.to_proto()),
                    size: Some(size_expr.to_proto()),
                    end: Some(end_expr.to_proto()),
                },
            )),
        });

        self.add_interval_linking(&start_expr, &size_expr, &end_expr, &enforcement);

        self.add_size_nonneg(&size_expr, &enforcement);

        IntervalVar(idx)
    }

    fn add_interval_linking(
        &mut self,
        start: &LinearExpr,
        size: &LinearExpr,
        end: &LinearExpr,
        enforcement: &[i32],
    ) {
        let mut vars = Vec::new();
        let mut coeffs = Vec::new();
        for &(v, c) in &start.terms {
            vars.push(v.0);
            coeffs.push(c);
        }
        for &(v, c) in &size.terms {
            vars.push(v.0);
            coeffs.push(c);
        }
        for &(v, c) in &end.terms {
            vars.push(v.0);
            coeffs.push(
                c.checked_neg()
                    .expect("Interval linking: end coefficient overflow on negation"),
            );
        }
        let offset = start
            .constant
            .checked_add(size.constant)
            .and_then(|v| v.checked_sub(end.constant))
            .expect("Interval linking: constant overflow in start + size - end");
        let neg_offset = offset
            .checked_neg()
            .expect("Interval linking: offset overflow on negation");

        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: enforcement.to_vec(),
            constraint: Some(constraint_proto::Constraint::Linear(
                LinearConstraintProto {
                    vars,
                    coeffs,
                    domain: vec![neg_offset, neg_offset],
                },
            )),
        });
    }

    fn add_size_nonneg(&mut self, size: &LinearExpr, enforcement: &[i32]) {
        if size.terms.is_empty() && size.constant >= 0 {
            return;
        }
        let vars: Vec<i32> = size.terms.iter().map(|(v, _)| v.0).collect();
        let coeffs: Vec<i64> = size.terms.iter().map(|(_, c)| *c).collect();

        let lb = size
            .constant
            .checked_neg()
            .expect("Interval size_nonneg: constant overflow on negation");
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: enforcement.to_vec(),
            constraint: Some(constraint_proto::Constraint::Linear(
                LinearConstraintProto {
                    vars,
                    coeffs,
                    domain: vec![lb, i64::MAX],
                },
            )),
        });
    }

    /// Add a bounded linear constraint: `lb <= expr <= ub`.
    ///
    /// # Panics
    ///
    /// Panics if adjusting bounds by the expression constant overflows `i64`.
    pub fn add(&mut self, bounded: BoundedLinearExpr) -> &mut Self {
        let expr = bounded.expr;
        let adjusted_lower = if bounded.lb == i64::MIN {
            i64::MIN
        } else {
            bounded.lb.checked_sub(expr.constant).unwrap_or_else(|| {
                panic!(
                    "Constraint bound overflow: lb={} - constant={} overflows i64",
                    bounded.lb, expr.constant
                )
            })
        };
        let adjusted_upper = if bounded.ub == i64::MAX {
            i64::MAX
        } else {
            bounded.ub.checked_sub(expr.constant).unwrap_or_else(|| {
                panic!(
                    "Constraint bound overflow: ub={} - constant={} overflows i64",
                    bounded.ub, expr.constant
                )
            })
        };

        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::Linear(
                LinearConstraintProto {
                    vars: expr.terms.iter().map(|(v, _)| v.0).collect(),
                    coeffs: expr.terms.iter().map(|(_, c)| *c).collect(),
                    domain: vec![adjusted_lower, adjusted_upper],
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
                    exprs: vars
                        .iter()
                        .map(|v| LinearExpressionProto {
                            vars: vec![v.0],
                            coeffs: vec![1],
                            offset: 0,
                        })
                        .collect(),
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

    /// 2D no-overlap: `x_intervals[i]` and `y_intervals[i]` define rectangles
    /// that must not overlap.
    ///
    /// # Panics
    ///
    /// Panics if `x_intervals` and `y_intervals` have different lengths.
    pub fn add_no_overlap_2d(
        &mut self,
        x_intervals: &[IntervalVar],
        y_intervals: &[IntervalVar],
    ) -> &mut Self {
        assert_eq!(
            x_intervals.len(),
            y_intervals.len(),
            "add_no_overlap_2d: x_intervals ({}) and y_intervals ({}) must have the same length",
            x_intervals.len(),
            y_intervals.len()
        );
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
    ///
    /// # Panics
    ///
    /// Panics if `intervals` and `demands` have different lengths.
    pub fn add_cumulative(
        &mut self,
        intervals: &[IntervalVar],
        demands: &[LinearExpr],
        capacity: impl Into<LinearExpr>,
    ) -> &mut Self {
        assert_eq!(
            intervals.len(),
            demands.len(),
            "add_cumulative: intervals ({}) and demands ({}) must have the same length",
            intervals.len(),
            demands.len()
        );
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::Cumulative(
                CumulativeConstraintProto {
                    intervals: intervals.iter().map(|iv| iv.0).collect(),
                    demands: demands.iter().map(LinearExpr::to_proto).collect(),
                    capacity: Some(capacity.into().to_proto()),
                },
            )),
        });
        self
    }

    /// Circuit constraint: find a Hamiltonian circuit.
    /// Each arc is `(tail_node, head_node, literal)`.
    pub fn add_circuit(&mut self, arcs: &[(i32, i32, BoolVar)]) -> &mut Self {
        let mut tails = Vec::with_capacity(arcs.len());
        let mut heads = Vec::with_capacity(arcs.len());
        let mut literals = Vec::with_capacity(arcs.len());
        for &(t, h, l) in arcs {
            tails.push(t);
            heads.push(h);
            literals.push(l.index());
        }
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::Circuit(
                CircuitConstraintProto {
                    tails,
                    heads,
                    literals,
                },
            )),
        });
        self
    }

    /// Table constraint: `vars` must take one of the allowed tuples.
    ///
    /// # Panics
    ///
    /// Panics if any tuple has a different length than `vars`.
    pub fn add_table(&mut self, vars: &[IntVar], tuples: &[Vec<i64>], negated: bool) -> &mut Self {
        for (i, tuple) in tuples.iter().enumerate() {
            assert_eq!(
                tuple.len(),
                vars.len(),
                "add_table: tuple {i} has {} entries, expected {} (one per variable)",
                tuple.len(),
                vars.len()
            );
        }
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::Table(TableConstraintProto {
                vars: vars.iter().map(|v| v.0).collect(),
                values: tuples.iter().flatten().copied().collect(),
                negated,
                exprs: vec![],
            })),
        });
        self
    }

    /// Automaton constraint.
    ///
    /// Each transition is `(tail_state, head_state, label)` matching
    /// the OR-Tools Python API convention.
    pub fn add_automaton(
        &mut self,
        vars: &[IntVar],
        starting_state: i64,
        final_states: &[i64],
        transitions: &[(i64, i64, i64)],
    ) -> &mut Self {
        let mut tails = Vec::with_capacity(transitions.len());
        let mut heads = Vec::with_capacity(transitions.len());
        let mut labels = Vec::with_capacity(transitions.len());
        for &(tail, head, label) in transitions {
            tails.push(tail);
            heads.push(head);
            labels.push(label);
        }
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

    /// Element constraint: `target == array[index]`.
    ///
    /// # Panics
    ///
    /// Panics if `array` is empty.
    pub fn add_element(&mut self, index: IntVar, array: &[IntVar], target: IntVar) -> &mut Self {
        assert!(!array.is_empty(), "add_element: array must not be empty");
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
    ///
    /// # Panics
    ///
    /// Panics if `literals` is empty (would create an always-false constraint).
    pub fn add_bool_or(&mut self, literals: &[BoolVar]) -> &mut Self {
        assert!(
            !literals.is_empty(),
            "add_bool_or: literals must not be empty"
        );
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::BoolOr(BoolArgumentProto {
                literals: literals.iter().map(|l| l.index()).collect(),
            })),
        });
        self
    }

    /// All literals must be true.
    pub fn add_bool_and(&mut self, literals: &[BoolVar]) -> &mut Self {
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::BoolAnd(BoolArgumentProto {
                literals: literals.iter().map(|l| l.index()).collect(),
            })),
        });
        self
    }

    /// Exactly one literal must be true.
    ///
    /// # Panics
    ///
    /// Panics if `literals` is empty (would create an always-false constraint).
    pub fn add_exactly_one(&mut self, literals: &[BoolVar]) -> &mut Self {
        assert!(
            !literals.is_empty(),
            "add_exactly_one: literals must not be empty"
        );
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
            constraint: Some(constraint_proto::Constraint::AtMostOne(BoolArgumentProto {
                literals: literals.iter().map(|l| l.index()).collect(),
            })),
        });
        self
    }

    /// Implication: if `a` is true then `b` must be true.
    pub fn add_implication(&mut self, a: BoolVar, b: BoolVar) -> &mut Self {
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![a.index()],
            constraint: Some(constraint_proto::Constraint::BoolAnd(BoolArgumentProto {
                literals: vec![b.index()],
            })),
        });
        self
    }

    /// Max constraint: `target == max(vars)`.
    pub fn add_max_equality(&mut self, target: IntVar, vars: &[IntVar]) -> &mut Self {
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::LinMax(LinearArgumentProto {
                target: Some(LinearExpressionProto {
                    vars: vec![target.0],
                    coeffs: vec![1],
                    offset: 0,
                }),
                exprs: vars
                    .iter()
                    .map(|v| LinearExpressionProto {
                        vars: vec![v.0],
                        coeffs: vec![1],
                        offset: 0,
                    })
                    .collect(),
            })),
        });
        self
    }

    /// Min constraint: `target == min(vars)`.
    pub fn add_min_equality(&mut self, target: IntVar, vars: &[IntVar]) -> &mut Self {
        self.proto.constraints.push(ConstraintProto {
            name: String::new(),
            enforcement_literal: vec![],
            constraint: Some(constraint_proto::Constraint::LinMax(LinearArgumentProto {
                target: Some(LinearExpressionProto {
                    vars: vec![target.0],
                    coeffs: vec![-1],
                    offset: 0,
                }),
                exprs: vars
                    .iter()
                    .map(|v| LinearExpressionProto {
                        vars: vec![v.0],
                        coeffs: vec![-1],
                        offset: 0,
                    })
                    .collect(),
            })),
        });
        self
    }

    /// Minimize the given expression.
    #[expect(
        clippy::cast_precision_loss,
        reason = "CP-SAT CpObjectiveProto.offset is f64"
    )]
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
    ///
    /// # Panics
    ///
    /// Panics if any coefficient is `i64::MIN` (cannot be negated).
    #[expect(
        clippy::cast_precision_loss,
        reason = "CP-SAT CpObjectiveProto.offset is f64"
    )]
    pub fn maximize(&mut self, expr: impl Into<LinearExpr>) {
        let e = expr.into();
        self.proto.objective = Some(CpObjectiveProto {
            vars: e.terms.iter().map(|(v, _)| v.0).collect(),
            coeffs: e
                .terms
                .iter()
                .map(|(_, c)| {
                    c.checked_neg().unwrap_or_else(|| {
                        panic!("maximize: coefficient {c} cannot be negated (i64 overflow)")
                    })
                })
                .collect(),
            offset: -(e.constant as f64),
            scaling_factor: -1.0,
            domain: vec![],
            integer_after_offset: 0,
            integer_scaling_factor: 0,
            integer_before_offset: 0,
            scaling_was_exact: false,
        });
    }

    /// Provide a solution hint for a variable.
    pub fn add_hint(&mut self, var: IntVar, value: i64) {
        let hint = self
            .proto
            .solution_hint
            .get_or_insert_with(|| PartialVariableAssignment {
                vars: vec![],
                values: vec![],
            });
        hint.vars.push(var.0);
        hint.values.push(value);
    }

    /// Get a reference to the underlying proto.
    pub fn to_proto(&self) -> &CpModelProto {
        &self.proto
    }

    /// Get a mutable reference to the underlying proto (escape hatch).
    ///
    /// # Warning
    ///
    /// Modifying the proto directly (reordering, deleting variables or
    /// constraints) will invalidate all previously returned `IntVar`,
    /// `BoolVar`, and `IntervalVar` handles. Only use this for adding
    /// fields the builder doesn't yet support.
    pub fn raw_proto_mut(&mut self) -> &mut CpModelProto {
        &mut self.proto
    }

    /// Serialize the model to protobuf bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.proto.encode_to_vec()
    }

    /// Deserialize a model from protobuf bytes.
    ///
    /// # Errors
    ///
    /// Returns `prost::DecodeError` if the bytes are not a valid `CpModelProto`.
    ///
    /// Note: variable and constraint handles (`IntVar`, `IntervalVar`) from the
    /// original model are valid on the deserialized copy (same indices).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, prost::DecodeError> {
        let proto = CpModelProto::decode(bytes)?;
        Ok(Self { proto })
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
            if self.proto.objective.is_some() {
                ", has objective"
            } else {
                ""
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "not sorted/disjoint/non-adjacent")]
    fn domain_i64_max_interval_panics() {
        let _ = vec![(0, i64::MAX), (0, 1)].into_domain();
    }

    #[test]
    #[should_panic(expected = "offset overflow")]
    fn interval_linking_offset_i64_min_panics() {
        let mut model = CpModel::new();
        let _ = model.new_interval_var(i64::MIN, 0, 0, "overflow");
    }
}
