//! Timer IRQ handler.

use x86_64::structures::idt::InterruptStackFrame;

use crate::interrupts::{pic, IRQ_TIMER};

pub extern "x86-interrupt" fn handler(_stack_frame: InterruptStackFrame) {
    crate::timer::tick();

    unsafe {
        pic::end_of_interrupt(IRQ_TIMER);
    }
}
