# Feature: Lint, Doc, and Package

## Sub-features
- Formatting discipline enforced via `cargo fmt --all --check` under Rust Edition `2024` (`rust-version = "1.88"`).
- Strict workspace lint configuration in `Cargo.toml`: `unsafe_op_in_unsafe_fn = "deny"`, `unused_qualifications = "warn"`, `undocumented_unsafe_blocks = "warn"`, `multiple_unsafe_ops_per_block = "warn"`, and `clippy::pedantic` enabled with `-D warnings` across all targets.
- Documentation build gate (`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`) and `.crate` tarball hygiene (`Cargo.toml` `exclude` keeping `.claude/**`, `.DS_Store`, and `rust-toolchain.toml` out of published packages).

## How to get to it (user POV)
- Run `cargo fmt --all --check`, `cargo clippy --all-targets --no-deps --locked -- -D warnings`, `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked`, and `cargo package --list --locked` before tagging or publishing.

## Driving it with verify-cpsat.sh
```bash
./.claude/skills/verify-cpsat/scripts/verify-cpsat.sh drive lint-doc-and-package proof-lint
```
The runner executes format check, full-target `clippy::pedantic` check, zero-warning `rustdoc` generation, and `cargo package --list --allow-dirty --locked`, asserting that `.claude/`, `.DS_Store`, and `rust-toolchain.toml` are absent from the package file list.

## Gotchas
- `cargo package --list` without `--allow-dirty` fails if the working tree has uncommitted changes; commit changes before running release `cargo package` / `cargo publish --dry-run --locked`.
- Generated protobuf modules in `src/lib.rs` carry a narrow `#[allow(missing_docs, clippy::pedantic, clippy::all)]` strictly on `mod proto` because `prost-build` emits upstream proto comments and types.
