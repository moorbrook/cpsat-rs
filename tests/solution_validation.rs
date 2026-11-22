use cpsat_rs::prelude::*;

#[test]
fn interval_linking_is_enforced() {
    let mut model = CpModel::new();
    let s = model.new_int_var(0..=100, "s");
    let d = model.new_constant(10);
    let e = model.new_int_var(0..=100, "e");
    let _iv = model.new_interval_var(s, d, e, "task");

    model.minimize(e);

    let resp = CpSolver::solve(&model).unwrap();
    assert!(resp.is_optimal());

    let sv = resp.value(s);
    let ev = resp.value(e);
    let dv = 10_i64;

    assert_eq!(
        ev,
        sv + dv,
        "Interval linking violated: end({ev}) != start({sv}) + size({dv})"
    );
    assert_eq!(sv, 0, "Start should be 0 (minimizing end)");
    assert_eq!(ev, 10, "End should be 10");
}

#[test]
fn no_overlap_actually_prevents_overlap() {
    let mut model = CpModel::new();

    let s0 = model.new_int_var(0..=20, "s0");
    let d0 = model.new_constant(5);
    let e0 = model.new_int_var(0..=20, "e0");
    let iv0 = model.new_interval_var(s0, d0, e0, "t0");

    let s1 = model.new_int_var(0..=20, "s1");
    let d1 = model.new_constant(5);
    let e1 = model.new_int_var(0..=20, "e1");
    let iv1 = model.new_interval_var(s1, d1, e1, "t1");

    model.add_no_overlap(&[iv0, iv1]);

    let makespan = model.new_int_var(0..=20, "makespan");
    model.add_max_equality(makespan, &[e0, e1]);
    model.minimize(makespan);

    let resp = CpSolver::solve(&model).unwrap();
    assert!(resp.is_optimal());

    let s0v = resp.value(s0);
    let e0v = resp.value(e0);
    let s1v = resp.value(s1);
    let e1v = resp.value(e1);

    assert!(
        e0v <= s1v || e1v <= s0v,
        "Tasks overlap! t0=[{s0v},{e0v}), t1=[{s1v},{e1v})"
    );

    assert_eq!(e0v, s0v + 5, "t0 linking: end({e0v}) != start({s0v}) + 5");
    assert_eq!(e1v, s1v + 5, "t1 linking: end({e1v}) != start({s1v}) + 5");

    assert_eq!(resp.value(makespan), 10, "Makespan should be 5+5=10");
}

#[test]
fn precedences_respected_in_solution() {
    let mut model = CpModel::new();

    let horizon = 20_i64;

    let s00 = model.new_int_var(0..=horizon, "s00");
    let d00 = model.new_constant(3);
    let e00 = model.new_int_var(0..=horizon, "e00");
    let iv00 = model.new_interval_var(s00, d00, e00, "j0o0");

    let s01 = model.new_int_var(0..=horizon, "s01");
    let d01 = model.new_constant(4);
    let e01 = model.new_int_var(0..=horizon, "e01");
    let iv01 = model.new_interval_var(s01, d01, e01, "j0o1");

    let s10 = model.new_int_var(0..=horizon, "s10");
    let d10 = model.new_constant(2);
    let e10 = model.new_int_var(0..=horizon, "e10");
    let iv10 = model.new_interval_var(s10, d10, e10, "j1o0");

    let s11 = model.new_int_var(0..=horizon, "s11");
    let d11 = model.new_constant(5);
    let e11 = model.new_int_var(0..=horizon, "e11");
    let iv11 = model.new_interval_var(s11, d11, e11, "j1o1");

    model.add((LinearExpr::from(e00) - LinearExpr::from(s01)).le(0));
    model.add((LinearExpr::from(e10) - LinearExpr::from(s11)).le(0));

    model.add_no_overlap(&[iv00, iv11]);
    model.add_no_overlap(&[iv01, iv10]);

    let makespan = model.new_int_var(0..=horizon, "makespan");
    model.add_max_equality(makespan, &[e01, e11]);
    model.minimize(makespan);

    let resp = CpSolver::solve(&model).unwrap();
    assert!(resp.is_optimal());

    let vals = |s: IntVar, e: IntVar| (resp.value(s), resp.value(e));
    let (s00v, e00v) = vals(s00, e00);
    let (s01v, e01v) = vals(s01, e01);
    let (s10v, e10v) = vals(s10, e10);
    let (s11v, e11v) = vals(s11, e11);

    assert_eq!(e00v - s00v, 3, "j0o0 duration mismatch");
    assert_eq!(e01v - s01v, 4, "j0o1 duration mismatch");
    assert_eq!(e10v - s10v, 2, "j1o0 duration mismatch");
    assert_eq!(e11v - s11v, 5, "j1o1 duration mismatch");

    assert!(
        e00v <= s01v,
        "j0 precedence violated: e00={e00v} > s01={s01v}"
    );
    assert!(
        e10v <= s11v,
        "j1 precedence violated: e10={e10v} > s11={s11v}"
    );

    assert!(
        e00v <= s11v || e11v <= s00v,
        "M0 overlap: j0o0=[{s00v},{e00v}), j1o1=[{s11v},{e11v})"
    );
    assert!(
        e01v <= s10v || e10v <= s01v,
        "M1 overlap: j0o1=[{s01v},{e01v}), j1o0=[{s10v},{e10v})"
    );

    eprintln!(
        "  Schedule: j0=[{s00v}-{e00v}, {s01v}-{e01v}], j1=[{s10v}-{e10v}, {s11v}-{e11v}], makespan={}",
        resp.value(makespan)
    );
}

#[test]
fn without_no_overlap_tasks_can_overlap() {
    let mut model = CpModel::new();

    let s0 = model.new_int_var(0..=10, "s0");
    let d0 = model.new_constant(5);
    let e0 = model.new_int_var(0..=10, "e0");
    let _iv0 = model.new_interval_var(s0, d0, e0, "t0");

    let s1 = model.new_int_var(0..=10, "s1");
    let d1 = model.new_constant(5);
    let e1 = model.new_int_var(0..=10, "e1");
    let _iv1 = model.new_interval_var(s1, d1, e1, "t1");

    let makespan = model.new_int_var(0..=10, "makespan");
    model.add_max_equality(makespan, &[e0, e1]);
    model.minimize(makespan);

    let resp = CpSolver::solve(&model).unwrap();
    assert!(resp.is_optimal());

    assert_eq!(
        resp.value(makespan),
        5,
        "Without no-overlap, makespan should be 5 (tasks overlap)"
    );
    assert_eq!(resp.value(s0), 0);
    assert_eq!(resp.value(s1), 0);
}

#[test]
fn optional_interval_respects_presence() {
    let mut model = CpModel::new();

    let s = model.new_int_var(0..=100, "s");
    let d = model.new_constant(10);
    let e = model.new_int_var(0..=100, "e");
    let present = model.new_bool_var("present");

    let _iv = model.new_optional_interval_var(s, d, e, present, "task");

    model.add(LinearExpr::from(present.as_int_var()).eq(0));

    model.minimize(e);

    let resp = CpSolver::solve(&model).unwrap();
    assert!(resp.is_optimal());

    assert_eq!(
        resp.value(e),
        0,
        "When not present, end should be 0 (unconstrained)"
    );
    assert!(!resp.bool_value(present));
}

#[test]
fn interval_with_variable_size() {
    let mut model = CpModel::new();

    let s = model.new_int_var(0..=100, "s");
    let size = model.new_int_var(0..=10, "size");
    let e = model.new_int_var(0..=100, "e");
    let _iv = model.new_interval_var(s, size, e, "task");

    model.minimize(size);

    let resp = CpSolver::solve(&model).unwrap();
    assert!(resp.is_optimal(), "Status: {:?}", resp.status());

    let sv = resp.value(s);
    let szv = resp.value(size);
    let ev = resp.value(e);

    assert_eq!(szv, 0, "Minimum size should be 0");
    assert_eq!(
        ev,
        sv + szv,
        "Linking: end({ev}) != start({sv}) + size({szv})"
    );
}
