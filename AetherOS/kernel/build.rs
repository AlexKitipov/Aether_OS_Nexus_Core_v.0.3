use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=linker.ld");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let linker_script = manifest_dir.join("linker.ld");

    println!("cargo:rustc-link-arg=-T");
    println!("cargo:rustc-link-arg={}", linker_script.display());
}
