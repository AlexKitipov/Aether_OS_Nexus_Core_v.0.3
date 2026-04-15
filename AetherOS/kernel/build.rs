use std::path::PathBuf;

fn main() {
    // This build.rs is simplified to be a no-op when `bootimage` is used.
    // The actual bootloader binaries are compiled by `bootimage` itself.
    // We just emit placeholder cargo:rustc-env variables to satisfy the build script.

    #[cfg(feature = "uefi")]
    {
        // This path is where bootimage will place the final UEFI bootloader image.
        // We are declaring it here for build script satisfaction, not actually building it.
        println!("cargo:rustc-env=UEFI_BOOTLOADER_PATH={}", 
            PathBuf::from(std::env::var("OUT_DIR").unwrap())
                .join("bin")
                .join("bootloader-x86_64-uefi.efi")
                .display()
        );
    }
    
    #[cfg(feature = "bios")]
    {
        // These are placeholder paths for BIOS, if the BIOS feature were active.
        // In our UEFI-only setup, this branch is not taken.
        println!("cargo:rustc-env=BIOS_BOOT_SECTOR_PATH={}", "/path/to/bootloader/boot_sector");
        println!("cargo:rustc-env=BIOS_STAGE_2_PATH={}", "/path/to/bootloader/stage_2");
        println!("cargo:rustc-env=BIOS_STAGE_3_PATH={}", "/path/to/bootloader/stage_3");
        println!("cargo:rustc-env=BIOS_STAGE_4_PATH={}", "/path/to/bootloader/stage_4");
    }
}