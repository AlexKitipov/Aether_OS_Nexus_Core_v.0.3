// kernel/src/main.rs

#![cfg_attr(target_os = "none", no_std)] // Don't link the Rust standard library for bare-metal builds
#![cfg_attr(target_os = "none", no_main)] // Disable Rust entry points for bare-metal builds

#[cfg(target_os = "none")]
use core::panic::PanicInfo;
#[cfg(target_os = "none")]
use core::arch::global_asm;
#[cfg(target_os = "none")]
use bootloader_api::BootInfo; // Import BootInfo from the bootloader_api crate
use aetheros_kernel::{init, task};

#[cfg(target_os = "none")]
global_asm!(
    r#"
    .section .text._start, "ax"
    .global _start
_start:
    mov rbx, rdi
    call init_stack
    mov rdi, rbx
    call kernel_entry
1:
    hlt
    jmp 1b
"#
);

#[cfg(target_os = "none")]
#[repr(align(16))]
struct KernelStack([u8; 4096 * 4]);

#[cfg(target_os = "none")]
#[no_mangle]
#[link_section = ".bss.stack"]
static mut STACK: KernelStack = KernelStack([0; 4096 * 4]);

/// Initializes the bootstrap kernel stack and enforces SysV 16-byte alignment.
#[cfg(target_os = "none")]
#[no_mangle]
pub unsafe extern "C" fn init_stack() {
    core::arch::asm!(
        "lea rsp, [{stack} + {size}]",
        "and rsp, -16",
        stack = sym STACK,
        size = const 4096 * 4,
        options(nostack, preserves_flags)
    );
}

/// Kernel entry point in `no_std`/`no_main` mode.
///
/// We export `_start` with `#[no_mangle]` so the symbol name stays exactly `_start`
/// and the bootloader/CPU can jump to it directly.
#[no_mangle] // Don't mangle the name of this function, so the bootloader can find it
#[cfg(target_os = "none")]
pub extern "C" fn kernel_entry(boot_info: &'static mut BootInfo) -> ! {
    // Kernel early initialization starts here.
    // Initialize all core kernel modules.
    // We pass the boot_info.memory_regions to the kernel's init function.
    init(
        &boot_info.memory_regions,
        boot_info.framebuffer.as_mut(),
        boot_info.physical_memory_offset,
    );

    aetheros_kernel::kprintln!("[kernel] Boot sequence complete, entering scheduler loop.");

    // Enter an infinite loop to keep the kernel running.
    // In a real OS, this would be the idle loop, scheduling tasks.
    loop {
        if task::scheduler::take_reschedule_request() {
            task::schedule(); // Perform scheduling only when requested (e.g. from timer IRQ)
        }
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
