//! Memory-safety & leak test for the C-ABI surface.
//!
//! Runs the C harness (compiled at build time via the `cc` crate) under
//! valgrind against the Rust-built `libldacBT_dec.so`, asserting zero
//! errors and zero leaks.  Skips gracefully if valgrind is unavailable.

use std::path::{Path, PathBuf};
use std::process::Command;

fn find_rust_cdylib() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let profile = exe.parent()?.parent()?;
    let cand = profile.join("libldacBT_dec.so");
    cand.exists().then_some(cand)
}

#[test]
fn valgrind_no_leaks() {
    if Command::new("valgrind").arg("--version").output().is_err() {
        eprintln!("valgrind not found, skipping");
        return;
    }
    let Some(so) = find_rust_cdylib() else {
        panic!("libldacBT_dec.so not built");
    };

    let harness = env!("LDAC_VALGRIND_HARNESS");
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures");

    let out = Command::new("valgrind")
        .args([
            "--error-exitcode=42",
            "--leak-check=full",
            "--show-leak-kinds=definite,indirect,possible",
            "--errors-for-leak-kinds=definite,indirect,possible",
        ])
        .arg(harness)
        .arg(&so)
        .arg(&fixtures)
        .output()
        .expect("run valgrind");

    let stderr = String::from_utf8_lossy(&out.stderr);
    eprintln!("{stderr}");

    assert!(
        out.status.success(),
        "valgrind reported errors or leaks (exit {:?})",
        out.status.code()
    );
    assert!(
        stderr.contains("ERROR SUMMARY: 0 errors"),
        "valgrind error summary not clean"
    );
}
