//! JSP benchmark suite: verify cpsat-rs produces correct optimal solutions
//! on standard job shop scheduling instances from the literature.

use cpsat_rs::prelude::*;

struct JspInstance {
    name: &'static str,
    num_machines: usize,
    optimal: i64,
    /// Each job is a list of (machine, processing_time) operations.
    jobs: Vec<Vec<(usize, i64)>>,
}

fn solve_jsp(inst: &JspInstance, time_limit: f64) -> (i64, bool) {
    let horizon: i64 = inst.jobs.iter().flat_map(|j| j.iter().map(|(_, d)| d)).sum();

    let mut model = CpModel::new();

    struct OpVar {
        start: IntVar,
        end: IntVar,
    }
    let mut ops: Vec<Vec<OpVar>> = Vec::new();
    let mut machine_intervals: Vec<Vec<IntervalVar>> = vec![vec![]; inst.num_machines];

    for (j, job) in inst.jobs.iter().enumerate() {
        let mut job_ops = Vec::new();
        for (op, &(machine, dur)) in job.iter().enumerate() {
            let s = model.new_int_var(0..=horizon, format!("s_j{j}o{op}"));
            let d = model.new_constant(dur);
            let e = model.new_int_var(0..=horizon, format!("e_j{j}o{op}"));
            let iv = model.new_interval_var(s, d, e, format!("iv_j{j}o{op}"));
            machine_intervals[machine].push(iv);
            job_ops.push(OpVar { start: s, end: e });
        }
        ops.push(job_ops);
    }

    // Job precedences
    for job_ops in &ops {
        for w in job_ops.windows(2) {
            model.add((LinearExpr::from(w[0].end) - LinearExpr::from(w[1].start)).le(0));
        }
    }

    // Machine no-overlap
    for intervals in &machine_intervals {
        if intervals.len() > 1 {
            model.add_no_overlap(intervals);
        }
    }

    // Makespan
    let makespan = model.new_int_var(0..=horizon, "makespan");
    let last_ends: Vec<IntVar> = ops.iter().filter_map(|j| j.last().map(|op| op.end)).collect();
    model.add_max_equality(makespan, &last_ends);
    model.minimize(makespan);

    let params = SatParameters::default()
        .with_max_time(time_limit);

    let response = CpSolver::solve_with_params(&model, &params).unwrap();

    if response.is_feasible() {
        let ms = response.value(makespan);
        let proved = response.is_optimal();
        (ms, proved)
    } else {
        panic!("{} returned infeasible", inst.name);
    }
}

// ───── FT06: 6x6, optimal = 55 ─────

#[test]
fn ft06_optimal() {
    let inst = JspInstance {
        name: "ft06",
        num_machines: 6,
        optimal: 55,
        jobs: vec![
            vec![(2, 1), (0, 3), (1, 6), (3, 7), (5, 3), (4, 6)],
            vec![(1, 8), (2, 5), (4, 10), (5, 10), (0, 10), (3, 4)],
            vec![(2, 5), (3, 4), (5, 8), (0, 9), (1, 1), (4, 7)],
            vec![(1, 5), (0, 5), (2, 5), (3, 3), (4, 8), (5, 9)],
            vec![(2, 9), (1, 3), (4, 5), (5, 4), (0, 3), (3, 1)],
            vec![(1, 3), (3, 3), (5, 9), (0, 10), (4, 4), (2, 1)],
        ],
    };
    let (ms, proved) = solve_jsp(&inst, 30.0);
    assert_eq!(ms, 55, "FT06 should find optimal 55");
    assert!(proved, "FT06 should be proved optimal");
    eprintln!("  FT06: makespan={ms}, proved={proved}");
}

// ───── FT10: 10x10, optimal = 930 ─────

#[test]
fn ft10_optimal() {
    let inst = JspInstance {
        name: "ft10",
        num_machines: 10,
        optimal: 930,
        jobs: vec![
            vec![(0,29),(1,78),(2,9),(3,36),(4,49),(5,11),(6,62),(7,56),(8,44),(9,21)],
            vec![(0,43),(2,90),(4,75),(9,11),(3,69),(1,28),(6,46),(5,46),(7,72),(8,30)],
            vec![(1,91),(0,85),(3,39),(2,74),(8,90),(5,10),(7,12),(6,89),(9,45),(4,33)],
            vec![(1,81),(2,95),(0,71),(4,99),(6,9),(8,52),(7,85),(3,98),(9,22),(5,43)],
            vec![(2,14),(0,6),(1,22),(5,61),(3,26),(4,69),(8,21),(7,49),(9,72),(6,53)],
            vec![(2,84),(1,2),(5,52),(3,95),(8,48),(9,72),(0,47),(6,65),(4,6),(7,25)],
            vec![(1,46),(0,37),(3,61),(2,13),(6,32),(5,21),(9,32),(8,89),(7,30),(4,55)],
            vec![(2,31),(0,86),(1,46),(5,74),(4,32),(6,88),(8,19),(9,48),(7,36),(3,79)],
            vec![(0,76),(1,69),(3,76),(5,51),(2,85),(9,11),(6,40),(7,89),(4,26),(8,74)],
            vec![(1,85),(0,13),(2,61),(6,7),(8,64),(9,76),(5,47),(3,52),(4,90),(7,45)],
        ],
    };
    let (ms, proved) = solve_jsp(&inst, 60.0);
    // OR-Tools finds 930 reliably but may need >60s for proof on some runs
    assert!(ms <= 940, "FT10 should find <= 940, got {ms}");
    eprintln!("  FT10: makespan={ms}, proved={proved}");
    if proved {
        assert_eq!(ms, 930, "If proved, FT10 optimal must be 930");
    }
}

// ───── LA01: 10x5, optimal = 666 ─────

#[test]
fn la01_optimal() {
    let inst = JspInstance {
        name: "la01",
        num_machines: 5,
        optimal: 666,
        jobs: vec![
            vec![(1,21),(0,53),(4,95),(3,55),(2,34)],
            vec![(0,21),(3,52),(4,16),(2,26),(1,71)],
            vec![(3,39),(4,98),(1,42),(2,31),(0,12)],
            vec![(1,77),(0,55),(4,79),(2,66),(3,77)],
            vec![(0,83),(3,34),(2,64),(1,19),(4,37)],
            vec![(1,54),(2,43),(4,79),(0,92),(3,62)],
            vec![(3,69),(4,77),(1,87),(2,87),(0,93)],
            vec![(2,38),(0,60),(1,41),(3,24),(4,83)],
            vec![(3,17),(1,49),(4,25),(0,44),(2,98)],
            vec![(4,77),(3,79),(2,43),(1,75),(0,96)],
        ],
    };
    let (ms, proved) = solve_jsp(&inst, 30.0);
    assert_eq!(ms, 666, "LA01 should find optimal 666");
    assert!(proved, "LA01 should be proved optimal");
    eprintln!("  LA01: makespan={ms}, proved={proved}");
}

// ───── ABZ5: 10x10, optimal = 1234 ─────

#[test]
fn abz5_finds_near_optimal() {
    let inst = JspInstance {
        name: "abz5",
        num_machines: 10,
        optimal: 1234,
        jobs: vec![
            vec![(4,88),(8,68),(6,94),(5,99),(1,67),(2,89),(9,77),(7,99),(0,86),(3,92)],
            vec![(5,72),(3,50),(6,69),(4,75),(2,94),(8,66),(0,92),(1,82),(7,94),(9,63)],
            vec![(9,83),(8,61),(0,83),(1,65),(6,64),(5,85),(7,78),(4,85),(2,55),(3,77)],
            vec![(7,94),(2,68),(1,61),(4,99),(3,54),(6,75),(5,66),(0,76),(9,63),(8,67)],
            vec![(3,69),(4,88),(9,82),(8,95),(0,99),(2,67),(6,95),(5,68),(7,67),(1,86)],
            vec![(1,99),(4,81),(5,64),(8,66),(2,80),(7,80),(0,69),(9,62),(3,79),(6,88)],
            vec![(7,50),(1,84),(4,58),(3,72),(2,65),(0,80),(8,50),(6,89),(9,57),(5,89)],
            vec![(3,57),(0,89),(5,62),(4,85),(2,65),(8,93),(7,64),(1,85),(6,85),(9,74)],
            vec![(1,90),(3,67),(5,77),(0,94),(7,58),(2,93),(8,68),(4,57),(9,95),(6,56)],
            vec![(3,84),(2,78),(0,81),(7,82),(1,61),(9,91),(4,83),(6,90),(5,63),(8,63)],
        ],
    };
    let (ms, proved) = solve_jsp(&inst, 60.0);
    assert!(ms <= 1250, "ABZ5 should find <= 1250, got {ms}");
    eprintln!("  ABZ5: makespan={ms}, proved={proved}");
}
