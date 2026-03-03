// kernel/src/arch/x86_64/idt.rs

#![allow(dead_code)]

use x86_64::registers::control::Cr2;
use x86_64::structures::idt::{
    HandlerFunc, InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode,
};

use crate::{arch::x86_64::gdt, hlt_loop, kprintln};

/// Static Interrupt Descriptor Table, initialized during early boot.
static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

/// Initializes the IDT with core CPU exception handlers and loads it via `lidt`.
pub fn init() {
    unsafe {
        kprintln!("[kernel] idt: Initializing IDT...");

        IDT.breakpoint.set_handler_fn(breakpoint_handler);
        IDT.page_fault.set_handler_fn(page_fault_handler);
        IDT.general_protection_fault
            .set_handler_fn(general_protection_fault_handler);
        IDT.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);

        IDT.load();
        kprintln!("[kernel] idt: IDT loaded.");
    }
}

/// Registers an external IRQ handler into the IDT at a given vector.
pub fn set_irq_handler(vector: u8, handler: HandlerFunc) {
    unsafe {
        IDT[vector as usize].set_handler_fn(handler);
    }
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    kprintln!("[kernel] EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    let accessed_address = Cr2::read();
    let accessed_address_raw = accessed_address.as_u64() as usize;
    kprintln!("[kernel] EXCEPTION: PAGE FAULT");
    kprintln!("[kernel] Accessed Address: {:?}", accessed_address);
    kprintln!("[kernel] Error Code: {:?}", error_code);
    kprintln!("[kernel] Stack Frame:\n{:#?}", stack_frame);

    if accessed_address_raw >= crate::config::USER_SPACE_START
        && accessed_address_raw < crate::config::USER_SPACE_END_EXCLUSIVE
    {
        kprintln!("[kernel] page fault: invalid userspace pointer detected; terminating task.");
        crate::task::scheduler::terminate_current_task();
        return;
    }

    hlt_loop();
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    kprintln!("[kernel] EXCEPTION: GENERAL PROTECTION FAULT");
    kprintln!("[kernel] Error Code: {}", error_code);
    kprintln!("[kernel] Stack Frame:\n{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    kprintln!("[kernel] EXCEPTION: DOUBLE FAULT");
    kprintln!("[kernel] Error Code: {}", error_code);
    kprintln!("[kernel] Stack Frame:\n{:#?}", stack_frame);
    hlt_loop();
}
