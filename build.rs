use std::env;
use std::path::PathBuf;

fn main() {
    // 1. Compile protos with prost
    prost_build::compile_protos(
        &["proto/cp_model.proto", "proto/sat_parameters.proto"],
        &["proto/"],
    )
    .expect("proto compilation failed");

    // 2. Find OR-Tools
    let ortools_prefix = env::var("ORTOOLS_PREFIX")
        .or_else(|_| env::var("ORTOOL_PREFIX"))
        .unwrap_or_else(|_| {
            for path in &[
                "/opt/homebrew/opt/or-tools",
                "/usr/local/opt/or-tools",
                "/opt/ortools",
                "/usr/local",
            ] {
                if PathBuf::from(path).join("include/ortools").exists() {
                    return path.to_string();
                }
            }
            panic!(
                "OR-Tools not found. Install with `brew install or-tools` \
                 or set ORTOOLS_PREFIX env var."
            );
        });

    let ortools_lib = format!("{ortools_prefix}/lib");

    // 3. Link OR-Tools (uses the C API directly, no C++ shim needed)
    println!("cargo:rustc-link-search=native={ortools_lib}");
    println!("cargo:rustc-link-lib=dylib=ortools");

    // Link C++ stdlib (needed because OR-Tools is C++)
    println!("cargo:rustc-link-lib=dylib=c++");

    // Rerun triggers
    println!("cargo:rerun-if-changed=proto/cp_model.proto");
    println!("cargo:rerun-if-changed=proto/sat_parameters.proto");
    println!("cargo:rerun-if-env-changed=ORTOOLS_PREFIX");
    println!("cargo:rerun-if-env-changed=ORTOOL_PREFIX");
}
