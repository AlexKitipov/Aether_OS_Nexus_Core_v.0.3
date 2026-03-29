use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=linker.ld");

    let toolchain = Command::new("rustc")
        .arg("--version")
        .output()
        .expect("failed to run rustc --version");

    let version = String::from_utf8_lossy(&toolchain.stdout);
    if !version.contains("nightly") {
        panic!("AetherOS requires nightly Rust toolchain.");
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
    let linker_script = manifest_dir.join("linker.ld");

    println!("cargo:rustc-link-arg-bin=aetheros-kernel=-T{}", linker_script.display());
}
