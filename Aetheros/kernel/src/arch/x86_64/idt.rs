// kernel/src/arch/x86_64/idt.rs

#![allow(dead_code)]

use crate::{
    arch::x86_64::{gdt, irq},
    hlt_loop, kprintln, timer,
};
use x86_64::{
    registers::control::Cr2,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    #[inline]
    pub const fn as_usize(self) -> usize {
        self.as_u8() as usize
    }
}

static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

pub fn init() {
    unsafe {
        kprintln!("[kernel] idt: Initializing IDT...");

        IDT.divide_error.set_handler_fn(divide_error_handler);
        IDT.debug.set_handler_fn(debug_handler);
        IDT.non_maskable_interrupt
            .set_handler_fn(non_maskable_interrupt_handler);
        IDT.breakpoint.set_handler_fn(breakpoint_handler);
        IDT.overflow.set_handler_fn(overflow_handler);
        IDT.bound_range_exceeded
            .set_handler_fn(bound_range_exceeded_handler);
        IDT.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        IDT.device_not_available
            .set_handler_fn(device_not_available_handler);
        IDT.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        IDT.invalid_tss.set_handler_fn(invalid_tss_handler);
        IDT.segment_not_present
            .set_handler_fn(segment_not_present_handler);
        IDT.stack_segment_fault
            .set_handler_fn(stack_segment_fault_handler);
        IDT.general_protection_fault
            .set_handler_fn(general_protection_fault_handler);
        IDT.page_fault.set_handler_fn(page_fault_handler);
        IDT.x87_floating_point
            .set_handler_fn(x87_floating_point_handler);
        IDT.alignment_check.set_handler_fn(alignment_check_handler);
        IDT.machine_check.set_handler_fn(machine_check_handler);
        IDT.simd_floating_point
            .set_handler_fn(simd_floating_point_handler);
        IDT.virtualization.set_handler_fn(virtualization_handler);
        IDT.security_exception
            .set_handler_fn(security_exception_handler);

        IDT[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        IDT[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);

        IDT.load();
        kprintln!("[kernel] idt: IDT loaded.");
    }
}

macro_rules! exception_handler {
    ($name:ident, $msg:literal) => {
        extern "x86-interrupt" fn $name(stack_frame: InterruptStackFrame) {
            kprintln!("[kernel] EXCEPTION: {}\n{:#?}", $msg, stack_frame);
            hlt_loop();
        }
    };
}

macro_rules! exception_handler_with_error {
    ($name:ident, $msg:literal) => {
        extern "x86-interrupt" fn $name(stack_frame: InterruptStackFrame, error_code: u64) {
            kprintln!(
                "[kernel] EXCEPTION: {}\nError Code: {}\n{:#?}",
                $msg,
                error_code,
                stack_frame
            );
            hlt_loop();
        }
    };
}

exception_handler!(divide_error_handler, "DIVIDE ERROR");
exception_handler!(debug_handler, "DEBUG");
exception_handler!(non_maskable_interrupt_handler, "NON MASKABLE INTERRUPT");
exception_handler!(breakpoint_handler, "BREAKPOINT");
exception_handler!(overflow_handler, "OVERFLOW");
exception_handler!(bound_range_exceeded_handler, "BOUND RANGE EXCEEDED");
exception_handler!(invalid_opcode_handler, "INVALID OPCODE");
exception_handler!(device_not_available_handler, "DEVICE NOT AVAILABLE");
exception_handler_with_error!(invalid_tss_handler, "INVALID TSS");
exception_handler_with_error!(segment_not_present_handler, "SEGMENT NOT PRESENT");
exception_handler_with_error!(stack_segment_fault_handler, "STACK SEGMENT FAULT");
exception_handler_with_error!(general_protection_fault_handler, "GENERAL PROTECTION FAULT");
exception_handler!(x87_floating_point_handler, "X87 FLOATING POINT");
exception_handler_with_error!(alignment_check_handler, "ALIGNMENT CHECK");
exception_handler!(simd_floating_point_handler, "SIMD FLOATING POINT");
exception_handler!(virtualization_handler, "VIRTUALIZATION");
exception_handler_with_error!(security_exception_handler, "SECURITY EXCEPTION");

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    kprintln!(
        "[kernel] EXCEPTION: PAGE FAULT\nAccessed Address: {:?}\nError Code: {:?}\n{:#?}",
        Cr2::read(),
        error_code,
        stack_frame
    );
    hlt_loop();
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
    hlt_loop();
}

extern "x86-interrupt" fn machine_check_handler(stack_frame: InterruptStackFrame) -> ! {
    kprintln!("[kernel] EXCEPTION: MACHINE CHECK\n{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    timer::tick();
    irq::handle_irq(0);
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    irq::handle_irq(1);
}
