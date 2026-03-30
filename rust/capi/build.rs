//! Compile the C valgrind harness at build time using the `cc` crate
//! so the test doesn't have to discover a compiler at runtime.

use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let src = "tests/valgrind_harness.c";
    println!("cargo:rerun-if-changed={src}");

    // We want a standalone executable, not an object for linking into
    // the Rust crate, so drive the compiler manually via cc::Build's
    // tool discovery rather than .compile().
    let tool = cc::Build::new().get_compiler();
    let harness = out_dir.join("valgrind_harness");

    let status = tool
        .to_command()
        .args(["-O1", "-g", "-o"])
        .arg(&harness)
        .arg(src)
        .arg("-ldl")
        .status()
        .expect("invoke C compiler");
    if !status.success() {
        panic!("failed to compile {src}");
    }

    // Expose the path to the test via an env var.
    println!(
        "cargo:rustc-env=LDAC_VALGRIND_HARNESS={}",
        harness.display()
    );
}
