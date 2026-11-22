use cpsat_rs::prelude::*;

#[test]
fn two_tasks_one_machine() {
    let mut model = CpModel::new();
    let s0 = model.new_int_var(0..=100, "s0");
    let d0 = model.new_constant(3);
    let e0 = model.new_int_var(0..=100, "e0");
    let iv0 = model.new_interval_var(s0, d0, e0, "task0");

    let s1 = model.new_int_var(0..=100, "s1");
    let d1 = model.new_constant(5);
    let e1 = model.new_int_var(0..=100, "e1");
    let iv1 = model.new_interval_var(s1, d1, e1, "task1");

    model.add_no_overlap(&[iv0, iv1]);

    let makespan = model.new_int_var(0..=100, "makespan");
    model.add_max_equality(makespan, &[e0, e1]);
    model.minimize(makespan);

    let response = CpSolver::solve(&model).unwrap();
    assert!(response.is_optimal());
    assert_eq!(response.value(makespan), 8);
}

#[test]
fn ft06_job_shop() {
    struct OpVar {
        start: IntVar,
        end: IntVar,
    }

    let jobs: Vec<Vec<(usize, i64)>> = vec![
        vec![(2, 1), (0, 3), (1, 6), (3, 7), (5, 3), (4, 6)],
        vec![(1, 8), (2, 5), (4, 10), (5, 10), (0, 10), (3, 4)],
        vec![(2, 5), (3, 4), (5, 8), (0, 9), (1, 1), (4, 7)],
        vec![(1, 5), (0, 5), (2, 5), (3, 3), (4, 8), (5, 9)],
        vec![(2, 9), (1, 3), (4, 5), (5, 4), (0, 3), (3, 1)],
        vec![(1, 3), (3, 3), (5, 9), (0, 10), (4, 4), (2, 1)],
    ];
    let n_machines = 6;
    let horizon: i64 = jobs.iter().flat_map(|j| j.iter().map(|(_, d)| d)).sum();

    let mut model = CpModel::new();

    let mut ops: Vec<Vec<OpVar>> = Vec::new();
    let mut machine_intervals: Vec<Vec<IntervalVar>> = vec![vec![]; n_machines];

    for (j, job) in jobs.iter().enumerate() {
        let mut job_ops = Vec::new();
        for (op, &(machine, dur)) in job.iter().enumerate() {
            let name = format!("j{j}o{op}");
            let s = model.new_int_var(0..=horizon, format!("s_{name}"));
            let d = model.new_constant(dur);
            let e = model.new_int_var(0..=horizon, format!("e_{name}"));
            let iv = model.new_interval_var(s, d, e, &name);
            machine_intervals[machine].push(iv);
            job_ops.push(OpVar { start: s, end: e });
        }
        ops.push(job_ops);
    }

    for job_ops in &ops {
        for w in job_ops.windows(2) {
            model.add((LinearExpr::from(w[0].end) - LinearExpr::from(w[1].start)).le(0));
        }
    }

    for intervals in &machine_intervals {
        if intervals.len() > 1 {
            model.add_no_overlap(intervals);
        }
    }

    let makespan = model.new_int_var(0..=horizon, "makespan");
    let all_ends: Vec<IntVar> = ops
        .iter()
        .filter_map(|j| j.last().map(|op| op.end))
        .collect();
    model.add_max_equality(makespan, &all_ends);
    model.minimize(makespan);

    let params = SatParameters::default()
        .with_max_time(30.0)
        .with_num_workers(8);

    let response = CpSolver::solve_with_params(&model, &params).unwrap();
    assert!(response.is_optimal(), "FT06 should be proved optimal");
    assert_eq!(response.value(makespan), 55, "FT06 optimal makespan is 55");
    eprintln!(
        "FT06: makespan={}, time={:.3}s",
        response.value(makespan),
        response.wall_time()
    );
}
