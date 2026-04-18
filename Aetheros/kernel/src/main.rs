// kernel/src/main.rs

#![no_std] // Don't link the Rust standard library
#![no_main] // Disable all Rust-level entry points

use bootloader_api::BootInfo; // Import BootInfo from the bootloader_api crate
use core::panic::PanicInfo;

/// The main entry point for the AetherOS kernel.
/// This function is called by the bootloader after setting up basic environment.
#[no_mangle] // Don't mangle the name of this function, so the bootloader can find it
pub extern "C" fn _start(boot_info: &'static mut BootInfo) -> ! {
    crate::init(&boot_info.memory_regions);

    crate::kprintln!("[kernel] Welcome to AetherOS!");
    crate::kprintln!("[kernel] Entering idle loop; scheduler is timer-preemptive.");

    loop {
        x86_64::instructions::hlt(); // Resume on interrupt; timer drives scheduling.
    }
}

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crate::kprintln!("[kernel] !!! KERNEL PANIC !!!");
    crate::kprintln!("[kernel] Error: {}", info);
    loop {
        x86_64::instructions::hlt();
    }
}
