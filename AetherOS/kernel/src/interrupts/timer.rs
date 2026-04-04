//! Timer IRQ handler.

use x86_64::structures::idt::InterruptStackFrame;

use crate::interrupts::{pic, IRQ_TIMER};

pub extern "x86-interrupt" fn handler(_stack_frame: InterruptStackFrame) {
    crate::timer::tick();
    if let Some(Some(handler)) = crate::device::with_manager(|m| m.irq_handler(IRQ_TIMER)) {
        handler.handle_irq();
    }
    // Do not invoke the scheduler directly from interrupt context. The idle
    // loop consumes this flag at a safe boundary and performs the switch.
    crate::task::scheduler::request_reschedule_from_irq();

    unsafe {
        // SAFETY: We are running in the timer IRQ context, so acknowledging
        // the corresponding PIC line is required to re-enable future timer
        // interrupts.
        pic::end_of_interrupt(IRQ_TIMER);
    }
}
