// kernel/src/arch/x86_64/idt.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use crate::kprintln;

/// Static mutable Interrupt Descriptor Table.
/// It will be initialized once during boot.
static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

/// Initializes the IDT by setting up handlers for common exceptions.
/// Loads the IDT into the CPU.
pub fn init() {
    // SAFETY: We are initializing a static mutable IDT once during boot.
    // We are setting interrupt handlers, which is unsafe but necessary for OS development.
    unsafe {
        kprintln!("[kernel] idt: Initializing IDT...");

        // Set handlers for some common exceptions
        IDT.breakpoint_handler.set_handler_fn(breakpoint_handler);
        IDT.double_fault_handler.set_handler_fn(double_fault_handler);

        // Load the IDT into the CPU
        IDT.load();
        kprintln!("[kernel] idt: IDT loaded.");
    }
}

/// Handler for the breakpoint exception.
/// This is a simple handler that just logs the event.
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    kprintln!("[kernel] EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
    // For now, we halt after a breakpoint. In a debugger, this would return.
    loop {}
}

/// Handler for the double fault exception.
/// This is a critical error, so we log and then halt.
extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) -> ! {
    kprintln!("[kernel] EXCEPTION: DOUBLE FAULT\nError Code: {}\n{:#?}", error_code, stack_frame);
    // Double fault is unrecoverable, so we halt the system.
    loop {}
}


