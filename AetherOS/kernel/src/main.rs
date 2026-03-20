// kernel/src/main.rs

#![cfg_attr(target_os = "none", no_std)] // Don't link the Rust standard library for bare-metal builds
#![cfg_attr(target_os = "none", no_main)] // Disable Rust entry points for bare-metal builds

#[cfg(target_os = "none")]
use core::panic::PanicInfo;
#[cfg(target_os = "none")]
use bootloader_api::BootInfo; // Import BootInfo from the bootloader_api crate
use aetheros_kernel::{init, task};

/// Kernel entry point in `no_std`/`no_main` mode.
///
/// We export `_start` with `#[no_mangle]` so the symbol name stays exactly `_start`
/// and the bootloader/CPU can jump to it directly.
#[no_mangle] // Don't mangle the name of this function, so the bootloader can find it
#[cfg(target_os = "none")]
pub extern "C" fn _start(boot_info: &'static mut BootInfo) -> ! {
    // Kernel early initialization starts here.
    // Initialize all core kernel modules.
    // We pass the boot_info.memory_regions to the kernel's init function.
    init(&boot_info.memory_regions, boot_info.framebuffer.as_mut());

    aetheros_kernel::kprintln!("[kernel] Welcome to AetherOS!");

    // Enter an infinite loop to keep the kernel running.
    // In a real OS, this would be the idle loop, scheduling tasks.
    loop {
        task::schedule(); // Give control to the scheduler
        x86_64::instructions::hlt(); // Halt the CPU until the next interrupt
    }
}

#[cfg(not(target_os = "none"))]
fn main() {
    println!("aetheros-kernel host stub: build the real kernel with the bare-metal target.");
}

/// This function is called on panic.
#[cfg(target_os = "none")]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    aetheros_kernel::kprintln!("[kernel] !!! KERNEL PANIC !!!");
    aetheros_kernel::kprintln!("[kernel] Error: {}", info);
    // In a production system, this would involve a stack trace, dumping registers,
    // or rebooting. For now, we simply halt the system.
    loop {
        x86_64::instructions::hlt(); // Halt the CPU
    }
}
