// kernel/src/main.rs

#![cfg_attr(target_os = "none", no_std)] // Don't link the Rust standard library for bare-metal builds
#![cfg_attr(target_os = "none", no_main)] // Disable Rust entry points for bare-metal builds

#[cfg(target_os = "none")]
use core::panic::PanicInfo;
#[cfg(target_os = "none")]
use core::arch::global_asm;
#[cfg(target_os = "none")]
use bootloader::BootInfo; // Import BootInfo from the bootloader_api crate
use aetheros_kernel::{init, task};

#[cfg(target_os = "none")]
const KERNEL_BOOT_STACK_SIZE: usize = 4096 * 4;

#[cfg(target_os = "none")]
const _: () = {
    // SysV x86_64 requires 16-byte stack alignment at call boundaries.
    // Keep the static boot stack size a multiple of 16 so the computed
    // stack-top can always be aligned without crossing the allocation.
    assert!(KERNEL_BOOT_STACK_SIZE % 16 == 0);
};

#[cfg(target_os = "none")]
global_asm!(
    r#"
    .section .text._start, "ax"
    .global _start
_start:
    # bootloader_api 0.11 x86_64 handoff contract:
    # rdi = *mut BootInfo
    # SysV ABI notes:
    # - rbx is callee-saved, so we can carry BootInfo while switching stacks.
    # - We must not call into Rust before setting rsp because Rust functions can
    #   legally emit a prologue that touches stack memory.
    # - SysV ABI requires a cleared direction flag on function entry.
    # - SysV ABI requires 16-byte alignment before a `call`.
    mov rbx, rdi
    cld
    lea rsp, [{stack} + {size}]
    and rsp, -16
    xor rbp, rbp
    mov rdi, rbx
    call kernel_entry
1:
    hlt
    jmp 1b
"#,
    stack = sym STACK,
    size = const KERNEL_BOOT_STACK_SIZE,
);

#[cfg(target_os = "none")]
#[repr(C, align(16))]
struct KernelStack([u8; KERNEL_BOOT_STACK_SIZE]);

#[cfg(target_os = "none")]
#[no_mangle]
#[link_section = ".bss.stack"]
#[used]
static mut STACK: KernelStack = KernelStack([0; KERNEL_BOOT_STACK_SIZE]);

/// Kernel entry point in `no_std`/`no_main` mode.
///
/// Calling convention contract:
/// - `_start` (defined in global assembly above) receives `rdi = *mut BootInfo`.
/// - `_start` forwards that raw pointer unchanged to this function.
/// - this function is `extern "C"` so register/stack ABI matches the handoff.
#[no_mangle]
#[cfg(target_os = "none")]
pub unsafe extern "C" fn kernel_entry(boot_info_ptr: *mut BootInfo) -> ! {
    // SAFETY: bootloader_api's handoff contract guarantees:
    // - `boot_info_ptr` is non-null and points to a valid BootInfo.
    // - The kernel receives unique mutable access during early boot.
    // We convert exactly once here and do not create competing mutable aliases.
    let boot_info = unsafe {
        core::ptr::NonNull::new(boot_info_ptr)
            .expect("bootloader contract violated: BootInfo pointer was null")
            .as_mut()
    };

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

    // Enter an infinite loop to keep the kernel running.
    // In a real OS, this would be the idle loop, scheduling tasks.
    loop {
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
