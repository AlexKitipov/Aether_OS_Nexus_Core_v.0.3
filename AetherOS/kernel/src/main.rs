// kernel/src/main.rs

#![cfg_attr(target_os = "none", no_std)] // Don't link the Rust standard library for bare-metal builds
#![cfg_attr(target_os = "none", no_main)] // Disable Rust entry points for bare-metal builds

#[cfg(target_os = "none")]
use adi::interface::ADIInterface;
#[cfg(target_os = "none")]
use aetheros_kernel::{init, task};
#[cfg(target_os = "none")]
use apm::ApmManager;
#[cfg(target_os = "none")]
use arx::ArxManager;
#[cfg(target_os = "none")]
use bootloader_api::{entry_point, BootInfo, BootloaderConfig};
#[cfg(target_os = "none")]
use core::panic::PanicInfo;

#[cfg(target_os = "none")]
const BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    // Keep the stack-size contract from the previous hand-written entry path,
    // but let bootloader_api 0.11 own the stack setup and BootInfo ABI wrapper.
    config.kernel_stack_size = 4096 * 4;
    config
};

#[cfg(target_os = "none")]
entry_point!(kernel_entry, config = &BOOTLOADER_CONFIG);

/// Kernel entry point in `no_std`/`no_main` mode.
///
/// Calling convention contract:
/// - `bootloader_api::entry_point!` emits `_start` with the checked 0.11 ABI.
/// - the generated wrapper receives `&'static mut BootInfo` from the bootloader.
/// - this function never materializes a raw BootInfo pointer, so the unique
///   mutable handoff contract remains owned by the bootloader_api wrapper.
#[cfg(target_os = "none")]
fn kernel_entry(boot_info: &'static mut BootInfo) -> ! {
    // BootInfo layout assumptions (bootloader_api 0.11.15):
    // - `memory_regions` is passed by shared reference into allocator bootstrap.
    // - `framebuffer` is `Optional<FrameBuffer>` and is converted via `as_mut()`.
    // - `physical_memory_offset` is `Optional<u64>` and must be unwrapped via `into_option()`.
    // Kernel early initialization starts here.
    // Initialize all core kernel modules.
    // We pass the boot_info.memory_regions to the kernel's init function.
    init(
        &boot_info.memory_regions,
        boot_info.framebuffer.as_mut(),
        boot_info.physical_memory_offset,
    );

    aetheros_kernel::kprintln!("[kernel] Boot sequence complete, entering scheduler loop.");

    let adi = ADIInterface;
    let mut arx = ArxManager::new(&adi);
    let mut apm = ApmManager::new(&adi);

    // Enter an infinite loop to keep the kernel running.
    // In a real OS, this would be the idle loop, scheduling tasks.
    loop {
        aetheros_kernel::dev_interface::poll_once();
        arx.tick();
        apm.tick();

        if task::scheduler::take_reschedule_request() {
            task::schedule(); // Perform scheduling only when requested (e.g. from timer IRQ)
        }
        // Atomically (re-)enable interrupts and halt to avoid a race where an
        // IRQ arrives between the flag check above and a plain `hlt`.
        x86_64::instructions::interrupts::enable_and_hlt();
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
