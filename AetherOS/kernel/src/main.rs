// kernel/src/main.rs

#![no_std] // Don't link the Rust standard library
#![no_main] // Disable all Rust-level entry points

use core::panic::PanicInfo;
use bootloader_api::BootInfo; // Import BootInfo from the bootloader_api crate

/// The main entry point for the AetherOS kernel.
/// This function is called by the bootloader after setting up basic environment.
#[no_mangle] // Don't mangle the name of this function, so the bootloader can find it
pub extern "C" fn _start(boot_info: &'static mut BootInfo) -> ! {
    // Initialize all core kernel modules.
    // We pass the boot_info.memory_regions to the kernel's init function.
    crate::init(&boot_info.memory_regions);

    crate::kprintln!("[kernel] Welcome to AetherOS!");

    // Enter an infinite loop to keep the kernel running.
    // In a real OS, this would be the idle loop, scheduling tasks.
    loop {
        crate::task::schedule(); // Give control to the scheduler
        x86_64::instructions::hlt(); // Halt the CPU until the next interrupt
    }
}

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crate::kprintln!("[kernel] !!! KERNEL PANIC !!!");
    crate::kprintln!("[kernel] Error: {}", info);
    // In a production system, this would involve a stack trace, dumping registers,
    // or rebooting. For now, we simply halt the system.
    loop {
        x86_64::instructions::hlt(); // Halt the CPU
    }
}

