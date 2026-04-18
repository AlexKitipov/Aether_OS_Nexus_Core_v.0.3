// kernel/src/arch/x86_64/idt.rs

#![allow(dead_code)]

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

use crate::arch::x86_64::gdt;
use crate::kprintln;

/// Static mutable Interrupt Descriptor Table.
/// It will be initialized once during boot.
static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

/// Initializes the IDT by setting up handlers for common exceptions.
/// Loads the IDT into the CPU.
pub fn init() {
    // SAFETY: We initialize and load the single global IDT during early boot.
    unsafe {
        kprintln!("[kernel] idt: Initializing IDT...");

        IDT.breakpoint_handler.set_handler_fn(breakpoint_handler);
        IDT.double_fault_handler
            .set_handler_fn(double_fault_handler)
            .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);

        IDT.load();
        kprintln!("[kernel] idt: IDT loaded.");
    }
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    kprintln!("[kernel] EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
    loop {}
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    kprintln!(
        "[kernel] EXCEPTION: DOUBLE FAULT\nError Code: {}\n{:#?}",
        error_code,
        stack_frame
    );
    loop {}
}
