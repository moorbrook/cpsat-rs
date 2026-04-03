//! Property-based tests for cpsat-rs.
//!
//! Tests invariants that should hold for ALL inputs, not just specific examples.

use cpsat_rs::prelude::*;
use proptest::prelude::*;

// ───── Model serialization roundtrip ─────

proptest! {
    /// A model with N variables survives serialize/deserialize roundtrip.
    #[test]
    fn model_roundtrip(n in 1usize..20, lb in -100i64..0, ub in 0i64..100) {
        let mut model = CpModel::new();
        for i in 0..n {
            model.new_int_var(lb..=ub, format!("x{i}"));
        }

        let bytes = model.to_bytes();
        let restored = CpModel::from_bytes(&bytes).unwrap();
        let proto1 = model.to_proto();
        let proto2 = restored.to_proto();

        prop_assert_eq!(proto1.variables.len(), proto2.variables.len());
        for (v1, v2) in proto1.variables.iter().zip(proto2.variables.iter()) {
            prop_assert_eq!(&v1.domain, &v2.domain);
            prop_assert_eq!(&v1.name, &v2.name);
        }
    }

    /// Model with constraints survives roundtrip.
    #[test]
    fn model_with_constraints_roundtrip(n in 2usize..8, bound in 10i64..100) {
        let mut model = CpModel::new();
        let vars: Vec<IntVar> = (0..n)
            .map(|i| model.new_int_var(0..=bound, format!("x{i}")))
            .collect();
        model.add_all_different(&vars);
        model.minimize(LinearExpr::sum(&vars));

        let bytes = model.to_bytes();
        let restored = CpModel::from_bytes(&bytes).unwrap();

        prop_assert_eq!(
            model.to_proto().constraints.len(),
            restored.to_proto().constraints.len()
        );
        prop_assert!(restored.to_proto().objective.is_some());
    }
}

// ───── Domain validation ─────

proptest! {
    /// Valid domains always produce valid IntVar handles.
    #[test]
    fn valid_domain_produces_valid_handle(lb in -1000i64..0, ub in 0i64..1000) {
        let mut model = CpModel::new();
        let var = model.new_int_var(lb..=ub, "x");
        prop_assert!(var.index() >= 0);
    }

    /// Solve a trivial minimize with random bounds: result == lb.
    #[test]
    fn trivial_minimize_returns_lb(lb in 0i64..50, ub in 50i64..100) {
        let mut model = CpModel::new();
        let x = model.new_int_var(lb..=ub, "x");
        model.minimize(x);

        let resp = CpSolver::solve(&model).unwrap();
        prop_assert!(resp.is_optimal());
        prop_assert_eq!(resp.value(x), lb);
    }

    /// Solve a trivial maximize with random bounds: result == ub.
    #[test]
    fn trivial_maximize_returns_ub(lb in 0i64..50, ub in 50i64..100) {
        let mut model = CpModel::new();
        let x = model.new_int_var(lb..=ub, "x");
        model.maximize(x);

        let resp = CpSolver::solve(&model).unwrap();
        prop_assert!(resp.is_optimal());
        prop_assert_eq!(resp.value(x), ub);
    }
}

// ───── BoolVar invariants ─────

proptest! {
    /// Double negation of a BoolVar returns the original index.
    #[test]
    fn boolvar_double_negation(idx in 0i32..1000) {
        let mut model = CpModel::new();
        let b = model.new_bool_var(format!("b{idx}"));
        let double_neg = !(!b);
        prop_assert_eq!(double_neg.index(), b.index());
    }

    /// Negated BoolVar has negative index, original has non-negative.
    #[test]
    fn boolvar_negation_sign(_idx in 0i32..100) {
        let mut model = CpModel::new();
        let b = model.new_bool_var("b");
        prop_assert!(b.index() >= 0);
        prop_assert!((!b).index() < 0);
        prop_assert!((!b).is_negated());
        prop_assert!(!b.is_negated());
    }
}

// ───── Interval linking ─────

proptest! {
    /// Interval with constant size: minimizing end gives start=0, end=size.
    #[test]
    fn interval_minimize_end(dur in 1i64..50) {
        let mut model = CpModel::new();
        let s = model.new_int_var(0..=100, "s");
        let d = model.new_constant(dur);
        let e = model.new_int_var(0..=100, "e");
        let _iv = model.new_interval_var(s, d, e, "task");
        model.minimize(e);

        let resp = CpSolver::solve(&model).unwrap();
        prop_assert!(resp.is_optimal());
        prop_assert_eq!(resp.value(s), 0);
        prop_assert_eq!(resp.value(e), dur);
    }
}
