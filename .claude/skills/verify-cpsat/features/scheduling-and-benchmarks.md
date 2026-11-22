# Feature: Scheduling and Benchmarks

## Sub-features
- Interval variables (`new_interval_var`), cumulative/no-overlap machine scheduling (`add_no_overlap`), and makespan minimization (`add_max_equality`).
- Standard literature Job Shop Scheduling Problem benchmarks (`ft06` opt `55`, `la01` opt `666`, `ft10` opt `930`, and canonical OR-Library `abz5` opt `1234`) asserted against `JspInstance::optimal`.
- Runnable CLI examples (`examples/nqueens.rs` and `examples/jobshop.rs`) exercising multi-worker OR-Tools solving.

## How to get to it (user POV)
- Run `cargo run --example nqueens` for an 8-Queens solution grid or `cargo run --example jobshop` to solve FT06 with search progress enabled (`Optimal makespan: 55`).
- Run `cargo nextest run --test scheduling --test jsp_benchmarks --locked`.

## Driving it with verify-cpsat.sh
```bash
./.claude/skills/verify-cpsat/scripts/verify-cpsat.sh drive scheduling-and-benchmarks proof-sched
```
The runner validates all scheduling tests, verifies all 4 JSP benchmark instances (`ft06_optimal`, `la01_optimal`, `ft10_finds_near_optimal`, `abz5_finds_near_optimal`), runs both examples, and asserts stdout contains `8-Queens solution:` and `Optimal makespan: 55`.

## Gotchas
- `ft10_finds_near_optimal` and `abz5_finds_near_optimal` cap search at `60.0` seconds: they enforce exact equality with `inst.optimal` whenever OR-Tools proves optimality and bounded optimality slack otherwise.
- Non-TTY interactive runs of `cargo nextest list` should set `PAGER=cat` to prevent nextest from invoking a pager.
