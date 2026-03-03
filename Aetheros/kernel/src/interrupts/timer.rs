//! Timer IRQ handler.

use x86_64::structures::idt::InterruptStackFrame;

use crate::interrupts::pic;

pub extern "x86-interrupt" fn handler(_stack_frame: InterruptStackFrame) {
    crate::timer::tick();

    unsafe {
        pic::notify_end_of_interrupt(pic::IRQ_TIMER);
    }
}
