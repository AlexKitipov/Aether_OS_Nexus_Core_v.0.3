#![no_std] // Don't link the Rust standard library
#![feature(abi_x86_interrupt)] // Required for x86_64 interrupt handling
#![feature(alloc_error_handler)] // Required for implementing a custom allocator
#![feature(const_mut_refs)] // Required for certain const mutable references

extern crate alloc;

use bootloader_api::BootInfo;
use core::panic::PanicInfo;

pub mod aetherfs;
pub mod arch;
pub mod caps;
pub mod console;
pub mod drivers;
pub mod elf;
pub mod heap;
pub mod ipc;
pub mod memory;
pub mod syscall;
pub mod task;
pub mod timer;
pub mod vnode_loader;

// Initialize the kernel.
pub fn init(boot_info: &'static BootInfo) {
    drivers::serial::init();
    kprintln!("[kernel] Serial output initialized.");

    arch::x86_64::gdt::init();
    kprintln!("[kernel] GDT initialized.");

    arch::x86_64::idt::init();
    kprintln!("[kernel] IDT initialized.");

    unsafe {
        arch::x86_64::irq::init_pic();
    }
    kprintln!("[kernel] PIC initialized.");

    x86_64::instructions::interrupts::enable();
    kprintln!("[kernel] Interrupts enabled.");

    memory::init(boot_info);
    kprintln!("[kernel] Memory manager initialized.");

    heap::init_heap();
    kprintln!("[kernel] Heap initialized.");

    task::init();
    kprintln!("[kernel] Task scheduler initialized.");

    ipc::init();
    kprintln!("[kernel] IPC system initialized.");

    aetherfs::init();
    kprintln!("[kernel] AetherFS initialized.");

    caps::init();
    kprintln!("[kernel] Capability system initialized.");

    timer::init();
    kprintln!("[kernel] Timer initialized.");

    kprintln!("[kernel] AetherOS kernel initialized successfully.");
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout)
}

// Macros for printing to the console
#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => ($crate::console::print_fmt(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! kprintln {
    () => ($crate::kprint!("
"));
    ($fmt:expr, $($arg:tt)*) => ($crate::kprint!(concat!($fmt, "
"), $($arg)*));
    ($fmt:expr) => ($crate::kprint!(concat!($fmt, "
")));
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kprintln!("[kernel] !!! KERNEL PANIC !!!");
    kprintln!("[kernel] Error: {}", info);
    // In a production system, this would involve a stack trace, dumping registers,
    // or rebooting. For now, we simply halt the system.
    hlt_loop();
}
