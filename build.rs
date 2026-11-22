use std::env;
use std::path::PathBuf;

const ORTOOLS_PROTO_VERSION: &str = "9.15";

fn main() {
    println!("cargo:rustc-env=CPSAT_RS_ORTOOLS_PROTO_VERSION={ORTOOLS_PROTO_VERSION}");

    prost_build::compile_protos(
        &["proto/cp_model.proto", "proto/sat_parameters.proto"],
        &["proto/"],
    )
    .expect(
        "proto compilation failed. Ensure `protoc` is installed: \
         `brew install protobuf` (macOS) or `apt install protobuf-compiler` (Linux), \
         or set the PROTOC env var to the protoc binary path.",
    );

    let ortools_prefix = env::var("ORTOOLS_PREFIX")
        .or_else(|_| env::var("ORTOOL_PREFIX"))
        .unwrap_or_else(|_| {
            let candidates = [
                "/opt/homebrew/opt/or-tools",
                "/usr/local/opt/or-tools",
                "/usr",
                "/usr/local",
                "/opt/or-tools",
                "/opt/ortools",
            ];
            for path in &candidates {
                if PathBuf::from(path).join("include/ortools").exists() {
                    return path.to_string();
                }
            }
            panic!(
                "OR-Tools not found. Install with:\n  \
                 macOS:  brew install or-tools\n  \
                 Linux:  see https://developers.google.com/optimization/install\n  \
                 Or set ORTOOLS_PREFIX env var to your OR-Tools installation prefix."
            );
        });

    let ortools_lib = format!("{ortools_prefix}/lib");

    println!("cargo:rustc-link-search=native={ortools_lib}");
    println!("cargo:rustc-link-lib=dylib=ortools");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    match target_os.as_str() {
        "macos" => println!("cargo:rustc-link-lib=dylib=c++"),
        "windows" => {}
        _ => println!("cargo:rustc-link-lib=dylib=stdc++"),
    }

    let protobuf_lib = format!("{ortools_prefix}/lib");
    if PathBuf::from(&protobuf_lib)
        .join("libprotobuf.dylib")
        .exists()
        || PathBuf::from(&protobuf_lib).join("libprotobuf.so").exists()
    {
        println!("cargo:rustc-link-lib=dylib=protobuf");
    }
    if PathBuf::from("/opt/homebrew/opt/protobuf/lib").exists() {
        println!("cargo:rustc-link-search=native=/opt/homebrew/opt/protobuf/lib");
        println!("cargo:rustc-link-lib=dylib=protobuf");
    }

    println!("cargo:rerun-if-changed=proto/cp_model.proto");
    println!("cargo:rerun-if-changed=proto/sat_parameters.proto");
    println!("cargo:rerun-if-env-changed=ORTOOLS_PREFIX");
    println!("cargo:rerun-if-env-changed=ORTOOL_PREFIX");
    println!("cargo:rerun-if-env-changed=PROTOC");
}
