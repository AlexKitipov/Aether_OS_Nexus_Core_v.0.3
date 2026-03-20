use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let toolchain = Command::new("rustc")
        .arg("--version")
        .output()
        .expect("failed to run rustc --version");

    let version = String::from_utf8_lossy(&toolchain.stdout);
    if !version.contains("nightly") {
        panic!("AetherOS requires nightly Rust toolchain.");
    }
}
