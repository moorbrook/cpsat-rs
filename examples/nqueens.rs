//! N-Queens: place N queens on an NxN chessboard so that no two attack each other.

use cpsat_rs::prelude::*;

fn main() {
    let n = 8;
    let mut model = CpModel::new();

    // One variable per row: queens[i] = column of the queen in row i.
    let queens: Vec<IntVar> = (0..n)
        .map(|i| model.new_int_var(0..=(n - 1), format!("q{i}")))
        .collect();

    // All queens in different columns.
    model.add_all_different(&queens);

    // All queens on different diagonals.
    let diag1: Vec<IntVar> = (0..n)
        .map(|i| {
            // diag1[i] = queens[i] + i
            let d = model.new_int_var(0..=(2 * n - 2), format!("d1_{i}"));
            model.add((LinearExpr::from(queens[i as usize]) - LinearExpr::from(d) + i).eq(0));
            d
        })
        .collect();
    model.add_all_different(&diag1);

    let diag2: Vec<IntVar> = (0..n)
        .map(|i| {
            // diag2[i] = queens[i] - i
            let d = model.new_int_var(-(n - 1)..=(n - 1), format!("d2_{i}"));
            model.add((LinearExpr::from(queens[i as usize]) - LinearExpr::from(d) - i).eq(0));
            d
        })
        .collect();
    model.add_all_different(&diag2);

    let response = CpSolver::solve(&model).unwrap();

    if response.is_feasible() {
        println!("{n}-Queens solution:");
        for &q in queens.iter().take(n as usize) {
            let col = response.value(q);
            let mut row = vec!['.'; n as usize];
            row[col as usize] = 'Q';
            println!("  {}", row.iter().collect::<String>());
        }
    } else {
        println!("No solution found.");
    }
}
