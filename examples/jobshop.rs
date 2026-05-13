//! Job Shop Scheduling: minimize makespan for the classic FT06 benchmark.
//!
//! FT06 (Fisher & Thompson, 1963): 6 jobs, 6 machines, optimal makespan = 55.

use cpsat_rs::prelude::*;

fn main() {
    // Each job is a list of (machine, processing_time) operations.
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

    // Create interval variables for each operation.
    struct OpVar {
        start: IntVar,
        end: IntVar,
        _interval: IntervalVar,
    }
    let mut ops: Vec<Vec<OpVar>> = Vec::new();
    let mut machine_intervals: Vec<Vec<IntervalVar>> = vec![vec![]; n_machines];

    for (j, job) in jobs.iter().enumerate() {
        let mut job_ops = Vec::new();
        for (op, &(machine, dur)) in job.iter().enumerate() {
            let s = model.new_int_var(0..=horizon, format!("s_j{j}o{op}"));
            let d = model.new_constant(dur);
            let e = model.new_int_var(0..=horizon, format!("e_j{j}o{op}"));
            let iv = model.new_interval_var(s, d, e, format!("iv_j{j}o{op}"));
            machine_intervals[machine].push(iv);
            job_ops.push(OpVar {
                start: s,
                end: e,
                _interval: iv,
            });
        }
        ops.push(job_ops);
    }

    // Job precedences: each operation must finish before the next starts.
    for job_ops in &ops {
        for w in job_ops.windows(2) {
            model.add((LinearExpr::from(w[0].end) - LinearExpr::from(w[1].start)).le(0));
        }
    }

    // Machine no-overlap: operations on the same machine cannot overlap.
    for intervals in &machine_intervals {
        if intervals.len() > 1 {
            model.add_no_overlap(intervals);
        }
    }

    // Makespan = max of all job completion times.
    let makespan = model.new_int_var(0..=horizon, "makespan");
    let last_ends: Vec<IntVar> = ops
        .iter()
        .filter_map(|j| j.last().map(|op| op.end))
        .collect();
    model.add_max_equality(makespan, &last_ends);
    model.minimize(makespan);

    // Solve with 8 workers, 30s time limit.
    let params = SatParameters::default()
        .with_max_time(30.0)
        .with_num_workers(8)
        .with_log_search_progress(true);

    let response = CpSolver::solve_with_params(&model, &params).unwrap();

    match response.status() {
        CpSolverStatus::Optimal => {
            println!("Optimal makespan: {}", response.value(makespan));
        }
        CpSolverStatus::Feasible => {
            println!(
                "Best makespan found: {} (not proved optimal)",
                response.value(makespan)
            );
        }
        _ => {
            println!("No solution found (status: {:?})", response.status());
            return;
        }
    }

    // Print schedule.
    println!("\nSchedule:");
    for (j, job_ops) in ops.iter().enumerate() {
        print!("  Job {j}: ");
        for (op, ov) in job_ops.iter().enumerate() {
            let s = response.value(ov.start);
            let e = response.value(ov.end);
            let machine = jobs[j][op].0;
            print!("M{machine}[{s}-{e}] ");
        }
        println!();
    }
    println!("\nSolved in {:.3}s", response.wall_time());
}
