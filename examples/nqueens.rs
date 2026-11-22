use cpsat_rs::prelude::*;

fn main() {
    let n = 8usize;
    let max_coord = i64::try_from(n - 1).expect("board size fits i64");
    let mut model = CpModel::new();

    let queens: Vec<IntVar> = (0..n)
        .map(|i| model.new_int_var(0..=max_coord, format!("q{i}")))
        .collect();

    model.add_all_different(&queens);

    let diag1: Vec<IntVar> = (0..n)
        .map(|i| {
            let offset = i64::try_from(i).expect("row index fits i64");
            let d = model.new_int_var(0..=(2 * max_coord), format!("d1_{i}"));
            model.add((LinearExpr::from(queens[i]) - LinearExpr::from(d) + offset).eq(0));
            d
        })
        .collect();
    model.add_all_different(&diag1);

    let diag2: Vec<IntVar> = (0..n)
        .map(|i| {
            let offset = i64::try_from(i).expect("row index fits i64");
            let d = model.new_int_var((-max_coord)..=max_coord, format!("d2_{i}"));
            model.add((LinearExpr::from(queens[i]) - LinearExpr::from(d) - offset).eq(0));
            d
        })
        .collect();
    model.add_all_different(&diag2);

    let response = CpSolver::solve(&model).unwrap();

    if response.is_feasible() {
        println!("{n}-Queens solution:");
        for &q in &queens {
            let col = usize::try_from(response.value(q)).expect("column index fits usize");
            let mut row = vec!['.'; n];
            row[col] = 'Q';
            println!("  {}", row.iter().collect::<String>());
        }
    } else {
        println!("No solution found.");
    }
}
