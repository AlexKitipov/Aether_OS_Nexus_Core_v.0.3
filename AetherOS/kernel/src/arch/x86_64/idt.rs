// kernel/src/arch/x86_64/idt.rs

#![allow(dead_code)]

use x86_64::registers::control::Cr2;
use x86_64::PrivilegeLevel;
use x86_64::instructions::interrupts;
use x86_64::structures::idt::{
    InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode,
};

use crate::{arch::x86_64::gdt, hlt_loop, kprintln};

/// Static Interrupt Descriptor Table, initialized during early boot.
static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

/// Initializes the IDT with core CPU exception handlers and loads it via `lidt`.
pub fn init() {
    interrupts::without_interrupts(|| unsafe {
        // Early boot runs on a single core before task scheduling starts.
        // We fully populate CPU exception entries before `lidt` so no interrupt
        // can observe partially initialized descriptors.
        kprintln!("[kernel] idt: Initializing IDT...");

        let idt = &mut *core::ptr::addr_of_mut!(IDT);

        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.general_protection_fault
            .set_handler_fn(general_protection_fault_handler);
        // SAFETY: `DOUBLE_FAULT_IST_INDEX` points to an IST slot initialized by
        // `gdt::init()` before IDT setup, so loading this stack index is valid.
        idt.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        idt[0x80]
            .set_handler_fn(syscall_interrupt_handler)
            .set_privilege_level(PrivilegeLevel::Ring3);
        // Safety invariant: `lidt` is executed only after all required entries
        // are configured, and while interrupts are disabled.
        idt.load();
        kprintln!("[kernel] idt: IDT loaded.");
    })
}

/// Registers an external IRQ handler into the IDT at a given vector.
pub fn set_irq_handler(vector: u8, handler: extern "x86-interrupt" fn(InterruptStackFrame)) {
    interrupts::without_interrupts(|| unsafe {
        // We only mutate the table with interrupts disabled to avoid observing
        // partially written entries.
        let idt = &mut *core::ptr::addr_of_mut!(IDT);
        idt[vector as usize].set_handler_fn(handler);
    });
}

/// Reloads IDT register from the global table after descriptor updates.
pub fn reload() {
    interrupts::without_interrupts(|| unsafe {
        // The descriptor table has `'static` lifetime and can be safely reloaded.
        let idt = &*core::ptr::addr_of!(IDT);
        idt.load();
    });
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
    let caused_by_user_mode = error_code.contains(PageFaultErrorCode::USER_MODE);
    let is_protection_violation = error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION);
    let is_write = error_code.contains(PageFaultErrorCode::CAUSED_BY_WRITE);
    let is_instruction_fetch = error_code.contains(PageFaultErrorCode::INSTRUCTION_FETCH);

    kprintln!("[kernel] EXCEPTION: PAGE FAULT");
    kprintln!("[kernel] Accessed Address: {:?}", accessed_address);
    kprintln!("[kernel] Error Code: {:?}", error_code);
    kprintln!(
        "[kernel] page fault details: mode={}, access={}, reason={}, fetch={}",
        if caused_by_user_mode { "user" } else { "kernel" },
        if is_write { "write" } else { "read" },
        if is_protection_violation {
            "protection violation"
        } else {
            "non-present page"
        },
        if is_instruction_fetch { "yes" } else { "no" }
    );
    kprintln!("[kernel] Stack Frame:\n{:#?}", stack_frame);

    let in_user_space_range = accessed_address_raw >= crate::config::USER_SPACE_START
        && accessed_address_raw < crate::config::USER_SPACE_END_EXCLUSIVE;
    let current_task_id = crate::task::scheduler::get_current_task_id();

    if caused_by_user_mode && in_user_space_range && current_task_id != 0
    {
        kprintln!(
            "[kernel] page fault: terminating task {} due to invalid userspace memory access.",
            current_task_id
        );
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
    // NOTE: x86_64 v0.14 currently expects the double-fault handler type to be
    // `extern "x86-interrupt" fn(...) -> !`.
    hlt_loop();
}

extern "x86-interrupt" fn syscall_interrupt_handler(stack_frame: InterruptStackFrame) {
    crate::kprintln!("[kernel] syscall: software interrupt entry (int 0x80) rip={:#x}.", stack_frame.instruction_pointer.as_u64());
}
