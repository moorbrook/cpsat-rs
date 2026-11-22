//! N-Queens: place N queens on an `NxN` chessboard so that no two attack each other.

use cpsat_rs::prelude::*;

fn main() {
    let n = 8usize;
    let max_coord = i64::try_from(n - 1).expect("board size fits i64");
    let mut model = CpModel::new();

    // One variable per row: queens[i] = column of the queen in row i.
    let queens: Vec<IntVar> = (0..n)
        .map(|i| model.new_int_var(0..=max_coord, format!("q{i}")))
        .collect();

    // All queens in different columns.
    model.add_all_different(&queens);

    // All queens on different diagonals.
    let diag1: Vec<IntVar> = (0..n)
        .map(|i| {
            let offset = i64::try_from(i).expect("row index fits i64");
            // diag1[i] = queens[i] + i
            let d = model.new_int_var(0..=(2 * max_coord), format!("d1_{i}"));
            model.add((LinearExpr::from(queens[i]) - LinearExpr::from(d) + offset).eq(0));
            d
        })
        .collect();
    model.add_all_different(&diag1);

    let diag2: Vec<IntVar> = (0..n)
        .map(|i| {
            let offset = i64::try_from(i).expect("row index fits i64");
            // diag2[i] = queens[i] - i
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
