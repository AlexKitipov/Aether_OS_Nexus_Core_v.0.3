use std::env;
use std::path::PathBuf;
use std::process;

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

    let builder = DiskImageBuilder::new(kernel.clone());

    match mode.as_str() {
        "bios" => builder
            .create_bios_image(&output)
            .map_err(|err| format!("failed to create BIOS image: {err}"))?,
        "uefi" => builder
            .create_uefi_image(&output)
            .map_err(|err| format!("failed to create UEFI image: {err}"))?,
        _ => return Err(usage()),
    }

    println!(
        "[image_builder] Created {mode} boot image from {} at {}",
        kernel.display(),
        output.display()
    );

    Ok(())
}

fn usage() -> String {
    "usage: aetheros-image-builder <bios|uefi> <kernel-elf> <output-image>".to_owned()
}
