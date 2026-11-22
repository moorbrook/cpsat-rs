# verify-cpsat Feature Map

Project-local verification coverage for `cpsat-rs`. Each feature document uses the four-section structure (`Sub-features`, `How to get to it (user POV)`, `Driving it with verify-cpsat.sh`, `Gotchas`).

| Feature | Crate / Test Surface | Script Selector | Doc |
| --- | --- | --- | --- |
| Solver and Constraints | `smoke`, `solution_validation`, crate doc-tests | `solver-and-constraints` | [solver-and-constraints.md](solver-and-constraints.md) |
| Scheduling and Benchmarks | `scheduling`, `jsp_benchmarks` (`ft06`, `ft10`, `la01`, `abz5`), `nqueens`, `jobshop` | `scheduling-and-benchmarks` | [scheduling-and-benchmarks.md](scheduling-and-benchmarks.md) |
| Properties and Safety | `proptest_roundtrip`, debug & `--release` `lib` FFI / checked-arithmetic unit tests | `properties-and-safety` | [properties-and-safety.md](properties-and-safety.md) |
| Lint, Doc, and Package | `cargo fmt --check`, strict `clippy`, `rustdoc`, `cargo package --list` exclusions | `lint-doc-and-package` | [lint-doc-and-package.md](lint-doc-and-package.md) |
