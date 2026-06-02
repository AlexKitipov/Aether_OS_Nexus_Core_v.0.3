fn main() {
    // The kernel package uses the bootloader_api/bootloader 0.11 flow instead of
    // legacy cargo-bootimage. The repository-level image builder consumes the
    // compiled kernel ELF after `cargo build`; this build script intentionally
    // avoids advertising bootimage-specific output paths.
    println!("cargo:rerun-if-changed=build.rs");
}
