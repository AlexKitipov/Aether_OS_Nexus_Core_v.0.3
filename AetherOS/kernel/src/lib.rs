#![no_std] // Don't link the Rust standard library
#![feature(abi_x86_interrupt)] // Required for x86_64 interrupt handling
#![cfg_attr(target_os = "none", feature(alloc_error_handler))] // Only needed for bare-metal allocator error hooks

extern crate alloc;
extern crate core;


use bootloader::info::{FrameBuffer, MemoryRegions, Optional};

pub mod arch;
pub mod drivers;
pub mod device;
pub mod memory;
pub mod task;
pub mod ipc;
pub mod syscall;
pub mod console;
pub mod heap;
pub mod aetherfs;
pub mod elf;
pub mod vnode_loader;
pub mod caps;
pub mod timer;
pub mod gdt;
pub mod idt;
pub mod interrupts;
pub mod usercopy;
pub mod config;
pub mod network;
pub mod runtime;
pub mod dev_interface;
pub mod snapshot_engine;

/// Initialize all kernel subsystems in a deterministic startup order.
pub fn init(
    memory_regions: &'static MemoryRegions,
    framebuffer: Option<&'static mut FrameBuffer>,
    physical_memory_offset: Optional<u64>,
) {
    // Responsiveness lifecycle:
    // - interrupts disabled during descriptor/controller setup
    // - runtime subsystems initialized with deterministic state
    // - interrupts enabled only after scheduler + IPC paths are ready
    //
    // Keeping this boundary explicit lets higher-level tooling reason about
    // "pre-responsive" vs "responsive" kernel phases.
    x86_64::instructions::interrupts::disable();

    init_console(framebuffer);

    gdt::init();
    kprintln!("[kernel] GDT initialized.");

    idt::init();
    kprintln!("[kernel] IDT initialized.");

    interrupts::init();
    kprintln!("[kernel] Interrupts initialized.");

    init_memory_and_heap(memory_regions, physical_memory_offset);
    init_runtime_subsystems();

    x86_64::instructions::interrupts::enable();
    kprintln!("[kernel] Interrupts enabled.");
    kprintln!("[kernel] Nexus Core v0.3 READY.");
}

fn init_console(framebuffer: Option<&'static mut FrameBuffer>) {
    drivers::serial::init();
    drivers::vga_text::init();
    kprintln!("[kernel] Console: serial + VGA text initialized.");

    if let Some(framebuffer) = framebuffer {
        drivers::framebuffer::init(framebuffer);
        kprintln!("[kernel] Console: framebuffer initialized.");
    } else {
        kprintln!("[kernel] Console: framebuffer unavailable; using text fallback.");
    }
}

fn init_memory_and_heap(
    memory_regions: &'static MemoryRegions,
    physical_memory_offset: Optional<u64>,
) {
    memory::init(memory_regions);
    let offset = physical_memory_offset
        .into_option()
        .expect("[kernel] heap mapping failed: physical_memory_offset is unavailable");
    arch::x86_64::paging::configure_physical_memory_offset(offset);
    memory::init_virtual_memory_bootstrap();
    kprintln!("[kernel] Memory manager initialized.");

    let heap_result = memory::with_frame_allocator(|frame_allocator| {
        let mut mapper = unsafe { arch::x86_64::paging::init_mapper(x86_64::VirtAddr::new(offset)) };
        arch::x86_64::paging::map_heap_region(
            &mut mapper,
            frame_allocator,
            x86_64::VirtAddr::new(heap::HEAP_MAPPED_START),
            heap::HEAP_SIZE,
        )
    });

    match heap_result {
        Some(Ok(())) => {
            // Ordering invariant:
            // 1) Boot memory map validated and frame allocator initialized.
            // 2) Active mapper built from confirmed direct-map offset.
            // 3) Heap pages mapped.
            // Only after these steps do we expose dynamic page allocation.
            memory::finalize_allocator_init();
            heap::init_heap();
            kprintln!("[kernel] Heap initialized.");
        }
        Some(Err(error)) => {
            panic!("[kernel] heap mapping failed: {}", error);
        }
        None => {
            panic!("[kernel] heap mapping failed: frame allocator unavailable");
        }
    }
}

fn init_runtime_subsystems() {
    task::init();
    kprintln!("[kernel] Task scheduler initialized.");

    if task::bootstrap_first_dynamic_task() {
        kprintln!("[kernel] Task subsystem: first dynamic task registered.");
    }

    ipc::init();
    kprintln!("[kernel] IPC system initialized.");

    aetherfs::init();
    kprintln!("[kernel] AetherFS initialized.");

    caps::init();
    kprintln!("[kernel] Capability system initialized.");
    runtime::init();
    kprintln!("[kernel] Runtime services initialized.");

    dev_interface::init();
    kprintln!("[kernel] Developer interface bridge initialized.");

    network::init();
    kprintln!("[kernel] Network stack initialized.");

    device::init();
    device::boot_discover_devices();
    kprintln!("[kernel] Device manager initialized.");

    syscall::init();
    kprintln!("[kernel] Syscall interface initialized.");
}

#[cfg(target_os = "none")]
#[alloc_error_handler]
fn alloc_error_handler(_layout: alloc::alloc::Layout) -> ! {
    loop {}
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
