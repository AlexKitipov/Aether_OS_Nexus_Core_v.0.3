use std::env;
use std::path::{Path, PathBuf};
use std::process;

#[cfg(any(feature = "bios", feature = "uefi"))]
use bootloader::DiskImageBuilder;

fn main() {
    if let Err(err) = run() {
        eprintln!("[image_builder] ERROR: {err}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let mode = args.next().ok_or_else(usage)?;
    let kernel = PathBuf::from(args.next().ok_or_else(usage)?);
    let output = PathBuf::from(args.next().ok_or_else(usage)?);

    if args.next().is_some() {
        return Err(usage());
    }

    if !kernel.is_file() {
        return Err(format!("kernel ELF does not exist: {}", kernel.display()));
    }

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create output directory {}: {err}",
                parent.display()
            )
        })?;
    }

    match mode.as_str() {
        "bios" => create_bios_image(&kernel, &output)?,
        "uefi" => create_uefi_image(&kernel, &output)?,
        _ => return Err(usage()),
    }

    println!(
        "[image_builder] Created {mode} boot image from {} at {}",
        kernel.display(),
        output.display()
    );

    Ok(())
}

#[cfg(feature = "bios")]
fn create_bios_image(kernel: &Path, output: &Path) -> Result<(), String> {
    DiskImageBuilder::new(kernel.to_path_buf())
        .create_bios_image(output)
        .map_err(|err| format!("failed to create BIOS image: {err}"))
}

#[cfg(not(feature = "bios"))]
fn create_bios_image(_kernel: &Path, _output: &Path) -> Result<(), String> {
    Err(
        "BIOS image support is disabled; rebuild aetheros-image-builder with --features bios"
            .to_owned(),
    )
}

#[cfg(feature = "uefi")]
fn create_uefi_image(kernel: &Path, output: &Path) -> Result<(), String> {
    DiskImageBuilder::new(kernel.to_path_buf())
        .create_uefi_image(output)
        .map_err(|err| format!("failed to create UEFI image: {err}"))
}

#[cfg(not(feature = "uefi"))]
fn create_uefi_image(_kernel: &Path, _output: &Path) -> Result<(), String> {
    Err(
        "UEFI image support is disabled; rebuild aetheros-image-builder with --features uefi"
            .to_owned(),
    )
}

fn usage() -> String {
    "usage: aetheros-image-builder <bios|uefi> <kernel-elf> <output-image>".to_owned()
}
