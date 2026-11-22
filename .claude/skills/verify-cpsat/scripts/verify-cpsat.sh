#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEFAULT_REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
REPO_ROOT="${REPO_ROOT:-$DEFAULT_REPO_ROOT}"
WORK_BASE="/tmp/cpsat-verify-work"
PROOF_BASE="/tmp/cpsat-verify-proofs"

usage() {
  cat <<'EOF'
Usage: verify-cpsat.sh <subcommand> [args...]

Subcommands:
  launch                      Build all library, test, and example targets (--locked)
  doctor                      Validate cargo, nextest, protoc, jq, OR-Tools, and writable /tmp
  drive <feature|all> [id]    Run verification suite(s) and store proofs under /tmp/cpsat-verify-proofs/<id>
  cleanup <id>                Remove scratch workspace /tmp/cpsat-verify-work/<id> while preserving proofs

Features for drive:
  solver-and-constraints
  scheduling-and-benchmarks
  properties-and-safety
  lint-doc-and-package
  all
EOF
}

die() {
  echo "ERROR: $*" >&2
  exit 1
}

find_ortools_prefix() {
  if [ -n "${ORTOOLS_PREFIX:-}" ] && [ -d "${ORTOOLS_PREFIX}/include/ortools" ]; then
    printf '%s\n' "$ORTOOLS_PREFIX"
    return 0
  fi
  if [ -n "${ORTOOL_PREFIX:-}" ] && [ -d "${ORTOOL_PREFIX}/include/ortools" ]; then
    printf '%s\n' "$ORTOOL_PREFIX"
    return 0
  fi
  local p
  for p in \
    /opt/homebrew/opt/or-tools \
    /usr/local/opt/or-tools \
    /usr \
    /usr/local \
    /opt/or-tools \
    /opt/ortools
  do
    if [ -d "$p/include/ortools" ]; then
      printf '%s\n' "$p"
      return 0
    fi
  done
  return 1
}

record_step() {
  local proof_dir="$1"
  local step_name="$2"
  local expected_ec="$3"
  shift 3

  local step_dir="$proof_dir/$step_name"
  mkdir -p "$step_dir"
  printf '%q ' "$@" > "$step_dir/command.txt"
  printf '\n' >> "$step_dir/command.txt"

  local actual_ec=0
  (
    cd "$REPO_ROOT"
    "$@"
  ) > "$step_dir/stdout.txt" 2> "$step_dir/stderr.txt" || actual_ec=$?
  printf '%s\n' "$actual_ec" > "$step_dir/exit_code.txt"

  if [ "$actual_ec" != "$expected_ec" ]; then
    echo "FAIL [$step_name]: expected exit $expected_ec, got $actual_ec" >&2
    cat "$step_dir/stderr.txt" >&2 || true
    exit 1
  fi
}

cmd_launch() {
  [ -f "$REPO_ROOT/Cargo.toml" ] || die "Cargo.toml not found under REPO_ROOT=$REPO_ROOT"
  (
    cd "$REPO_ROOT"
    cargo test --no-run --all-targets --locked
  )
  echo "LAUNCHED: all targets compiled cleanly in $REPO_ROOT"
}

cmd_doctor() {
  [ -f "$REPO_ROOT/Cargo.toml" ] || die "Cargo.toml not found under REPO_ROOT=$REPO_ROOT"
  command -v cargo >/dev/null 2>&1 || die "cargo not found on PATH"
  command -v cargo-nextest >/dev/null 2>&1 || die "cargo-nextest not found on PATH"
  command -v protoc >/dev/null 2>&1 || die "protoc not found on PATH"
  command -v jq >/dev/null 2>&1 || die "jq not found on PATH"

  local ortools
  ortools="$(find_ortools_prefix)" || die "OR-Tools headers (include/ortools) not found; set ORTOOLS_PREFIX"

  mkdir -p "$WORK_BASE" "$PROOF_BASE"
  local probe
  probe="$(mktemp "$WORK_BASE/.doctor.XXXXXX")"
  rm -f "$probe"
  echo "DOCTOR OK: repo=$REPO_ROOT protoc=$(protoc --version) ortools=$ortools"
}

drive_solver_and_constraints() {
  local proof_dir="$1"

  record_step "$proof_dir" "nextest_smoke_and_validation" 0 \
    cargo nextest run --test smoke --test solution_validation --locked --no-fail-fast --status-level pass

  record_step "$proof_dir" "doctests" 0 \
    cargo test --doc --locked
}

drive_scheduling_and_benchmarks() {
  local proof_dir="$1"

  record_step "$proof_dir" "nextest_scheduling_and_jsp" 0 \
    cargo nextest run --test scheduling --test jsp_benchmarks --locked --no-fail-fast --status-level pass

  record_step "$proof_dir" "example_nqueens" 0 \
    cargo run --example nqueens --locked
  grep -q '8-Queens solution:' "$proof_dir/example_nqueens/stdout.txt" \
    || die "example_nqueens did not print expected solution header"

  record_step "$proof_dir" "example_jobshop" 0 \
    cargo run --example jobshop --locked
  grep -q 'Optimal makespan: 55' "$proof_dir/example_jobshop/stdout.txt" \
    || die "example_jobshop did not report Optimal makespan: 55"
}

drive_properties_and_safety() {
  local proof_dir="$1"

  record_step "$proof_dir" "nextest_proptest_and_lib" 0 \
    cargo nextest run --test proptest_roundtrip --lib --locked --no-fail-fast --status-level pass

  record_step "$proof_dir" "release_lib_overflow_and_ffi_tests" 0 \
    cargo test --release --lib --locked
}

drive_lint_doc_and_package() {
  local proof_dir="$1"

  record_step "$proof_dir" "rustfmt_check" 0 \
    cargo fmt --all --check

  record_step "$proof_dir" "clippy_pedantic_all_targets" 0 \
    cargo clippy --all-targets --no-deps --locked -- -D warnings

  record_step "$proof_dir" "rustdoc_warnings_denied" 0 \
    env RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked

  record_step "$proof_dir" "package_list" 0 \
    cargo package --list --allow-dirty --locked
  if grep -qE '^(\.claude/|\.DS_Store$|rust-toolchain\.toml$)' "$proof_dir/package_list/stdout.txt"; then
    die "package_list unexpectedly includes excluded dev/verification files"
  fi
}

cmd_drive() {
  local feature="${1:-}"
  local run_id="${2:-run-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
  [ -n "$feature" ] || die "drive requires <feature|all>"

  cmd_doctor >/dev/null

  local work_dir="$WORK_BASE/$run_id"
  local proof_dir="$PROOF_BASE/$run_id"
  mkdir -p "$work_dir" "$proof_dir"
  printf 'run_id=%s\nfeature=%s\n' "$run_id" "$feature" > "$work_dir/session.meta"

  local features_ran=()
  case "$feature" in
    solver-and-constraints)
      drive_solver_and_constraints "$proof_dir"
      features_ran+=("solver-and-constraints")
      ;;
    scheduling-and-benchmarks)
      drive_scheduling_and_benchmarks "$proof_dir"
      features_ran+=("scheduling-and-benchmarks")
      ;;
    properties-and-safety)
      drive_properties_and_safety "$proof_dir"
      features_ran+=("properties-and-safety")
      ;;
    lint-doc-and-package)
      drive_lint_doc_and_package "$proof_dir"
      features_ran+=("lint-doc-and-package")
      ;;
    all)
      drive_solver_and_constraints "$proof_dir"
      drive_scheduling_and_benchmarks "$proof_dir"
      drive_properties_and_safety "$proof_dir"
      drive_lint_doc_and_package "$proof_dir"
      features_ran+=(
        "solver-and-constraints"
        "scheduling-and-benchmarks"
        "properties-and-safety"
        "lint-doc-and-package"
      )
      ;;
    *)
      die "unknown feature '$feature' (expected solver-and-constraints|scheduling-and-benchmarks|properties-and-safety|lint-doc-and-package|all)"
      ;;
  esac

  local features_json
  features_json="$(printf '%s\n' "${features_ran[@]}" | jq -R . | jq -s .)"
  local steps_count
  steps_count="$(find "$proof_dir" -mindepth 1 -maxdepth 1 -type d | wc -l | tr -d ' ')"

  jq -n \
    --arg run_id "$run_id" \
    --arg feature "$feature" \
    --arg work_dir "$work_dir" \
    --arg proof_dir "$proof_dir" \
    --arg repo_root "$REPO_ROOT" \
    --argjson features "$features_json" \
    --argjson steps_count "$steps_count" \
    '{
      ok: true,
      run_id: $run_id,
      requested_feature: $feature,
      features_passed: $features,
      steps_recorded: $steps_count,
      work_dir: $work_dir,
      proof_dir: $proof_dir,
      repo_root: $repo_root
    }' > "$proof_dir/summary.json"

  echo "DRIVE OK: feature=$feature steps=$steps_count"
  echo "PROOF_DIR=$proof_dir"
  echo "WORK_DIR=$work_dir"
}

cmd_cleanup() {
  local run_id="${1:-}"
  [ -n "$run_id" ] || die "cleanup requires <run-id>"
  local work_dir="$WORK_BASE/$run_id"
  local proof_dir="$PROOF_BASE/$run_id"

  rm -rf "$work_dir"
  [ ! -e "$work_dir" ] || die "failed to remove scratch workspace: $work_dir"
  [ -d "$proof_dir" ] || die "expected proof directory missing: $proof_dir"
  [ -f "$proof_dir/summary.json" ] || die "expected proof summary missing: $proof_dir/summary.json"
  echo "CLEANUP OK: removed=$work_dir preserved_proofs=$proof_dir"
}

main() {
  local subcmd="${1:-}"
  case "$subcmd" in
    launch) shift; cmd_launch "$@" ;;
    doctor) shift; cmd_doctor "$@" ;;
    drive) shift; cmd_drive "$@" ;;
    cleanup) shift; cmd_cleanup "$@" ;;
    -h|--help|"") usage ;;
    *) die "unknown subcommand '$subcmd'" ;;
  esac
}

main "$@"
